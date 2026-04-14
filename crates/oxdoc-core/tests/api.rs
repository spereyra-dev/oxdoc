use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use oxdoc_core::vfs::{OoxmlLimits, OoxmlPackage};
use oxdoc_core::{OxdocError, XlsxCsvOptions};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;

#[test]
fn extracts_docx_text_through_public_api() {
    let file = fixtures::build_package("docx/basic", "fixture.docx");

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(
        extraction.value.trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn extracts_xlsx_csv_through_public_api() {
    let file = fixtures::build_package("xlsx/basic", "fixture.xlsx");
    let mut csv = Vec::new();

    let extraction = oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Sales Q1"),
            sheet_index: None,
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(
        String::from_utf8(csv).unwrap().trim_end(),
        fixtures::read_snapshot("xlsx_basic_csv.txt").trim_end()
    );
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
fn extracts_xlsx_csv_with_boolean_error_blank_and_empty_row_cells() {
    let file = create_ooxml(
        "mixed-cells.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Mixed" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/sharedStrings.xml",
                r#"<sst><si><t>shared</t></si></sst>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="b"><v>1</v></c><c r="C1" t="e"><v>#DIV/0!</v></c><c r="D1" t="inlineStr"><is><t>inline</t></is></c><c r="E1"/></row><row r="2"/></sheetData></worksheet>"#,
            ),
        ],
    );
    let mut csv = Vec::new();

    oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Mixed"),
            sheet_index: None,
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(csv).unwrap(),
        "shared,TRUE,#DIV/0!,inline,\n\n"
    );
}

#[test]
fn reports_missing_requested_xlsx_sheet() {
    let file = create_ooxml(
        "missing-sheet.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Present" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            ("xl/worksheets/sheet1.xml", r#"<worksheet/>"#),
        ],
    );
    let mut csv = Vec::new();

    let err = oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Missing"),
            sheet_index: None,
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap_err();

    assert!(matches!(err, OxdocError::MissingPart(part) if part == "visible sheet named Missing"));
}

#[test]
fn extracts_xlsx_csv_by_visible_sheet_index() {
    let file = create_ooxml(
        "sheet-index.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Hidden" sheetId="1" state="hidden" r:id="rId1"/><sheet name="First Visible" sheetId="2" r:id="rId2"/><sheet name="Second Visible" sheetId="3" r:id="rId3"/></sheets></workbook>"#,
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
    let mut csv = Vec::new();

    oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: None,
            sheet_index: Some(2),
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap();

    assert_eq!(String::from_utf8(csv).unwrap(), "second\n");
}

#[test]
fn reports_invalid_xlsx_sheet_selection_combinations() {
    let file = create_ooxml(
        "duplicate-sheets.xlsx",
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
    let mut csv = Vec::new();

    let err = oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Dup"),
            sheet_index: None,
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap_err();
    assert!(
        matches!(err, OxdocError::InvalidArgument(message) if message.contains("multiple visible sheets named Dup"))
    );

    let err = oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Dup"),
            sheet_index: Some(1),
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap_err();
    assert!(
        matches!(err, OxdocError::InvalidArgument(message) if message.contains("by name or index"))
    );
}

#[test]
fn reads_metadata_through_public_api() {
    let file = fixtures::build_package("pptx/basic", "fixture.pptx");

    let extraction = oxdoc_core::read_info(&file).unwrap();

    assert_eq!(
        serde_json::to_string_pretty(&extraction.value).unwrap(),
        fixtures::read_snapshot("pptx_basic_info.json").trim_end()
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn reports_missing_zip_entry_through_vfs() {
    let file = File::open(create_ooxml("missing-entry.docx", &[])).unwrap();
    let mut package = OoxmlPackage::new(file).unwrap();

    let err = package.read_to_string("word/document.xml").unwrap_err();

    assert_eq!(err.code().as_str(), "E003");
    assert!(matches!(err, OxdocError::MissingPart(part) if part == "word/document.xml"));
}

#[test]
fn rejects_non_zip_files() {
    let file = create_plain_file("not-ooxml.docx", "not a zip");

    let err = oxdoc_core::extract_docx_text(&file).unwrap_err();

    assert_eq!(err.code().as_str(), "E002");
    assert!(matches!(err, OxdocError::CorruptedZip(_)));
}

#[test]
fn rejects_oversized_zip_entries_before_reading() {
    let file = File::open(create_ooxml(
        "oversized-part.docx",
        &[("word/document.xml", "0123456789")],
    ))
    .unwrap();
    let mut package = OoxmlPackage::with_limits(
        file,
        OoxmlLimits {
            max_part_uncompressed_size: 5,
            ..OoxmlLimits::default()
        },
    )
    .unwrap();

    let err = package.read_to_string("word/document.xml").unwrap_err();

    assert!(matches!(
        err,
        OxdocError::PartTooLarge {
            path,
            size: 10,
            limit: 5
        } if path == "word/document.xml"
    ));
}

#[test]
fn rejects_zip_bomb_like_compression_ratios() {
    let repeated_xml = "<worksheet>".repeat(512);
    let file = File::open(create_ooxml_with_method(
        "suspicious-ratio.xlsx",
        &[("xl/worksheets/sheet1.xml", repeated_xml.as_str())],
        CompressionMethod::Deflated,
    ))
    .unwrap();
    let mut package = OoxmlPackage::with_limits(
        file,
        OoxmlLimits {
            max_part_uncompressed_size: 64 * 1024,
            max_part_compression_ratio: 2,
            min_ratio_check_size: 1,
        },
    )
    .unwrap();

    let err = package
        .read_to_string("xl/worksheets/sheet1.xml")
        .unwrap_err();

    assert!(matches!(
        err,
        OxdocError::SuspiciousZipEntry { path, .. } if path == "xl/worksheets/sheet1.xml"
    ));
}

#[test]
fn reports_encrypted_zip_entries_as_unsupported() {
    let path = create_ooxml(
        "encrypted-flag.docx",
        &[("word/document.xml", "<w:document/>")],
    );
    mark_first_entry_encrypted(&path);
    let file = File::open(path).unwrap();
    let mut package = OoxmlPackage::new(file).unwrap();

    let err = package.read_to_string("word/document.xml").unwrap_err();

    assert!(matches!(
        err,
        OxdocError::UnsupportedEncryptedPart(path) if path == "word/document.xml"
    ));
}

#[test]
fn rejects_required_parts_that_resolve_to_directories() {
    let file = File::open(create_ooxml_directory(
        "directory-part.docx",
        "word/document.xml/",
    ))
    .unwrap();
    let mut package = OoxmlPackage::new(file).unwrap();

    let err = package.read_to_string("word/document.xml/").unwrap_err();

    assert!(matches!(
        err,
        OxdocError::SuspiciousZipEntry { path, reason }
            if path == "word/document.xml/" && reason.contains("directory")
    ));
}

#[test]
fn rejects_zip_entries_not_enclosed_in_the_package() {
    let file = File::open(create_ooxml(
        "unsafe-entry-name.docx",
        &[("../word/document.xml", "<w:document/>")],
    ))
    .unwrap();
    let mut package = OoxmlPackage::new(file).unwrap();

    let err = package.read_to_string("../word/document.xml").unwrap_err();

    assert!(matches!(
        err,
        OxdocError::SuspiciousZipEntry { path, reason }
            if path == "../word/document.xml" && reason.contains("not enclosed")
    ));
}

#[test]
fn rejects_external_root_relationship_targets() {
    let file = fixtures::build_package("docx/external-target", "fixture.docx");

    let err = oxdoc_core::extract_docx_text(&file).unwrap_err();

    assert_eq!(
        err.to_string(),
        fixtures::read_snapshot("docx_external_target_error.txt").trim_end()
    );
}

#[test]
fn rejects_workbook_relationship_targets_that_escape_package_root() {
    let file = create_ooxml(
        "escaping-sheet-target.xlsx",
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
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="../../outside.xml"/></Relationships>"#,
            ),
        ],
    );
    let mut csv = Vec::new();

    let err = oxdoc_core::extract_xlsx_csv(&file, XlsxCsvOptions::default(), &mut csv).unwrap_err();

    assert!(matches!(
        err,
        OxdocError::SuspiciousRelationshipTarget { path, target, .. }
            if path == "xl/_rels/workbook.xml.rels" && target == "../../outside.xml"
    ));
}

#[test]
fn fixture_provenance_notes_are_present() {
    for provenance in [
        "docx-basic.md",
        "xlsx-basic.md",
        "pptx-basic.md",
        "docx-external-target.md",
    ] {
        let note = fixtures::read_provenance(provenance);
        assert!(note.contains("Source:"));
        assert!(note.contains("Redistribution:"));
    }
}

#[test]
fn fuzz_entry_points_parse_minimal_xml_inputs() {
    oxdoc_core::fuzz_docx_text(
        br#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Fuzz</w:t></w:r></w:p></w:body></w:document>"#,
    )
    .unwrap();
    oxdoc_core::fuzz_relationships(
        br#"<Relationships><Relationship Id="rId1" Type="officeDocument" Target="word/document.xml"/></Relationships>"#,
    )
    .unwrap();
    oxdoc_core::fuzz_metadata(
        br#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc"><dc:creator>Ada</dc:creator></cp:coreProperties>"#,
    )
    .unwrap();
    oxdoc_core::fuzz_parse_shared_strings(br#"<sst><si><t>value</t></si></sst>"#).unwrap();
    oxdoc_core::fuzz_parse_sheet(
        br#"<worksheet><sheetData><row><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
    )
    .unwrap();
}

fn create_ooxml(name: &str, entries: &[(&str, &str)]) -> PathBuf {
    create_ooxml_with_method(name, entries, CompressionMethod::Stored)
}

fn create_ooxml_with_method(
    name: &str,
    entries: &[(&str, &str)],
    compression_method: CompressionMethod,
) -> PathBuf {
    let path = unique_path(name);
    let file = File::create(&path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(compression_method);

    for (entry_name, content) in entries {
        zip.start_file(entry_name, options).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }

    zip.finish().unwrap();
    path
}

fn create_ooxml_directory(name: &str, entry_name: &str) -> PathBuf {
    let path = unique_path(name);
    let file = File::create(&path).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.add_directory(entry_name, SimpleFileOptions::default())
        .unwrap();
    zip.finish().unwrap();
    path
}

fn mark_first_entry_encrypted(path: &PathBuf) {
    let mut bytes = fs::read(path).unwrap();
    set_zip_encryption_flag(&mut bytes, &[0x50, 0x4b, 0x03, 0x04], 6);
    set_zip_encryption_flag(&mut bytes, &[0x50, 0x4b, 0x01, 0x02], 8);
    fs::write(path, bytes).unwrap();
}

fn set_zip_encryption_flag(bytes: &mut [u8], signature: &[u8; 4], flag_offset: usize) {
    let position = bytes
        .windows(signature.len())
        .position(|window| window == signature)
        .unwrap();
    bytes[position + flag_offset] |= 0x01;
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
