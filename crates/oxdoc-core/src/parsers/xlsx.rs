use std::io::{BufRead, BufReader, Cursor, Read, Seek, Write};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{Extraction, OutputWarning, XlsxCsvOptions};
use crate::parsers::xlsx_shared_strings::{
    DEFAULT_SHARED_STRING_MEMORY_LIMIT, SharedStringLookup, SharedStringStore,
};
use crate::parsers::{
    attr_value, decode_xml_reference, decode_xml_text, merge_warnings, name_eq, parent_dir,
    parse_relationship_map, rels_path_for, resolve_relationship_target,
};
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkbookSheet {
    name: String,
    relation_id: String,
    visible: bool,
}

#[derive(Debug, Default)]
struct CellState {
    value_type: Option<String>,
    column_index: Option<usize>,
    value: String,
    in_value: bool,
    in_inline_text: bool,
}

pub(crate) fn write_csv<R: Read + Seek, W: Write>(
    package: &mut OoxmlPackage<R>,
    options: XlsxCsvOptions<'_>,
    mut writer: W,
) -> Result<Extraction<()>> {
    write_csv_with_shared_string_memory_limit(
        package,
        options,
        DEFAULT_SHARED_STRING_MEMORY_LIMIT,
        &mut writer,
    )
}

fn write_csv_with_shared_string_memory_limit<R: Read + Seek, W: Write>(
    package: &mut OoxmlPackage<R>,
    options: XlsxCsvOptions<'_>,
    shared_string_memory_limit: usize,
    writer: &mut W,
) -> Result<Extraction<()>> {
    let workbook_path = crate::parsers::find_office_document_path(package, "xl/workbook.xml")?;
    let workbook_xml = package.read_to_string(&workbook_path)?;
    let workbook = parse_workbook_sheets(&workbook_xml, &workbook_path)?;

    let workbook_rels_path = rels_path_for(&workbook_path);
    let workbook_rels_xml = package.read_to_string(&workbook_rels_path)?;
    let workbook_rels = parse_relationship_map(&workbook_rels_xml, &workbook_rels_path)?;

    let selected_sheet = select_sheet(&workbook.value, options.sheet_name, options.sheet_index)?;
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

    let sheet = package.with_entry(&sheet_path, |entry| {
        let reader = BufReader::new(entry);
        write_sheet_csv(
            reader,
            &sheet_path,
            &mut shared_strings.value,
            options.delimiter,
            writer,
        )
    })?;

    Ok(Extraction::with_warnings(
        (),
        merge_warnings(
            merge_warnings(workbook.warnings, shared_strings.warnings),
            sheet.warnings,
        ),
    ))
}

fn select_sheet<'a>(
    sheets: &'a [WorkbookSheet],
    sheet_name: Option<&str>,
    sheet_index: Option<usize>,
) -> Result<&'a WorkbookSheet> {
    if sheet_name.is_some() && sheet_index.is_some() {
        return Err(OxdocError::InvalidArgument(
            "select an XLSX sheet by name or index, not both".to_owned(),
        ));
    }

    if let Some(sheet_name) = sheet_name {
        let mut matches = sheets
            .iter()
            .filter(|sheet| sheet.visible && sheet.name == sheet_name);
        let selected = matches
            .next()
            .ok_or_else(|| OxdocError::MissingPart(format!("visible sheet named {sheet_name}")))?;

        if matches.next().is_some() {
            return Err(OxdocError::InvalidArgument(format!(
                "multiple visible sheets named {sheet_name}; use sheet index to disambiguate"
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
            .filter(|sheet| sheet.visible)
            .nth(sheet_index - 1)
            .ok_or_else(|| OxdocError::MissingPart(format!("visible sheet index {sheet_index}")));
    }

    sheets
        .iter()
        .find(|sheet| sheet.visible)
        .ok_or_else(|| OxdocError::MissingPart("visible workbook sheets".to_owned()))
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
                        visible: is_visible_sheet_state(attr_value(&element, b"state")),
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

fn is_visible_sheet_state(state: Option<String>) -> bool {
    !state.is_some_and(|state| {
        state.eq_ignore_ascii_case("hidden") || state.eq_ignore_ascii_case("veryHidden")
    })
}

fn write_sheet_csv<R: BufRead, W: Write>(
    source: R,
    path: &str,
    shared_strings: &mut impl SharedStringLookup,
    delimiter: u8,
    writer: &mut W,
) -> Result<Extraction<()>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut warnings = Vec::new();
    let mut row = Vec::new();
    let mut current_cell: Option<CellState> = None;
    let mut in_row = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"row") {
                    in_row = true;
                    row.clear();
                } else if name_eq(element.name().as_ref(), b"c") {
                    current_cell = Some(cell_state_from_element(&element, row.len()));
                } else if let Some(cell) = &mut current_cell {
                    if name_eq(element.name().as_ref(), b"v") {
                        cell.in_value = true;
                    } else if name_eq(element.name().as_ref(), b"t") {
                        cell.in_inline_text = true;
                    }
                }
            }
            Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"row") {
                    row.clear();
                    writer.write_all(b"\n")?;
                    in_row = false;
                } else if name_eq(element.name().as_ref(), b"c") && in_row {
                    let cell = cell_state_from_element(&element, row.len());
                    push_cell_value(&mut row, cell, shared_strings, path, &mut warnings)?;
                }
            }
            Ok(Event::Text(value)) => {
                if let Some(cell) = &mut current_cell
                    && (cell.in_value || cell.in_inline_text)
                {
                    cell.value.push_str(&decode_xml_text(value.as_ref()));
                }
            }
            Ok(Event::CData(value)) => {
                if let Some(cell) = &mut current_cell
                    && (cell.in_value || cell.in_inline_text)
                {
                    cell.value.push_str(&decode_xml_text(value.as_ref()));
                }
            }
            Ok(Event::GeneralRef(value)) => {
                if let Some(cell) = &mut current_cell
                    && (cell.in_value || cell.in_inline_text)
                {
                    cell.value.push_str(&decode_xml_reference(value.as_ref()));
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
                    if let Some(cell) = current_cell.take() {
                        push_cell_value(&mut row, cell, shared_strings, path, &mut warnings)?;
                    }
                } else if name_eq(element.name().as_ref(), b"row") && in_row {
                    write_csv_row(writer, &row, delimiter)?;
                    in_row = false;
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
    let mut sink = Vec::new();
    let mut shared_strings = SharedStringStore::empty();
    let _ = write_sheet_csv(
        Cursor::new(xml),
        "xl/worksheets/sheet1.xml",
        &mut shared_strings,
        b',',
        &mut sink,
    )?;
    Ok(())
}

fn push_cell_value(
    row: &mut Vec<String>,
    cell: CellState,
    shared_strings: &mut impl SharedStringLookup,
    path: &str,
    warnings: &mut Vec<OutputWarning>,
) -> Result<()> {
    let target_column = cell.column_index.unwrap_or(row.len());
    while row.len() < target_column {
        row.push(String::new());
    }

    let value = if cell.value_type.as_deref() == Some("s") {
        match cell.value.trim().parse::<usize>() {
            Ok(index) => match shared_strings.lookup(index)? {
                Some(value) => value,
                None => {
                    warnings.push(OutputWarning::shared_string_index_out_of_bounds(
                        path, index,
                    ));
                    String::new()
                }
            },
            Err(_) => {
                warnings.push(OutputWarning::invalid_shared_string_index(
                    path,
                    cell.value.clone(),
                ));
                cell.value
            }
        }
    } else if cell.value_type.as_deref() == Some("b") {
        match cell.value.trim() {
            "1" | "true" | "TRUE" => "TRUE".to_owned(),
            "0" | "false" | "FALSE" => "FALSE".to_owned(),
            _ => cell.value,
        }
    } else {
        cell.value
    };

    if row.len() == target_column {
        row.push(value);
    } else if let Some(slot) = row.get_mut(target_column) {
        *slot = value;
    }

    Ok(())
}

fn cell_state_from_element(
    element: &quick_xml::events::BytesStart<'_>,
    fallback_column: usize,
) -> CellState {
    CellState {
        value_type: attr_value(element, b"t"),
        column_index: attr_value(element, b"r")
            .and_then(|cell_ref| parse_cell_column(&cell_ref).or(Some(fallback_column))),
        ..CellState::default()
    }
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

fn write_csv_row<W: Write>(writer: &mut W, row: &[String], delimiter: u8) -> Result<()> {
    for (index, value) in row.iter().enumerate() {
        if index > 0 {
            writer.write_all(&[delimiter])?;
        }
        write_csv_field(writer, value, delimiter)?;
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
    use std::io::Cursor;

    use crate::OxdocError;
    use crate::parsers::xlsx_shared_strings::{SharedStringLookup, SharedStringStore};

    use super::{parse_cell_column, parse_workbook_sheets, select_sheet, write_sheet_csv};

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
        assert!(result.value[0].visible);
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
            &mut output,
        )
        .unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "id,,\"10,5\"\n,42\n");
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
            &mut output,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "shared,TRUE,#DIV/0!,inline,\n\n"
        );
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
            &mut output,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "gamma,\"beta, needs quotes\",alpha\n"
        );
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
        assert_eq!(select_sheet(&sheets.value, None, None).unwrap().name, "B");

        let err = select_sheet(&sheets.value, Some("missing"), None).unwrap_err();
        assert!(
            matches!(err, OxdocError::MissingPart(part) if part == "visible sheet named missing")
        );

        let err = select_sheet(&sheets.value, Some("B"), Some(1)).unwrap_err();
        assert!(
            matches!(err, OxdocError::InvalidArgument(message) if message.contains("by name or index"))
        );

        let err = select_sheet(&sheets.value, None, Some(0)).unwrap_err();
        assert!(
            matches!(err, OxdocError::InvalidArgument(message) if message.contains("1 or greater"))
        );

        let err = select_sheet(&[], None, None).unwrap_err();
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
            select_sheet(&sheets.value, None, None).unwrap().name,
            "Visible A"
        );
        assert_eq!(
            select_sheet(&sheets.value, None, Some(1)).unwrap().name,
            "Visible A"
        );
        assert_eq!(
            select_sheet(&sheets.value, None, Some(2)).unwrap().name,
            "Visible B"
        );

        let err = select_sheet(&sheets.value, Some("Hidden"), None).unwrap_err();
        assert!(
            matches!(err, OxdocError::MissingPart(part) if part == "visible sheet named Hidden")
        );

        let err = select_sheet(&sheets.value, None, Some(3)).unwrap_err();
        assert!(matches!(err, OxdocError::MissingPart(part) if part == "visible sheet index 3"));
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

        let err = select_sheet(&sheets.value, Some("Dup"), None).unwrap_err();
        assert!(
            matches!(err, OxdocError::InvalidArgument(message) if message.contains("multiple visible sheets named Dup"))
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
            &mut output,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "second,\"Tom & \"\"Jerry\"\"\"\n"
        );
    }
}
