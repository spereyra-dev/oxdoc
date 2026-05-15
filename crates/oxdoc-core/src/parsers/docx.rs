use std::io::Cursor;
use std::io::{BufRead, BufReader, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{Extraction, OutputWarning, StructuredText, TextBlock};
use crate::parsers::find_office_document_path;
use crate::parsers::{
    decode_xml_reference, decode_xml_text, name_eq, parent_dir, parse_relationships, rels_path_for,
    resolve_relationship_target,
};
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

pub(crate) fn extract_text<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
) -> Result<Extraction<String>> {
    let document_path = find_office_document_path(package, "word/document.xml")?;
    let document = extract_part_text(package, &document_path)?;
    let relationships_path = rels_path_for(&document_path);

    let mut text = document.value;
    let mut warnings = document.warnings;

    let relationships_xml = match package.read_to_string(&relationships_path) {
        Ok(xml) => xml,
        Err(OxdocError::MissingPart(_)) => return Ok(Extraction::with_warnings(text, warnings)),
        Err(err) => return Err(err),
    };

    for relationship in parse_relationships(&relationships_xml, &relationships_path)? {
        if !is_related_docx_text_part(relationship.relationship_type.as_deref()) {
            continue;
        }

        let part_path = resolve_relationship_target(
            parent_dir(&document_path),
            &relationship,
            &relationships_path,
        )?;
        match extract_part_text(package, &part_path) {
            Ok(part) => {
                append_related_text(&mut text, &part.value);
                warnings.extend(part.warnings);
            }
            Err(OxdocError::MissingPart(part)) => warnings.push(OutputWarning::new(
                &relationships_path,
                format!("skipped related DOCX text part {part}: missing part"),
            )),
            Err(err) => return Err(err),
        }
    }

    Ok(Extraction::with_warnings(text, warnings))
}

pub(crate) fn extract_structured_text<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
) -> Result<Extraction<StructuredText>> {
    let document_path = find_office_document_path(package, "word/document.xml")?;
    let document = extract_part_text(package, &document_path)?;
    let mut blocks = Vec::new();
    push_text_block(&mut blocks, "main", &document_path, document.value);
    let mut warnings = document.warnings;

    let relationships_path = rels_path_for(&document_path);
    let relationships_xml = match package.read_to_string(&relationships_path) {
        Ok(xml) => xml,
        Err(OxdocError::MissingPart(_)) => {
            return Ok(Extraction::with_warnings(
                StructuredText {
                    document_type: "docx".to_owned(),
                    blocks,
                },
                warnings,
            ));
        }
        Err(err) => return Err(err),
    };

    for relationship in parse_relationships(&relationships_xml, &relationships_path)? {
        let Some(part_type) =
            related_docx_text_part_type(relationship.relationship_type.as_deref())
        else {
            continue;
        };

        let part_path = resolve_relationship_target(
            parent_dir(&document_path),
            &relationship,
            &relationships_path,
        )?;
        match extract_part_text(package, &part_path) {
            Ok(part) => {
                push_text_block(&mut blocks, part_type, &part_path, part.value);
                warnings.extend(part.warnings);
            }
            Err(OxdocError::MissingPart(part)) => warnings.push(OutputWarning::new(
                &relationships_path,
                format!("skipped related DOCX text part {part}: missing part"),
            )),
            Err(err) => return Err(err),
        }
    }

    Ok(Extraction::with_warnings(
        StructuredText {
            document_type: "docx".to_owned(),
            blocks,
        },
        warnings,
    ))
}

fn extract_part_text<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    path: &str,
) -> Result<Extraction<String>> {
    package.with_entry(path, |entry| {
        let reader = BufReader::new(entry);
        extract_xml_text(reader, path)
    })
}

fn is_related_docx_text_part(relationship_type: Option<&str>) -> bool {
    related_docx_text_part_type(relationship_type).is_some()
}

fn related_docx_text_part_type(relationship_type: Option<&str>) -> Option<&'static str> {
    let kind = relationship_type?;
    if kind.ends_with("/header") {
        Some("header")
    } else if kind.ends_with("/footer") {
        Some("footer")
    } else if kind.ends_with("/footnotes") {
        Some("footnotes")
    } else if kind.ends_with("/endnotes") {
        Some("endnotes")
    } else if kind.ends_with("/comments") {
        Some("comments")
    } else {
        None
    }
}

fn push_text_block(blocks: &mut Vec<TextBlock>, part_type: &str, part_path: &str, text: String) {
    if text.is_empty() {
        return;
    }
    blocks.push(TextBlock::new(part_type, part_path, blocks.len() + 1, text));
}

fn append_related_text(text: &mut String, related_text: &str) {
    if related_text.is_empty() {
        return;
    }

    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(related_text);
}

fn extract_xml_text<R: BufRead>(source: R, path: &str) -> Result<Extraction<String>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut warnings = Vec::new();
    let mut in_text_node = false;
    let mut deleted_revision_depth = 0usize;
    let mut table_contexts = Vec::new();
    let mut pending_cell_paragraph_separator = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"del") {
                    deleted_revision_depth += 1;
                } else if name_eq(element.name().as_ref(), b"tbl") {
                    if in_table_cell(&table_contexts) {
                        flush_cell_paragraph_separator(
                            &mut text,
                            &mut pending_cell_paragraph_separator,
                        );
                    }
                    table_contexts.push(TableContext::default());
                } else if let Some(table) = table_contexts.last_mut()
                    && name_eq(element.name().as_ref(), b"tr")
                {
                    table.start_row();
                    pending_cell_paragraph_separator = false;
                } else if let Some(table) = table_contexts.last_mut()
                    && name_eq(element.name().as_ref(), b"tc")
                {
                    table.start_cell(&mut text);
                    pending_cell_paragraph_separator = false;
                } else if deleted_revision_depth == 0 && name_eq(element.name().as_ref(), b"t") {
                    in_text_node = true;
                }
            }
            Ok(Event::Empty(element)) if deleted_revision_depth == 0 => {
                if name_eq(element.name().as_ref(), b"tab") {
                    flush_cell_paragraph_separator(
                        &mut text,
                        &mut pending_cell_paragraph_separator,
                    );
                    text.push('\t');
                } else if name_eq(element.name().as_ref(), b"br")
                    || name_eq(element.name().as_ref(), b"cr")
                {
                    pending_cell_paragraph_separator = false;
                    push_newline(&mut text);
                }
            }
            Ok(Event::Text(value)) if in_text_node => {
                push_text(
                    &mut text,
                    &decode_xml_text(value.as_ref()),
                    &mut pending_cell_paragraph_separator,
                );
            }
            Ok(Event::CData(value)) if in_text_node => {
                push_text(
                    &mut text,
                    &decode_xml_text(value.as_ref()),
                    &mut pending_cell_paragraph_separator,
                );
            }
            Ok(Event::GeneralRef(value)) if in_text_node => {
                push_text(
                    &mut text,
                    &decode_xml_reference(value.as_ref()),
                    &mut pending_cell_paragraph_separator,
                );
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text_node = false;
                } else if name_eq(element.name().as_ref(), b"del") {
                    deleted_revision_depth = deleted_revision_depth.saturating_sub(1);
                    in_text_node = false;
                } else if name_eq(element.name().as_ref(), b"p") {
                    if deleted_revision_depth == 0 {
                        if in_table_cell(&table_contexts) {
                            pending_cell_paragraph_separator = !text.is_empty();
                        } else {
                            push_newline(&mut text);
                        }
                    }
                } else if name_eq(element.name().as_ref(), b"tc") {
                    if let Some(table) = table_contexts.last_mut() {
                        table.end_cell();
                    }
                    pending_cell_paragraph_separator = false;
                } else if name_eq(element.name().as_ref(), b"tr") {
                    let nested_table = table_contexts.len() > 1;
                    if let Some(table) = table_contexts.last_mut()
                        && table.finish_row()
                    {
                        pending_cell_paragraph_separator = false;
                        if nested_table {
                            pending_cell_paragraph_separator = true;
                        } else {
                            push_newline(&mut text);
                        }
                    }
                } else if name_eq(element.name().as_ref(), b"tbl") {
                    table_contexts.pop();
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

    Ok(Extraction::with_warnings(text, warnings))
}

#[doc(hidden)]
pub fn fuzz_extract_text(xml: &[u8]) -> Result<()> {
    let _ = extract_xml_text(Cursor::new(xml), "word/document.xml")?;
    Ok(())
}

#[derive(Debug, Default)]
struct TableContext {
    row_depth: usize,
    cell_depth: usize,
    row_has_cells: bool,
}

impl TableContext {
    fn start_row(&mut self) {
        self.row_depth += 1;
        self.row_has_cells = false;
    }

    fn start_cell(&mut self, text: &mut String) {
        if self.row_depth == 0 {
            return;
        }

        if self.row_has_cells {
            text.push('\t');
        } else {
            self.row_has_cells = true;
        }
        self.cell_depth += 1;
    }

    fn end_cell(&mut self) {
        self.cell_depth = self.cell_depth.saturating_sub(1);
    }

    fn finish_row(&mut self) -> bool {
        let had_cells = self.row_has_cells;
        self.row_depth = self.row_depth.saturating_sub(1);
        self.row_has_cells = false;
        had_cells
    }
}

fn in_table_cell(table_contexts: &[TableContext]) -> bool {
    table_contexts
        .last()
        .is_some_and(|table| table.cell_depth > 0)
}

fn push_text(text: &mut String, value: &str, pending_cell_paragraph_separator: &mut bool) {
    if value.is_empty() {
        return;
    }
    if *pending_cell_paragraph_separator {
        flush_cell_paragraph_separator(text, pending_cell_paragraph_separator);
    }
    text.push_str(value);
}

fn flush_cell_paragraph_separator(text: &mut String, pending_cell_paragraph_separator: &mut bool) {
    if *pending_cell_paragraph_separator && !text.chars().last().is_some_and(char::is_whitespace) {
        text.push(' ');
    }
    *pending_cell_paragraph_separator = false;
}

fn push_newline(text: &mut String) {
    if !text.ends_with('\n') {
        text.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::extract_xml_text;

    #[test]
    fn extracts_word_text_with_logical_breaks() {
        let xml = r#"
            <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
              <w:body>
                <w:p><w:r><w:t>Hola</w:t></w:r><w:r><w:tab/><w:t>Mundo</w:t></w:r></w:p>
                <w:p><w:r><w:t>Segundo &amp; final</w:t></w:r></w:p>
              </w:body>
            </w:document>
        "#;

        let result = extract_xml_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();

        assert_eq!(result.value, "Hola\tMundo\nSegundo & final\n");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn returns_partial_text_after_malformed_xml() {
        let xml = br#"<w:document><w:p><w:r><w:t>Hola</w:t></w:r></w:p><"#;

        let result = extract_xml_text(Cursor::new(xml.as_slice()), "word/document.xml").unwrap();

        assert_eq!(result.value, "Hola\n");
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn handles_cdata_breaks_and_empty_document() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:p><w:r><w:t><![CDATA[A < B]]></w:t><w:br/><w:t>&#67;</w:t></w:r></w:p>
                <w:p><w:r><w:cr/></w:r></w:p>
              </w:body>
            </w:document>
        "#;

        let result = extract_xml_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();
        let empty = extract_xml_text(Cursor::new(b"<w:document/>"), "word/document.xml").unwrap();

        assert_eq!(result.value, "A < B\nC\n");
        assert!(empty.value.is_empty());
    }

    #[test]
    fn extracts_drawing_text_by_local_text_name() {
        let xml = r#"
            <w:document xmlns:w="w" xmlns:a="a">
              <w:body>
                <w:p><w:r><w:drawing><a:t>Drawing text</a:t></w:drawing></w:r></w:p>
              </w:body>
            </w:document>
        "#;

        let result = extract_xml_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();

        assert_eq!(result.value, "Drawing text\n");
    }

    #[test]
    fn extracts_table_cells_with_logical_separators() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:tbl>
                  <w:tr>
                    <w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc>
                  </w:tr>
                  <w:tr>
                    <w:tc>
                      <w:p><w:r><w:t>C one</w:t></w:r></w:p>
                      <w:p><w:r><w:t>C two</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc><w:p><w:r><w:t>D</w:t></w:r></w:p></w:tc>
                  </w:tr>
                </w:tbl>
              </w:body>
            </w:document>
        "#;

        let result = extract_xml_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();

        assert_eq!(result.value, "A\tB\nC one C two\tD\n");
    }

    #[test]
    fn flattens_nested_tables_without_resetting_outer_rows() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:tbl>
                  <w:tr>
                    <w:tc>
                      <w:p><w:r><w:t>Outer</w:t></w:r></w:p>
                      <w:tbl>
                        <w:tr>
                          <w:tc><w:p><w:r><w:t>Inner</w:t></w:r></w:p></w:tc>
                        </w:tr>
                      </w:tbl>
                    </w:tc>
                    <w:tc><w:p><w:r><w:t>Sibling</w:t></w:r></w:p></w:tc>
                  </w:tr>
                </w:tbl>
              </w:body>
            </w:document>
        "#;

        let result = extract_xml_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();

        assert_eq!(result.value, "Outer Inner\tSibling\n");
    }

    #[test]
    fn omits_deleted_revision_text_and_keeps_inserted_text() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:p>
                  <w:r><w:t>Keep </w:t></w:r>
                  <w:del><w:r><w:t>deleted</w:t></w:r></w:del>
                  <w:ins><w:r><w:t>inserted</w:t></w:r></w:ins>
                </w:p>
              </w:body>
            </w:document>
        "#;

        let result = extract_xml_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();

        assert_eq!(result.value, "Keep inserted\n");
    }

    #[test]
    fn keeps_field_results_omits_list_markers_and_includes_hidden_runs() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:p>
                  <w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>
                  <w:r><w:t>List item</w:t></w:r>
                </w:p>
                <w:p>
                  <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                  <w:r><w:instrText>DATE</w:instrText></w:r>
                  <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                  <w:r><w:t>2026-04-14</w:t></w:r>
                  <w:r><w:fldChar w:fldCharType="end"/></w:r>
                </w:p>
                <w:p>
                  <w:r><w:rPr><w:vanish/></w:rPr><w:t>Hidden text</w:t></w:r>
                </w:p>
              </w:body>
            </w:document>
        "#;

        let result = extract_xml_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();

        assert_eq!(result.value, "List item\n2026-04-14\nHidden text\n");
    }
}
