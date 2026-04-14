use std::io::{BufRead, BufReader, Read, Seek, Write};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{Extraction, OutputWarning, XlsxCsvOptions};
use crate::parsers::{
    attr_value, decode_xml_reference, decode_xml_text, merge_warnings, name_eq,
    normalize_part_path, parent_dir, parse_relationship_map, rels_path_for,
};
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkbookSheet {
    name: String,
    relation_id: String,
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
    let workbook_path = crate::parsers::find_office_document_path(package, "xl/workbook.xml")?;
    let workbook_xml = package.read_to_string(&workbook_path)?;
    let workbook = parse_workbook_sheets(&workbook_xml, &workbook_path)?;

    let workbook_rels_path = rels_path_for(&workbook_path);
    let workbook_rels_xml = package.read_to_string(&workbook_rels_path)?;
    let workbook_rels = parse_relationship_map(&workbook_rels_xml, &workbook_rels_path)?;

    let selected_sheet = select_sheet(&workbook.value, options.sheet_name)?;
    let target = workbook_rels
        .get(&selected_sheet.relation_id)
        .ok_or_else(|| OxdocError::MissingPart(selected_sheet.relation_id.clone()))?;
    let sheet_path = normalize_part_path(parent_dir(&workbook_path), target);

    let shared_strings = if package.contains("xl/sharedStrings.xml") {
        package.with_entry("xl/sharedStrings.xml", |entry| {
            let reader = BufReader::new(entry);
            parse_shared_strings(reader, "xl/sharedStrings.xml")
        })?
    } else {
        Extraction::new(Vec::new())
    };

    let sheet = package.with_entry(&sheet_path, |entry| {
        let reader = BufReader::new(entry);
        write_sheet_csv(
            reader,
            &sheet_path,
            &shared_strings.value,
            options.delimiter,
            &mut writer,
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
) -> Result<&'a WorkbookSheet> {
    if let Some(sheet_name) = sheet_name {
        return sheets
            .iter()
            .find(|sheet| sheet.name == sheet_name)
            .ok_or_else(|| OxdocError::MissingPart(format!("sheet named {sheet_name}")));
    }

    sheets
        .first()
        .ok_or_else(|| OxdocError::MissingPart("xl/workbook.xml sheets".to_owned()))
}

fn parse_workbook_sheets(xml: &str, path: &str) -> Result<Extraction<Vec<WorkbookSheet>>> {
    let mut reader = Reader::from_reader(std::io::Cursor::new(xml.as_bytes()));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut sheets = Vec::new();
    let mut warnings = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"sheet") {
                    match (attr_value(&element, b"name"), attr_value(&element, b"id")) {
                        (Some(name), Some(relation_id)) => {
                            sheets.push(WorkbookSheet { name, relation_id });
                        }
                        _ => warnings.push(OutputWarning::new(
                            path,
                            "ignored workbook sheet without name or relationship id",
                        )),
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::new(
                    path,
                    format!("stopped after malformed XML: {source}"),
                ));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings(sheets, warnings))
}

fn parse_shared_strings<R: BufRead>(source: R, path: &str) -> Result<Extraction<Vec<String>>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut strings = Vec::new();
    let mut warnings = Vec::new();
    let mut current_string: Option<String> = None;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"si") {
                    current_string = Some(String::new());
                } else if current_string.is_some() && name_eq(element.name().as_ref(), b"t") {
                    in_text = true;
                }
            }
            Ok(Event::Empty(element)) => {
                if let Some(value) = &mut current_string {
                    if name_eq(element.name().as_ref(), b"tab") {
                        value.push('\t');
                    } else if name_eq(element.name().as_ref(), b"br")
                        || name_eq(element.name().as_ref(), b"cr")
                    {
                        value.push('\n');
                    }
                }
            }
            Ok(Event::Text(value)) if in_text => {
                if let Some(current) = &mut current_string {
                    current.push_str(&decode_xml_text(value.as_ref()));
                }
            }
            Ok(Event::CData(value)) if in_text => {
                if let Some(current) = &mut current_string {
                    current.push_str(&decode_xml_text(value.as_ref()));
                }
            }
            Ok(Event::GeneralRef(value)) if in_text => {
                if let Some(current) = &mut current_string {
                    current.push_str(&decode_xml_reference(value.as_ref()));
                }
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text = false;
                } else if name_eq(element.name().as_ref(), b"si")
                    && let Some(current) = current_string.take()
                {
                    strings.push(current);
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::new(
                    path,
                    format!("stopped after malformed XML: {source}"),
                ));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings(strings, warnings))
}

fn write_sheet_csv<R: BufRead, W: Write>(
    source: R,
    path: &str,
    shared_strings: &[String],
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
                    current_cell = Some(CellState {
                        value_type: attr_value(&element, b"t"),
                        column_index: attr_value(&element, b"r")
                            .and_then(|cell_ref| parse_cell_column(&cell_ref).or(Some(row.len()))),
                        ..CellState::default()
                    });
                } else if let Some(cell) = &mut current_cell {
                    if name_eq(element.name().as_ref(), b"v") {
                        cell.in_value = true;
                    } else if name_eq(element.name().as_ref(), b"t") {
                        cell.in_inline_text = true;
                    }
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
                        push_cell_value(&mut row, cell, shared_strings, path, &mut warnings);
                    }
                } else if name_eq(element.name().as_ref(), b"row") && in_row {
                    write_csv_row(writer, &row, delimiter)?;
                    in_row = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::new(
                    path,
                    format!("stopped after malformed XML: {source}"),
                ));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings((), warnings))
}

fn push_cell_value(
    row: &mut Vec<String>,
    cell: CellState,
    shared_strings: &[String],
    path: &str,
    warnings: &mut Vec<OutputWarning>,
) {
    let target_column = cell.column_index.unwrap_or(row.len());
    while row.len() < target_column {
        row.push(String::new());
    }

    let value = if cell.value_type.as_deref() == Some("s") {
        match cell.value.trim().parse::<usize>() {
            Ok(index) => shared_strings.get(index).cloned().unwrap_or_else(|| {
                warnings.push(OutputWarning::new(
                    path,
                    format!("shared string index {index} is out of bounds"),
                ));
                String::new()
            }),
            Err(_) => {
                warnings.push(OutputWarning::new(
                    path,
                    format!("invalid shared string index '{}'", cell.value),
                ));
                cell.value
            }
        }
    } else {
        cell.value
    };

    if row.len() == target_column {
        row.push(value);
    } else if let Some(slot) = row.get_mut(target_column) {
        *slot = value;
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

    use super::{
        parse_cell_column, parse_shared_strings, parse_workbook_sheets, select_sheet,
        write_sheet_csv,
    };

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
    }

    #[test]
    fn parses_shared_strings_with_rich_text_runs() {
        let xml = r#"
            <sst>
              <si><t>Cliente</t></si>
              <si><r><t>A</t></r><r><t> &amp; B</t></r></si>
            </sst>
        "#;

        let result =
            parse_shared_strings(Cursor::new(xml.as_bytes()), "xl/sharedStrings.xml").unwrap();

        assert_eq!(result.value, vec!["Cliente", "A & B"]);
    }

    #[test]
    fn parses_shared_strings_with_cdata_and_empty_breaks() {
        let xml = r#"
            <sst>
              <si><t><![CDATA[A < B]]></t><r><br/></r><r><tab/></r><r><t>&quot;ok&quot;</t></r></si>
            </sst>
        "#;

        let result =
            parse_shared_strings(Cursor::new(xml.as_bytes()), "xl/sharedStrings.xml").unwrap();

        assert_eq!(result.value, vec!["A < B\n\t\"ok\""]);
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
        let shared_strings = vec!["id".to_owned()];
        let mut output = Vec::new();

        write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &shared_strings,
            b',',
            &mut output,
        )
        .unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "id,,\"10,5\"\n,42\n");
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
        assert_eq!(select_sheet(&sheets.value, None).unwrap().name, "B");
        assert!(select_sheet(&sheets.value, Some("missing")).is_err());
        assert!(select_sheet(&[], None).is_err());
    }

    #[test]
    fn warns_on_malformed_shared_strings_and_sheet_xml() {
        let shared = parse_shared_strings(
            Cursor::new(br#"<sst><si><t>first</t></si><"#.as_slice()),
            "xl/sharedStrings.xml",
        )
        .unwrap();
        let mut output = Vec::new();
        let sheet = write_sheet_csv(
            Cursor::new(br#"<worksheet><sheetData><row><c r="A1"><v>1</v></c><"#.as_slice()),
            "xl/worksheets/sheet1.xml",
            &[],
            b',',
            &mut output,
        )
        .unwrap();

        assert_eq!(shared.value, vec!["first"]);
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

        let result = write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &["only".to_owned()],
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

        write_sheet_csv(
            Cursor::new(xml.as_bytes()),
            "xl/worksheets/sheet1.xml",
            &[],
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
