use std::collections::HashMap;
use std::io::{BufRead, BufReader, Cursor, Read, Seek, Write};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{
    Extraction, OutputWarning, XlsxCell, XlsxCellValue, XlsxCsvOptions, XlsxReadOptions, XlsxRow,
    XlsxRowControl, XlsxSheet, XlsxSheetOptions, XlsxSheetVisibility, XlsxValueMode,
};
use crate::parsers::xlsx_shared_strings::{
    DEFAULT_SHARED_STRING_MEMORY_LIMIT, SharedStringLookup, SharedStringStore,
};
use crate::parsers::{
    append_decoded_xml_reference, append_decoded_xml_text, attr_value, merge_warnings, name_eq,
    parent_dir, parse_relationship_map, rels_path_for, resolve_relationship_target,
};
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkbookSheet {
    name: String,
    relation_id: String,
    visibility: XlsxSheetVisibility,
}

#[derive(Debug, Default)]
struct CellState {
    value_type: Option<String>,
    column_index: Option<usize>,
    style_index: Option<usize>,
    value: String,
    has_formula: bool,
    in_value: bool,
    in_inline_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRow {
    row: XlsxRow,
    next_column: usize,
}

impl ParsedRow {
    fn new(index: usize) -> Self {
        Self {
            row: XlsxRow {
                row_index: index,
                cells: Vec::new(),
            },
            next_column: 0,
        }
    }

    fn next_column(&self) -> usize {
        self.next_column
    }

    fn set(&mut self, cell: XlsxCell) {
        self.next_column = self.next_column.max(cell.column_index.saturating_add(1));
        match self
            .row
            .cells
            .binary_search_by_key(&cell.column_index, |cell| cell.column_index)
        {
            Ok(index) => self.row.cells[index] = cell,
            Err(index) => self.row.cells.insert(index, cell),
        }
    }
}

impl XlsxCellValue {
    fn csv_value(&self) -> &str {
        match self {
            Self::Blank => "",
            Self::String { value, .. } | Self::Error { raw: value } => value,
            Self::Boolean {
                value: Some(true), ..
            } => "TRUE",
            Self::Boolean {
                value: Some(false), ..
            } => "FALSE",
            Self::Boolean {
                raw, value: None, ..
            } => raw,
            Self::Number {
                formatted: Some(value),
                ..
            } => value,
            Self::Number {
                raw,
                formatted: None,
            } => raw,
        }
    }
}

trait SheetRowSink {
    fn emit(&mut self, row: &XlsxRow) -> Result<XlsxRowControl>;
}

struct CsvRowSink<'a, W> {
    writer: &'a mut W,
    delimiter: u8,
}

impl<W: Write> SheetRowSink for CsvRowSink<'_, W> {
    fn emit(&mut self, row: &XlsxRow) -> Result<XlsxRowControl> {
        write_csv_row(self.writer, row, self.delimiter)?;
        Ok(XlsxRowControl::Continue)
    }
}

struct CallbackRowSink<F> {
    visitor: F,
}

impl<F> SheetRowSink for CallbackRowSink<F>
where
    F: FnMut(&XlsxRow) -> Result<XlsxRowControl>,
{
    fn emit(&mut self, row: &XlsxRow) -> Result<XlsxRowControl> {
        (self.visitor)(row)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateSystem {
    Excel1900,
    Excel1904,
}

#[derive(Debug, Default)]
struct XlsxStyles {
    cell_formats: Vec<CellFormat>,
}

#[derive(Debug, Clone, Default)]
struct CellFormat {
    number_format: Option<String>,
}

struct SheetFormatContext<'a> {
    value_mode: XlsxValueMode,
    date_system: DateSystem,
    styles: &'a XlsxStyles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatKind {
    Date,
    Time,
    DateTime,
    Percent(usize),
    Currency(usize),
    Decimal(usize),
}

pub(crate) fn write_csv<R: Read + Seek, W: Write>(
    package: &mut OoxmlPackage<R>,
    options: XlsxCsvOptions<'_>,
    value_mode: XlsxValueMode,
    mut writer: W,
) -> Result<Extraction<()>> {
    write_csv_with_shared_string_memory_limit(
        package,
        options,
        value_mode,
        DEFAULT_SHARED_STRING_MEMORY_LIMIT,
        &mut writer,
    )
}

pub(crate) fn list_sheets<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    include_hidden: bool,
) -> Result<Extraction<Vec<XlsxSheet>>> {
    let workbook_path = crate::parsers::find_office_document_path(package, "xl/workbook.xml")?;
    let workbook_xml = package.read_to_string(&workbook_path)?;
    let workbook = parse_workbook_sheets(&workbook_xml, &workbook_path)?;
    let sheets = workbook
        .value
        .into_iter()
        .filter(|sheet| include_hidden || sheet.visibility == XlsxSheetVisibility::Visible)
        .enumerate()
        .map(|(index, sheet)| XlsxSheet {
            index: index + 1,
            name: sheet.name,
            visibility: sheet.visibility,
        })
        .collect();

    Ok(Extraction::with_warnings(sheets, workbook.warnings))
}

pub(crate) fn visit_rows<R, F>(
    package: &mut OoxmlPackage<R>,
    options: XlsxSheetOptions<'_>,
    value_mode: XlsxValueMode,
    visitor: F,
) -> Result<Extraction<()>>
where
    R: Read + Seek,
    F: FnMut(&XlsxRow) -> Result<XlsxRowControl>,
{
    visit_rows_with_read_options(package, XlsxReadOptions::new(options), value_mode, visitor)
}

pub(crate) fn visit_rows_with_read_options<R, F>(
    package: &mut OoxmlPackage<R>,
    options: XlsxReadOptions<'_>,
    value_mode: XlsxValueMode,
    visitor: F,
) -> Result<Extraction<()>>
where
    R: Read + Seek,
    F: FnMut(&XlsxRow) -> Result<XlsxRowControl>,
{
    let workbook_path = crate::parsers::find_office_document_path(package, "xl/workbook.xml")?;
    let workbook_xml = package.read_to_string(&workbook_path)?;
    let workbook = parse_workbook_sheets(&workbook_xml, &workbook_path)?;
    let date_system = parse_workbook_date_system(&workbook_xml);

    let workbook_rels_path = rels_path_for(&workbook_path);
    let workbook_rels_xml = package.read_to_string(&workbook_rels_path)?;
    let workbook_rels = parse_relationship_map(&workbook_rels_xml, &workbook_rels_path)?;

    let selected_sheet = select_sheet(
        &workbook.value,
        options.sheet.sheet_name,
        options.sheet.sheet_index,
        options.sheet.include_hidden,
    )?;
    let target = workbook_rels
        .get(&selected_sheet.relation_id)
        .ok_or_else(|| OxdocError::MissingPart(selected_sheet.relation_id.clone()))?;
    let sheet_path =
        resolve_relationship_target(parent_dir(&workbook_path), target, &workbook_rels_path)?;

    let mut shared_strings = if package.contains("xl/sharedStrings.xml") {
        package.with_entry("xl/sharedStrings.xml", |entry| {
            SharedStringStore::parse_with_memory_limit(
                BufReader::new(entry),
                "xl/sharedStrings.xml",
                DEFAULT_SHARED_STRING_MEMORY_LIMIT,
            )
        })?
    } else {
        Extraction::new(SharedStringStore::empty())
    };

    let styles = if value_mode == XlsxValueMode::Formatted && package.contains("xl/styles.xml") {
        parse_styles(&package.read_to_string("xl/styles.xml")?, "xl/styles.xml")
    } else {
        Extraction::new(XlsxStyles::default())
    };

    let read_sheet = |entry: &mut dyn Read| {
        let format_context = SheetFormatContext {
            value_mode,
            date_system,
            styles: &styles.value,
        };
        let mut sink = CallbackRowSink { visitor };
        parse_sheet_rows(
            BufReader::new(entry),
            &sheet_path,
            &mut shared_strings.value,
            &format_context,
            &mut sink,
        )
    };
    let sheet = if let Some(limits) = options.worksheet_limits {
        package.with_entry_limits(&sheet_path, limits, read_sheet)
    } else {
        package.with_entry(&sheet_path, read_sheet)
    }?;

    Ok(Extraction::with_warnings(
        (),
        merge_warnings(
            merge_warnings(
                merge_warnings(workbook.warnings, shared_strings.warnings),
                styles.warnings,
            ),
            sheet.warnings,
        ),
    ))
}

fn write_csv_with_shared_string_memory_limit<R: Read + Seek, W: Write>(
    package: &mut OoxmlPackage<R>,
    options: XlsxCsvOptions<'_>,
    value_mode: XlsxValueMode,
    shared_string_memory_limit: usize,
    writer: &mut W,
) -> Result<Extraction<()>> {
    let workbook_path = crate::parsers::find_office_document_path(package, "xl/workbook.xml")?;
    let workbook_xml = package.read_to_string(&workbook_path)?;
    let workbook = parse_workbook_sheets(&workbook_xml, &workbook_path)?;
    let date_system = parse_workbook_date_system(&workbook_xml);

    let workbook_rels_path = rels_path_for(&workbook_path);
    let workbook_rels_xml = package.read_to_string(&workbook_rels_path)?;
    let workbook_rels = parse_relationship_map(&workbook_rels_xml, &workbook_rels_path)?;

    let selected_sheet = select_sheet(
        &workbook.value,
        options.sheet_name,
        options.sheet_index,
        options.include_hidden,
    )?;
    let target = workbook_rels
        .get(&selected_sheet.relation_id)
        .ok_or_else(|| OxdocError::MissingPart(selected_sheet.relation_id.clone()))?;
    let sheet_path =
        resolve_relationship_target(parent_dir(&workbook_path), target, &workbook_rels_path)?;

    let mut shared_strings = if package.contains("xl/sharedStrings.xml") {
        package.with_entry("xl/sharedStrings.xml", |entry| {
            let reader = BufReader::new(entry);
            SharedStringStore::parse_with_memory_limit(
                reader,
                "xl/sharedStrings.xml",
                shared_string_memory_limit,
            )
        })?
    } else {
        Extraction::new(SharedStringStore::empty())
    };

    let styles = if value_mode == XlsxValueMode::Formatted && package.contains("xl/styles.xml") {
        let styles_xml = package.read_to_string("xl/styles.xml")?;
        parse_styles(&styles_xml, "xl/styles.xml")
    } else {
        Extraction::new(XlsxStyles::default())
    };

    let sheet = package.with_entry(&sheet_path, |entry| {
        let reader = BufReader::new(entry);
        let format_context = SheetFormatContext {
            value_mode,
            date_system,
            styles: &styles.value,
        };
        write_sheet_csv(
            reader,
            &sheet_path,
            &mut shared_strings.value,
            options.delimiter,
            &format_context,
            writer,
        )
    })?;

    Ok(Extraction::with_warnings(
        (),
        merge_warnings(
            merge_warnings(
                merge_warnings(workbook.warnings, shared_strings.warnings),
                styles.warnings,
            ),
            sheet.warnings,
        ),
    ))
}

fn select_sheet<'a>(
    sheets: &'a [WorkbookSheet],
    sheet_name: Option<&str>,
    sheet_index: Option<usize>,
    include_hidden: bool,
) -> Result<&'a WorkbookSheet> {
    if sheet_name.is_some() && sheet_index.is_some() {
        return Err(OxdocError::InvalidArgument(
            "select an XLSX sheet by name or index, not both".to_owned(),
        ));
    }

    if let Some(sheet_name) = sheet_name {
        let mut matches = sheets.iter().filter(|sheet| {
            (include_hidden || sheet.visibility == XlsxSheetVisibility::Visible)
                && sheet.name == sheet_name
        });
        let selected = matches
            .next()
            .ok_or_else(|| OxdocError::MissingPart(sheet_name_error(sheet_name, include_hidden)))?;

        if matches.next().is_some() {
            let scope = if include_hidden {
                "workbook"
            } else {
                "visible"
            };
            return Err(OxdocError::InvalidArgument(format!(
                "multiple {scope} sheets named {sheet_name}; use sheet index to disambiguate"
            )));
        }

        return Ok(selected);
    }

    if let Some(sheet_index) = sheet_index {
        if sheet_index == 0 {
            return Err(OxdocError::InvalidArgument(
                "sheet index must be 1 or greater".to_owned(),
            ));
        }

        return sheets
            .iter()
            .filter(|sheet| include_hidden || sheet.visibility == XlsxSheetVisibility::Visible)
            .nth(sheet_index - 1)
            .ok_or_else(|| {
                OxdocError::MissingPart(sheet_index_error(sheet_index, include_hidden))
            });
    }

    sheets
        .iter()
        .find(|sheet| include_hidden || sheet.visibility == XlsxSheetVisibility::Visible)
        .ok_or_else(|| OxdocError::MissingPart(sheet_scope_error(include_hidden)))
}

fn sheet_name_error(sheet_name: &str, include_hidden: bool) -> String {
    if include_hidden {
        format!("workbook sheet named {sheet_name}")
    } else {
        format!("visible sheet named {sheet_name}")
    }
}

fn sheet_index_error(sheet_index: usize, include_hidden: bool) -> String {
    if include_hidden {
        format!("workbook sheet index {sheet_index}")
    } else {
        format!("visible sheet index {sheet_index}")
    }
}

fn sheet_scope_error(include_hidden: bool) -> String {
    if include_hidden {
        "workbook sheets".to_owned()
    } else {
        "visible workbook sheets".to_owned()
    }
}

fn parse_workbook_sheets(xml: &str, path: &str) -> Result<Extraction<Vec<WorkbookSheet>>> {
    let mut reader = Reader::from_reader(std::io::Cursor::new(xml.as_bytes()));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut sheets = Vec::new();
    let mut warnings = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if name_eq(element.name().as_ref(), b"sheet") =>
            {
                match (attr_value(&element, b"name"), attr_value(&element, b"id")) {
                    (Some(name), Some(relation_id)) => sheets.push(WorkbookSheet {
                        name,
                        relation_id,
                        visibility: sheet_visibility(attr_value(&element, b"state")),
                    }),
                    _ => warnings.push(OutputWarning::ignored_workbook_sheet(path)),
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::malformed_xml(path, source));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings(sheets, warnings))
}

fn parse_workbook_date_system(xml: &str) -> DateSystem {
    let mut reader = Reader::from_reader(std::io::Cursor::new(xml.as_bytes()));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if name_eq(element.name().as_ref(), b"workbookPr") =>
            {
                return match attr_value(&element, b"date1904").as_deref() {
                    Some("1" | "true" | "TRUE") => DateSystem::Excel1904,
                    _ => DateSystem::Excel1900,
                };
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    DateSystem::Excel1900
}

fn parse_styles(xml: &str, path: &str) -> Extraction<XlsxStyles> {
    let mut reader = Reader::from_reader(std::io::Cursor::new(xml.as_bytes()));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut warnings = Vec::new();
    let mut custom_formats = HashMap::new();
    let mut cell_formats = Vec::new();
    let mut in_cell_xfs = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"cellXfs") {
                    in_cell_xfs = true;
                } else if in_cell_xfs && name_eq(element.name().as_ref(), b"xf") {
                    cell_formats.push(cell_format_from_element(&element, &custom_formats));
                } else if name_eq(element.name().as_ref(), b"numFmt") {
                    insert_custom_number_format(&element, &mut custom_formats);
                }
            }
            Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"numFmt") {
                    insert_custom_number_format(&element, &mut custom_formats);
                } else if in_cell_xfs && name_eq(element.name().as_ref(), b"xf") {
                    cell_formats.push(cell_format_from_element(&element, &custom_formats));
                }
            }
            Ok(Event::End(element)) if name_eq(element.name().as_ref(), b"cellXfs") => {
                in_cell_xfs = false;
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::malformed_xml(path, source));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Extraction::with_warnings(XlsxStyles { cell_formats }, warnings)
}

fn insert_custom_number_format(
    element: &quick_xml::events::BytesStart<'_>,
    custom_formats: &mut HashMap<u32, String>,
) {
    if let (Some(id), Some(code)) = (
        attr_value(element, b"numFmtId").and_then(|value| value.parse::<u32>().ok()),
        attr_value(element, b"formatCode"),
    ) {
        custom_formats.insert(id, code);
    }
}

fn cell_format_from_element(
    element: &quick_xml::events::BytesStart<'_>,
    custom_formats: &HashMap<u32, String>,
) -> CellFormat {
    let number_format = attr_value(element, b"numFmtId")
        .and_then(|value| value.parse::<u32>().ok())
        .and_then(|id| {
            custom_formats
                .get(&id)
                .cloned()
                .or_else(|| builtin_format(id))
        });

    CellFormat { number_format }
}

fn builtin_format(id: u32) -> Option<String> {
    match id {
        1 => Some("0".to_owned()),
        2 => Some("0.00".to_owned()),
        3 => Some("#,##0".to_owned()),
        4 => Some("#,##0.00".to_owned()),
        5..=8 | 37..=44 => Some("$#,##0.00".to_owned()),
        9 => Some("0%".to_owned()),
        10 => Some("0.00%".to_owned()),
        11 => Some("0.00E+00".to_owned()),
        12 => Some("# ?/?".to_owned()),
        13 => Some("# ??/??".to_owned()),
        14..=17 => Some("m/d/yyyy".to_owned()),
        18..=20 => Some("h:mm".to_owned()),
        21 | 22 => Some("m/d/yyyy h:mm".to_owned()),
        45..=47 => Some("h:mm:ss".to_owned()),
        _ => None,
    }
}

fn sheet_visibility(state: Option<String>) -> XlsxSheetVisibility {
    match state.as_deref() {
        Some(state) if state.eq_ignore_ascii_case("hidden") => XlsxSheetVisibility::Hidden,
        Some(state) if state.eq_ignore_ascii_case("veryHidden") => XlsxSheetVisibility::VeryHidden,
        _ => XlsxSheetVisibility::Visible,
    }
}

fn write_sheet_csv<R: BufRead, W: Write>(
    source: R,
    path: &str,
    shared_strings: &mut impl SharedStringLookup,
    delimiter: u8,
    format_context: &SheetFormatContext<'_>,
    writer: &mut W,
) -> Result<Extraction<()>> {
    let mut sink = CsvRowSink { writer, delimiter };
    parse_sheet_rows(source, path, shared_strings, format_context, &mut sink)
}

fn parse_sheet_rows<R: BufRead>(
    source: R,
    path: &str,
    shared_strings: &mut impl SharedStringLookup,
    format_context: &SheetFormatContext<'_>,
    sink: &mut impl SheetRowSink,
) -> Result<Extraction<()>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut warnings = Vec::new();
    let mut row: Option<ParsedRow> = None;
    let mut current_cell: Option<CellState> = None;
    let mut next_row_index = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"row") {
                    let row_index = row_index_from_element(&element, next_row_index);
                    next_row_index = row_index.saturating_add(1);
                    row = Some(ParsedRow::new(row_index));
                } else if name_eq(element.name().as_ref(), b"c") {
                    let fallback_column = row.as_ref().map_or(0, ParsedRow::next_column);
                    current_cell = Some(cell_state_from_element(&element, fallback_column));
                } else if let Some(cell) = &mut current_cell {
                    if name_eq(element.name().as_ref(), b"v") {
                        cell.in_value = true;
                    } else if name_eq(element.name().as_ref(), b"t") {
                        cell.in_inline_text = true;
                    } else if name_eq(element.name().as_ref(), b"f") {
                        cell.has_formula = true;
                    }
                }
            }
            Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"row") {
                    let row_index = row_index_from_element(&element, next_row_index);
                    next_row_index = row_index.saturating_add(1);
                    if sink.emit(&ParsedRow::new(row_index).row)? == XlsxRowControl::Stop {
                        break;
                    }
                    row = None;
                } else if name_eq(element.name().as_ref(), b"c")
                    && let Some(row) = &mut row
                {
                    let cell = cell_state_from_element(&element, row.next_column());
                    push_typed_cell(
                        row,
                        cell,
                        shared_strings,
                        path,
                        format_context,
                        &mut warnings,
                    )?;
                } else if name_eq(element.name().as_ref(), b"f")
                    && let Some(cell) = &mut current_cell
                {
                    cell.has_formula = true;
                }
            }
            Ok(Event::Text(value)) => {
                if let Some(cell) = &mut current_cell
                    && (cell.in_value || cell.in_inline_text)
                {
                    append_decoded_xml_text(value.as_ref(), &mut cell.value);
                }
            }
            Ok(Event::CData(value)) => {
                if let Some(cell) = &mut current_cell
                    && (cell.in_value || cell.in_inline_text)
                {
                    append_decoded_xml_text(value.as_ref(), &mut cell.value);
                }
            }
            Ok(Event::GeneralRef(value)) => {
                if let Some(cell) = &mut current_cell
                    && (cell.in_value || cell.in_inline_text)
                {
                    append_decoded_xml_reference(value.as_ref(), &mut cell.value);
                }
            }
            Ok(Event::End(element)) => {
                if let Some(cell) = &mut current_cell {
                    if name_eq(element.name().as_ref(), b"v") {
                        cell.in_value = false;
                    } else if name_eq(element.name().as_ref(), b"t") {
                        cell.in_inline_text = false;
                    }
                }

                if name_eq(element.name().as_ref(), b"c") {
                    if let (Some(row), Some(cell)) = (&mut row, current_cell.take()) {
                        push_typed_cell(
                            row,
                            cell,
                            shared_strings,
                            path,
                            format_context,
                            &mut warnings,
                        )?;
                    }
                } else if name_eq(element.name().as_ref(), b"row")
                    && let Some(row) = row.take()
                    && sink.emit(&row.row)? == XlsxRowControl::Stop
                {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::malformed_xml(path, source));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings((), warnings))
}

#[doc(hidden)]
pub fn fuzz_parse_shared_strings(xml: &[u8]) -> Result<()> {
    let _ = SharedStringStore::parse(Cursor::new(xml), "xl/sharedStrings.xml")?;
    Ok(())
}

#[doc(hidden)]
pub fn fuzz_parse_sheet(xml: &[u8]) -> Result<()> {
    let mut output = Vec::new();
    let mut sink = CsvRowSink {
        writer: &mut output,
        delimiter: b',',
    };
    let mut shared_strings = SharedStringStore::empty();
    let styles = XlsxStyles::default();
    let format_context = SheetFormatContext {
        value_mode: XlsxValueMode::Raw,
        date_system: DateSystem::Excel1900,
        styles: &styles,
    };
    let _ = parse_sheet_rows(
        Cursor::new(xml),
        "xl/worksheets/sheet1.xml",
        &mut shared_strings,
        &format_context,
        &mut sink,
    )?;
    Ok(())
}

fn push_typed_cell(
    row: &mut ParsedRow,
    cell: CellState,
    shared_strings: &mut impl SharedStringLookup,
    path: &str,
    format_context: &SheetFormatContext<'_>,
    warnings: &mut Vec<OutputWarning>,
) -> Result<()> {
    let target_column = cell.column_index.unwrap_or(row.next_column());

    let value = if cell.value_type.as_deref() == Some("s") {
        match cell.value.trim().parse::<usize>() {
            Ok(index) => match shared_strings.lookup(index)? {
                Some(value) => XlsxCellValue::String {
                    raw: cell.value.clone(),
                    value,
                },
                None => {
                    warnings.push(OutputWarning::shared_string_index_out_of_bounds(
                        path, index,
                    ));
                    XlsxCellValue::String {
                        raw: cell.value.clone(),
                        value: String::new(),
                    }
                }
            },
            Err(_) => {
                warnings.push(OutputWarning::invalid_shared_string_index(
                    path,
                    cell.value.clone(),
                ));
                XlsxCellValue::String {
                    raw: cell.value.clone(),
                    value: cell.value.clone(),
                }
            }
        }
    } else if cell.value_type.as_deref() == Some("b") {
        let boolean = match cell.value.trim() {
            "1" | "true" | "TRUE" => Some(true),
            "0" | "false" | "FALSE" => Some(false),
            _ => None,
        };
        XlsxCellValue::Boolean {
            raw: cell.value.clone(),
            value: boolean,
        }
    } else if matches!(cell.value_type.as_deref(), Some("str" | "inlineStr")) {
        XlsxCellValue::String {
            raw: cell.value.clone(),
            value: cell.value.clone(),
        }
    } else if cell.value_type.as_deref() == Some("e") {
        XlsxCellValue::Error {
            raw: cell.value.clone(),
        }
    } else if cell.value.is_empty() {
        XlsxCellValue::Blank
    } else {
        let formatted = (format_context.value_mode == XlsxValueMode::Formatted)
            .then(|| format_cell_value(&cell, format_context.styles, format_context.date_system))
            .flatten();
        XlsxCellValue::Number {
            raw: cell.value.clone(),
            formatted,
        }
    };

    row.set(XlsxCell {
        column_index: target_column,
        value,
        has_formula: cell.has_formula,
    });

    Ok(())
}

fn row_index_from_element(
    element: &quick_xml::events::BytesStart<'_>,
    fallback_index: usize,
) -> usize {
    attr_value(element, b"r")
        .and_then(|index| index.parse::<usize>().ok())
        .and_then(|index| index.checked_sub(1))
        .unwrap_or(fallback_index)
}

fn cell_state_from_element(
    element: &quick_xml::events::BytesStart<'_>,
    fallback_column: usize,
) -> CellState {
    CellState {
        value_type: attr_value(element, b"t"),
        column_index: attr_value(element, b"r")
            .and_then(|cell_ref| parse_cell_column(&cell_ref).or(Some(fallback_column))),
        style_index: attr_value(element, b"s").and_then(|style| style.parse::<usize>().ok()),
        ..CellState::default()
    }
}

fn format_cell_value(
    cell: &CellState,
    styles: &XlsxStyles,
    date_system: DateSystem,
) -> Option<String> {
    if matches!(
        cell.value_type.as_deref(),
        Some("s" | "b" | "str" | "inlineStr" | "e")
    ) {
        return None;
    }

    let value = cell.value.trim().parse::<f64>().ok()?;
    let style_index = cell.style_index?;
    let format_code = styles
        .cell_formats
        .get(style_index)?
        .number_format
        .as_deref()?;
    let kind = classify_number_format(format_code)?;

    match kind {
        FormatKind::Date => format_excel_datetime(value, date_system, false, true),
        FormatKind::Time => Some(format_time_fraction(value.fract())),
        FormatKind::DateTime => format_excel_datetime(value, date_system, true, true),
        FormatKind::Percent(decimals) => {
            Some(format!("{}%", format_number(value * 100.0, decimals, true)))
        }
        FormatKind::Currency(decimals) => {
            Some(format!("${}", format_number(value, decimals, true)))
        }
        FormatKind::Decimal(decimals) => Some(format_number(value, decimals, true)),
    }
}

fn classify_number_format(format_code: &str) -> Option<FormatKind> {
    let normalized = normalize_number_format(format_code);
    if normalized.is_empty() || normalized.contains('?') || normalized.contains("e+") {
        return None;
    }

    let has_percent = normalized.contains('%');
    let has_currency = normalized.contains('$');
    let has_time =
        normalized.chars().any(|ch| matches!(ch, 'h' | 's')) || normalized.contains("am/pm");
    let has_year_or_day = normalized.chars().any(|ch| matches!(ch, 'y' | 'd'));
    let has_date = has_year_or_day || (normalized.chars().any(|ch| ch == 'm') && !has_time);

    if has_date && has_time {
        Some(FormatKind::DateTime)
    } else if has_date {
        Some(FormatKind::Date)
    } else if has_time {
        Some(FormatKind::Time)
    } else if has_percent {
        Some(FormatKind::Percent(decimal_places(&normalized)))
    } else if has_currency {
        Some(FormatKind::Currency(decimal_places(&normalized)))
    } else if looks_decimal_format(&normalized) {
        Some(FormatKind::Decimal(decimal_places(&normalized)))
    } else {
        None
    }
}

fn normalize_number_format(format_code: &str) -> String {
    let first_section = format_code.split(';').next().unwrap_or(format_code);
    let mut normalized = String::new();
    let mut chars = first_section.chars().peekable();
    let mut in_quote = false;
    let mut in_bracket = false;
    let mut bracket_contains_currency = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => in_quote = !in_quote,
            '[' => {
                in_bracket = true;
                bracket_contains_currency = false;
            }
            ']' => {
                if bracket_contains_currency {
                    normalized.push('$');
                }
                in_bracket = false;
            }
            '\\' | '_' | '*' => {
                chars.next();
            }
            '$' if in_quote => normalized.push('$'),
            '$' if in_bracket => bracket_contains_currency = true,
            _ if in_quote || in_bracket => {}
            _ => normalized.push(ch.to_ascii_lowercase()),
        }
    }

    normalized.replace(',', "")
}

fn looks_decimal_format(format_code: &str) -> bool {
    format_code
        .chars()
        .all(|ch| matches!(ch, '#' | '0' | '.' | '-' | ' '))
        && format_code.chars().any(|ch| ch == '0')
}

fn decimal_places(format_code: &str) -> usize {
    format_code
        .split('.')
        .nth(1)
        .map(|tail| {
            tail.chars()
                .take_while(|ch| matches!(ch, '0' | '#'))
                .filter(|ch| *ch == '0')
                .count()
        })
        .unwrap_or(0)
}

fn format_number(value: f64, decimals: usize, trim_negative_zero: bool) -> String {
    let mut output = format!("{value:.decimals$}");
    if trim_negative_zero
        && output.starts_with("-0")
        && output.parse::<f64>().unwrap_or(value) == 0.0
    {
        output.remove(0);
    }
    output
}

fn format_excel_datetime(
    serial: f64,
    date_system: DateSystem,
    include_time: bool,
    include_date: bool,
) -> Option<String> {
    if !serial.is_finite() {
        return None;
    }

    let whole_days = serial.floor() as i64;
    let fraction = serial - serial.floor();
    let (year, month, day) = excel_serial_to_date(whole_days, date_system)?;
    let date = format!("{year:04}-{month:02}-{day:02}");

    if include_date && include_time {
        Some(format!("{date}T{}", format_time_fraction(fraction)))
    } else if include_date {
        Some(date)
    } else {
        Some(format_time_fraction(fraction))
    }
}

fn excel_serial_to_date(serial_days: i64, date_system: DateSystem) -> Option<(i32, u32, u32)> {
    match date_system {
        DateSystem::Excel1904 => civil_from_days(serial_days + days_from_civil(1904, 1, 1)),
        DateSystem::Excel1900 if serial_days == 60 => Some((1900, 2, 29)),
        DateSystem::Excel1900 => {
            let adjusted_days = if serial_days > 60 {
                serial_days - 1
            } else {
                serial_days
            };
            civil_from_days(adjusted_days + days_from_civil(1899, 12, 31))
        }
    }
}

fn format_time_fraction(fraction: f64) -> String {
    let mut seconds = (fraction.rem_euclid(1.0) * 86_400.0).round() as i64;
    if seconds == 86_400 {
        seconds = 0;
    }
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146_097 + doe - 719_468)
}

fn civil_from_days(days: i64) -> Option<(i32, u32, u32)> {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    Some((i32::try_from(year).ok()?, month as u32, day as u32))
}

fn parse_cell_column(cell_ref: &str) -> Option<usize> {
    let mut column = 0usize;
    let mut saw_letter = false;

    for byte in cell_ref.bytes() {
        if !byte.is_ascii_alphabetic() {
            break;
        }
        saw_letter = true;
        column = column * 26 + usize::from(byte.to_ascii_uppercase() - b'A' + 1);
    }

    saw_letter.then_some(column.saturating_sub(1))
}

fn write_csv_row<W: Write>(writer: &mut W, row: &XlsxRow, delimiter: u8) -> Result<()> {
    let mut next_column = 0;
    for cell in &row.cells {
        while next_column < cell.column_index {
            if next_column > 0 {
                writer.write_all(&[delimiter])?;
            }
            next_column += 1;
        }
        if next_column > 0 {
            writer.write_all(&[delimiter])?;
        }
        write_csv_field(writer, cell.value.csv_value(), delimiter)?;
        next_column = cell.column_index.saturating_add(1);
    }
    writer.write_all(b"\n")?;
    Ok(())
}

fn write_csv_field<W: Write>(writer: &mut W, value: &str, delimiter: u8) -> Result<()> {
    let delimiter = char::from(delimiter);
    let needs_quotes = value
        .chars()
        .any(|ch| ch == delimiter || ch == '"' || ch == '\n' || ch == '\r');

    if !needs_quotes {
        writer.write_all(value.as_bytes())?;
        return Ok(());
    }

    writer.write_all(b"\"")?;
    for ch in value.chars() {
        if ch == '"' {
            writer.write_all(b"\"\"")?;
        } else {
            write!(writer, "{ch}")?;
        }
    }
    writer.write_all(b"\"")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use crate::OxdocError;
    use crate::models::{
        XlsxCellValue, XlsxReadOptions, XlsxRow, XlsxRowControl, XlsxSheetOptions,
        XlsxSheetVisibility, XlsxValueMode,
    };
    use crate::parsers::xlsx_shared_strings::{SharedStringLookup, SharedStringStore};
    use crate::vfs::{OoxmlLimits, OoxmlPackage};
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        CellFormat, CellState, DateSystem, SheetFormatContext, SheetRowSink, XlsxStyles,
        classify_number_format, format_cell_value, format_number, parse_cell_column,
        parse_sheet_rows, parse_styles, parse_workbook_date_system, parse_workbook_sheets,
        select_sheet, visit_rows_with_read_options, write_sheet_csv,
    };

    #[derive(Default)]
    struct CollectRows {
        rows: Vec<XlsxRow>,
    }

    impl SheetRowSink for CollectRows {
        fn emit(&mut self, row: &XlsxRow) -> crate::Result<XlsxRowControl> {
            self.rows.push(row.clone());
            Ok(XlsxRowControl::Continue)
        }
    }

    fn raw_context(styles: &XlsxStyles) -> SheetFormatContext<'_> {
        SheetFormatContext {
            value_mode: XlsxValueMode::Raw,
            date_system: DateSystem::Excel1900,
            styles,
        }
    }

    fn xlsx_package(entries: &[(&str, &str)]) -> Cursor<Vec<u8>> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut bytes);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for (path, content) in entries {
                zip.start_file(path, options).unwrap();
                zip.write_all(content.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        bytes.set_position(0);
        bytes
    }

    fn formatted_context(styles: &XlsxStyles) -> SheetFormatContext<'_> {
        SheetFormatContext {
            value_mode: XlsxValueMode::Formatted,
            date_system: DateSystem::Excel1900,
            styles,
        }
    }

    #[test]
    fn parses_workbook_sheet_relationships() {
        let xml = r#"
            <workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
              <sheets>
                <sheet name="Ventas Q1" sheetId="1" r:id="rId1"/>
              </sheets>
            </workbook>
        "#;

        let result = parse_workbook_sheets(xml, "xl/workbook.xml").unwrap();

        assert_eq!(result.value[0].name, "Ventas Q1");
        assert_eq!(result.value[0].relation_id, "rId1");
        assert_eq!(result.value[0].visibility, XlsxSheetVisibility::Visible);
    }

    #[test]
    fn parses_shared_strings_with_rich_text_runs() {
        let xml = r#"
            <sst>
              <si><t>Cliente</t></si>
              <si><r><t>A</t></r><r><t> &amp; B</t></r></si>
            </sst>
        "#;

        let mut store =
            SharedStringStore::parse(Cursor::new(xml.as_bytes()), "xl/sharedStrings.xml")
                .unwrap()
                .value;

        assert_eq!(store.lookup(0).unwrap().as_deref(), Some("Cliente"));
        assert_eq!(store.lookup(1).unwrap().as_deref(), Some("A & B"));
    }

    #[test]
    fn parses_shared_strings_with_cdata_and_empty_breaks() {
        let xml = r#"
            <sst>
              <si><t><![CDATA[A < B]]></t><r><br/></r><r><tab/></r><r><t>&quot;ok&quot;</t></r></si>
              <si/>
            </sst>
        "#;

        let mut store =
            SharedStringStore::parse(Cursor::new(xml.as_bytes()), "xl/sharedStrings.xml")
                .unwrap()
                .value;

        assert_eq!(store.lookup(0).unwrap().as_deref(), Some("A < B\n\t\"ok\""));
        assert_eq!(store.lookup(1).unwrap().as_deref(), Some(""));
    }

    #[test]
    fn writes_sparse_rows_and_escapes_csv() {
        let xml = r#"
            <worksheet>
              <sheetData>
                <row r="1">
                  <c r="A1" t="s"><v>0</v></c>
                  <c r="C1" t="inlineStr"><is><t>10,5</t></is></c>
                </row>
                <row r="2">
                  <c r="B2"><v>42</v></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut shared_strings = SharedStringStore::from_values(vec!["id".to_owned()]);
        let mut output = Vec::new();

        write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &raw_context(&XlsxStyles::default()),
            &mut output,
        )
        .unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "id,,\"10,5\"\n,42\n");
    }

    #[test]
    fn writes_very_sparse_rows_without_materializing_empty_cells() {
        let xml = r#"
            <worksheet>
              <sheetData>
                <row r="1">
                  <c r="A1"><v>left</v></c>
                  <c r="XFD1"><v>right</v></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut shared_strings = SharedStringStore::empty();
        let mut output = Vec::new();

        write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &raw_context(&XlsxStyles::default()),
            &mut output,
        )
        .unwrap();

        let csv = String::from_utf8(output).unwrap();
        assert!(csv.starts_with("left,,"));
        assert!(csv.ends_with(",right\n"));
        assert_eq!(csv.bytes().filter(|byte| *byte == b',').count(), 16_383);
    }

    #[test]
    fn writes_empty_rows_cells_and_boolean_values() {
        let xml = r#"
            <worksheet>
              <sheetData>
                <row r="1">
                  <c r="A1" t="s"><v>0</v></c>
                  <c r="B1" t="b"><v>1</v></c>
                  <c r="C1" t="e"><v>#DIV/0!</v></c>
                  <c r="D1" t="inlineStr"><is><t>inline</t></is></c>
                  <c r="E1"/>
                </row>
                <row r="2"/>
              </sheetData>
            </worksheet>
        "#;
        let mut shared_strings = SharedStringStore::from_values(vec!["shared".to_owned()]);
        let mut output = Vec::new();

        write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &raw_context(&XlsxStyles::default()),
            &mut output,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "shared,TRUE,#DIV/0!,inline,\n\n"
        );
    }

    #[test]
    fn emits_typed_sparse_rows_to_sink() {
        let styles = XlsxStyles {
            cell_formats: vec![CellFormat {
                number_format: Some("m/d/yyyy".to_owned()),
            }],
        };
        let xml = r#"
            <worksheet>
              <sheetData>
                <row r="3">
                  <c r="A3"/>
                  <c r="C3" t="b"><v>1</v></c>
                  <c r="D3" t="e"><v>#N/A</v></c>
                  <c r="E3" s="0"><f>TODAY()</f><v>44927</v></c>
                  <c r="G3" t="s"><v>0</v></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut shared_strings = SharedStringStore::from_values(vec!["text".to_owned()]);
        let mut sink = CollectRows::default();

        parse_sheet_rows(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            &formatted_context(&styles),
            &mut sink,
        )
        .unwrap();

        assert_eq!(sink.rows.len(), 1);
        let row = &sink.rows[0];
        assert_eq!(row.row_index, 2);
        assert_eq!(
            row.cells
                .iter()
                .map(|cell| cell.column_index)
                .collect::<Vec<_>>(),
            vec![0, 2, 3, 4, 6]
        );
        assert_eq!(row.cells[0].value, XlsxCellValue::Blank);
        assert_eq!(
            row.cells[1].value,
            XlsxCellValue::Boolean {
                raw: "1".to_owned(),
                value: Some(true),
            }
        );
        assert_eq!(
            row.cells[2].value,
            XlsxCellValue::Error {
                raw: "#N/A".to_owned(),
            }
        );
        assert!(row.cells[3].has_formula);
        assert_eq!(
            row.cells[3].value,
            XlsxCellValue::Number {
                raw: "44927".to_owned(),
                formatted: Some("2023-01-01".to_owned()),
            }
        );
        assert_eq!(
            row.cells[4].value,
            XlsxCellValue::String {
                raw: "0".to_owned(),
                value: "text".to_owned(),
            }
        );
    }

    #[test]
    fn typed_row_read_options_override_only_selected_worksheet_limit() {
        let large_sheet = format!(
            r#"<worksheet><sheetData><row><c r="A1"><v>1</v></c></row></sheetData><!--{}--></worksheet>"#,
            "x".repeat(768)
        );
        let mut package = OoxmlPackage::with_limits(
            xlsx_package(&[
                (
                    "_rels/.rels",
                    r#"<Relationships><Relationship Id="rId1" Type="officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
                ),
                (
                    "xl/workbook.xml",
                    r#"<workbook xmlns:r="r"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
                ),
                (
                    "xl/_rels/workbook.xml.rels",
                    r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
                ),
                ("xl/worksheets/sheet1.xml", &large_sheet),
            ]),
            OoxmlLimits {
                max_part_uncompressed_size: 512,
                ..OoxmlLimits::default()
            },
        )
        .unwrap();
        let mut rows = Vec::new();

        visit_rows_with_read_options(
            &mut package,
            XlsxReadOptions::new(XlsxSheetOptions::default()).with_worksheet_limits(OoxmlLimits {
                max_part_uncompressed_size: 2 * 1024,
                ..OoxmlLimits::default()
            }),
            XlsxValueMode::Raw,
            |row| {
                rows.push(row.clone());
                Ok(XlsxRowControl::Continue)
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells[0].value.csv_value(), "1");
    }

    #[test]
    fn typed_row_read_options_leave_shared_strings_on_package_limits() {
        let large_shared_strings =
            format!(r#"<sst><si><t>{}</t></si></sst>"#, "shared".repeat(128));
        let mut package = OoxmlPackage::with_limits(
            xlsx_package(&[
                (
                    "_rels/.rels",
                    r#"<Relationships><Relationship Id="rId1" Type="officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
                ),
                (
                    "xl/workbook.xml",
                    r#"<workbook xmlns:r="r"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
                ),
                (
                    "xl/_rels/workbook.xml.rels",
                    r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
                ),
                ("xl/sharedStrings.xml", &large_shared_strings),
                (
                    "xl/worksheets/sheet1.xml",
                    r#"<worksheet><sheetData><row><c r="A1" t="s"><v>0</v></c></row></sheetData></worksheet>"#,
                ),
            ]),
            OoxmlLimits {
                max_part_uncompressed_size: 512,
                ..OoxmlLimits::default()
            },
        )
        .unwrap();

        let err = visit_rows_with_read_options(
            &mut package,
            XlsxReadOptions::new(XlsxSheetOptions::default()).with_worksheet_limits(OoxmlLimits {
                max_part_uncompressed_size: 2 * 1024,
                ..OoxmlLimits::default()
            }),
            XlsxValueMode::Raw,
            |_| Ok(XlsxRowControl::Continue),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            OxdocError::PartTooLarge { path, limit: 512, .. }
                if path == "xl/sharedStrings.xml"
        ));
    }

    #[test]
    fn typed_rows_sort_cells_and_keep_the_last_duplicate() {
        let xml = r#"
            <worksheet>
              <sheetData>
                <row>
                  <c r="C1"><v>first</v></c>
                  <c r="A1"><v>left</v></c>
                  <c r="C1"><v>last</v></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut shared_strings = SharedStringStore::empty();
        let mut sink = CollectRows::default();

        parse_sheet_rows(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            &raw_context(&XlsxStyles::default()),
            &mut sink,
        )
        .unwrap();

        assert_eq!(
            sink.rows[0]
                .cells
                .iter()
                .map(|cell| (cell.column_index, cell.value.csv_value()))
                .collect::<Vec<_>>(),
            vec![(0, "left"), (2, "last")]
        );
    }

    #[test]
    fn typed_rows_keep_completed_rows_before_malformed_xml() {
        let xml = r#"
            <worksheet>
              <sheetData>
                <row><c r="A1"><v>complete</v></c></row>
                <row><c r="A2"><v>incomplete</v></c><
        "#;
        let mut shared_strings = SharedStringStore::empty();
        let mut sink = CollectRows::default();

        let result = parse_sheet_rows(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            &raw_context(&XlsxStyles::default()),
            &mut sink,
        )
        .unwrap();

        assert_eq!(result.warnings.len(), 1);
        assert_eq!(sink.rows.len(), 1);
        assert_eq!(sink.rows[0].cells[0].value.csv_value(), "complete");
    }

    #[test]
    fn writes_sheet_csv_with_disk_backed_shared_strings() {
        let shared_xml = r#"
            <sst>
              <si><t>alpha</t></si>
              <si><t>beta, needs quotes</t></si>
              <si><t>gamma</t></si>
            </sst>
        "#;
        let sheet_xml = r#"
            <worksheet>
              <sheetData>
                <row>
                  <c r="A1" t="s"><v>2</v></c>
                  <c r="B1" t="s"><v>1</v></c>
                  <c r="C1" t="s"><v>0</v></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut shared_strings = SharedStringStore::parse_with_memory_limit(
            Cursor::new(shared_xml.as_bytes()),
            "xl/sharedStrings.xml",
            4,
        )
        .unwrap()
        .value;
        let mut output = Vec::new();

        write_sheet_csv(
            Cursor::new(sheet_xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &raw_context(&XlsxStyles::default()),
            &mut output,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "gamma,\"beta, needs quotes\",alpha\n"
        );
    }

    #[test]
    fn formats_xlsx_values_when_requested() {
        let styles_xml = r#"
            <styleSheet>
              <numFmts>
                <numFmt numFmtId="164" formatCode="yyyy-mm-dd h:mm:ss"/>
              </numFmts>
              <cellXfs>
                <xf numFmtId="0"/>
                <xf numFmtId="14"/>
                <xf numFmtId="164"/>
                <xf numFmtId="10"/>
                <xf numFmtId="2"/>
                <xf numFmtId="44"/>
              </cellXfs>
            </styleSheet>
        "#;
        let styles = parse_styles(styles_xml, "xl/styles.xml").value;
        let sheet_xml = r#"
            <worksheet>
              <sheetData>
                <row>
                  <c r="A1" s="0"><v>44927</v></c>
                  <c r="B1" s="1"><v>44927</v></c>
                  <c r="C1" s="2"><v>44927.25</v></c>
                  <c r="D1" s="3"><v>0.12345</v></c>
                  <c r="E1" s="4"><v>42.5</v></c>
                  <c r="F1" s="5"><v>9.5</v></c>
                  <c r="G1" s="4"><f>SUM(E1:E1)</f><v>42.5</v></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut output = Vec::new();
        let mut shared_strings = SharedStringStore::empty();

        write_sheet_csv(
            Cursor::new(sheet_xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &formatted_context(&styles),
            &mut output,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "44927,2023-01-01,2023-01-01T06:00:00,12.35%,42.50,$9.50,42.50\n"
        );
    }

    #[test]
    fn keeps_xlsx_values_raw_by_default() {
        let styles = XlsxStyles {
            cell_formats: vec![CellFormat {
                number_format: Some("m/d/yyyy".to_owned()),
            }],
        };
        let sheet_xml = r#"
            <worksheet>
              <sheetData><row><c r="A1" s="0"><v>44927</v></c></row></sheetData>
            </worksheet>
        "#;
        let mut output = Vec::new();
        let mut shared_strings = SharedStringStore::empty();

        write_sheet_csv(
            Cursor::new(sheet_xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &raw_context(&styles),
            &mut output,
        )
        .unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "44927\n");
    }

    #[test]
    fn formats_xlsx_date_system_edge_cases() {
        let styles = XlsxStyles {
            cell_formats: vec![CellFormat {
                number_format: Some("m/d/yyyy".to_owned()),
            }],
        };
        let cell = |value: &str| CellState {
            value: value.to_owned(),
            style_index: Some(0),
            ..CellState::default()
        };

        assert_eq!(
            format_cell_value(&cell("0"), &styles, DateSystem::Excel1904).as_deref(),
            Some("1904-01-01")
        );
        assert_eq!(
            format_cell_value(&cell("60"), &styles, DateSystem::Excel1900).as_deref(),
            Some("1900-02-29")
        );
        assert_eq!(
            format_cell_value(&cell("61"), &styles, DateSystem::Excel1900).as_deref(),
            Some("1900-03-01")
        );
    }

    #[test]
    fn parses_xlsx_date_system_and_classifies_formats() {
        assert_eq!(
            parse_workbook_date_system(r#"<workbook><workbookPr date1904="1"/></workbook>"#),
            DateSystem::Excel1904
        );
        assert_eq!(
            parse_workbook_date_system(r#"<workbook><workbookPr/></workbook>"#),
            DateSystem::Excel1900
        );
        assert_eq!(
            classify_number_format(r#"[$$-409]#,##0.00"#),
            Some(super::FormatKind::Currency(2))
        );
        assert_eq!(
            classify_number_format("\"$\"#,##0.00"),
            Some(super::FormatKind::Currency(2))
        );
        assert_eq!(
            classify_number_format(r#"yyyy-mm-dd h:mm"#),
            Some(super::FormatKind::DateTime)
        );
        assert_eq!(
            classify_number_format("h:mm:ss"),
            Some(super::FormatKind::Time)
        );
    }

    #[test]
    fn parses_xlsx_styles_with_start_tags_and_warnings() {
        let styles = parse_styles(
            r#"
            <styleSheet>
              <numFmts><numFmt numFmtId="164" formatCode="yyyy-mm-dd"></numFmt></numFmts>
              <cellXfs><xf numFmtId="164"></xf><xf numFmtId="3"></xf></cellXfs>
            </styleSheet>
            "#,
            "xl/styles.xml",
        );

        assert!(styles.warnings.is_empty());
        assert_eq!(styles.value.cell_formats.len(), 2);
        assert_eq!(
            styles.value.cell_formats[0].number_format.as_deref(),
            Some("yyyy-mm-dd")
        );
        assert_eq!(
            styles.value.cell_formats[1].number_format.as_deref(),
            Some("#,##0")
        );

        let malformed = parse_styles("<styleSheet><cellXfs><", "xl/styles.xml");
        assert_eq!(malformed.warnings.len(), 1);
    }

    #[test]
    fn formats_xlsx_values_with_fallback_edges() {
        let styles = XlsxStyles {
            cell_formats: vec![
                CellFormat {
                    number_format: Some("h:mm:ss".to_owned()),
                },
                CellFormat {
                    number_format: Some("0.00E+00".to_owned()),
                },
            ],
        };

        let time_cell = CellState {
            value: "0.9999999".to_owned(),
            style_index: Some(0),
            ..CellState::default()
        };
        assert_eq!(
            format_cell_value(&time_cell, &styles, DateSystem::Excel1900).as_deref(),
            Some("00:00:00")
        );

        let unsupported_cell = CellState {
            value: "123".to_owned(),
            style_index: Some(1),
            ..CellState::default()
        };
        assert_eq!(
            format_cell_value(&unsupported_cell, &styles, DateSystem::Excel1900),
            None
        );

        let text_formula_cell = CellState {
            value_type: Some("str".to_owned()),
            value: "cached text".to_owned(),
            style_index: Some(0),
            ..CellState::default()
        };
        assert_eq!(
            format_cell_value(&text_formula_cell, &styles, DateSystem::Excel1900),
            None
        );

        assert_eq!(format_number(-0.0001, 2, true), "0.00");
    }

    #[test]
    fn parses_excel_column_references() {
        assert_eq!(parse_cell_column("A1"), Some(0));
        assert_eq!(parse_cell_column("Z99"), Some(25));
        assert_eq!(parse_cell_column("AA12"), Some(26));
        assert_eq!(parse_cell_column("BC7"), Some(54));
        assert_eq!(parse_cell_column("12"), None);
    }

    #[test]
    fn handles_workbook_sheet_selection_errors() {
        let sheets = parse_workbook_sheets(
            r#"<workbook><sheets><sheet name="A"/><sheet name="B" r:id="rId2"/></sheets></workbook>"#,
            "xl/workbook.xml",
        )
        .unwrap();

        assert_eq!(sheets.value.len(), 1);
        assert_eq!(sheets.warnings.len(), 1);
        assert_eq!(
            select_sheet(&sheets.value, None, None, false).unwrap().name,
            "B"
        );

        let err = select_sheet(&sheets.value, Some("missing"), None, false).unwrap_err();
        assert!(
            matches!(err, OxdocError::MissingPart(part) if part == "visible sheet named missing")
        );

        let err = select_sheet(&sheets.value, Some("B"), Some(1), false).unwrap_err();
        assert!(
            matches!(err, OxdocError::InvalidArgument(message) if message.contains("by name or index"))
        );

        let err = select_sheet(&sheets.value, None, Some(0), false).unwrap_err();
        assert!(
            matches!(err, OxdocError::InvalidArgument(message) if message.contains("1 or greater"))
        );

        let err = select_sheet(&[], None, None, false).unwrap_err();
        assert!(matches!(err, OxdocError::MissingPart(part) if part == "visible workbook sheets"));
    }

    #[test]
    fn selects_visible_workbook_sheets_by_one_based_index() {
        let sheets = parse_workbook_sheets(
            r#"
            <workbook xmlns:r="r">
              <sheets>
                <sheet name="Hidden" sheetId="1" state="hidden" r:id="rId1"/>
                <sheet name="Visible A" sheetId="2" state="visible" r:id="rId2"/>
                <sheet name="Very Hidden" sheetId="3" state="veryHidden" r:id="rId3"/>
                <sheet name="Visible B" sheetId="4" r:id="rId4"/>
              </sheets>
            </workbook>
            "#,
            "xl/workbook.xml",
        )
        .unwrap();

        assert_eq!(
            select_sheet(&sheets.value, None, None, false).unwrap().name,
            "Visible A"
        );
        assert_eq!(
            select_sheet(&sheets.value, None, Some(1), false)
                .unwrap()
                .name,
            "Visible A"
        );
        assert_eq!(
            select_sheet(&sheets.value, None, Some(2), false)
                .unwrap()
                .name,
            "Visible B"
        );

        let err = select_sheet(&sheets.value, Some("Hidden"), None, false).unwrap_err();
        assert!(
            matches!(err, OxdocError::MissingPart(part) if part == "visible sheet named Hidden")
        );

        let err = select_sheet(&sheets.value, None, Some(3), false).unwrap_err();
        assert!(matches!(err, OxdocError::MissingPart(part) if part == "visible sheet index 3"));
    }

    #[test]
    fn selects_hidden_workbook_sheets_only_with_explicit_opt_in() {
        let sheets = parse_workbook_sheets(
            r#"
            <workbook xmlns:r="r">
              <sheets>
                <sheet name="Hidden" sheetId="1" state="hidden" r:id="rId1"/>
                <sheet name="Visible" sheetId="2" r:id="rId2"/>
                <sheet name="Very Hidden" sheetId="3" state="veryHidden" r:id="rId3"/>
              </sheets>
            </workbook>
            "#,
            "xl/workbook.xml",
        )
        .unwrap();

        assert_eq!(
            select_sheet(&sheets.value, None, Some(1), true)
                .unwrap()
                .name,
            "Hidden"
        );
        assert_eq!(
            select_sheet(&sheets.value, Some("Very Hidden"), None, true)
                .unwrap()
                .visibility,
            XlsxSheetVisibility::VeryHidden
        );

        let err = select_sheet(&sheets.value, None, Some(4), true).unwrap_err();
        assert!(matches!(err, OxdocError::MissingPart(part) if part == "workbook sheet index 4"));
    }

    #[test]
    fn rejects_duplicate_visible_sheet_names() {
        let sheets = parse_workbook_sheets(
            r#"
            <workbook xmlns:r="r">
              <sheets>
                <sheet name="Dup" sheetId="1" r:id="rId1"/>
                <sheet name="Dup" sheetId="2" r:id="rId2"/>
              </sheets>
            </workbook>
            "#,
            "xl/workbook.xml",
        )
        .unwrap();

        let err = select_sheet(&sheets.value, Some("Dup"), None, false).unwrap_err();
        assert!(
            matches!(err, OxdocError::InvalidArgument(message) if message.contains("multiple visible sheets named Dup"))
        );
    }

    #[test]
    fn rejects_duplicate_names_across_visibility_when_hidden_sheets_are_included() {
        let sheets = parse_workbook_sheets(
            r#"
            <workbook xmlns:r="r">
              <sheets>
                <sheet name="Dup" sheetId="1" r:id="rId1"/>
                <sheet name="Dup" sheetId="2" state="hidden" r:id="rId2"/>
              </sheets>
            </workbook>
            "#,
            "xl/workbook.xml",
        )
        .unwrap();

        assert_eq!(
            select_sheet(&sheets.value, Some("Dup"), None, false)
                .unwrap()
                .visibility,
            XlsxSheetVisibility::Visible
        );

        let err = select_sheet(&sheets.value, Some("Dup"), None, true).unwrap_err();
        assert!(
            matches!(err, OxdocError::InvalidArgument(message) if message.contains("multiple workbook sheets named Dup"))
        );
    }

    #[test]
    fn warns_on_malformed_shared_strings_and_sheet_xml() {
        let shared = SharedStringStore::parse(
            Cursor::new(br#"<sst><si><t>first</t></si><"#.as_slice()),
            "xl/sharedStrings.xml",
        )
        .unwrap();
        let mut shared_value = shared.value;
        let mut output = Vec::new();
        let mut empty_shared_strings = SharedStringStore::empty();
        let sheet = write_sheet_csv(
            Cursor::new(br#"<worksheet><sheetData><row><c r="A1"><v>1</v></c><"#.as_slice()),
            "xl/worksheets/sheet1.xml",
            &mut empty_shared_strings,
            b',',
            &raw_context(&XlsxStyles::default()),
            &mut output,
        )
        .unwrap();

        assert_eq!(shared_value.lookup(0).unwrap().as_deref(), Some("first"));
        assert_eq!(shared.warnings.len(), 1);
        assert_eq!(sheet.warnings.len(), 1);
    }

    #[test]
    fn warns_on_malformed_workbook_xml() {
        let result = parse_workbook_sheets("<workbook><", "xl/workbook.xml").unwrap();

        assert!(result.value.is_empty());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn warns_on_invalid_shared_string_indexes() {
        let xml = r#"
            <worksheet>
              <sheetData>
                <row>
                  <c r="A1" t="s"><v>5</v></c>
                  <c r="B1" t="s"><v>not-a-number</v></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut output = Vec::new();
        let mut shared_strings = SharedStringStore::from_values(vec!["only".to_owned()]);

        let result = write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &raw_context(&XlsxStyles::default()),
            &mut output,
        )
        .unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), ",not-a-number\n");
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn writes_cdata_general_refs_duplicate_cells_and_quotes() {
        let xml = r#"
            <worksheet>
              <sheetData>
                <row>
                  <c r="A1" t="inlineStr"><is><t>first</t></is></c>
                  <c r="A1" t="inlineStr"><is><t><![CDATA[second]]></t></is></c>
                  <c r="B1" t="inlineStr"><is><t>Tom &amp; "Jerry"</t></is></c>
                </row>
              </sheetData>
            </worksheet>
        "#;
        let mut output = Vec::new();
        let mut shared_strings = SharedStringStore::empty();

        write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &mut shared_strings,
            b',',
            &raw_context(&XlsxStyles::default()),
            &mut output,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "second,\"Tom & \"\"Jerry\"\"\"\n"
        );
    }
}
