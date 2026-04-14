use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use oxdoc_core::vfs::OoxmlPackage;
use oxdoc_core::{OxdocError, XlsxCsvOptions};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

#[test]
fn extracts_docx_text_through_public_api() {
    let file = create_ooxml(
        "public-api.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="custom/main.xml"/></Relationships>"#,
            ),
            (
                "custom/main.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Hello &amp; API</w:t></w:r></w:p></w:body></w:document>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(extraction.value, "Hello & API\n");
    assert!(extraction.warnings.is_empty());
}

#[test]
fn extracts_xlsx_csv_through_public_api() {
    let file = create_ooxml(
        "public-api.xlsx",
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
                "xl/sharedStrings.xml",
                r#"<sst><si><t>name</t></si><si><t>Ada</t></si></sst>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1" t="s"><v>0</v></c><c r="B1" t="inlineStr"><is><t>score</t></is></c></row><row><c r="A2" t="s"><v>1</v></c><c r="B2"><v>42</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    let mut csv = Vec::new();

    let extraction = oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Data"),
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(String::from_utf8(csv).unwrap(), "name,score\nAda,42\n");
}

#[test]
fn extracts_xlsx_csv_without_shared_strings() {
    let file = create_ooxml(
        "no-shared-strings.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Inline" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1" t="inlineStr"><is><t>inline</t></is></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    let mut csv = Vec::new();

    oxdoc_core::extract_xlsx_csv(&file, XlsxCsvOptions::default(), &mut csv).unwrap();

    assert_eq!(String::from_utf8(csv).unwrap(), "inline\n");
}

#[test]
fn reads_metadata_through_public_api() {
    let file = create_ooxml(
        "public-api.docm",
        &[
            (
                "docProps/core.xml",
                r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc" xmlns:dcterms="dcterms"><dc:creator>Ada &amp; Linus</dc:creator><cp:lastModifiedBy>Grace</cp:lastModifiedBy><dcterms:created>2024-03-12T10:00:00Z</dcterms:created><dcterms:modified>2024-03-13T10:00:00Z</dcterms:modified><cp:revision>4</cp:revision></cp:coreProperties>"#,
            ),
            (
                "docProps/app.xml",
                r#"<Properties><Application>TestOffice</Application><Company>Example Inc</Company><Words>1542</Words><Pages>12</Pages><Slides>3</Slides><Worksheets>2</Worksheets></Properties>"#,
            ),
            ("word/vbaProject.bin", "not a real macro project"),
        ],
    );

    let extraction = oxdoc_core::read_info(&file).unwrap();
    let info = extraction.value;

    assert!(info.file.ends_with("public-api.docm"));
    assert_eq!(info.author.as_deref(), Some("Ada & Linus"));
    assert_eq!(info.last_modified_by.as_deref(), Some("Grace"));
    assert_eq!(info.created_at.as_deref(), Some("2024-03-12T10:00:00Z"));
    assert_eq!(info.modified_at.as_deref(), Some("2024-03-13T10:00:00Z"));
    assert_eq!(info.application.as_deref(), Some("TestOffice"));
    assert_eq!(info.company.as_deref(), Some("Example Inc"));
    assert_eq!(info.word_count, Some(1542));
    assert_eq!(info.page_count, Some(12));
    assert_eq!(info.slide_count, Some(3));
    assert_eq!(info.worksheet_count, Some(2));
    assert_eq!(info.revision.as_deref(), Some("4"));
    assert!(info.has_macros);
    assert!(serde_json::to_string(&info).unwrap().contains("has_macros"));
}

#[test]
fn reports_missing_zip_entry_through_vfs() {
    let file = File::open(create_ooxml("missing-entry.docx", &[])).unwrap();
    let mut package = OoxmlPackage::new(file).unwrap();

    let err = package.read_to_string("word/document.xml").unwrap_err();

    assert!(matches!(err, OxdocError::MissingPart(part) if part == "word/document.xml"));
}

#[test]
fn rejects_non_zip_files() {
    let file = create_plain_file("not-ooxml.docx", "not a zip");

    let err = oxdoc_core::extract_docx_text(&file).unwrap_err();

    assert!(matches!(err, OxdocError::CorruptedZip(_)));
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

fn create_plain_file(name: &str, content: &str) -> PathBuf {
    let path = unique_path(name);
    let mut file = File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

fn unique_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("oxdoc-core-{}-{nonce}-{name}", std::process::id()))
}
