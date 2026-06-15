use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use oxdoc_core::vfs::{OoxmlLimits, OoxmlPackage};
use oxdoc_core::{
    DocumentType, OxdocError, XlsxCellValue, XlsxCsvOptions, XlsxRowControl, XlsxSheetOptions,
    XlsxSheetVisibility, XlsxValueMode,
};
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
fn extracts_docx_text_from_read_seek_reader() {
    let file = fixtures::build_package("docx/basic", "fixture.docx");
    let bytes = fs::read(file).unwrap();

    let extraction = oxdoc_core::extract_docx_text_from_reader(Cursor::new(bytes)).unwrap();

    assert_eq!(
        extraction.value.trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn extracts_application_generated_docx_text_fixture() {
    let file = fixtures::fixture_file("docx/python-docx-basic.docx");

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(
        extraction.value.trim_end(),
        fixtures::read_snapshot("docx_python_docx_text.txt").trim_end()
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn extracts_docx_text_from_related_parts_in_relationship_order() {
    let file = create_ooxml(
        "docx-related-parts.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w" xmlns:r="r"><w:body><w:p><w:r><w:t>Body </w:t></w:r><w:hyperlink r:id="rLink"><w:r><w:t>visible link</w:t></w:r></w:hyperlink></w:p></w:body></w:document>"#,
            ),
            (
                "word/_rels/document.xml.rels",
                r#"<Relationships><Relationship Id="rFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="rFootnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/><Relationship Id="rEndnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes" Target="endnotes.xml"/><Relationship Id="rComments" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/><Relationship Id="rLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External" Target="https://example.invalid"/></Relationships>"#,
            ),
            (
                "word/footer1.xml",
                r#"<w:ftr xmlns:w="w"><w:p><w:r><w:t>Footer text</w:t></w:r></w:p></w:ftr>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:p><w:r><w:t>Header text</w:t></w:r></w:p></w:hdr>"#,
            ),
            (
                "word/footnotes.xml",
                r#"<w:footnotes xmlns:w="w"><w:footnote w:id="1"><w:p><w:r><w:t>Footnote text</w:t></w:r></w:p></w:footnote></w:footnotes>"#,
            ),
            (
                "word/endnotes.xml",
                r#"<w:endnotes xmlns:w="w"><w:endnote w:id="1"><w:p><w:r><w:t>Endnote text</w:t></w:r></w:p></w:endnote></w:endnotes>"#,
            ),
            (
                "word/comments.xml",
                r#"<w:comments xmlns:w="w"><w:comment w:id="1"><w:p><w:r><w:t>Comment text</w:t></w:r></w:p></w:comment></w:comments>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(
        extraction.value,
        "Body visible link\nFooter text\nHeader text\nFootnote text\nEndnote text\nComment text\n"
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn warns_on_missing_docx_related_part() {
    let file = create_ooxml(
        "docx-missing-related-part.docx",
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
                r#"<Relationships><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="missing-header.xml"/></Relationships>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(extraction.value, "Body\n");
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(extraction.warnings[0].path, "word/_rels/document.xml.rels");
    assert!(
        extraction.warnings[0]
            .message
            .contains("skipped related DOCX text part word/missing-header.xml")
    );
}

#[test]
fn keeps_partial_related_docx_text_and_warns_on_malformed_part() {
    let file = create_ooxml(
        "docx-malformed-related-part.docx",
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
                r#"<Relationships><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:p><w:r><w:t>Header before break</w:t></w:r></w:p><"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(extraction.value, "Body\nHeader before break\n");
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(extraction.warnings[0].path, "word/header1.xml");
    assert_eq!(extraction.warnings[0].code().as_str(), "W001");
}

#[test]
fn rejects_external_docx_related_part_targets() {
    let file = create_ooxml(
        "docx-external-related-part.docx",
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
                r#"<Relationships><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" TargetMode="External" Target="https://example.invalid/header.xml"/></Relationships>"#,
            ),
        ],
    );

    let err = oxdoc_core::extract_docx_text(&file).unwrap_err();

    assert!(
        matches!(err, OxdocError::SuspiciousRelationshipTarget { path, target, reason }
            if path == "word/_rels/document.xml.rels"
                && target == "https://example.invalid/header.xml"
                && reason.contains("external"))
    );
}

#[test]
fn extracts_pptx_text_through_public_api() {
    let file = fixtures::build_package("pptx/text", "fixture.pptx");

    let extraction = oxdoc_core::extract_pptx_text(&file).unwrap();

    assert_eq!(
        extraction.value.trim_end(),
        fixtures::read_snapshot("pptx_text.txt").trim_end()
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn extracts_pptx_text_from_read_seek_reader() {
    let file = fixtures::build_package("pptx/text", "fixture.pptx");
    let bytes = fs::read(file).unwrap();

    let extraction = oxdoc_core::extract_pptx_text_from_reader(Cursor::new(bytes)).unwrap();

    assert_eq!(
        extraction.value.trim_end(),
        fixtures::read_snapshot("pptx_text.txt").trim_end()
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn extracts_application_generated_pptx_text_fixture() {
    let file = fixtures::fixture_file("pptx/python-pptx-basic.pptx");

    let extraction = oxdoc_core::extract_pptx_text(&file).unwrap();

    assert_eq!(
        extraction.value.trim_end(),
        fixtures::read_snapshot("pptx_python_pptx_text.txt").trim_end()
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn keeps_partial_pptx_text_and_warns_on_malformed_slide_xml() {
    let file = create_ooxml(
        "malformed-slide.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId r:id="rId1"/><p:sldId r:id="rId2"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/><Relationship Id="rId2" Type="slide" Target="slides/slide2.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>before break</a:t></a:r><"#,
            ),
            (
                "ppt/slides/slide2.xml",
                r#"<p:sld><a:p><a:r><a:t>after break</a:t></a:r></a:p></p:sld>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_text(&file).unwrap();

    assert_eq!(extraction.value, "before break\nafter break\n");
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(extraction.warnings[0].path, "ppt/slides/slide1.xml");
    assert_eq!(extraction.warnings[0].code().as_str(), "W001");
}

#[test]
fn keeps_partial_docx_text_and_warns_on_malformed_document_xml() {
    let file = create_ooxml(
        "malformed-document.docx",
        &[(
            "word/document.xml",
            r#"<w:document><w:p><w:r><w:t>before break</w:t></w:r></w:p><"#,
        )],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(extraction.value, "before break\n");
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(extraction.warnings[0].path, "word/document.xml");
    assert_eq!(extraction.warnings[0].code().as_str(), "W001");
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
            include_hidden: false,
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
fn extracts_xlsx_csv_from_read_seek_reader() {
    let file = fixtures::build_package("xlsx/basic", "fixture.xlsx");
    let reader = File::open(file).unwrap();
    let mut csv = Vec::new();

    let extraction = oxdoc_core::extract_xlsx_csv_from_reader(
        reader,
        XlsxCsvOptions {
            sheet_name: Some("Sales Q1"),
            sheet_index: None,
            include_hidden: false,
            delimiter: b';',
        },
        &mut csv,
    )
    .unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(
        String::from_utf8(csv).unwrap().trim_end(),
        fixtures::read_snapshot("cli_extract_csv.txt").trim_end()
    );
}

#[test]
fn extracts_application_generated_xlsx_csv_fixture() {
    let file = fixtures::fixture_file("xlsx/openpyxl-basic.xlsx");
    let mut csv = Vec::new();

    let extraction = oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Data"),
            sheet_index: None,
            include_hidden: false,
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(
        String::from_utf8(csv).unwrap(),
        fixtures::read_snapshot("xlsx_openpyxl_csv.txt")
    );
}

#[test]
fn extracts_formatted_xlsx_csv_through_public_api() {
    let file = create_ooxml(
        "formatted-api.xlsx",
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
                r#"<styleSheet><cellXfs><xf numFmtId="14"/><xf numFmtId="10"/></cellXfs></styleSheet>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1" s="0"><v>44927</v></c><c r="B1" s="1"><v>0.5</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    let mut csv = Vec::new();

    let extraction = oxdoc_core::extract_xlsx_csv_with_value_mode(
        &file,
        XlsxCsvOptions::default(),
        XlsxValueMode::Formatted,
        &mut csv,
    )
    .unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(String::from_utf8(csv).unwrap(), "2023-01-01,50.00%\n");
}

#[test]
fn visits_typed_xlsx_rows_through_path_api() {
    let file = create_ooxml(
        "typed-rows-path.xlsx",
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
                r#"<styleSheet><cellXfs><xf numFmtId="14"/></cellXfs></styleSheet>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row r="3"><c r="A3"/><c r="C3" t="b"><v>1</v></c><c r="D3" t="e"><v>#N/A</v></c><c r="E3" s="0"><f>TODAY()</f><v>44927</v></c><c r="C3" t="inlineStr"><is><t>last</t></is></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    let mut rows = Vec::new();

    let extraction = oxdoc_core::visit_xlsx_rows(
        &file,
        XlsxSheetOptions {
            sheet_name: Some("Data"),
            ..XlsxSheetOptions::default()
        },
        XlsxValueMode::Formatted,
        |row| {
            rows.push(row.clone());
            Ok(XlsxRowControl::Continue)
        },
    )
    .unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].row_index, 2);
    assert_eq!(
        rows[0]
            .cells
            .iter()
            .map(|cell| cell.column_index)
            .collect::<Vec<_>>(),
        vec![0, 2, 3, 4]
    );
    assert_eq!(rows[0].cells[0].value, XlsxCellValue::Blank);
    assert_eq!(
        rows[0].cells[1].value,
        XlsxCellValue::String {
            raw: "last".to_owned(),
            value: "last".to_owned(),
        }
    );
    assert_eq!(
        rows[0].cells[2].value,
        XlsxCellValue::Error {
            raw: "#N/A".to_owned(),
        }
    );
    assert!(rows[0].cells[3].has_formula);
    assert_eq!(
        rows[0].cells[3].value,
        XlsxCellValue::Number {
            raw: "44927".to_owned(),
            formatted: Some("2023-01-01".to_owned()),
        }
    );
}

#[test]
fn visits_typed_xlsx_rows_from_reader_and_stops_early() {
    let file = fixtures::build_package("xlsx/basic", "fixture.xlsx");
    let reader = File::open(file).unwrap();
    let mut visited = 0;

    let extraction = oxdoc_core::visit_xlsx_rows_from_reader(
        reader,
        XlsxSheetOptions {
            sheet_name: Some("Sales Q1"),
            ..XlsxSheetOptions::default()
        },
        XlsxValueMode::Raw,
        |_| {
            visited += 1;
            Ok(XlsxRowControl::Stop)
        },
    )
    .unwrap();

    assert_eq!(visited, 1);
    assert!(extraction.warnings.is_empty());
}

#[test]
fn propagates_typed_xlsx_row_callback_errors() {
    let file = fixtures::build_package("xlsx/basic", "fixture.xlsx");

    let err = oxdoc_core::visit_xlsx_rows(
        &file,
        XlsxSheetOptions::default(),
        XlsxValueMode::Raw,
        |_| {
            Err(OxdocError::InvalidArgument(
                "callback rejected row".to_owned(),
            ))
        },
    )
    .unwrap_err();

    assert!(
        matches!(err, OxdocError::InvalidArgument(message) if message == "callback rejected row")
    );
}

#[test]
fn extracts_docx_structured_text_blocks_with_related_part_sources() {
    let file = create_ooxml(
        "structured-docx.docx",
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
                r#"<Relationships><Relationship Id="rIdHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="rIdFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/><Relationship Id="rIdMissing" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/></Relationships>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:p><w:r><w:t>Header</w:t></w:r></w:p><"#,
            ),
            ("word/footer1.xml", r#"<w:ftr xmlns:w="w"/>"#),
        ],
    );

    let extraction = oxdoc_core::extract_docx_structured_text(&file).unwrap();

    assert_eq!(extraction.value.document_type, "docx");
    assert_eq!(extraction.value.blocks.len(), 2);
    assert_eq!(extraction.value.blocks[0].part_type, "main");
    assert_eq!(extraction.value.blocks[0].part_path, "word/document.xml");
    assert_eq!(extraction.value.blocks[0].ordinal, 1);
    assert_eq!(extraction.value.blocks[0].text, "Body\n");
    assert_eq!(extraction.value.blocks[1].part_type, "header");
    assert_eq!(extraction.value.blocks[1].part_path, "word/header1.xml");
    assert_eq!(extraction.value.blocks[1].ordinal, 2);
    assert_eq!(extraction.value.blocks[1].text, "Header\n");
    assert_eq!(extraction.warnings.len(), 2);
    assert!(extraction.warnings.iter().any(|warning| {
        warning.path == "word/header1.xml" && warning.code().as_str() == "W001"
    }));
    assert!(extraction.warnings.iter().any(|warning| {
        warning.path == "word/_rels/document.xml.rels" && warning.message.contains("comments.xml")
    }));
}

#[test]
fn extracts_docx_structured_text_without_relationships_from_reader() {
    let file = create_ooxml(
        "structured-docx-no-rels.docx",
        &[(
            "word/document.xml",
            r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Only body</w:t></w:r></w:p></w:body></w:document>"#,
        )],
    );
    let reader = File::open(file).unwrap();

    let extraction = oxdoc_core::extract_docx_structured_text_from_reader(reader).unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(extraction.value.document_type, "docx");
    assert_eq!(extraction.value.blocks.len(), 1);
    assert_eq!(extraction.value.blocks[0].part_type, "main");
    assert_eq!(extraction.value.blocks[0].text, "Only body\n");
}

#[test]
fn extracts_pptx_structured_text_blocks_with_notes_sources() {
    let file = create_ooxml(
        "structured-pptx.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:p="p" xmlns:r="r"><p:sldIdLst><p:sldId id="1" r:id="rIdSlide1"/><p:sldId id="2" r:id="rIdSlide2"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rIdSlide1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rIdSlide2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Slide 1</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
            ),
            (
                "ppt/slides/_rels/slide1.xml.rels",
                r#"<Relationships><Relationship Id="rIdNotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/notesSlides/notesSlide1.xml",
                r#"<p:notes xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Notes 1</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#,
            ),
            (
                "ppt/slides/slide2.xml",
                r#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Slide 2</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_structured_text(&file).unwrap();

    assert_eq!(extraction.value.document_type, "pptx");
    assert!(extraction.warnings.is_empty());
    let blocks = extraction.value.blocks;
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].part_type, "slide");
    assert_eq!(blocks[0].part_path, "ppt/slides/slide1.xml");
    assert_eq!(blocks[0].ordinal, 1);
    assert_eq!(blocks[1].part_type, "notes");
    assert_eq!(blocks[1].part_path, "ppt/notesSlides/notesSlide1.xml");
    assert_eq!(blocks[1].ordinal, 2);
    assert_eq!(blocks[2].part_type, "slide");
    assert_eq!(blocks[2].part_path, "ppt/slides/slide2.xml");
    assert_eq!(blocks[2].ordinal, 3);
    assert_eq!(blocks[0].text, "Slide 1\n");
    assert_eq!(blocks[1].text, "Notes 1\n");
    assert_eq!(blocks[2].text, "Slide 2\n");
}

#[test]
fn detects_document_type_from_content_types() {
    let docx = fixtures::build_package("docx/basic", "renamed.bin");
    let pptx = fixtures::build_package("pptx/text", "renamed.data");
    let xlsx = fixtures::build_package("xlsx/basic", "renamed.package");
    let unknown = create_ooxml("no-content-types.bin", &[]);

    assert_eq!(
        oxdoc_core::detect_document_type(&docx).unwrap(),
        DocumentType::Docx
    );
    assert_eq!(
        oxdoc_core::detect_document_type(&pptx).unwrap(),
        DocumentType::Pptx
    );
    assert_eq!(
        oxdoc_core::detect_document_type(&xlsx).unwrap(),
        DocumentType::Xlsx
    );
    assert_eq!(
        oxdoc_core::detect_document_type(&unknown).unwrap(),
        DocumentType::Unknown
    );
}

#[test]
fn lists_visible_xlsx_sheets_without_opening_sheet_data() {
    let file = create_ooxml(
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

    let extraction = oxdoc_core::list_xlsx_sheets(&file).unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(extraction.value.len(), 2);
    assert_eq!(extraction.value[0].index, 1);
    assert_eq!(extraction.value[0].name, "Ventas Q1");
    assert_eq!(extraction.value[0].visibility, XlsxSheetVisibility::Visible);
    assert_eq!(extraction.value[1].index, 2);
    assert_eq!(extraction.value[1].name, "Resumen");
    assert_eq!(extraction.value[1].visibility, XlsxSheetVisibility::Visible);
}

#[test]
fn lists_hidden_xlsx_sheets_with_visibility_when_requested() {
    let file = create_ooxml(
        "list-hidden-sheets.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Visible" sheetId="1" r:id="rId1"/><sheet name="Hidden" sheetId="2" state="hidden" r:id="rId2"/><sheet name="Very Hidden" sheetId="3" state="veryHidden" r:id="rId3"/></sheets></workbook>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::list_xlsx_sheets_with_hidden(&file, true).unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(extraction.value.len(), 3);
    assert_eq!(extraction.value[0].index, 1);
    assert_eq!(extraction.value[0].visibility, XlsxSheetVisibility::Visible);
    assert_eq!(extraction.value[1].index, 2);
    assert_eq!(extraction.value[1].name, "Hidden");
    assert_eq!(extraction.value[1].visibility, XlsxSheetVisibility::Hidden);
    assert_eq!(extraction.value[2].index, 3);
    assert_eq!(extraction.value[2].name, "Very Hidden");
    assert_eq!(
        extraction.value[2].visibility,
        XlsxSheetVisibility::VeryHidden
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
fn escapes_xlsx_sparse_fields_for_comma_and_semicolon_csv() {
    let file = create_ooxml(
        "escaping-and-sparse.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Escaping" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1" t="inlineStr"><is><t>alpha;beta</t></is></c><c r="C1" t="inlineStr"><is><t>He said &quot;hi&quot;</t></is></c><c r="E1" t="inlineStr"><is><t>line&#10;break</t></is></c></row></sheetData></worksheet>"#,
            ),
        ],
    );

    let mut comma_csv = Vec::new();
    oxdoc_core::extract_xlsx_csv(&file, XlsxCsvOptions::default(), &mut comma_csv).unwrap();

    assert_eq!(
        String::from_utf8(comma_csv).unwrap(),
        "alpha;beta,,\"He said \"\"hi\"\"\",,\"line\nbreak\"\n"
    );

    let mut semicolon_csv = Vec::new();
    oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Escaping"),
            sheet_index: None,
            include_hidden: false,
            delimiter: b';',
        },
        &mut semicolon_csv,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(semicolon_csv).unwrap(),
        "\"alpha;beta\";;\"He said \"\"hi\"\"\";;\"line\nbreak\"\n"
    );
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
            include_hidden: false,
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
fn extracts_xlsx_csv_cell_type_edge_cases_as_stable_snapshot() {
    let file = create_ooxml(
        "xlsx-cell-types.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Types" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/sharedStrings.xml",
                r#"<sst><si><t>shared</t></si><si/></sst>"#,
            ),
            (
                "xl/styles.xml",
                r#"<styleSheet><cellXfs><xf numFmtId="14"/></cellXfs></styleSheet>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><dimension ref="A1:L3"/><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="C1" t="b"><v>0</v></c><c r="D1" t="b"><v>true</v></c><c r="E1" t="b"><v>false</v></c><c r="F1" t="e"><v>#N/A</v></c><c r="G1"><f>SUM(A2:A3)</f><v>7</v></c><c r="H1" t="str"><f>&quot;done&quot;</f><v>done</v></c><c r="I1" s="1"><v>45291</v></c><c r="J1"><v>1234.50</v></c><c r="L1" t="inlineStr"><is><t>needs, &quot;quotes&quot;&#10;and newline</t></is></c></row><row r="3"><c r="B3"><v>tail</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    let mut csv = Vec::new();

    oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Types"),
            sheet_index: None,
            include_hidden: false,
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(csv).unwrap(),
        fixtures::read_snapshot("xlsx_cell_types_csv.txt")
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
            include_hidden: false,
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
            include_hidden: false,
            delimiter: b',',
        },
        &mut csv,
    )
    .unwrap();

    assert_eq!(String::from_utf8(csv).unwrap(), "second\n");
}

#[test]
fn extracts_hidden_xlsx_csv_only_when_explicitly_included() {
    let file = create_ooxml(
        "hidden-sheet-index.xlsx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Hidden" sheetId="1" state="hidden" r:id="rId1"/><sheet name="Visible" sheetId="2" r:id="rId2"/><sheet name="Very Hidden" sheetId="3" state="veryHidden" r:id="rId3"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="worksheet" Target="worksheets/hidden.xml"/><Relationship Id="rId2" Type="worksheet" Target="worksheets/visible.xml"/><Relationship Id="rId3" Type="worksheet" Target="worksheets/very-hidden.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/hidden.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>hidden</v></c></row></sheetData></worksheet>"#,
            ),
            (
                "xl/worksheets/visible.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>visible</v></c></row></sheetData></worksheet>"#,
            ),
            (
                "xl/worksheets/very-hidden.xml",
                r#"<worksheet><sheetData><row><c r="A1"><v>very hidden</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );

    let mut default_csv = Vec::new();
    oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: None,
            sheet_index: Some(1),
            include_hidden: false,
            delimiter: b',',
        },
        &mut default_csv,
    )
    .unwrap();
    assert_eq!(String::from_utf8(default_csv).unwrap(), "visible\n");

    let mut hidden_csv = Vec::new();
    oxdoc_core::extract_xlsx_csv(
        &file,
        XlsxCsvOptions {
            sheet_name: Some("Very Hidden"),
            sheet_index: None,
            include_hidden: true,
            delimiter: b',',
        },
        &mut hidden_csv,
    )
    .unwrap();
    assert_eq!(String::from_utf8(hidden_csv).unwrap(), "very hidden\n");
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
            include_hidden: false,
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
            include_hidden: false,
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
fn reads_metadata_from_read_seek_reader() {
    let file = fixtures::build_package("pptx/basic", "fixture.pptx");
    let bytes = fs::read(file).unwrap();

    let extraction =
        oxdoc_core::read_info_from_reader(Cursor::new(bytes), "embedded-name.pptx").unwrap();

    assert_eq!(extraction.value.file, "embedded-name.pptx");
    assert_eq!(extraction.value.author.as_deref(), Some("Ada"));
    assert_eq!(extraction.value.application.as_deref(), Some("Impress"));
    assert!(extraction.warnings.is_empty());
}

#[test]
fn reads_optional_app_metadata_from_fixture() {
    let file = fixtures::build_package("xlsx/app-metadata", "fixture.xlsx");

    let extraction = oxdoc_core::read_info(&file).unwrap();

    assert_eq!(
        extraction.value.application.as_deref(),
        Some("Fixture Generator")
    );
    assert_eq!(extraction.value.company.as_deref(), Some("Fixture Labs"));
    assert_eq!(extraction.value.worksheet_count, Some(3));
    assert_eq!(extraction.value.page_count, None);
    assert_eq!(extraction.value.slide_count, None);
    assert_eq!(extraction.value.custom_properties, None);
    assert!(extraction.warnings.is_empty());
}

#[test]
fn reads_custom_metadata_properties() {
    let file = create_ooxml(
        "custom-metadata.docx",
        &[(
            "docProps/custom.xml",
            r#"
                <Properties xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
                  <property name="Department">
                    <vt:lpwstr>Research &amp; Development</vt:lpwstr>
                  </property>
                  <property name="Reviewed">
                    <vt:bool>true</vt:bool>
                  </property>
                </Properties>
            "#,
        )],
    );

    let extraction = oxdoc_core::read_info(&file).unwrap();
    let custom = extraction.value.custom_properties.as_ref().unwrap();

    assert_eq!(
        custom.get("Department").map(String::as_str),
        Some("Research & Development")
    );
    assert_eq!(custom.get("Reviewed").map(String::as_str), Some("true"));
    assert!(extraction.warnings.is_empty());
}

#[test]
fn detects_macros_from_content_types() {
    let file = create_ooxml(
        "macro-content-type.docm",
        &[(
            "[Content_Types].xml",
            r#"
                <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
                  <Override PartName="/custom/path/project.bin" ContentType="application/vnd.ms-office.vbaProject"/>
                </Types>
            "#,
        )],
    );

    let extraction = oxdoc_core::read_info(&file).unwrap();

    assert!(extraction.value.has_macros);
    assert_eq!(extraction.value.custom_properties, None);
    assert!(extraction.warnings.is_empty());
}

#[test]
fn keeps_partial_metadata_and_warns_on_malformed_custom_parts() {
    let file = create_ooxml(
        "malformed-extra-metadata.docm",
        &[
            (
                "[Content_Types].xml",
                r#"<Types><Override ContentType="application/vnd.ms-office.vbaProject"/><"#,
            ),
            (
                "docProps/custom.xml",
                r#"<Properties><property name="Broken"><vt:lpwstr>kept</vt:lpwstr></property><"#,
            ),
        ],
    );

    let extraction = oxdoc_core::read_info(&file).unwrap();

    assert!(extraction.value.has_macros);
    assert_eq!(
        extraction
            .value
            .custom_properties
            .as_ref()
            .and_then(|props| props.get("Broken"))
            .map(String::as_str),
        Some("kept")
    );
    assert_eq!(extraction.warnings.len(), 2);
    assert_eq!(extraction.warnings[0].path, "[Content_Types].xml");
    assert_eq!(extraction.warnings[1].path, "docProps/custom.xml");
    assert!(
        extraction
            .warnings
            .iter()
            .all(|warning| warning.code().as_str() == "W001")
    );
}

#[test]
fn reads_audit_signals_through_public_api() {
    let file = create_ooxml(
        "audit-signals.xlsm",
        &[
            (
                "[Content_Types].xml",
                r#"<Types><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/vbaProject.bin" ContentType="application/vnd.ms-office.vbaProject"/></Types>"#,
            ),
            (
                "docProps/custom.xml",
                r#"<Properties xmlns:vt="vt"><property name="Department"><vt:lpwstr>Finance</vt:lpwstr></property></Properties>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns:r="r"><sheets><sheet name="Visible" sheetId="1" r:id="rId1"/><sheet name="Hidden" sheetId="2" state="hidden" r:id="rId2"/><sheet name="Very Hidden" sheetId="3" state="veryHidden" r:id="rId3"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rIdExternal" Type="hyperlink" TargetMode="External" Target="https://example.invalid/model"/></Relationships>"#,
            ),
            ("xl/vbaProject.bin", "macro bytes"),
        ],
    );

    let extraction = oxdoc_core::read_audit(&file).unwrap();
    let audit = extraction.value;

    assert_eq!(audit.document_type, "xlsx");
    assert!(audit.metadata.has_macros);
    assert_eq!(
        audit
            .metadata
            .custom_properties
            .as_ref()
            .and_then(|props| props.get("Department"))
            .map(String::as_str),
        Some("Finance")
    );
    assert_signal(&audit.signals, "macros", "high", "VBA macro");
    assert_signal(
        &audit.signals,
        "custom_properties",
        "info",
        "custom document properties",
    );
    assert_signal(&audit.signals, "hidden_sheet", "warning", "Hidden");
    assert_signal(&audit.signals, "hidden_sheet", "warning", "Very Hidden");
    assert_signal(
        &audit.signals,
        "hyperlink",
        "warning",
        "https://example.invalid/model",
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn audit_classifies_external_relationships_by_type() {
    let file = create_ooxml(
        "audit-external-relationships.docx",
        &[(
            "word/_rels/document.xml.rels",
            r#"<Relationships><Relationship Id="rHyperlink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External" Target="https://example.invalid/link"/><Relationship Id="rExternalLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLink" TargetMode="External" Target="https://example.invalid/book.xlsx"/><Relationship Id="rTemplate" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/attachedTemplate" TargetMode="External" Target="https://example.invalid/template.dotm"/><Relationship Id="rUnknown" Type="https://example.invalid/relationships/custom" TargetMode="External" Target="https://example.invalid/custom"/></Relationships>"#,
        )],
    );

    let audit = oxdoc_core::read_audit(&file).unwrap().value;

    assert_signal(&audit.signals, "hyperlink", "warning", "/link");
    assert_signal(&audit.signals, "external_link", "warning", "book.xlsx");
    assert_signal(
        &audit.signals,
        "attached_template",
        "warning",
        "template.dotm",
    );
    assert_signal(&audit.signals, "relationship_target", "warning", "/custom");
}

#[test]
fn audit_detects_internal_embedded_relationships_and_workbook_protection() {
    let file = create_ooxml(
        "audit-embedded.xlsx",
        &[
            (
                "[Content_Types].xml",
                r#"<Types><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/></Types>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook><workbookProtection lockStructure="1"/><sheets/></workbook>"#,
            ),
            (
                "xl/worksheets/_rels/sheet1.xml.rels",
                r#"<Relationships><Relationship Id="rOle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="../embeddings/oleObject1.bin"/><Relationship Id="rPackage" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="../embeddings/package1.bin"/></Relationships>"#,
            ),
        ],
    );

    let audit = oxdoc_core::read_audit(&file).unwrap().value;

    assert_signal(
        &audit.signals,
        "workbook_protection",
        "warning",
        "protection settings",
    );
    assert_signal(&audit.signals, "ole_object", "warning", "oleObject1.bin");
    assert_signal(
        &audit.signals,
        "embedded_package",
        "warning",
        "package1.bin",
    );
}

#[test]
fn audit_keeps_recoverable_parser_warnings_as_signals() {
    let file = create_ooxml(
        "audit-warning.docx",
        &[(
            "docProps/core.xml",
            r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc"><dc:creator>Ada</dc:creator><"#,
        )],
    );

    let extraction = oxdoc_core::read_audit(&file).unwrap();

    assert_eq!(extraction.value.metadata.author.as_deref(), Some("Ada"));
    assert_eq!(extraction.warnings.len(), 1);
    assert_signal(
        &extraction.value.signals,
        "parser_warning",
        "warning",
        "W001",
    );
}

#[test]
fn reports_missing_zip_entry_through_vfs() {
    let file = File::open(create_ooxml("missing-entry.docx", &[])).unwrap();
    let mut package = OoxmlPackage::new(file).unwrap();

    let err = package.read_to_string("word/document.xml").unwrap_err();

    assert_eq!(err.code().as_str(), "E003");
    assert!(matches!(err, OxdocError::MissingPart(part) if part == "word/document.xml"));
}

fn assert_signal(
    signals: &[oxdoc_core::AuditSignal],
    kind: &str,
    severity: &str,
    message_contains: &str,
) {
    assert!(
        signals.iter().any(|signal| {
            signal.kind == kind
                && signal.severity == severity
                && signal.message.contains(message_contains)
        }),
        "missing signal kind={kind} severity={severity} containing {message_contains:?}: {signals:#?}"
    );
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
fn allows_entry_specific_vfs_limits_without_changing_package_default() {
    let file = File::open(create_ooxml(
        "entry-specific-limit.xlsx",
        &[("xl/sharedStrings.xml", "0123456789")],
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

    let err = package.read_to_string("xl/sharedStrings.xml").unwrap_err();
    assert!(matches!(
        err,
        OxdocError::PartTooLarge {
            path,
            size: 10,
            limit: 5
        } if path == "xl/sharedStrings.xml"
    ));

    let content = package
        .with_entry_limits(
            "xl/sharedStrings.xml",
            OoxmlLimits {
                max_part_uncompressed_size: 10,
                ..OoxmlLimits::default()
            },
            |entry| {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;
                Ok(content)
            },
        )
        .unwrap();

    assert_eq!(content, "0123456789");
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
fn rejects_pptx_notes_relationship_targets_that_escape_package_root() {
    let file = create_ooxml(
        "escaping-notes-target.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>slide</a:t></a:r></a:p></p:sld>"#,
            ),
            (
                "ppt/slides/_rels/slide1.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../../../outside.xml"/></Relationships>"#,
            ),
        ],
    );

    let err = oxdoc_core::extract_pptx_text(&file).unwrap_err();

    assert!(matches!(
        err,
        OxdocError::SuspiciousRelationshipTarget { path, target, .. }
            if path == "ppt/slides/_rels/slide1.xml.rels" && target == "../../../outside.xml"
    ));
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
        assert!(note.contains("Source:"));
        assert!(note.contains("Producer:"));
        assert!(note.contains("Redistribution:"));
        assert!(note.contains("Purpose:"));
    }

    for provenance in ["xlsx-basic.md", "xlsx-app-metadata.md"] {
        let note = fixtures::read_provenance(provenance);
        assert!(note.contains("no `.xlsx` binary is checked in"));
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
    oxdoc_core::fuzz_pptx_text(br#"<p:sld><a:p><a:r><a:t>Fuzz</a:t></a:r></a:p></p:sld>"#).unwrap();
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
