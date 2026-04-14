use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

#[test]
fn extracts_text_to_stdout() {
    let docx = sample_docx();

    let output = oxdoc(["extract", "text", docx.to_str().unwrap()]);

    assert!(output.status.success());
    assert_eq!(stdout(output), "Hello CLI\n");
}

#[test]
fn extracts_text_as_json() {
    let docx = sample_docx();

    let output = oxdoc([
        "extract",
        "text",
        docx.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = stdout(output);

    assert!(stdout.contains("sample.docx"));
    assert!(stdout.contains(r#""text": "Hello CLI\n""#));
}

#[test]
fn extracts_csv_to_stdout() {
    let xlsx = sample_xlsx();

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--sheet",
        "Ventas Q1",
        "--delimiter",
        ";",
    ]);

    assert!(output.status.success());
    assert_eq!(stdout(output), "id;Cliente A\n1;5000\n");
}

#[test]
fn rejects_invalid_delimiter() {
    let xlsx = sample_xlsx();

    let output = oxdoc([
        "extract",
        "csv",
        xlsx.to_str().unwrap(),
        "--delimiter",
        "::",
    ]);

    assert!(!output.status.success());
    assert!(stderr(output).contains("delimiter must be a single-byte character"));
}

#[test]
fn prints_info_as_json_and_text() {
    let docx = sample_docx();

    let json = oxdoc(["info", docx.to_str().unwrap(), "--format", "json"]);
    let text = oxdoc(["info", docx.to_str().unwrap(), "--format", "text"]);

    assert!(json.status.success());
    assert!(stdout(json).contains(r#""author": "Ada""#));
    assert!(text.status.success());
    assert!(stdout(text).contains("author: Ada"));
}

#[test]
fn reports_missing_files() {
    let missing = unique_path("missing.docx");

    let output = oxdoc(["extract", "text", missing.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(stderr(output).contains("I/O error"));
}

fn oxdoc<const N: usize>(args: [&str; N]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_oxdoc"))
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: std::process::Output) -> String {
    String::from_utf8(output.stdout).unwrap()
}

fn stderr(output: std::process::Output) -> String {
    String::from_utf8(output.stderr).unwrap()
}

fn sample_docx() -> PathBuf {
    create_ooxml(
        "sample.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Hello CLI</w:t></w:r></w:p></w:body></w:document>"#,
            ),
            (
                "docProps/core.xml",
                r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc" xmlns:dcterms="dcterms"><dc:creator>Ada</dc:creator><dcterms:created>2024-03-12T10:00:00Z</dcterms:created></cp:coreProperties>"#,
            ),
            (
                "docProps/app.xml",
                r#"<Properties><Application>TestOffice</Application><Words>2</Words></Properties>"#,
            ),
        ],
    )
}

fn sample_xlsx() -> PathBuf {
    create_ooxml(
        "sample.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Ventas Q1" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/sharedStrings.xml",
                r#"<sst><si><t>id</t></si><si><t>Cliente A</t></si></sst>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row><row><c r="A2"><v>1</v></c><c r="B2"><v>5000</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    )
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
