use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
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
fn prints_info_as_json_and_text() {
    let pptx = fixtures::build_package("pptx/basic", "fixture.pptx");

    let json = oxdoc(["info", pptx.to_str().unwrap(), "--format", "json"]);
    let text = oxdoc(["info", pptx.to_str().unwrap(), "--format", "text"]);

    assert!(json.status.success());
    assert!(stderr(&json).is_empty());
    let actual_stdout = stdout(&json);
    let expected_snapshot = fixtures::read_snapshot("pptx_basic_info.json");
    let actual: Value = serde_json::from_str(&actual_stdout).unwrap();
    let expected: Value = serde_json::from_str(&expected_snapshot).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual_stdout.trim_end(), expected_snapshot.trim_end());

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
fn rejects_missing_required_file_argument() {
    let output = oxdoc(["extract", "text"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("the following required arguments were not provided"));
    assert!(stderr(&output).contains("Usage: oxdoc"));
    assert!(stderr(&output).contains("extract text <FILE>"));
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
    let note = fixtures::read_provenance("docx-basic.md");

    assert!(note.contains("Source:"));
    assert!(note.contains("Redistribution:"));
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
    assert!(stdout(&output).contains(r#""text": "Hello JSON\n""#));
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
