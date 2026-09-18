use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use oxdoc_core::vfs::{OoxmlLimits, OoxmlPackage};
use oxdoc_core::{
    DocumentType, DocxRevisionMode, DocxTableBlock, DocxTextOptions, DocxVerticalMerge, OxdocError,
    XlsxCellValue, XlsxCsvOptions, XlsxReadOptions, XlsxRowControl, XlsxSheetOptions,
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
fn applies_docx_text_options_through_public_api() {
    let file = fixtures::build_package("docx/policies", "policies.docx");
    let default = oxdoc_core::extract_docx_text(&file).unwrap();
    assert_eq!(default.value, "List item\nHiddenInserted\nComment\n");

    let extraction = oxdoc_core::extract_docx_text_with_options(
        &file,
        DocxTextOptions {
            include_hidden_text: false,
            include_comments: false,
            revision_mode: DocxRevisionMode::Original,
            include_related_parts: false,
            include_list_markers: true,
        },
    )
    .unwrap();
    assert_eq!(extraction.value, "- List item\nDeleted\n");
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
fn keeps_unreferenced_docx_related_parts_in_relationship_order() {
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
fn extracts_docx_tables_through_public_api() {
    let file = create_ooxml(
        "docx-tables.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:tbl><w:tblGrid><w:gridCol/><w:gridCol/><w:gridCol/></w:tblGrid><w:tr><w:trPr><w:gridBefore w:val="1"/></w:trPr><w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>Alpha</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:vMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>Merge start</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:trPr><w:gridBefore w:val="1"/></w:trPr><w:tc><w:p><w:r><w:t>Before nested</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Nested</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:tc><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc></w:tr></w:tbl></w:body></w:document>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_tables(&file).unwrap();

    assert!(extraction.warnings.is_empty());
    assert_eq!(extraction.value.document_type, "docx");
    assert_eq!(extraction.value.tables.len(), 1);
    let table = &extraction.value.tables[0];
    assert_eq!(table.part_type, "main");
    assert_eq!(table.part_path, "word/document.xml");
    assert_eq!(table.table_ordinal, 1);
    assert!(table.complete);
    assert_eq!(table.grid_column_count, Some(3));
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].row_ordinal, 1);
    assert_eq!(table.rows[0].grid_before, 1);
    assert_eq!(table.rows[0].cells[0].cell_ordinal, 1);
    assert_eq!(table.rows[0].cells[0].grid_start, 1);
    assert_eq!(table.rows[0].cells[0].grid_span, 2);
    assert_eq!(
        table.rows[0].cells[1].vertical_merge,
        DocxVerticalMerge::Restart
    );
    assert_eq!(
        table.rows[1].cells[1].vertical_merge,
        DocxVerticalMerge::Continue
    );
    assert_eq!(
        table.rows[0].cells[0].blocks,
        vec![DocxTableBlock::Paragraph {
            text: "Alpha".to_owned(),
        }]
    );
    let DocxTableBlock::Table { rows, complete, .. } = &table.rows[1].cells[0].blocks[1] else {
        panic!("expected nested table block");
    };
    assert!(*complete);
    assert_eq!(
        rows[0].cells[0].blocks,
        vec![DocxTableBlock::Paragraph {
            text: "Nested".to_owned(),
        }]
    );
}

#[test]
fn extracts_docx_tables_from_read_seek_reader() {
    let file = create_ooxml(
        "docx-table-reader.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Reader table</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
            ),
        ],
    );
    let bytes = fs::read(file).unwrap();

    let extraction = oxdoc_core::extract_docx_tables_from_reader(Cursor::new(bytes)).unwrap();

    assert_eq!(
        extraction.value.tables[0].rows[0].cells[0].blocks,
        vec![DocxTableBlock::Paragraph {
            text: "Reader table".to_owned(),
        }]
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn keeps_unreferenced_docx_table_parts_in_relationship_order() {
    let file = create_ooxml(
        "docx-related-table-parts.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Main table</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
            ),
            (
                "word/_rels/document.xml.rels",
                r#"<Relationships><Relationship Id="rComments" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#,
            ),
            (
                "word/comments.xml",
                r#"<w:comments xmlns:w="w"><w:comment w:id="1"><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Comment table</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:comment></w:comments>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Header table</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:hdr>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_tables(&file).unwrap();

    let parts = extraction
        .value
        .tables
        .iter()
        .map(|table| {
            let DocxTableBlock::Paragraph { text } = &table.rows[0].cells[0].blocks[0] else {
                panic!("expected paragraph");
            };
            (
                table.part_type.as_str(),
                table.part_path.as_str(),
                text.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        parts,
        vec![
            ("main", "word/document.xml", "Main table"),
            // Rule 5: notes (comments) follow strictly after all headers and
            // footers; the orphan header keeps rels-order priority.
            ("header", "word/header1.xml", "Header table"),
            ("comments", "word/comments.xml", "Comment table"),
        ]
    );
    assert!(extraction.warnings.is_empty());
}

#[test]
fn docx_tables_keep_closed_prefix_after_malformed_xml() {
    let file = create_ooxml(
        "docx-malformed-table-api.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Complete</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>Discard open"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_tables(&file).unwrap();

    assert_eq!(extraction.value.tables.len(), 1);
    assert!(!extraction.value.tables[0].complete);
    assert_eq!(extraction.value.tables[0].rows.len(), 1);
    assert_eq!(
        extraction.value.tables[0].rows[0].cells[0].blocks,
        vec![DocxTableBlock::Paragraph {
            text: "Complete".to_owned(),
        }]
    );
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(extraction.warnings[0].path, "word/document.xml");
    assert_eq!(extraction.warnings[0].code().as_str(), "W001");
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
fn extracts_pptx_sldid_without_id_attribute_unchanged() {
    let file = create_ooxml(
        "sldid-without-id.pptx",
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
                r#"<p:sld><a:p><a:r><a:t>Alpha Slide</a:t></a:r></a:p></p:sld>"#,
            ),
        ],
    );

    let text = oxdoc_core::extract_pptx_text(&file).unwrap();
    let structured = oxdoc_core::extract_pptx_structured_text(&file).unwrap();

    assert_eq!(text.value, "Alpha Slide\n");
    assert!(text.warnings.is_empty());
    let parts: Vec<(String, String, usize, String)> = structured
        .value
        .blocks
        .iter()
        .map(|block| {
            (
                block.part_type.clone(),
                block.part_path.clone(),
                block.ordinal,
                block.text.clone(),
            )
        })
        .collect();
    assert_eq!(
        parts,
        vec![(
            "slide".to_owned(),
            "ppt/slides/slide1.xml".to_owned(),
            1,
            "Alpha Slide\n".to_owned(),
        )]
    );
    assert!(structured.warnings.is_empty());
}

#[test]
fn extracts_pptx_sldid_with_id_attribute_unchanged() {
    let file = create_ooxml(
        "sldid-with-id.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="257" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>Beta Slide</a:t></a:r></a:p></p:sld>"#,
            ),
        ],
    );

    let text = oxdoc_core::extract_pptx_text(&file).unwrap();

    assert_eq!(text.value, "Beta Slide\n");
    assert!(text.warnings.is_empty());
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
fn extracts_pptx_slides_with_slide_ids_and_ordinals_in_sldidlst_order() {
    let file = fixtures::build_package("pptx/text", "slides-deck.pptx");

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 2);
    assert_eq!(extraction.value[0].slide_id, Some(256));
    assert_eq!(extraction.value[0].slide_ordinal, 1);
    assert_eq!(extraction.value[0].slide_path, "ppt/slides/slide2.xml");
    assert_eq!(
        extraction.value[0].text,
        "First Slide\nAlpha\tBeta & Co\nGamma < Delta\n"
    );
    assert_eq!(extraction.value[0].notes, Some("Speaker note\n".to_owned()));
    assert_eq!(extraction.value[1].slide_id, Some(257));
    assert_eq!(extraction.value[1].slide_ordinal, 2);
    assert_eq!(extraction.value[1].slide_path, "ppt/slides/slide1.xml");
    assert_eq!(extraction.value[1].text, "Second Slide\n");
    assert_eq!(extraction.value[1].notes, None);
    assert!(extraction.warnings.is_empty());
}

#[test]
fn extracts_pptx_slides_slide_paths_match_structured_part_paths() {
    let file = fixtures::build_package("pptx/text", "slides-deck.pptx");

    let slides = oxdoc_core::extract_pptx_slides(&file).unwrap();
    let structured = oxdoc_core::extract_pptx_structured_text(&file).unwrap();

    let slide_text_paths: Vec<&str> = structured
        .value
        .blocks
        .iter()
        .filter(|block| block.part_type == "slide")
        .map(|block| block.part_path.as_str())
        .collect();
    let record_paths: Vec<&str> = slides
        .value
        .iter()
        .map(|record| record.slide_path.as_str())
        .collect();

    assert_eq!(record_paths, slide_text_paths);
}

#[test]
fn extracts_pptx_slides_without_notes_key_on_basic_corpus() {
    let file = fixtures::build_package("pptx/basic", "basic-deck.pptx");

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();
    let record = serde_json::to_value(&extraction.value[0]).unwrap();
    let record = record.as_object().unwrap();

    assert_eq!(extraction.value.len(), 1);
    assert!(record["text"].is_string());
    assert!(!record.contains_key("notes"));
    assert_eq!(record["slide_id"], 256);
    assert!(!record["slide_id"].is_null());
    assert_eq!(extraction.warnings.len(), 0);
}

#[test]
fn extracts_pptx_slides_from_reader_and_reader_with_limits() {
    let file = fixtures::build_package("pptx/text", "slides-deck.pptx");
    let bytes = fs::read(&file).unwrap();

    let from_reader =
        oxdoc_core::extract_pptx_slides_from_reader(Cursor::new(bytes.clone())).unwrap();
    let with_limits = oxdoc_core::extract_pptx_slides_from_reader_with_limits(
        Cursor::new(bytes),
        OoxmlLimits::default(),
    )
    .unwrap();
    let from_path = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(from_reader.value, from_path.value);
    assert_eq!(with_limits.value, from_path.value);
    assert_eq!(with_limits.warnings, from_path.warnings);
}

#[test]
fn skips_pptx_slides_with_locked_warnings_on_missing_target_fixture() {
    let file = fixtures::build_package("pptx/missing-target", "missing-target.pptx");

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 2);
    assert_eq!(extraction.value[0].slide_id, Some(256));
    assert_eq!(extraction.value[0].slide_ordinal, 1);
    assert_eq!(extraction.value[0].slide_path, "ppt/slides/slide1.xml");
    assert_eq!(extraction.value[0].text, "Intact Slide\n");
    assert_eq!(extraction.value[0].notes, None);
    assert_eq!(extraction.value[1].slide_id, Some(259));
    assert_eq!(extraction.value[1].slide_ordinal, 4);
    assert_eq!(extraction.value[1].slide_path, "ppt/slides/slide3.xml");
    assert_eq!(extraction.value[1].text, "Notes Slide\n");
    assert_eq!(extraction.value[1].notes, None);

    let warnings: Vec<(String, String)> = extraction
        .warnings
        .iter()
        .map(|warning| (warning.path.clone(), warning.message.clone()))
        .collect();
    assert_eq!(
        warnings,
        vec![
            (
                "ppt/presentation.xml".to_owned(),
                "skipped PPTX slide rId999: unknown relationship id".to_owned(),
            ),
            (
                "ppt/slides/absent.xml".to_owned(),
                "skipped related PPTX slide part ppt/slides/absent.xml: missing part".to_owned(),
            ),
            (
                "ppt/notesSlides/notesSlide3.xml".to_owned(),
                "skipped related PPTX notes part ppt/notesSlides/notesSlide3.xml: missing part"
                    .to_owned(),
            ),
        ]
    );
}

#[test]
fn treats_missing_presentation_rels_as_empty_relationship_map_without_warning() {
    let file = create_ooxml(
        "missing-presentation-rels.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/><p:sldId id="257" r:id="rId2"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>Alpha Slide</a:t></a:r></a:p></p:sld>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 0);
    let warnings: Vec<(String, String)> = extraction
        .warnings
        .iter()
        .map(|warning| (warning.path.clone(), warning.message.clone()))
        .collect();
    assert_eq!(
        warnings,
        vec![
            (
                "ppt/presentation.xml".to_owned(),
                "skipped PPTX slide rId1: unknown relationship id".to_owned(),
            ),
            (
                "ppt/presentation.xml".to_owned(),
                "skipped PPTX slide rId2: unknown relationship id".to_owned(),
            ),
        ]
    );
}

#[test]
fn extracts_pptx_slides_empty_record_set_with_warnings_when_all_slides_skipped() {
    let file = create_ooxml(
        "all-skipped.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId404"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>Unreachable Slide</a:t></a:r></a:p></p:sld>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 0);
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(
        extraction.warnings[0].message,
        "skipped PPTX slide rId404: unknown relationship id"
    );
}

#[test]
fn keeps_partial_pptx_slide_text_and_warns_on_malformed_xml_fixture() {
    let file = fixtures::build_package("pptx/malformed-xml", "malformed-xml.pptx");

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 2);
    assert_eq!(extraction.value[0].slide_id, Some(256));
    assert_eq!(extraction.value[0].slide_ordinal, 1);
    assert_eq!(extraction.value[0].slide_path, "ppt/slides/slide1.xml");
    assert_eq!(extraction.value[0].text, "Before Truncation\n");
    assert_eq!(extraction.value[0].notes, None);
    assert_eq!(extraction.value[1].slide_id, Some(257));
    assert_eq!(extraction.value[1].slide_ordinal, 2);
    assert_eq!(extraction.value[1].slide_path, "ppt/slides/slide2.xml");
    assert_eq!(extraction.value[1].text, "Partial Text\n");
    assert_eq!(extraction.value[1].notes, None);
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(extraction.warnings[0].path, "ppt/slides/slide2.xml");
    assert_eq!(extraction.warnings[0].code().as_str(), "W001");
    assert!(
        extraction.warnings[0]
            .message
            .starts_with("stopped after malformed XML: ")
    );
}

#[test]
fn emits_textless_pptx_slide_record_with_empty_text() {
    let file = create_ooxml(
        "textless-slide.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t></a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 1);
    assert_eq!(extraction.value[0].slide_ordinal, 1);
    assert_eq!(extraction.value[0].slide_path, "ppt/slides/slide1.xml");
    assert_eq!(extraction.value[0].text, "");
    assert!(extraction.warnings.is_empty());
}

#[test]
fn rejects_external_pptx_slide_relationship_targets_as_hard_error() {
    let file = create_ooxml(
        "external-slide-target.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" TargetMode="External" Target="https://example.invalid/slide1.xml"/></Relationships>"#,
            ),
        ],
    );

    let err = oxdoc_core::extract_pptx_slides(&file).unwrap_err();

    assert!(
        matches!(err, OxdocError::SuspiciousRelationshipTarget { path, target, reason }
            if path == "ppt/_rels/presentation.xml.rels"
                && target == "https://example.invalid/slide1.xml"
                && reason.contains("external"))
    );
}

#[test]
fn rejects_pptx_slide_relationship_targets_that_escape_package_root() {
    let file = create_ooxml(
        "escaping-slide-target.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="../../outside.xml"/></Relationships>"#,
            ),
        ],
    );

    let err = oxdoc_core::extract_pptx_slides(&file).unwrap_err();

    assert!(matches!(
        err,
        OxdocError::SuspiciousRelationshipTarget { path, target, .. }
            if path == "ppt/_rels/presentation.xml.rels" && target == "../../outside.xml"
    ));
}

#[test]
fn rejects_nul_bytes_in_pptx_slide_relationship_targets() {
    let file = create_ooxml(
        "nul-slide-target.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                "<Relationships><Relationship Id=\"rId1\" Type=\"slide\" Target=\"slides/slide&#0;1.xml\"/></Relationships>",
            ),
        ],
    );

    let err = oxdoc_core::extract_pptx_slides(&file).unwrap_err();

    assert!(matches!(
        err,
        OxdocError::SuspiciousRelationshipTarget { reason, .. } if reason.contains("NUL")
    ));
}

#[test]
fn fails_hard_when_pptx_presentation_part_is_missing() {
    let file = create_ooxml(
        "missing-presentation.pptx",
        &[(
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
        )],
    );

    let err = oxdoc_core::extract_pptx_slides(&file).unwrap_err();

    assert!(matches!(
        err,
        OxdocError::MissingPart(path) if path == "ppt/presentation.xml"
    ));
}

#[test]
fn old_pptx_paths_still_hard_error_on_missing_targets_while_slides_extract() {
    let file = fixtures::build_package("pptx/missing-target", "missing-target.pptx");

    let text_err = oxdoc_core::extract_pptx_text(&file).unwrap_err();
    let structured_err = oxdoc_core::extract_pptx_structured_text(&file).unwrap_err();
    let slides = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert!(matches!(
        text_err,
        OxdocError::MissingPart(path) if path == "rId999"
    ));
    assert!(matches!(
        structured_err,
        OxdocError::MissingPart(path) if path == "rId999"
    ));
    assert_eq!(slides.value.len(), 2);
    assert_eq!(
        slides
            .value
            .iter()
            .map(|record| record.slide_ordinal)
            .collect::<Vec<_>>(),
        [1, 4]
    );
    assert_eq!(slides.warnings.len(), 3);
}

#[test]
fn keeps_ordinal_gap_when_middle_slide_of_three_is_skipped() {
    let file = create_ooxml(
        "middle-skipped.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/><p:sldId id="257" r:id="rId2"/><p:sldId id="258" r:id="rId3"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/><Relationship Id="rId2" Type="slide" Target="slides/absent.xml"/><Relationship Id="rId3" Type="slide" Target="slides/slide3.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>First Slide</a:t></a:r></a:p></p:sld>"#,
            ),
            (
                "ppt/slides/slide3.xml",
                r#"<p:sld><a:p><a:r><a:t>Third Slide</a:t></a:r></a:p></p:sld>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(
        extraction
            .value
            .iter()
            .map(|record| (record.slide_ordinal, record.slide_path.as_str()))
            .collect::<Vec<_>>(),
        [(1, "ppt/slides/slide1.xml"), (3, "ppt/slides/slide3.xml")]
    );
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(
        extraction.warnings[0].message,
        "skipped related PPTX slide part ppt/slides/absent.xml: missing part"
    );
}

#[test]
fn reads_empty_readable_notes_part_as_present_empty_string() {
    let file = create_ooxml(
        "empty-notes.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>Slide With Silent Notes</a:t></a:r></a:p></p:sld>"#,
            ),
            (
                "ppt/slides/_rels/slide1.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/notesSlides/notesSlide1.xml",
                r#"<p:notes xmlns:p="p"/>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 1);
    assert_eq!(extraction.value[0].notes, Some(String::new()));
    let record = serde_json::to_value(&extraction.value[0]).unwrap();
    assert_eq!(record["notes"], "");
    assert!(extraction.warnings.is_empty());
}

#[test]
fn omits_notes_without_warning_when_slide_rels_part_is_absent() {
    let file = create_ooxml(
        "no-slide-rels.pptx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="slide" Target="slides/slide1.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld><a:p><a:r><a:t>Slide Without Rels</a:t></a:r></a:p></p:sld>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(extraction.value.len(), 1);
    assert_eq!(extraction.value[0].slide_path, "ppt/slides/slide1.xml");
    assert_eq!(extraction.value[0].text, "Slide Without Rels\n");
    assert_eq!(extraction.value[0].notes, None);
    let record = serde_json::to_value(&extraction.value[0]).unwrap();
    assert!(!record.as_object().unwrap().contains_key("notes"));
    assert!(extraction.warnings.is_empty());
}

#[test]
fn reads_missing_target_slides_identically_through_reader_wrappers() {
    let file = fixtures::build_package("pptx/missing-target", "missing-target.pptx");
    let bytes = fs::read(&file).unwrap();

    let from_reader =
        oxdoc_core::extract_pptx_slides_from_reader(Cursor::new(bytes.clone())).unwrap();
    let with_limits = oxdoc_core::extract_pptx_slides_from_reader_with_limits(
        Cursor::new(bytes),
        OoxmlLimits::default(),
    )
    .unwrap();
    let from_path = oxdoc_core::extract_pptx_slides(&file).unwrap();

    assert_eq!(from_reader.value, from_path.value);
    assert_eq!(from_reader.warnings, from_path.warnings);
    assert_eq!(with_limits.value, from_path.value);
    assert_eq!(with_limits.warnings, from_path.warnings);
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
fn warns_on_missing_and_unknown_docx_reference_ids() {
    let file = create_ooxml(
        "docx-reference-id-warnings.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rIdGhost"/><w:footerReference/></w:sectPr></w:pPr><w:r><w:t>Body</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rHeader"/></w:sectPr></w:body></w:document>"#,
            ),
            (
                "word/_rels/document.xml.rels",
                r#"<Relationships><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:p><w:r><w:t>Header text</w:t></w:r></w:p></w:hdr>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(extraction.value, "Body\nHeader text\n");
    let messages = extraction
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        messages,
        vec![
            "skipped DOCX headerReference rIdGhost: unknown relationship id",
            "skipped DOCX footerReference: missing r:id",
        ]
    );
    assert!(
        extraction
            .warnings
            .iter()
            .all(|warning| warning.path == "word/_rels/document.xml.rels")
    );
}

#[test]
fn preserves_docx_fast_path_with_sectpr_documents() {
    let file = create_ooxml(
        "docx-sectpr-fast-path.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Body</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rHeader"/></w:sectPr></w:body></w:document>"#,
            ),
            (
                "word/_rels/document.xml.rels",
                r#"<Relationships><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:p><w:r><w:t>Header text</w:t></w:r></w:p></w:hdr>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text_with_options(
        &file,
        DocxTextOptions {
            include_related_parts: false,
            ..DocxTextOptions::default()
        },
    )
    .unwrap();

    assert_eq!(extraction.value, "Body\n");
    assert!(extraction.warnings.is_empty());
}

#[test]
fn keeps_partial_docx_text_when_document_xml_malformed_with_sectpr() {
    let file = create_ooxml(
        "docx-malformed-document-sectpr.docx",
        &[
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rHeader"/></w:sectPr></w:pPr><w:r><w:t>before break</w:t></w:r></w:p><"#,
            ),
            (
                "word/_rels/document.xml.rels",
                r#"<Relationships><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#,
            ),
            (
                "word/header1.xml",
                r#"<w:hdr xmlns:w="w"><w:p><w:r><w:t>Header after break</w:t></w:r></w:p></w:hdr>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(extraction.value, "before break\nHeader after break\n");
    assert_eq!(extraction.warnings.len(), 1);
    assert_eq!(extraction.warnings[0].path, "word/document.xml");
    assert_eq!(extraction.warnings[0].code().as_str(), "W001");
}

// The three `orders_docx_*_by_section` tests consume one hand-authored JSON
// oracle (`tests/fixtures/docx/section-order/expected.json`) across all three
// extraction paths, so any drift between the paths fails here.

#[test]
fn orders_docx_text_related_parts_by_section() {
    let file = build_section_order_package();
    let oracle = section_order_oracle();

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    let expected_text: String = oracle["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| format!("{}\n", part["text"].as_str().unwrap()))
        .collect();
    assert_eq!(extraction.value, expected_text);
    let messages = extraction
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(messages, oracle_warning_messages(&oracle));
    assert!(
        extraction
            .warnings
            .iter()
            .all(|warning| warning.path == "word/_rels/document.xml.rels")
    );
}

#[test]
fn orders_docx_structured_blocks_by_section() {
    let file = build_section_order_package();
    let oracle = section_order_oracle();

    let extraction = oxdoc_core::extract_docx_structured_text(&file).unwrap();

    let parts = extraction
        .value
        .blocks
        .iter()
        .map(|block| {
            (
                block.part_type.clone(),
                block.part_path.clone(),
                block.variant.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(parts, oracle_parts_with_variant(&oracle));
    let messages = extraction
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(messages, oracle_warning_messages(&oracle));
}

#[test]
fn keeps_plain_text_free_of_variant_metadata() {
    let file = build_section_order_package();
    let oracle = section_order_oracle();

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    let expected_text: String = oracle["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| format!("{}\n", part["text"].as_str().unwrap()))
        .collect();
    assert_eq!(extraction.value, expected_text);
    for token in ["word/header", "word/footer", "variant", "ordinal"] {
        assert!(
            !extraction.value.contains(token),
            "flat text leaked provenance token {token}: {:?}",
            extraction.value
        );
    }
}

#[test]
fn keeps_non_variant_blocks_without_variant_key() {
    let file = build_section_order_package();

    let extraction = oxdoc_core::extract_docx_structured_text(&file).unwrap();
    let output = serde_json::to_value(&extraction.value).unwrap();
    for block in output["blocks"].as_array().unwrap() {
        let part_type = block["part_type"].as_str().unwrap();
        if part_type == "header" || part_type == "footer" {
            continue;
        }
        assert!(
            block.get("variant").is_none(),
            "{part_type} block serializes a variant key: {block}"
        );
    }

    let pptx = fixtures::build_package("pptx/text", "structured-variant-free.pptx");
    let pptx_extraction = oxdoc_core::extract_pptx_structured_text(&pptx).unwrap();
    let pptx_output = serde_json::to_value(&pptx_extraction.value).unwrap();
    for block in pptx_output["blocks"].as_array().unwrap() {
        assert!(
            block.get("variant").is_none(),
            "PPTX block serializes a variant key: {block}"
        );
    }
}

#[test]
fn ordinals_are_contiguous_in_output_order() {
    let file = build_section_order_package();

    let extraction = oxdoc_core::extract_docx_structured_text(&file).unwrap();

    let blocks = &extraction.value.blocks;
    assert_eq!(
        blocks.iter().map(|block| block.ordinal).collect::<Vec<_>>(),
        (1..=blocks.len()).collect::<Vec<_>>()
    );
}

#[test]
fn orders_docx_tables_by_section() {
    let file = build_section_order_package();
    let oracle = section_order_oracle();

    let extraction = oxdoc_core::extract_docx_tables(&file).unwrap();

    let parts = extraction
        .value
        .tables
        .iter()
        .map(|table| (table.part_type.clone(), table.part_path.clone()))
        .collect::<Vec<_>>();
    // Only the oracle parts that declare a table contribute tables, in oracle
    // order; `table_ordinal` stays the 1-based per-part encounter ordinal.
    let expected_table_parts: Vec<(String, String)> = oracle["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| part.get("table_ordinal").is_some())
        .map(|part| {
            (
                part["part_type"].as_str().unwrap().to_owned(),
                part["part_path"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(parts, expected_table_parts);
    let ordinals = extraction
        .value
        .tables
        .iter()
        .map(|table| table.table_ordinal)
        .collect::<Vec<_>>();
    let expected_ordinals: Vec<usize> = oracle["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|part| {
            part.get("table_ordinal")
                .map(|value| value.as_u64().unwrap() as usize)
        })
        .collect();
    assert_eq!(ordinals, expected_ordinals);
    let messages = extraction
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(messages, oracle_warning_messages(&oracle));
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
fn extracts_locale_format_fixture_with_deterministic_warnings() {
    let file = fixtures::build_package("xlsx/formatted-locale", "formatted-locale.xlsx");
    let mut csv = Vec::new();

    let extraction = oxdoc_core::extract_xlsx_csv_with_value_mode(
        &file,
        XlsxCsvOptions::default(),
        XlsxValueMode::Formatted,
        &mut csv,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(csv).unwrap(),
        fixtures::read_snapshot("xlsx_formatted_locale_csv.txt")
    );
    assert_eq!(
        extraction
            .warnings
            .iter()
            .map(|warning| warning.code().as_str())
            .collect::<Vec<_>>(),
        vec!["W005", "W006"]
    );
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
fn xlsx_formulas_corpus_loads_and_carries_the_documented_cells() {
    let file = fixtures::build_package("xlsx/formulas", "formula-provenance.xlsx");
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
    assert_eq!(rows.len(), 2);

    // Row 1: shared-string header controls, no formulas.
    assert_eq!(rows[0].row_index, 0);
    let headers = &rows[0].cells;
    assert_eq!(
        headers
            .iter()
            .map(|cell| cell.column_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        headers[0].value,
        XlsxCellValue::String {
            raw: "0".to_owned(),
            value: "alpha".to_owned(),
        }
    );
    assert_eq!(
        headers[1].value,
        XlsxCellValue::String {
            raw: "1".to_owned(),
            value: "text".to_owned(),
        }
    );
    assert_eq!(
        headers[2].value,
        XlsxCellValue::String {
            raw: "0".to_owned(),
            value: "alpha".to_owned(),
        }
    );
    for cell in headers {
        assert!(
            !cell.has_formula,
            "header cell {} must stay a formula-free control",
            cell.column_index
        );
    }

    // Row 2: the formula case matrix, asserted at today's v1 baseline.
    assert_eq!(rows[1].row_index, 1);
    assert_eq!(
        rows[1]
            .cells
            .iter()
            .map(|cell| cell.column_index)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9]
    );
    let cells = &rows[1].cells;

    // B2: cached numeric formula. S3 baseline: the parser emits no formula
    // state yet (behavior-neutral slice); S4 fills own-text capture and
    // asserts `formula.expression == "SUM(B1:B1)"` with `cached: true` here.
    assert!(cells[0].has_formula);
    assert!(cells[0].formula.is_none());
    assert!(matches!(
        &cells[0].value,
        XlsxCellValue::Number { raw, .. } if raw == "2"
    ));

    // C2: uncached formula stays blank.
    assert!(cells[1].has_formula);
    assert_eq!(cells[1].value, XlsxCellValue::Blank);

    // D2: empty-but-cached value stays blank.
    assert!(cells[2].has_formula);
    assert_eq!(cells[2].value, XlsxCellValue::Blank);

    // E2: error formula keeps the cached error string.
    assert!(cells[3].has_formula);
    assert_eq!(
        cells[3].value,
        XlsxCellValue::Error {
            raw: "#DIV/0!".to_owned(),
        }
    );

    // F2: string-result formula with entity-decoded cached value.
    assert!(cells[4].has_formula);
    assert_eq!(
        cells[4].value,
        XlsxCellValue::String {
            raw: "alpha & beta".to_owned(),
            value: "alpha & beta".to_owned(),
        }
    );

    // G2: shared-string formula cell resolves its index.
    assert!(cells[5].has_formula);
    assert_eq!(
        cells[5].value,
        XlsxCellValue::String {
            raw: "0".to_owned(),
            value: "alpha".to_owned(),
        }
    );

    // H2: CDATA formula text with a numeric cached value.
    assert!(cells[6].has_formula);
    assert!(matches!(
        &cells[6].value,
        XlsxCellValue::Number { raw, .. } if raw == "1"
    ));

    // I2: numeric character references in the formula text.
    assert!(cells[7].has_formula);
    assert!(matches!(
        &cells[7].value,
        XlsxCellValue::Number { raw, .. } if raw == "5"
    ));

    // J2: error cell without a formula element is the non-formula control.
    assert!(!cells[8].has_formula);
    assert_eq!(
        cells[8].value,
        XlsxCellValue::Error {
            raw: "#N/A".to_owned(),
        }
    );

    // Documented invariant over real parser output: `has_formula == false`
    // implies `formula == None`, in both directions it is guaranteed. The
    // converse does not hold in general (unresolved shared slaves in S5).
    for row in &rows {
        for cell in &row.cells {
            assert!(
                cell.has_formula || cell.formula.is_none(),
                "cell in column {} of row {} violates the formula invariant",
                cell.column_index,
                row.row_index
            );
        }
    }
}

#[test]
fn shared_formulas_corpus_loads_and_carries_the_documented_cells() {
    let file = fixtures::build_package("xlsx/shared-formulas", "shared-formulas.xlsx");

    let mut shared_rows = Vec::new();
    let shared_extraction = oxdoc_core::visit_xlsx_rows(
        &file,
        XlsxSheetOptions {
            sheet_name: Some("Shared"),
            ..XlsxSheetOptions::default()
        },
        XlsxValueMode::Formatted,
        |row| {
            shared_rows.push(row.clone());
            Ok(XlsxRowControl::Continue)
        },
    )
    .unwrap();
    assert!(shared_extraction.warnings.is_empty());

    // Sheet "Shared", row 2: shared master, dangling-si slave, array master.
    assert_eq!(shared_rows[0].row_index, 1);
    assert_eq!(
        shared_rows[0]
            .cells
            .iter()
            .map(|cell| cell.column_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 5]
    );
    let shared_row2 = &shared_rows[0].cells;
    // A2: shared master registers its own expression (v1 baseline: has_formula only).
    assert!(shared_row2[0].has_formula);
    assert!(matches!(
        &shared_row2[0].value,
        XlsxCellValue::Number { raw, .. } if raw == "6"
    ));
    // B2: dangling si=9 slave (v1 baseline: no warning yet, S5 adds it).
    assert!(shared_row2[1].has_formula);
    assert!(matches!(
        &shared_row2[1].value,
        XlsxCellValue::Number { raw, .. } if raw == "4"
    ));
    // F2: array master.
    assert!(shared_row2[2].has_formula);
    assert!(matches!(
        &shared_row2[2].value,
        XlsxCellValue::Number { raw, .. } if raw == "3"
    ));

    assert_eq!(shared_rows.len(), 6);

    // Sheet "Shared", row 3: cached slave of si=0, array region cell.
    assert_eq!(shared_rows[1].row_index, 2);
    assert_eq!(
        shared_rows[1]
            .cells
            .iter()
            .map(|cell| cell.column_index)
            .collect::<Vec<_>>(),
        vec![0, 5]
    );
    let row3 = &shared_rows[1].cells;
    // A3: cached slave keeps its own cached value at the v1 baseline.
    assert!(row3[0].has_formula);
    assert!(matches!(
        &row3[0].value,
        XlsxCellValue::Number { raw, .. } if raw == "9"
    ));
    // F3: array region cell with only <v> is the non-formula control.
    assert!(!row3[1].has_formula);
    assert!(matches!(
        &row3[1].value,
        XlsxCellValue::Number { raw, .. } if raw == "5"
    ));

    // Sheet "Shared", row 4: uncached slave of si=0.
    assert_eq!(shared_rows[2].row_index, 3);
    assert_eq!(shared_rows[2].cells.len(), 1);
    assert!(shared_rows[2].cells[0].has_formula);
    assert_eq!(shared_rows[2].cells[0].value, XlsxCellValue::Blank);

    // Sheet "Shared", row 5: first master of si=1.
    assert_eq!(shared_rows[3].row_index, 4);
    assert_eq!(shared_rows[3].cells.len(), 1);
    assert!(shared_rows[3].cells[0].has_formula);
    assert!(matches!(
        &shared_rows[3].cells[0].value,
        XlsxCellValue::Number { raw, .. } if raw == "1"
    ));

    // Sheet "Shared", row 6: duplicate master of si=1.
    assert_eq!(shared_rows[4].row_index, 5);
    assert_eq!(shared_rows[4].cells.len(), 1);
    assert!(shared_rows[4].cells[0].has_formula);
    assert!(matches!(
        &shared_rows[4].cells[0].value,
        XlsxCellValue::Number { raw, .. } if raw == "2"
    ));

    // Sheet "Shared", row 7: slave of si=1.
    assert_eq!(shared_rows[5].row_index, 6);
    assert_eq!(shared_rows[5].cells.len(), 1);
    assert!(shared_rows[5].cells[0].has_formula);
    assert_eq!(shared_rows[5].cells[0].value, XlsxCellValue::Blank);

    let mut prefixed_rows = Vec::new();
    let prefixed_extraction = oxdoc_core::visit_xlsx_rows(
        &file,
        XlsxSheetOptions {
            sheet_name: Some("Prefixed"),
            ..XlsxSheetOptions::default()
        },
        XlsxValueMode::Formatted,
        |row| {
            prefixed_rows.push(row.clone());
            Ok(XlsxRowControl::Continue)
        },
    )
    .unwrap();
    assert!(prefixed_extraction.warnings.is_empty());

    // Sheet "Prefixed": every element namespace-prefixed; slave before master,
    // later master, slave after master (v1 baseline: has_formula only).
    assert_eq!(prefixed_rows.len(), 3);
    for (row_index, row) in prefixed_rows.iter().enumerate() {
        assert_eq!(row.row_index, row_index);
        assert_eq!(
            row.cells
                .iter()
                .map(|cell| cell.column_index)
                .collect::<Vec<_>>(),
            vec![1],
            "only column B is populated on the prefixed sheet row {row_index}"
        );
        assert!(
            row.cells[0].has_formula,
            "prefixed sheet row {row_index} carries an <x:f> element"
        );
    }
    assert!(matches!(
        &prefixed_rows[1].cells[0].value,
        XlsxCellValue::Number { raw, .. } if raw == "1"
    ));
    assert_eq!(prefixed_rows[0].cells[0].value, XlsxCellValue::Blank);
    assert_eq!(prefixed_rows[2].cells[0].value, XlsxCellValue::Blank);

    // Documented invariant over real parser output on both sheets:
    // `has_formula == false` implies `formula == None`.
    for row in shared_rows.iter().chain(prefixed_rows.iter()) {
        for cell in &row.cells {
            assert!(
                cell.has_formula || cell.formula.is_none(),
                "cell in column {} of row {} violates the formula invariant",
                cell.column_index,
                row.row_index
            );
        }
    }
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
fn visits_typed_xlsx_rows_with_read_options_through_path_api() {
    let file = fixtures::build_package("xlsx/basic", "read-options-path.xlsx");
    let mut visited = 0;

    let extraction = oxdoc_core::visit_xlsx_rows_with_read_options(
        &file,
        XlsxReadOptions::new(XlsxSheetOptions {
            sheet_name: Some("Sales Q1"),
            ..XlsxSheetOptions::default()
        })
        .with_worksheet_limits(OoxmlLimits::default()),
        XlsxValueMode::Raw,
        |_| {
            visited += 1;
            Ok(XlsxRowControl::Continue)
        },
    )
    .unwrap();

    assert_eq!(visited, 2);
    assert!(extraction.warnings.is_empty());
}

#[test]
fn visits_typed_xlsx_rows_with_read_options_from_reader() {
    let file = fixtures::build_package("xlsx/basic", "read-options-reader.xlsx");
    let reader = File::open(file).unwrap();
    let mut rows = Vec::new();

    let extraction = oxdoc_core::visit_xlsx_rows_from_reader_with_read_options(
        reader,
        XlsxReadOptions::new(XlsxSheetOptions {
            sheet_index: Some(1),
            ..XlsxSheetOptions::default()
        }),
        XlsxValueMode::Raw,
        |row| {
            rows.push(row.row_index);
            Ok(XlsxRowControl::Stop)
        },
    )
    .unwrap();

    assert_eq!(rows, vec![0]);
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
            max_package_uncompressed_size: 256 * 1024 * 1024,
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
        "xlsx-formatted-locale.md",
        "xlsx-formulas.md",
        "xlsx-shared-formulas.md",
        "pptx-basic.md",
        "pptx-text.md",
        "pptx-python-pptx-basic.md",
        "docx-external-target.md",
        "pptx-missing-target.md",
        "pptx-malformed-xml.md",
    ] {
        let note = fixtures::read_provenance(provenance);
        assert!(note.contains("Source:"));
        assert!(note.contains("Producer:"));
        assert!(note.contains("Redistribution:"));
        assert!(note.contains("Purpose:"));
    }

    for provenance in [
        "xlsx-basic.md",
        "xlsx-app-metadata.md",
        "xlsx-formulas.md",
        "xlsx-shared-formulas.md",
    ] {
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

/// Zips the hand-authored section-order package tree deterministically
/// (mirrors `fixtures::build_package`, which reads from `fixtures/corpus`).
fn build_section_order_package() -> PathBuf {
    const PACKAGE_DIR: &str = "tests/fixtures/docx/section-order/package";
    const ENTRY_NAMES: [&str; 13] = [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/_rels/document.xml.rels",
        "word/comments.xml",
        "word/document.xml",
        "word/footer1.xml",
        "word/footer2.xml",
        "word/footnotes.xml",
        "word/header-default.xml",
        "word/header-even.xml",
        "word/header-first.xml",
        "word/header-orphan.xml",
        "word/header-titled.xml",
    ];
    let package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(PACKAGE_DIR);
    let entries: Vec<(String, String)> = ENTRY_NAMES
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                fs::read_to_string(package_dir.join(name)).unwrap(),
            )
        })
        .collect();
    let entry_refs: Vec<(&str, &str)> = entries
        .iter()
        .map(|(name, content)| (name.as_str(), content.as_str()))
        .collect();
    create_ooxml("docx-section-order.docx", &entry_refs)
}

fn section_order_oracle() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/docx/section-order/expected.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn oracle_parts_with_variant(oracle: &serde_json::Value) -> Vec<(String, String, Option<String>)> {
    oracle["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| {
            (
                part["part_type"].as_str().unwrap().to_owned(),
                part["part_path"].as_str().unwrap().to_owned(),
                part["variant"].as_str().map(|variant| variant.to_owned()),
            )
        })
        .collect()
}

fn oracle_warning_messages(oracle: &serde_json::Value) -> Vec<String> {
    oracle["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning["message"].as_str().unwrap().to_owned())
        .collect()
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

#[test]
fn extracts_docx_text_and_structured_text_from_readers_with_limits() {
    let file = fixtures::build_package("docx/basic", "limits-reader.docx");
    let bytes = fs::read(&file).unwrap();

    let text = oxdoc_core::extract_docx_text_from_reader_with_limits(
        Cursor::new(bytes.clone()),
        OoxmlLimits::default(),
    )
    .unwrap();
    assert_eq!(
        text.value.trim_end(),
        fixtures::read_snapshot("docx_basic_text.txt").trim_end()
    );
    assert!(text.warnings.is_empty());

    let structured = oxdoc_core::extract_docx_structured_text_from_reader_with_limits(
        Cursor::new(bytes),
        OoxmlLimits::default(),
    )
    .unwrap();
    assert_eq!(structured.value.blocks.len(), 1);
    assert_eq!(structured.value.blocks[0].part_type, "main");
    assert!(structured.warnings.is_empty());
}

#[test]
fn omits_related_parts_from_docx_structured_text_and_tables() {
    let file = fixtures::build_package("docx/policies", "policies-no-related.docx");
    let options = DocxTextOptions {
        include_related_parts: false,
        ..DocxTextOptions::default()
    };

    let structured = oxdoc_core::extract_docx_structured_text_with_options(&file, options).unwrap();
    assert_eq!(structured.value.blocks.len(), 1);
    assert_eq!(structured.value.blocks[0].part_type, "main");

    let tables = oxdoc_core::extract_docx_tables_with_options(&file, options).unwrap();
    assert!(tables.value.tables.is_empty());
    assert!(tables.warnings.is_empty());
}

#[test]
fn skips_unrelated_docx_parts_and_reports_missing_parts_for_every_extraction() {
    let file = create_ooxml(
        "docx-related-part-matrix.docx",
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
                r#"<Relationships><Relationship Id="rImage" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/><Relationship Id="rMissing" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="missing-footer.xml"/><Relationship Id="rFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/></Relationships>"#,
            ),
            (
                "word/footer1.xml",
                r#"<w:ftr xmlns:w="w"><w:p><w:r><w:t>Footer</w:t></w:r></w:p></w:ftr>"#,
            ),
        ],
    );

    let text = oxdoc_core::extract_docx_text(&file).unwrap();
    assert_eq!(text.value, "Body\nFooter\n");
    assert_eq!(text.warnings.len(), 1);
    assert!(
        text.warnings[0]
            .message
            .contains("skipped related DOCX text part word/missing-footer.xml")
    );

    let structured = oxdoc_core::extract_docx_structured_text(&file).unwrap();
    let part_types: Vec<&str> = structured
        .value
        .blocks
        .iter()
        .map(|block| block.part_type.as_str())
        .collect();
    assert_eq!(part_types, ["main", "footer"]);
    assert_eq!(structured.warnings.len(), 1);
    assert!(
        structured.warnings[0]
            .message
            .contains("skipped related DOCX text part word/missing-footer.xml")
    );

    let tables = oxdoc_core::extract_docx_tables(&file).unwrap();
    assert!(tables.value.tables.is_empty());
    assert_eq!(tables.warnings.len(), 1);
    assert!(
        tables.warnings[0]
            .message
            .contains("skipped related DOCX table part word/missing-footer.xml")
    );
}

#[test]
fn rejects_escaping_docx_related_part_targets_for_every_extraction() {
    let file = create_ooxml(
        "docx-escaping-related-part.docx",
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
                r#"<Relationships><Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="../../footer.xml"/></Relationships>"#,
            ),
        ],
    );

    for err in [
        oxdoc_core::extract_docx_text(&file).unwrap_err(),
        oxdoc_core::extract_docx_structured_text(&file).unwrap_err(),
        oxdoc_core::extract_docx_tables(&file).unwrap_err(),
    ] {
        assert!(
            matches!(err, OxdocError::SuspiciousRelationshipTarget { ref reason, .. }
                if reason.contains("escapes")),
            "unexpected error: {err:?}"
        );
    }
}

#[test]
fn propagates_resource_limit_errors_from_related_docx_parts() {
    let footer_text = "F".repeat(9000);
    let file = create_ooxml(
        "docx-related-part-limit.docx",
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
                r#"<Relationships><Relationship Id="rFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/></Relationships>"#,
            ),
            (
                "word/footer1.xml",
                &format!(
                    r#"<w:ftr xmlns:w="w"><w:p><w:r><w:t>{footer_text}</w:t></w:r></w:p></w:ftr>"#
                ),
            ),
        ],
    );
    let limits = OoxmlLimits {
        max_part_uncompressed_size: 4096,
        ..OoxmlLimits::default()
    };
    let bytes = fs::read(&file).unwrap();

    let text_err = oxdoc_core::extract_docx_text_from_reader_with_options_and_limits(
        Cursor::new(bytes.clone()),
        DocxTextOptions::default(),
        limits,
    )
    .unwrap_err();
    assert!(matches!(text_err, OxdocError::PartTooLarge { .. }));

    let structured_err =
        oxdoc_core::extract_docx_structured_text_from_reader_with_options_and_limits(
            Cursor::new(bytes.clone()),
            DocxTextOptions::default(),
            limits,
        )
        .unwrap_err();
    assert!(matches!(structured_err, OxdocError::PartTooLarge { .. }));

    let tables_err = oxdoc_core::extract_docx_tables_from_reader_with_options_and_limits(
        Cursor::new(bytes),
        DocxTextOptions::default(),
        limits,
    )
    .unwrap_err();
    assert!(matches!(tables_err, OxdocError::PartTooLarge { .. }));
}

#[test]
fn appends_related_docx_text_after_separator_and_skips_empty_parts() {
    let file = create_ooxml(
        "docx-related-text-separator.docx",
        &[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            (
                "word/document.xml",
                r#"<w:document xmlns:w="w"><w:body><w:r><w:t>A</w:t></w:r></w:body></w:document>"#,
            ),
            (
                "word/_rels/document.xml.rels",
                r#"<Relationships><Relationship Id="rEmpty" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/><Relationship Id="rFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer2.xml"/></Relationships>"#,
            ),
            ("word/footer1.xml", r#"<w:ftr xmlns:w="w"/>"#),
            (
                "word/footer2.xml",
                r#"<w:ftr xmlns:w="w"><w:p><w:r><w:t>F</w:t></w:r></w:p></w:ftr>"#,
            ),
        ],
    );

    let extraction = oxdoc_core::extract_docx_text(&file).unwrap();

    assert_eq!(extraction.value, "A\nF\n");
    assert!(extraction.warnings.is_empty());
}
