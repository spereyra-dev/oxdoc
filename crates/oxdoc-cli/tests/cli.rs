use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;

#[test]
fn extracts_text_to_stdout() {
    let docx = fixtures::build_package("docx/basic", "fixture.docx");

    let output = oxdoc(["extract", "text", docx.to_str().unwrap()]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
}

#[test]
fn extracts_text_as_json() {
    let docx = fixtures::build_package("docx/basic", "fixture.docx");

    let output = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());

    let actual_stdout = stdout(&output);
    let expected_snapshot = fixtures::read_snapshot("cli_extract_text_json.json");
    let actual: Value = serde_json::from_str(&actual_stdout).unwrap();
    let expected: Value = serde_json::from_str(&expected_snapshot).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(actual_stdout.trim_end(), expected_snapshot.trim_end());
}

#[test]
fn extracts_text_as_jsonl_with_partial_errors() {
    let docx = fixtures::build_package("docx/basic", "good-jsonl.docx");
    let missing = unique_path("missing-jsonl.docx");

    let output = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        missing.to_str().unwrap(),
        "--format",
        "jsonl",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());

    let records = jsonl_lines(&output);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["document_type"], "docx");
    assert!(
        records[0]["file"]
            .as_str()
            .unwrap()
            .ends_with("good-jsonl.docx")
    );
    assert_eq!(
        records[0]["text"].as_str().unwrap().trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
    assert_eq!(records[1]["document_type"], "unknown");
    assert!(
        records[1]["error"]["code"]
            .as_str()
            .unwrap()
            .starts_with('E')
    );
    assert!(!records[1]["error"]["message"].as_str().unwrap().is_empty());
}

#[test]
fn extracts_text_as_jsonl_from_stdin() {
    let docx = fixtures::build_package("docx/basic", "stdin-jsonl.docx");

    let output = oxdoc_with_stdin(
        ["extract", "text", "-", "--format", "jsonl"],
        &fs::read(&docx).unwrap(),
    );

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let records = jsonl_lines(&output);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["file"], "<stdin>");
    assert_eq!(records[0]["document_type"], "docx");
    assert_eq!(
        records[0]["text"].as_str().unwrap().trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
}

#[test]
fn extracts_application_generated_docx_text_to_stdout() {
    let docx = fixtures::fixture_file("docx/python-docx-basic.docx");

    let output = oxdoc(["extract", "text", docx.to_str().unwrap()]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("docx_python_docx_text.txt").trim_end()
    );
}

#[test]
fn extracts_pptx_text_to_stdout() {
    let pptx = fixtures::build_package("pptx/text", "fixture.PPTX");

    let output = oxdoc(["extract", "text", pptx.to_str().unwrap()]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("pptx_text.txt").trim_end()
    );
}

#[test]
fn extracts_pptx_text_as_json() {
    let pptx = fixtures::build_package("pptx/text", "fixture.pptx");

    let output = oxdoc([
        "extract",
        "text",
        pptx.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());

    let actual_stdout = stdout(&output);
    let actual: Value = serde_json::from_str(&actual_stdout).unwrap();

    assert_eq!(actual["file"], "fixture.pptx");
    assert_eq!(
        actual["text"].as_str().unwrap().trim_end(),
        fixtures::read_snapshot("pptx_text.txt").trim_end()
    );
}

#[test]
fn extracts_text_as_structured_json() {
    let docx = create_ooxml(
        "structured-cli.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Body</w:t></w:r></w:p></w:body></w:document>"#,
            ),
            (
                "word/_rels/document.xml.rels",
                r#"<Relationships><Relationship Id="rIdHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:p><w:r><w:t>Header</w:t></w:r></w:p></w:hdr>"#,
            ),
        ],
    );

    let output = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        "--format",
        "structured-json",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let actual: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(
        actual["file"]
            .as_str()
            .unwrap()
            .ends_with("structured-cli.docx")
    );
    assert_eq!(actual["document_type"], "docx");
    assert_eq!(actual["blocks"][0]["part_type"], "main");
    assert_eq!(actual["blocks"][0]["part_path"], "word/document.xml");
    assert_eq!(actual["blocks"][0]["ordinal"], 1);
    assert_eq!(actual["blocks"][0]["text"], "Body\n");
    assert_eq!(actual["blocks"][1]["part_type"], "header");
    assert_eq!(actual["blocks"][1]["part_path"], "word/header1.xml");
    assert_eq!(actual["blocks"][1]["ordinal"], 2);
    assert_eq!(actual["blocks"][1]["text"], "Header\n");
}

#[test]
fn extracts_structured_json_from_stdin_and_multiple_inputs() {
    let docx = fixtures::build_package("docx/basic", "structured-stdin.docx");
    let second = fixtures::build_package("docx/basic", "structured-second.docx");
    let missing = unique_path("missing-structured.docx");

    let stdin_output = oxdoc_with_stdin(
        ["extract", "text", "-", "--format", "structured-json"],
        &fs::read(&docx).unwrap(),
    );
    assert!(stdin_output.status.success());
    let stdin_actual: Value = serde_json::from_str(&stdout(&stdin_output)).unwrap();
    assert_eq!(stdin_actual["file"], "<stdin>");
    assert_eq!(stdin_actual["document_type"], "docx");
    assert_eq!(stdin_actual["blocks"][0]["part_type"], "main");

    let multiple_output = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        missing.to_str().unwrap(),
        second.to_str().unwrap(),
        "--format",
        "structured-json",
    ]);
    assert!(multiple_output.status.success());
    assert!(stderr(&multiple_output).contains("warning["));
    let records: Value = serde_json::from_str(&stdout(&multiple_output)).unwrap();
    let records = records.as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert!(
        records[0]["file"]
            .as_str()
            .unwrap()
            .ends_with("structured-stdin.docx")
    );
    assert!(
        records[1]["file"]
            .as_str()
            .unwrap()
            .ends_with("structured-second.docx")
    );
}

#[test]
fn rejects_structured_text_from_xlsx() {
    let xlsx = fixtures::build_package("xlsx/basic", "structured.xlsx");

    let output = oxdoc([
        "extract",
        "text",
        xlsx.to_str().unwrap(),
        "--format",
        "structured-json",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("cannot extract text from an XLSX workbook"));
}

#[test]
fn extracts_docx_text_from_content_types_when_extension_is_wrong() {
    let docx = fixtures::build_package("docx/basic", "fixture.bin");

    let output = oxdoc(["extract", "text", docx.to_str().unwrap()]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
}

#[test]
fn extracts_application_generated_pptx_text_to_stdout() {
    let pptx = fixtures::fixture_file("pptx/python-pptx-basic.pptx");

    let output = oxdoc(["extract", "text", pptx.to_str().unwrap()]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("pptx_python_pptx_text.txt").trim_end()
    );
}

#[test]
fn extracts_csv_to_stdout() {
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet",
        "Sales Q1",
        "--delimiter",
        ";",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("cli_extract_csv.txt").trim_end()
    );
}

#[test]
fn extracts_csv_with_formatted_xlsx_values() {
    let xlsx = create_ooxml(
        "formatted-values.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/styles.xml",
                r#"<styleSheet><cellXfs><xf numFmtId="0"/><xf numFmtId="14"/><xf numFmtId="10"/><xf numFmtId="2"/><xf numFmtId="44"/></cellXfs></styleSheet>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1" s="0"><v>44927</v></c><c r="B1" s="1"><v>44927</v></c><c r="C1" s="2"><v>0.25</v></c><c r="D1" s="3"><f>SUM(A1:A1)</f><v>42.5</v></c><c r="E1" s="4"><v>9.5</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet",
        "Data",
        "--value-mode",
        "formatted",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(stdout(&output), "44927,2023-01-01,25.00%,42.50,$9.50\n");
}

#[test]
fn extracts_application_generated_xlsx_csv_to_stdout() {
    let xlsx = fixtures::fixture_file("xlsx/openpyxl-basic.xlsx");

    let output = oxdoc(["extract", "csv", xlsx.to_str().unwrap(), "--sheet", "Data"]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        stdout(&output),
        fixtures::read_snapshot("xlsx_openpyxl_csv.txt")
    );
}

#[test]
fn extracts_csv_by_visible_sheet_index() {
    let xlsx = create_ooxml(
        "sheet-index.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Hidden" sheetId="1" state="hidden" r:id="rId1"/><sheet name="First" sheetId="2" r:id="rId2"/><sheet name="Second" sheetId="3" r:id="rId3"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/hidden.xml"/><Relationship Id="rId2" Type="worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId3" Type="worksheet" Target="worksheets/sheet2.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/hidden.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>hidden</v></c></row></sheetData></worksheet>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>first</v></c></row></sheetData></worksheet>"#,
            ),
            (
                "xl/worksheets/sheet2.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>second</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet-index",
        "2",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(stdout(&output), "second\n");
}

#[test]
fn lists_visible_sheets() {
    let xlsx = create_ooxml(
        "list-sheets.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Hidden" sheetId="1" state="hidden" r:id="rId1"/><sheet name="Ventas Q1" sheetId="2" r:id="rId2"/><sheet name="Resumen" sheetId="3" r:id="rId3"/></sheets></workbook>"#,
            ),
        ],
    );

    let output = oxdoc(["extract", "csv", xlsx.to_str().unwrap(), "--list-sheets"]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    assert_eq!(stdout(&output), "1: Ventas Q1\n2: Resumen\n");
}

#[test]
fn rejects_list_sheets_with_sheet_selectors() {
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--list-sheets",
        "--sheet",
        "Sales Q1",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("cannot be used with"));
}

#[test]
fn rejects_list_sheets_with_multiple_inputs() {
    let first = fixtures::build_package("xlsx/basic", "first.xlsx");
    let second = fixtures::build_package("xlsx/basic", "second.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        first.to_str().unwrap(),
        second.to_str().unwrap(),
        "--list-sheets",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("--list-sheets supports a single input file"));
}

#[test]
fn rejects_text_extraction_from_xlsx_detected_by_content_types() {
    let xlsx = fixtures::build_package("xlsx/basic", "workbook.bin");

    let output = oxdoc(["extract", "text", xlsx.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("cannot extract text from an XLSX workbook"));
}

#[test]
fn multiple_csv_files_continue_after_partial_error() {
    let xlsx = fixtures::build_package("xlsx/basic", "good.xlsx");
    let missing = unique_path("missing.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        missing.to_str().unwrap(),
        xlsx.to_str().unwrap(),
        "--sheet",
        "Sales Q1",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).contains("warning[batch/W998]:"));
    assert!(stderr(&output).contains("skipped after error[E001]"));
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("xlsx_basic_csv.txt").trim_end()
    );
}

#[test]
fn writes_text_and_csv_output_to_file() {
    let docx = fixtures::build_package("docx/basic", "fixture.docx");
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");
    let text_output = unique_path("extract.txt");
    let csv_output = unique_path("extract.csv");

    let text = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        "-o",
        text_output.to_str().unwrap(),
    ]);
    let csv = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet",
        "Sales Q1",
        "-o",
        csv_output.to_str().unwrap(),
    ]);

    assert!(text.status.success());
    assert!(stdout(&text).is_empty());
    assert_eq!(
        fs::read_to_string(text_output).unwrap().trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
    assert!(csv.status.success());
    assert!(stdout(&csv).is_empty());
    assert_eq!(
        fs::read_to_string(csv_output).unwrap().trim_end(),
        fixtures::read_snapshot("xlsx_basic_csv.txt").trim_end()
    );
}

#[test]
fn rejects_invalid_delimiter() {
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--delimiter",
        "::",
    ]);

    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("delimiter must be a single-byte character"));
}

#[test]
fn rejects_invalid_sheet_index() {
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet-index",
        "0",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("sheet index must be 1 or greater"));
}

#[test]
fn rejects_conflicting_sheet_selectors() {
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet",
        "Sales Q1",
        "--sheet-index",
        "1",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("cannot be used with"));
}

#[test]
fn reports_missing_sheet_selection_as_runtime_error() {
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet",
        "Missing",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output)
            .contains("error[E003]: missing required OOXML part: visible sheet named Missing")
    );
}

#[test]
fn reports_missing_visible_sheet_index_as_runtime_error() {
    let xlsx = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet-index",
        "99",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output)
            .contains("error[E003]: missing required OOXML part: visible sheet index 99")
    );
}

#[test]
fn rejects_duplicate_visible_sheet_names_as_runtime_error() {
    let xlsx = create_ooxml(
        "duplicate-visible-sheets.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Dup" sheetId="1" r:id="rId1"/><sheet name="Dup" sheetId="2" r:id="rId2"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="worksheet" Target="worksheets/sheet2.xml"/></Relationships>"#,
            ),
            ("xl/worksheets/sheet1.xml", r#"<worksheet/>"#),
            ("xl/worksheets/sheet2.xml", r#"<worksheet/>"#),
        ],
    );

    let output = oxdoc(["extract", "csv", xlsx.to_str().unwrap(), "--sheet", "Dup"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output)
            .contains("error[E010]: invalid argument: multiple visible sheets named Dup")
    );
    assert!(stderr(&output).contains("use sheet index to disambiguate"));
}

#[test]
fn prints_info_as_json_and_text() {
    let pptx = fixtures::build_package("pptx/basic", "fixture.pptx");

    let json = oxdoc(["info", pptx.to_str().unwrap(), "--format", "json"]);
    let text = oxdoc(["info", pptx.to_str().unwrap(), "--format", "text"]);

    assert!(json.status.success());
    assert!(stderr(&json).is_empty());
    let actual_stdout = stdout(&json);
    let actual: Value = serde_json::from_str(&actual_stdout).unwrap();
    let expected_snapshot = fixtures::read_snapshot("cli_info_json.json");
    let expected: Value = serde_json::from_str(&expected_snapshot).unwrap();
    assert_eq!(actual, expected);

    assert!(text.status.success());
    assert!(stderr(&text).is_empty());
    assert_eq!(
        stdout(&text).trim_end(),
        fixtures::read_snapshot("cli_info_text.txt").trim_end()
    );
}

#[test]
fn prints_audit_as_json_and_text() {
    let workbook = create_ooxml(
        "audit.xlsm",
        &[
            (
                "[Content_Types].xml",
                r#"<Types><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/vbaProject.bin" ContentType="application/vnd.ms-office.vbaProject"/></Types>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Visible" sheetId="1" r:id="rId1"/><sheet name="Hidden" sheetId="2" state="hidden" r:id="rId2"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rLink" Type="hyperlink" TargetMode="External" Target="https://example.invalid/audit"/></Relationships>"#,
            ),
            ("xl/vbaProject.bin", "macro bytes"),
        ],
    );

    let json = oxdoc(["audit", workbook.to_str().unwrap(), "--format", "json"]);
    let text = oxdoc(["audit", workbook.to_str().unwrap(), "--format", "text"]);

    assert!(json.status.success());
    assert!(stderr(&json).is_empty());
    let actual: Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert_eq!(actual["oxdoc_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(actual["document_type"], "xlsx");
    assert_eq!(actual["metadata"]["has_macros"], true);
    assert_json_signal(&actual, "macros", "high", "VBA macro");
    assert_json_signal(&actual, "hidden_sheet", "warning", "Hidden");
    assert_json_signal(
        &actual,
        "relationship_target",
        "warning",
        "https://example.invalid/audit",
    );

    assert!(text.status.success());
    assert!(stderr(&text).is_empty());
    let text_stdout = stdout(&text);
    assert!(text_stdout.contains("document_type: xlsx"));
    assert!(text_stdout.contains("has_macros: true"));
    assert!(text_stdout.contains("signal: high macros"));
}

#[test]
fn reads_audit_from_stdin() {
    let pptx = fixtures::build_package("pptx/basic", "stdin-audit.pptx");

    let output = oxdoc_with_stdin(
        ["audit", "-", "--format", "json"],
        &fs::read(&pptx).unwrap(),
    );

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let actual: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(actual["file"], "<stdin>");
    assert_eq!(actual["document_type"], "pptx");
}

#[test]
fn reports_missing_files() {
    let missing = unique_path("missing.docx");

    let output = oxdoc(["extract", "text", missing.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("I/O error"));
}

#[test]
fn extracts_multiple_text_files_as_json_array() {
    let docx = fixtures::build_package("docx/basic", "one.docx");
    let pptx = fixtures::build_package("pptx/text", "two.pptx");

    let output = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        pptx.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let actual: Value = serde_json::from_str(&stdout(&output)).unwrap();
    let entries = actual.as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["file"], "one.docx");
    assert_eq!(
        entries[0]["text"].as_str().unwrap().trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
    assert_eq!(entries[1]["file"], "two.pptx");
    assert_eq!(
        entries[1]["text"].as_str().unwrap().trim_end(),
        fixtures::read_snapshot("pptx_text.txt").trim_end()
    );
}

#[test]
fn multiple_text_files_continue_after_partial_error() {
    let docx = fixtures::build_package("docx/basic", "good.docx");
    let missing = unique_path("missing.docx");

    let output = oxdoc([
        "extract",
        "text",
        missing.to_str().unwrap(),
        docx.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).contains("warning[batch/W998]:"));
    assert!(stderr(&output).contains("skipped after error[E001]"));
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
}

#[test]
fn extracts_multiple_csv_files_in_argument_order() {
    let first = fixtures::build_package("xlsx/basic", "first.xlsx");
    let second = fixtures::build_package("xlsx/basic", "second.xlsx");

    let output = oxdoc([
        "extract",
        "csv",
        first.to_str().unwrap(),
        second.to_str().unwrap(),
        "--sheet",
        "Sales Q1",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let expected = fixtures::read_snapshot("xlsx_basic_csv.txt");
    assert_eq!(stdout(&output), format!("{expected}{expected}"));
}

#[test]
fn exports_all_visible_xlsx_sheets_with_manifest() {
    let workbook = create_ooxml(
        "all-sheets.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rIdWorkbook" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Sales Q1" sheetId="1" r:id="rId1"/><sheet name="Ops/Q1 🚀" sheetId="2" r:id="rId2"/><sheet name="Hidden" sheetId="3" state="hidden" r:id="rId3"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="worksheet" Target="worksheets/hidden.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>sales</v></c></row></sheetData></worksheet>"#,
            ),
            (
                "xl/worksheets/sheet2.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>ops</v></c></row></sheetData></worksheet>"#,
            ),
            (
                "xl/worksheets/hidden.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>hidden</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    let output_dir = unique_path("all-sheets-out");

    let output = oxdoc([
        "extract",
        "csv",
        workbook.to_str().unwrap(),
        "--all-sheets",
        "--output-dir",
        output_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).is_empty());
    assert_eq!(
        fs::read_to_string(output_dir.join("001-sales-q1.csv")).unwrap(),
        "sales\n"
    );
    assert_eq!(
        fs::read_to_string(output_dir.join("002-ops-q1.csv")).unwrap(),
        "ops\n"
    );
    assert!(!output_dir.join("003-hidden.csv").exists());

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(output_dir.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["oxdoc_version"], env!("CARGO_PKG_VERSION"));
    assert!(
        manifest["file"]
            .as_str()
            .unwrap()
            .ends_with("all-sheets.xlsx")
    );
    let sheets = manifest["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0]["index"], 1);
    assert_eq!(sheets[0]["name"], "Sales Q1");
    assert_eq!(sheets[0]["csv_path"], "001-sales-q1.csv");
    assert_eq!(sheets[0]["warnings"].as_array().unwrap().len(), 0);
    assert_eq!(sheets[1]["index"], 2);
    assert_eq!(sheets[1]["name"], "Ops/Q1 🚀");
    assert_eq!(sheets[1]["csv_path"], "002-ops-q1.csv");
}

#[test]
fn rejects_all_sheets_without_output_dir() {
    let workbook = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let output = oxdoc(["extract", "csv", workbook.to_str().unwrap(), "--all-sheets"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("required"));
    assert!(stderr(&output).contains("--output-dir"));
}

#[test]
fn all_sheets_manifest_records_sheet_level_errors() {
    let workbook = create_ooxml(
        "all-sheets-error.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rIdWorkbook" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Good" sheetId="1" r:id="rId1"/><sheet name="Missing" sheetId="2" r:id="rId2"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="worksheet" Target="worksheets/missing.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>good</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    let output_dir = unique_path("all-sheets-error-out");

    let output = oxdoc([
        "extract",
        "csv",
        workbook.to_str().unwrap(),
        "--all-sheets",
        "--output-dir",
        output_dir.to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("sheet export(s) failed"));
    assert_eq!(
        fs::read_to_string(output_dir.join("001-good.csv")).unwrap(),
        "good\n"
    );

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(output_dir.join("manifest.json")).unwrap())
            .unwrap();
    let sheets = manifest["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0]["error"], Value::Null);
    assert_eq!(sheets[1]["index"], 2);
    assert_eq!(sheets[1]["name"], "Missing");
    assert_eq!(sheets[1]["error"]["code"], "E003");
    assert!(
        sheets[1]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("missing.xml")
    );
}

#[test]
fn all_sheets_manifest_records_sheet_warnings() {
    let workbook = create_ooxml(
        "all-sheets-warning.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rIdWorkbook" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Warn" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>kept</v></c></row><"#,
            ),
        ],
    );
    let output_dir = unique_path("all-sheets-warning-out");

    let output = oxdoc([
        "extract",
        "csv",
        workbook.to_str().unwrap(),
        "--all-sheets",
        "--output-dir",
        output_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(output_dir.join("001-warn.csv")).unwrap(),
        "kept\n"
    );
    assert!(stderr(&output).contains("warning[parser/W001]"));

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(output_dir.join("manifest.json")).unwrap())
            .unwrap();
    let warnings = manifest["sheets"][0]["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["category"], "parser");
    assert_eq!(warnings[0]["code"], "W001");
    assert_eq!(warnings[0]["path"], "xl/worksheets/sheet1.xml");
}

#[test]
fn rejects_all_sheets_with_multiple_input_files() {
    let first = fixtures::build_package("xlsx/basic", "first.xlsx");
    let second = fixtures::build_package("xlsx/basic", "second.xlsx");
    let output_dir = unique_path("all-sheets-multiple-out");

    let output = oxdoc([
        "extract",
        "csv",
        first.to_str().unwrap(),
        second.to_str().unwrap(),
        "--all-sheets",
        "--output-dir",
        output_dir.to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("--all-sheets supports a single input file"));
}

#[test]
fn reads_text_csv_and_info_from_stdin() {
    let docx = fixtures::build_package("docx/basic", "stdin.docx");
    let xlsx = fixtures::build_package("xlsx/basic", "stdin.xlsx");
    let pptx = fixtures::build_package("pptx/basic", "stdin.pptx");

    let text = oxdoc_with_stdin(["extract", "text", "-"], &fs::read(&docx).unwrap());
    let csv = oxdoc_with_stdin(
        ["extract", "csv", "-", "--sheet", "Sales Q1"],
        &fs::read(&xlsx).unwrap(),
    );
    let info = oxdoc_with_stdin(["info", "-", "--format", "json"], &fs::read(&pptx).unwrap());

    assert!(text.status.success());
    assert_eq!(
        stdout(&text).trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
    assert!(csv.status.success());
    assert_eq!(
        stdout(&csv).trim_end(),
        fixtures::read_snapshot("xlsx_basic_csv.txt").trim_end()
    );
    assert!(info.status.success());
    let actual: Value = serde_json::from_str(&stdout(&info)).unwrap();
    assert_eq!(actual["file"], "<stdin>");
}

#[test]
fn rejects_missing_required_file_argument() {
    let output = oxdoc(["extract", "text"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("the following required arguments were not provided"));
    assert!(stderr(&output).contains("Usage: oxdoc"));
    assert!(stderr(&output).contains("extract text <FILES>..."));
}

#[test]
fn reports_suspicious_relationship_targets_on_stderr() {
    let docx = fixtures::build_package("docx/external-target", "fixture.docx");

    let output = oxdoc(["extract", "text", docx.to_str().unwrap()]);

    assert!(!output.status.success());
    assert_eq!(
        stderr(&output).trim_end(),
        format!(
            "error[E007]: {}",
            fixtures::read_snapshot("docx_external_target_error.txt").trim_end()
        )
    );
}

#[test]
fn fixture_provenance_notes_are_present() {
    for provenance in [
        "docx-basic.md",
        "docx-python-docx-basic.md",
        "xlsx-basic.md",
        "xlsx-app-metadata.md",
        "xlsx-openpyxl-basic.md",
        "pptx-basic.md",
        "pptx-text.md",
        "pptx-python-pptx-basic.md",
        "docx-external-target.md",
    ] {
        let note = fixtures::read_provenance(provenance);
        assert!(note.contains("Source:"), "{provenance}");
        assert!(note.contains("Producer:"), "{provenance}");
        assert!(note.contains("Redistribution:"), "{provenance}");
        assert!(note.contains("Purpose:"), "{provenance}");
    }
}

#[test]
fn suppresses_warnings_with_quiet_flag() {
    let docx = create_ooxml(
        "quiet-warning.docx",
        &[(
            "docProps/core.xml",
            r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc"><dc:creator>Ada</dc:creator><"#,
        )],
    );

    let output = oxdoc(["-q", "info", docx.to_str().unwrap(), "--format", "json"]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let actual: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(actual["author"], "Ada");
}

#[test]
fn keeps_csv_stdout_clean_when_xlsx_warnings_are_emitted() {
    let xlsx = create_ooxml(
        "xlsx-warnings.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet sheetId="1" r:id="ignored"/><sheet name="Data" sheetId="2" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>kept</v></c></row><"#,
            ),
        ],
    );

    let output = oxdoc(["extract", "csv", xlsx.to_str().unwrap(), "--sheet", "Data"]);

    assert!(output.status.success());
    assert_eq!(stdout(&output), "kept\n");
    assert!(
        stderr(&output).contains("warning[data/W002]: xl/workbook.xml: ignored workbook sheet")
    );
    assert!(
        stderr(&output).contains(
            "warning[parser/W001]: xl/worksheets/sheet1.xml: stopped after malformed XML"
        )
    );
}

#[test]
fn keeps_info_json_stdout_clean_when_metadata_warnings_are_emitted() {
    let docx = create_ooxml(
        "metadata-warning.docx",
        &[(
            "docProps/core.xml",
            r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc"><dc:creator>Ada</dc:creator><"#,
        )],
    );

    let output = oxdoc(["info", docx.to_str().unwrap(), "--format", "json"]);

    assert!(output.status.success());
    let actual_stdout = stdout(&output);
    let actual: Value = serde_json::from_str(&actual_stdout).unwrap();
    assert!(
        actual["file"]
            .as_str()
            .unwrap()
            .ends_with("metadata-warning.docx")
    );
    assert_eq!(actual["author"], "Ada");
    assert_eq!(actual["has_macros"], false);
    assert!(!actual_stdout.contains("warning["));
    assert!(
        stderr(&output)
            .contains("warning[parser/W001]: docProps/core.xml: stopped after malformed XML")
    );
}

#[test]
fn keeps_json_output_clean_when_warnings_are_emitted() {
    let docx = create_ooxml(
        "malformed-json.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Hello JSON</w:t></w:r></w:p><"#,
            ),
        ],
    );

    let output = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert!(output.status.success());
    let actual_stdout = stdout(&output);
    let actual: Value = serde_json::from_str(&actual_stdout).unwrap();
    assert_eq!(actual["text"], "Hello JSON\n");
    assert!(!actual_stdout.contains("warning["));
    assert!(
        stderr(&output)
            .contains("warning[parser/W001]: word/document.xml: stopped after malformed XML")
    );
}

#[test]
fn includes_structured_warnings_in_jsonl_records() {
    let docx = create_ooxml(
        "malformed-jsonl-warning.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Hello JSONL</w:t></w:r></w:p><"#,
            ),
        ],
    );

    let output = oxdoc([
        "--warnings",
        "none",
        "extract",
        "text",
        docx.to_str().unwrap(),
        "--format",
        "jsonl",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let records = jsonl_lines(&output);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["text"], "Hello JSONL\n");
    let warnings = records[0]["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["category"], "parser");
    assert_eq!(warnings[0]["code"], "W001");
    assert_eq!(warnings[0]["path"], "word/document.xml");
}

#[test]
fn emits_jsonl_error_record_for_xlsx_text_extraction() {
    let xlsx = fixtures::build_package("xlsx/basic", "text-jsonl.xlsx");

    let output = oxdoc([
        "extract",
        "text",
        xlsx.to_str().unwrap(),
        "--format",
        "jsonl",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let records = jsonl_lines(&output);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["document_type"], "xlsx");
    assert_eq!(records[0]["error"]["code"], "E010");
    assert!(
        records[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cannot extract text from an XLSX workbook")
    );
}

#[test]
fn emits_machine_readable_json_warnings() {
    let docx = create_ooxml(
        "malformed-json-warning.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Hello JSON</w:t></w:r></w:p><"#,
            ),
        ],
    );

    let output = oxdoc([
        "--warnings",
        "json",
        "extract",
        "text",
        docx.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert!(output.status.success());
    let actual_stdout = stdout(&output);
    let actual: Value = serde_json::from_str(&actual_stdout).unwrap();
    assert_eq!(actual["text"], "Hello JSON\n");

    let warnings = json_warning_lines(&output);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["category"], "parser");
    assert_eq!(warnings[0]["code"], "W001");
    assert_eq!(warnings[0]["path"], "word/document.xml");
    assert!(
        warnings[0]["message"]
            .as_str()
            .unwrap()
            .starts_with("stopped after malformed XML")
    );
}

#[test]
fn emits_machine_readable_batch_skip_warnings() {
    let docx = fixtures::build_package("docx/basic", "good.docx");
    let missing = unique_path("missing-json-warning.docx");

    let output = oxdoc([
        "--warnings",
        "json",
        "extract",
        "text",
        missing.to_str().unwrap(),
        docx.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output).trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );

    let warnings = json_warning_lines(&output);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["category"], "batch");
    assert_eq!(warnings[0]["code"], "W998");
    assert!(
        warnings[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("missing-json-warning.docx")
    );
    assert!(
        warnings[0]["message"]
            .as_str()
            .unwrap()
            .contains("skipped after error[E001]")
    );
}

#[test]
fn suppresses_warnings_with_warning_format_none() {
    let docx = create_ooxml(
        "none-warning.docx",
        &[(
            "docProps/core.xml",
            r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc"><dc:creator>Ada</dc:creator><"#,
        )],
    );

    let output = oxdoc([
        "--warnings",
        "none",
        "info",
        docx.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let actual: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(actual["author"], "Ada");
}

fn oxdoc<const N: usize>(args: [&str; N]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_oxdoc"))
        .args(args)
        .output()
        .unwrap()
}

fn oxdoc_with_stdin<const N: usize>(args: [&str; N], stdin: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_oxdoc"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin_pipe = child.stdin.take().unwrap();
    stdin_pipe.write_all(stdin).unwrap();
    drop(stdin_pipe);

    child.wait_with_output().unwrap()
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn json_warning_lines(output: &std::process::Output) -> Vec<Value> {
    stderr(output)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn jsonl_lines(output: &std::process::Output) -> Vec<Value> {
    stdout(output)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn assert_json_signal(output: &Value, kind: &str, severity: &str, message_contains: &str) {
    let signals = output["signals"].as_array().unwrap();
    assert!(
        signals.iter().any(|signal| {
            signal["kind"] == kind
                && signal["severity"] == severity
                && signal["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(message_contains))
        }),
        "missing JSON signal kind={kind} severity={severity} containing {message_contains:?}: {signals:#?}"
    );
}

fn create_ooxml(name: &str, entries: &[(&str, &str)]) -> PathBuf {
    let path = unique_path(name);
    let file = File::create(&path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    for (entry_name, content) in entries {
        zip.start_file(entry_name, options).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }

    zip.finish().unwrap();
    path
}

fn unique_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("oxdoc-cli-{}-{nonce}-{name}", std::process::id()))
}

#[test]
fn update_check_only_runs_successfully() {
    // Just verify the command doesn't crash
    let output = oxdoc(["update", "--check"]);
    assert!(output.status.success());
}

#[test]
fn update_with_current_version_runs_successfully() {
    let current = format!("v{}", env!("CARGO_PKG_VERSION"));
    let output = oxdoc(["update", "--version", &current]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("already up to date"));
}

#[test]
fn update_check_only_reports_available_version() {
    let output = oxdoc(["update", "--check", "--version", "v99.99.99"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Update available"));
    assert!(stdout(&output).contains("Run `oxdoc update` to install."));
}

#[test]
fn update_with_invalid_version_fails_gracefully() {
    let output = oxdoc(["update", "--version", "v99.99.99-nonexistent"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("download failed") || stderr(&output).contains("error[E013]"));
}
