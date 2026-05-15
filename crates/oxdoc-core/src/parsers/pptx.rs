use std::io::{BufRead, BufReader, Cursor, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::models::{Extraction, OutputWarning, StructuredText, TextBlock};
use crate::parsers::{
    append_decoded_xml_reference, append_decoded_xml_text, decode_xml_text, merge_warnings,
    name_eq, parent_dir, parse_relationship_map, rels_path_for, resolve_relationship_target,
};
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

pub(crate) fn extract_text<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
) -> Result<Extraction<String>> {
    let presentation_path =
        crate::parsers::find_office_document_path(package, "ppt/presentation.xml")?;
    let presentation_xml = package.read_to_string(&presentation_path)?;
    let slide_ids = parse_slide_relation_ids(&presentation_xml, &presentation_path)?;

    let presentation_rels_path = rels_path_for(&presentation_path);
    let presentation_rels_xml = package.read_to_string(&presentation_rels_path)?;
    let presentation_rels =
        parse_relationship_map(&presentation_rels_xml, &presentation_rels_path)?;

    let mut text = String::new();
    let mut warnings = slide_ids.warnings;

    for slide_id in slide_ids.value {
        let relationship = presentation_rels
            .get(&slide_id)
            .ok_or_else(|| OxdocError::MissingPart(slide_id.clone()))?;
        let slide_path = resolve_relationship_target(
            parent_dir(&presentation_path),
            relationship,
            &presentation_rels_path,
        )?;

        let slide = read_text_part(package, &slide_path)?;
        append_part_text(&mut text, &slide.value);
        warnings = merge_warnings(warnings, slide.warnings);

        let notes = read_notes_for_slide(package, &slide_path)?;
        append_part_text(&mut text, &notes.value);
        warnings = merge_warnings(warnings, notes.warnings);
    }

    Ok(Extraction::with_warnings(text, warnings))
}

pub(crate) fn extract_structured_text<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
) -> Result<Extraction<StructuredText>> {
    let presentation_path =
        crate::parsers::find_office_document_path(package, "ppt/presentation.xml")?;
    let presentation_xml = package.read_to_string(&presentation_path)?;
    let slide_ids = parse_slide_relation_ids(&presentation_xml, &presentation_path)?;

    let presentation_rels_path = rels_path_for(&presentation_path);
    let presentation_rels_xml = package.read_to_string(&presentation_rels_path)?;
    let presentation_rels =
        parse_relationship_map(&presentation_rels_xml, &presentation_rels_path)?;

    let mut blocks = Vec::new();
    let mut warnings = slide_ids.warnings;

    for slide_id in slide_ids.value {
        let relationship = presentation_rels
            .get(&slide_id)
            .ok_or_else(|| OxdocError::MissingPart(slide_id.clone()))?;
        let slide_path = resolve_relationship_target(
            parent_dir(&presentation_path),
            relationship,
            &presentation_rels_path,
        )?;

        let slide = read_text_part(package, &slide_path)?;
        push_text_block(&mut blocks, "slide", &slide_path, slide.value);
        warnings = merge_warnings(warnings, slide.warnings);

        let notes = read_notes_blocks_for_slide(package, &slide_path)?;
        blocks.extend(notes.value);
        renumber_blocks(&mut blocks);
        warnings = merge_warnings(warnings, notes.warnings);
    }

    Ok(Extraction::with_warnings(
        StructuredText {
            document_type: "pptx".to_owned(),
            blocks,
        },
        warnings,
    ))
}

fn parse_slide_relation_ids(xml: &str, path: &str) -> Result<Extraction<Vec<String>>> {
    let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes()));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut slide_ids = Vec::new();
    let mut warnings = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if name_eq(element.name().as_ref(), b"sldId") =>
            {
                if let Some(relation_id) = relationship_id_value(&element) {
                    slide_ids.push(relation_id);
                } else {
                    warnings.push(OutputWarning::new(
                        path,
                        "ignored presentation slide without relationship id",
                    ));
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

    Ok(Extraction::with_warnings(slide_ids, warnings))
}

fn relationship_id_value(element: &BytesStart<'_>) -> Option<String> {
    element
        .attributes()
        .with_checks(false)
        .flatten()
        .find(|attr| {
            let key = attr.key.as_ref();
            key.contains(&b':') && crate::parsers::local_name(key) == b"id"
        })
        .map(|attr| decode_xml_text(attr.value.as_ref()))
}

fn read_notes_for_slide<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    slide_path: &str,
) -> Result<Extraction<String>> {
    let slide_rels_path = rels_path_for(slide_path);
    let slide_rels_xml = match package.read_to_string(&slide_rels_path) {
        Ok(xml) => xml,
        Err(OxdocError::MissingPart(_)) => return Ok(Extraction::new(String::new())),
        Err(err) => return Err(err),
    };
    let slide_rels = parse_relationship_map(&slide_rels_xml, &slide_rels_path)?;
    let mut notes_relationships = slide_rels
        .iter()
        .filter(|(_, relationship)| {
            relationship
                .relationship_type
                .as_deref()
                .is_some_and(|kind| kind.ends_with("/notesSlide"))
        })
        .collect::<Vec<_>>();
    notes_relationships.sort_by(|left, right| left.0.cmp(right.0));

    let mut text = String::new();
    let mut warnings = Vec::new();

    for (_, relationship) in notes_relationships {
        let notes_path =
            resolve_relationship_target(parent_dir(slide_path), relationship, &slide_rels_path)?;
        let notes = read_text_part(package, &notes_path)?;
        append_part_text(&mut text, &notes.value);
        warnings = merge_warnings(warnings, notes.warnings);
    }

    Ok(Extraction::with_warnings(text, warnings))
}

fn read_notes_blocks_for_slide<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    slide_path: &str,
) -> Result<Extraction<Vec<TextBlock>>> {
    let slide_rels_path = rels_path_for(slide_path);
    let slide_rels_xml = match package.read_to_string(&slide_rels_path) {
        Ok(xml) => xml,
        Err(OxdocError::MissingPart(_)) => return Ok(Extraction::new(Vec::new())),
        Err(err) => return Err(err),
    };
    let slide_rels = parse_relationship_map(&slide_rels_xml, &slide_rels_path)?;
    let mut notes_relationships = slide_rels
        .iter()
        .filter(|(_, relationship)| {
            relationship
                .relationship_type
                .as_deref()
                .is_some_and(|kind| kind.ends_with("/notesSlide"))
        })
        .collect::<Vec<_>>();
    notes_relationships.sort_by(|left, right| left.0.cmp(right.0));

    let mut blocks = Vec::new();
    let mut warnings = Vec::new();

    for (_, relationship) in notes_relationships {
        let notes_path =
            resolve_relationship_target(parent_dir(slide_path), relationship, &slide_rels_path)?;
        let notes = read_text_part(package, &notes_path)?;
        push_text_block(&mut blocks, "notes", &notes_path, notes.value);
        warnings = merge_warnings(warnings, notes.warnings);
    }

    Ok(Extraction::with_warnings(blocks, warnings))
}

fn read_text_part<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    path: &str,
) -> Result<Extraction<String>> {
    package.with_entry(path, |entry| {
        let reader = BufReader::new(entry);
        extract_text_part(reader, path)
    })
}

fn extract_text_part<R: BufRead>(source: R, path: &str) -> Result<Extraction<String>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut warnings = Vec::new();
    let mut in_text_node = false;
    let mut paragraph_has_content = false;
    let mut decoded = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text_node = true;
                } else if name_eq(element.name().as_ref(), b"tab") {
                    text.push('\t');
                    paragraph_has_content = true;
                } else if name_eq(element.name().as_ref(), b"br")
                    || name_eq(element.name().as_ref(), b"cr")
                {
                    push_newline(&mut text);
                    paragraph_has_content = true;
                }
            }
            Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"tab") {
                    text.push('\t');
                    paragraph_has_content = true;
                } else if name_eq(element.name().as_ref(), b"br")
                    || name_eq(element.name().as_ref(), b"cr")
                {
                    push_newline(&mut text);
                    paragraph_has_content = true;
                }
            }
            Ok(Event::Text(value)) if in_text_node => {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                paragraph_has_content |= push_text(&mut text, &decoded);
            }
            Ok(Event::CData(value)) if in_text_node => {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                paragraph_has_content |= push_text(&mut text, &decoded);
            }
            Ok(Event::GeneralRef(value)) if in_text_node => {
                decoded.clear();
                append_decoded_xml_reference(value.as_ref(), &mut decoded);
                paragraph_has_content |= push_text(&mut text, &decoded);
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text_node = false;
                } else if name_eq(element.name().as_ref(), b"p") {
                    if paragraph_has_content {
                        push_newline(&mut text);
                    }
                    paragraph_has_content = false;
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

fn append_part_text(text: &mut String, part: &str) {
    if part.is_empty() {
        return;
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(part);
}

fn push_text_block(blocks: &mut Vec<TextBlock>, part_type: &str, part_path: &str, text: String) {
    if text.is_empty() {
        return;
    }
    blocks.push(TextBlock::new(part_type, part_path, blocks.len() + 1, text));
}

fn renumber_blocks(blocks: &mut [TextBlock]) {
    for (index, block) in blocks.iter_mut().enumerate() {
        block.ordinal = index + 1;
    }
}

fn push_text(text: &mut String, value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    text.push_str(value);
    true
}

fn push_newline(text: &mut String) {
    if !text.ends_with('\n') {
        text.push('\n');
    }
}

#[doc(hidden)]
pub fn fuzz_extract_text(xml: &[u8]) -> Result<()> {
    let _ = extract_text_part(Cursor::new(xml), "ppt/slides/slide1.xml")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{extract_text_part, parse_slide_relation_ids};

    #[test]
    fn extracts_drawing_text_with_breaks_tabs_and_cdata() {
        let xml = r#"
            <p:sld xmlns:p="p" xmlns:a="a">
              <p:cSld>
                <p:spTree>
                  <p:sp>
                    <p:txBody>
                      <a:p><a:r><a:t>Title</a:t></a:r></a:p>
                      <a:p><a:r><a:t>A</a:t></a:r><a:tab/><a:r><a:t>B &amp; C</a:t></a:r><a:br/><a:r><a:t><![CDATA[D < E]]></a:t></a:r></a:p>
                    </p:txBody>
                  </p:sp>
                </p:spTree>
              </p:cSld>
            </p:sld>
        "#;

        let result =
            extract_text_part(Cursor::new(xml.as_bytes()), "ppt/slides/slide1.xml").unwrap();

        assert_eq!(result.value, "Title\nA\tB & C\nD < E\n");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn keeps_partial_text_after_malformed_xml() {
        let result = extract_text_part(
            Cursor::new(br#"<p:sld><a:p><a:r><a:t>partial</a:t></a:r><"#.as_slice()),
            "ppt/slides/slide1.xml",
        )
        .unwrap();

        assert_eq!(result.value, "partial");
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn omits_empty_text_paragraphs() {
        let xml = r#"
            <p:sld xmlns:p="p" xmlns:a="a">
              <a:p><a:r><a:t></a:t></a:r></a:p>
              <a:p><a:r><a:t>visible</a:t></a:r></a:p>
            </p:sld>
        "#;

        let result =
            extract_text_part(Cursor::new(xml.as_bytes()), "ppt/slides/slide1.xml").unwrap();

        assert_eq!(result.value, "visible\n");
    }

    #[test]
    fn parses_slide_relationship_ids_in_presentation_order() {
        let xml = r#"
            <p:presentation xmlns:p="p" xmlns:r="r">
              <p:sldIdLst>
                <p:sldId id="256" r:id="rId2"/>
                <p:sldId id="257" r:id="rId1"/>
              </p:sldIdLst>
            </p:presentation>
        "#;

        let result = parse_slide_relation_ids(xml, "ppt/presentation.xml").unwrap();

        assert_eq!(result.value, ["rId2", "rId1"]);
        assert!(result.warnings.is_empty());
    }
}
