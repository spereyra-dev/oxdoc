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
    let expected_snapshot = fixtures::read_snapshot("pptx_basic_info.json");
    let expected: Value = serde_json::from_str(&expected_snapshot).unwrap();
    assert_eq!(actual["oxdoc_version"], env!("CARGO_PKG_VERSION"));
    let actual_without_version = actual
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, _)| key.as_str() != "oxdoc_version")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<String, Value>>();
    assert_eq!(Value::Object(actual_without_version), expected);

    assert!(text.status.success());
    assert!(stderr(&text).is_empty());
    assert_eq!(
        stdout(&text).trim_end(),
        fixtures::read_snapshot("cli_info_text.txt").trim_end()
    );
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
