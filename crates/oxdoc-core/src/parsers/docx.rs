use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::io::{BufRead, BufReader, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{
    DocxRevisionMode, DocxTableBlock as PublicDocxTableBlock, DocxTableCell as PublicDocxTableCell,
    DocxTableRow as PublicDocxTableRow, DocxTables, DocxTextOptions, Extraction, OutputWarning,
    StructuredText, TextBlock,
};
use crate::parsers::find_office_document_path;
use crate::parsers::{
    Relationship, append_decoded_xml_reference, append_decoded_xml_text, attr_value, name_eq,
    parent_dir, parse_relationships, rels_path_for, resolve_relationship_target,
};
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

pub(crate) fn extract_text<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    options: DocxTextOptions,
) -> Result<Extraction<String>> {
    let document_path = find_office_document_path(package, "word/document.xml")?;
    let document = extract_part_text(package, &document_path, options)?;
    let relationships_path = rels_path_for(&document_path);

    let mut text = document.value;
    let mut warnings = document.warnings;

    if !options.include_related_parts {
        return Ok(Extraction::with_warnings(text, warnings));
    }
    let relationships_xml = match package.read_to_string(&relationships_path) {
        Ok(xml) => xml,
        Err(OxdocError::MissingPart(_)) => return Ok(Extraction::with_warnings(text, warnings)),
        Err(err) => return Err(err),
    };

    let relationships = parse_relationships(&relationships_xml, &relationships_path)?;
    let plan = plan_related_part_order(
        package,
        &document_path,
        &relationships,
        &relationships_path,
        options,
        &mut warnings,
    )?;
    for entry in plan {
        let relationship = &relationships[entry.index];
        if !is_related_docx_text_part(relationship.relationship_type.as_deref(), options) {
            continue;
        }

        let part_path = resolve_relationship_target(
            parent_dir(&document_path),
            relationship,
            &relationships_path,
        )?;
        match extract_part_text(package, &part_path, options) {
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
    options: DocxTextOptions,
) -> Result<Extraction<StructuredText>> {
    let document_path = find_office_document_path(package, "word/document.xml")?;
    let document = extract_part_text(package, &document_path, options)?;
    let mut blocks = Vec::new();
    push_text_block(&mut blocks, "main", &document_path, None, document.value);
    let mut warnings = document.warnings;

    if !options.include_related_parts {
        return Ok(Extraction::with_warnings(
            StructuredText {
                document_type: "docx".to_owned(),
                blocks,
            },
            warnings,
        ));
    }
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

    let relationships = parse_relationships(&relationships_xml, &relationships_path)?;
    let plan = plan_related_part_order(
        package,
        &document_path,
        &relationships,
        &relationships_path,
        options,
        &mut warnings,
    )?;
    for entry in plan {
        let relationship = &relationships[entry.index];
        let Some(part_type) =
            related_docx_text_part_type(relationship.relationship_type.as_deref(), options)
        else {
            continue;
        };

        let part_path = resolve_relationship_target(
            parent_dir(&document_path),
            relationship,
            &relationships_path,
        )?;
        match extract_part_text(package, &part_path, options) {
            Ok(part) => {
                push_text_block(
                    &mut blocks,
                    part_type,
                    &part_path,
                    entry.variant,
                    part.value,
                );
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

pub(crate) fn extract_tables<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    options: DocxTextOptions,
) -> Result<Extraction<DocxTables>> {
    let document_path = find_office_document_path(package, "word/document.xml")?;
    let document = extract_part_tables(package, &document_path, "main", options)?;
    let mut tables = public_tables_for_part(document.value, "main", &document_path);
    let mut warnings = document.warnings;

    if !options.include_related_parts {
        return Ok(Extraction::with_warnings(
            DocxTables {
                document_type: "docx".to_owned(),
                tables,
            },
            warnings,
        ));
    }
    let relationships_path = rels_path_for(&document_path);
    let relationships_xml = match package.read_to_string(&relationships_path) {
        Ok(xml) => xml,
        Err(OxdocError::MissingPart(_)) => {
            return Ok(Extraction::with_warnings(
                DocxTables {
                    document_type: "docx".to_owned(),
                    tables,
                },
                warnings,
            ));
        }
        Err(err) => return Err(err),
    };

    let relationships = parse_relationships(&relationships_xml, &relationships_path)?;
    let plan = plan_related_part_order(
        package,
        &document_path,
        &relationships,
        &relationships_path,
        options,
        &mut warnings,
    )?;
    for entry in plan {
        let relationship = &relationships[entry.index];
        let Some(part_type) =
            related_docx_text_part_type(relationship.relationship_type.as_deref(), options)
        else {
            continue;
        };

        let part_path = resolve_relationship_target(
            parent_dir(&document_path),
            relationship,
            &relationships_path,
        )?;
        match extract_part_tables(package, &part_path, part_type, options) {
            Ok(part) => {
                tables.extend(public_tables_for_part(part.value, part_type, &part_path));
                warnings.extend(part.warnings);
            }
            Err(OxdocError::MissingPart(part)) => warnings.push(OutputWarning::new(
                &relationships_path,
                format!("skipped related DOCX table part {part}: missing part"),
            )),
            Err(err) => return Err(err),
        }
    }

    Ok(Extraction::with_warnings(
        DocxTables {
            document_type: "docx".to_owned(),
            tables,
        },
        warnings,
    ))
}

fn extract_part_text<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    path: &str,
    options: DocxTextOptions,
) -> Result<Extraction<String>> {
    package.with_entry(path, |entry| {
        let reader = BufReader::new(entry);
        extract_xml_text(reader, path, options)
    })
}

fn extract_part_tables<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    path: &str,
    _part_type: &str,
    options: DocxTextOptions,
) -> Result<Extraction<Vec<DocxTable>>> {
    package.with_entry(path, |entry| {
        let reader = BufReader::new(entry);
        parse_xml_tables(reader, path, options)
    })
}

fn is_related_docx_text_part(relationship_type: Option<&str>, options: DocxTextOptions) -> bool {
    related_docx_text_part_type(relationship_type, options).is_some()
}

fn related_docx_text_part_type(
    relationship_type: Option<&str>,
    options: DocxTextOptions,
) -> Option<&'static str> {
    let kind = relationship_type?;
    if kind.ends_with("/header") {
        Some("header")
    } else if kind.ends_with("/footer") {
        Some("footer")
    } else if kind.ends_with("/footnotes") {
        Some("footnotes")
    } else if kind.ends_with("/endnotes") {
        Some("endnotes")
    } else if options.include_comments && kind.ends_with("/comments") {
        Some("comments")
    } else {
        None
    }
}

/// Order rank: headers before footers within a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RelatedRefKind {
    Header,
    Footer,
}

/// Canonical variant rank: first, even, default (unknown/missing -> Default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RelatedRefVariant {
    First,
    Even,
    Default,
}

/// One `w:headerReference` / `w:footerReference` element, in sectPr element order.
#[derive(Debug)]
struct SectionReference {
    kind: RelatedRefKind,
    variant: RelatedRefVariant,
    /// None => element had no r:id (warning + skip at plan time).
    rid: Option<String>,
}

/// The iteration order shared by all three extraction paths.
/// Indices into the `parse_relationships` result vector, plus the variant
/// carried by the section reference that positioned each part.
type RelatedPartPlan = Vec<RelatedPartPlanEntry>;

/// One entry of the shared related-part iteration order: which relationship
/// to emit, plus the section-assigned variant when the reference that
/// positioned the part carried one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RelatedPartPlanEntry {
    /// Index into the `parse_relationships` result vector.
    index: usize,
    /// `Some(v)` only for header/footer parts positioned by a `sectPr`
    /// reference; `None` for orphans, footnotes, endnotes, and comments.
    variant: Option<RelatedRefVariant>,
}

impl RelatedRefVariant {
    /// Public label for the structured path; matches the ordering rank names.
    fn label(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Even => "even",
            Self::Default => "default",
        }
    }
}

fn related_ref_kind(name: &str) -> Option<RelatedRefKind> {
    if name_eq(name, "headerReference") {
        Some(RelatedRefKind::Header)
    } else if name_eq(name, "footerReference") {
        Some(RelatedRefKind::Footer)
    } else {
        None
    }
}

fn related_ref_kind_from_type(relationship_type: Option<&str>) -> Option<RelatedRefKind> {
    let kind = relationship_type?;
    if kind.ends_with("/header") {
        Some(RelatedRefKind::Header)
    } else if kind.ends_with("/footer") {
        Some(RelatedRefKind::Footer)
    } else {
        None
    }
}

fn related_ref_variant(type_value: &str) -> RelatedRefVariant {
    match type_value {
        "first" => RelatedRefVariant::First,
        "even" => RelatedRefVariant::Even,
        _ => RelatedRefVariant::Default,
    }
}

fn section_reference(
    kind: RelatedRefKind,
    element: &quick_xml::events::BytesStart<'_>,
) -> SectionReference {
    SectionReference {
        kind,
        variant: attr_value(element, "type").map_or(RelatedRefVariant::Default, |value| {
            related_ref_variant(&value)
        }),
        rid: attr_value(element, "id"),
    }
}

/// Streaming sectPr collector for `word/document.xml`.
///
/// Walks sections in document order: a `w:p/w:pPr/w:sectPr` closes a section at
/// that paragraph position and the body-level `w:sectPr` is the final section.
/// Infallible: on a parse error it stops and returns the sections collected so
/// far (deterministic prefix) so malformed `document.xml` keeps today's
/// W001 + partial-text behavior instead of becoming a hard error.
/// `w:titlePg` and other sectPr children are parsed past and ignored.
fn collect_section_references<R: BufRead>(source: R) -> Vec<Vec<SectionReference>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut sections = Vec::new();
    let mut pending_section = Vec::new();
    let mut in_sectpr = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                let name = element.name().as_ref().to_owned();
                if name_eq(&name, "sectPr") {
                    in_sectpr = true;
                    pending_section.clear();
                } else if in_sectpr && let Some(kind) = related_ref_kind(&name) {
                    pending_section.push(section_reference(kind, &element));
                }
            }
            Ok(Event::Empty(element)) => {
                let name = element.name().as_ref().to_owned();
                if name_eq(&name, "sectPr") {
                    in_sectpr = false;
                    sections.push(std::mem::take(&mut pending_section));
                } else if in_sectpr && let Some(kind) = related_ref_kind(&name) {
                    pending_section.push(section_reference(kind, &element));
                }
            }
            Ok(Event::End(element)) => {
                if in_sectpr && name_eq(element.name().as_ref(), "sectPr") {
                    in_sectpr = false;
                    sections.push(std::mem::take(&mut pending_section));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    sections
}

/// Computes the shared related-part iteration order for all extraction paths.
///
/// Headers and footers are emitted by section reference order (headers before
/// footers, `first`/`even`/`default` variants, dedup by resolved part path at
/// first reference); orphans follow in rels file order; footnotes, endnotes,
/// and comments (when enabled) follow last in rels file order. Missing or
/// dangling `r:id` references are skipped with stable warnings. Existing
/// `SuspiciousRelationshipTarget` errors propagate unchanged.
fn plan_related_part_order<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    document_path: &str,
    relationships: &[Relationship],
    relationships_path: &str,
    options: DocxTextOptions,
    warnings: &mut Vec<OutputWarning>,
) -> Result<RelatedPartPlan> {
    let sections = package.with_entry(document_path, |entry| {
        Ok::<_, OxdocError>(collect_section_references(BufReader::new(entry)))
    })?;

    let mut relationship_by_rid: HashMap<&str, usize> = HashMap::new();
    for (index, relationship) in relationships.iter().enumerate() {
        if let Some(rid) = relationship.id.as_deref() {
            relationship_by_rid.insert(rid, index);
        }
    }

    let mut emitted_paths = HashSet::new();
    let mut referenced = vec![false; relationships.len()];
    let mut plan = Vec::new();

    for section in sections {
        let mut references = section;
        references.sort_by_key(|reference| (reference.kind, reference.variant));
        for reference in references {
            let element_name = match reference.kind {
                RelatedRefKind::Header => "headerReference",
                RelatedRefKind::Footer => "footerReference",
            };
            let Some(rid) = reference.rid.as_deref() else {
                warnings.push(OutputWarning::new(
                    relationships_path,
                    format!("skipped DOCX {element_name}: missing r:id"),
                ));
                continue;
            };
            let Some(&index) = relationship_by_rid.get(rid) else {
                warnings.push(OutputWarning::new(
                    relationships_path,
                    format!("skipped DOCX {element_name} {rid}: unknown relationship id"),
                ));
                continue;
            };
            let relationship = &relationships[index];
            if related_ref_kind_from_type(relationship.relationship_type.as_deref())
                != Some(reference.kind)
            {
                continue;
            }
            let part_path = resolve_relationship_target(
                parent_dir(document_path),
                relationship,
                relationships_path,
            )?;
            if emitted_paths.insert(part_path) {
                plan.push(RelatedPartPlanEntry {
                    index,
                    variant: Some(reference.variant),
                });
                referenced[index] = true;
            }
        }
    }

    // Orphan header/footer relationships, in rels file order.
    for (index, relationship) in relationships.iter().enumerate() {
        if referenced[index]
            || related_ref_kind_from_type(relationship.relationship_type.as_deref()).is_none()
        {
            continue;
        }
        let part_path = resolve_relationship_target(
            parent_dir(document_path),
            relationship,
            relationships_path,
        )?;
        if emitted_paths.insert(part_path) {
            plan.push(RelatedPartPlanEntry {
                index,
                variant: None,
            });
            referenced[index] = true;
        }
    }

    // Footnotes, endnotes, and comments (when enabled), in rels file order.
    for (index, relationship) in relationships.iter().enumerate() {
        if referenced[index] {
            continue;
        }
        let Some(part_type) =
            related_docx_text_part_type(relationship.relationship_type.as_deref(), options)
        else {
            continue;
        };
        if part_type == "header" || part_type == "footer" {
            continue;
        }
        plan.push(RelatedPartPlanEntry {
            index,
            variant: None,
        });
    }

    Ok(plan)
}

fn push_text_block(
    blocks: &mut Vec<TextBlock>,
    part_type: &str,
    part_path: &str,
    variant: Option<RelatedRefVariant>,
    text: String,
) {
    if text.is_empty() {
        return;
    }
    let block = TextBlock::new(part_type, part_path, blocks.len() + 1, text);
    let block = match variant {
        Some(v) => block.with_variant(v.label()),
        None => block,
    };
    blocks.push(block);
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

fn public_tables_for_part(
    tables: Vec<DocxTable>,
    part_type: &str,
    part_path: &str,
) -> Vec<crate::models::DocxTable> {
    tables
        .into_iter()
        .enumerate()
        .map(|(index, table)| crate::models::DocxTable {
            part_type: part_type.to_owned(),
            part_path: part_path.to_owned(),
            table_ordinal: index + 1,
            complete: table.complete,
            grid_column_count: table.grid_column_count,
            rows: public_rows(table.rows),
        })
        .collect()
}

fn public_rows(rows: Vec<DocxTableRow>) -> Vec<PublicDocxTableRow> {
    rows.into_iter()
        .enumerate()
        .map(|(row_index, row)| {
            let mut grid_start = row.grid_before;
            let cells = row
                .cells
                .into_iter()
                .enumerate()
                .map(|(cell_index, cell)| {
                    let current_grid_start = grid_start;
                    grid_start += cell.grid_span;
                    PublicDocxTableCell {
                        cell_ordinal: cell_index + 1,
                        grid_start: current_grid_start,
                        grid_span: cell.grid_span,
                        vertical_merge: public_vertical_merge(cell.v_merge),
                        complete: true,
                        blocks: public_blocks(cell.blocks),
                    }
                })
                .collect();
            PublicDocxTableRow {
                row_ordinal: row_index + 1,
                grid_before: row.grid_before,
                grid_after: row.grid_after,
                complete: true,
                cells,
            }
        })
        .collect()
}

fn public_blocks(blocks: Vec<DocxCellBlock>) -> Vec<PublicDocxTableBlock> {
    blocks
        .into_iter()
        .map(|block| match block {
            DocxCellBlock::Paragraph(text) => PublicDocxTableBlock::Paragraph { text },
            DocxCellBlock::Table(table) => PublicDocxTableBlock::Table {
                complete: table.complete,
                grid_column_count: table.grid_column_count,
                rows: public_rows(table.rows),
            },
        })
        .collect()
}

fn public_vertical_merge(value: DocxVerticalMerge) -> crate::models::DocxVerticalMerge {
    match value {
        DocxVerticalMerge::None => crate::models::DocxVerticalMerge::None,
        DocxVerticalMerge::Restart => crate::models::DocxVerticalMerge::Restart,
        DocxVerticalMerge::Continue => crate::models::DocxVerticalMerge::Continue,
    }
}

fn extract_xml_text<R: BufRead>(
    source: R,
    path: &str,
    options: DocxTextOptions,
) -> Result<Extraction<String>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut warnings = Vec::new();
    let mut in_text_node = false;
    let mut excluded_revision_depth = 0usize;
    let mut run_hidden = Vec::new();
    let mut list_paragraph = false;
    let mut list_marker_emitted = false;
    let mut table_contexts = Vec::new();
    let mut pending_cell_paragraph_separator = false;
    let mut decoded = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if is_excluded_revision(element.name().as_ref(), options.revision_mode) {
                    excluded_revision_depth += 1;
                } else if name_eq(element.name().as_ref(), "r") {
                    run_hidden.push(false);
                } else if name_eq(element.name().as_ref(), "vanish") {
                    if let Some(hidden) = run_hidden.last_mut() {
                        *hidden = true;
                    }
                } else if name_eq(element.name().as_ref(), "numPr") {
                    list_paragraph = true;
                } else if name_eq(element.name().as_ref(), "p") {
                    list_paragraph = false;
                    list_marker_emitted = false;
                } else if name_eq(element.name().as_ref(), "tbl") {
                    if in_table_cell(&table_contexts) {
                        flush_cell_paragraph_separator(
                            &mut text,
                            &mut pending_cell_paragraph_separator,
                        );
                    }
                    table_contexts.push(TableContext::default());
                } else if let Some(table) = table_contexts.last_mut()
                    && name_eq(element.name().as_ref(), "tr")
                {
                    table.start_row();
                    pending_cell_paragraph_separator = false;
                } else if let Some(table) = table_contexts.last_mut()
                    && name_eq(element.name().as_ref(), "tc")
                {
                    table.start_cell(&mut text);
                    pending_cell_paragraph_separator = false;
                } else if excluded_revision_depth == 0
                    && (options.include_hidden_text || !run_hidden.last().copied().unwrap_or(false))
                    && is_docx_text_element(element.name().as_ref(), options.revision_mode)
                {
                    if options.include_list_markers && list_paragraph && !list_marker_emitted {
                        push_text(&mut text, "- ", &mut pending_cell_paragraph_separator);
                        list_marker_emitted = true;
                    }
                    in_text_node = true;
                }
            }
            Ok(Event::Empty(element)) if excluded_revision_depth == 0 => {
                if name_eq(element.name().as_ref(), "vanish") {
                    if let Some(hidden) = run_hidden.last_mut() {
                        *hidden = true;
                    }
                } else if name_eq(element.name().as_ref(), "numPr") {
                    list_paragraph = true;
                } else if name_eq(element.name().as_ref(), "tab")
                    && (options.include_hidden_text || !run_hidden.last().copied().unwrap_or(false))
                {
                    flush_cell_paragraph_separator(
                        &mut text,
                        &mut pending_cell_paragraph_separator,
                    );
                    text.push('\t');
                } else if (name_eq(element.name().as_ref(), "br")
                    || name_eq(element.name().as_ref(), "cr"))
                    && (options.include_hidden_text || !run_hidden.last().copied().unwrap_or(false))
                {
                    pending_cell_paragraph_separator = false;
                    push_newline(&mut text);
                }
            }
            Ok(Event::Text(value))
                if in_text_node
                    && (options.include_hidden_text
                        || !run_hidden.last().copied().unwrap_or(false)) =>
            {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                push_text(&mut text, &decoded, &mut pending_cell_paragraph_separator);
            }
            Ok(Event::CData(value))
                if in_text_node
                    && (options.include_hidden_text
                        || !run_hidden.last().copied().unwrap_or(false)) =>
            {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                push_text(&mut text, &decoded, &mut pending_cell_paragraph_separator);
            }
            Ok(Event::GeneralRef(value))
                if in_text_node
                    && (options.include_hidden_text
                        || !run_hidden.last().copied().unwrap_or(false)) =>
            {
                decoded.clear();
                append_decoded_xml_reference(value.as_ref(), &mut decoded);
                push_text(&mut text, &decoded, &mut pending_cell_paragraph_separator);
            }
            Ok(Event::End(element)) => {
                if is_docx_text_element(element.name().as_ref(), options.revision_mode) {
                    in_text_node = false;
                } else if is_excluded_revision(element.name().as_ref(), options.revision_mode) {
                    excluded_revision_depth = excluded_revision_depth.saturating_sub(1);
                    in_text_node = false;
                } else if name_eq(element.name().as_ref(), "r") {
                    run_hidden.pop();
                } else if name_eq(element.name().as_ref(), "p") {
                    if excluded_revision_depth == 0 {
                        if in_table_cell(&table_contexts) {
                            pending_cell_paragraph_separator = !text.is_empty();
                        } else {
                            push_newline(&mut text);
                        }
                    }
                } else if name_eq(element.name().as_ref(), "tc") {
                    if let Some(table) = table_contexts.last_mut() {
                        table.end_cell();
                    }
                    pending_cell_paragraph_separator = false;
                } else if name_eq(element.name().as_ref(), "tr") {
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
                } else if name_eq(element.name().as_ref(), "tbl") {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocxTable {
    complete: bool,
    grid_column_count: Option<usize>,
    rows: Vec<DocxTableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocxTableRow {
    grid_before: usize,
    grid_after: usize,
    deleted: bool,
    cells: Vec<DocxTableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocxTableCell {
    grid_span: usize,
    v_merge: DocxVerticalMerge,
    blocks: Vec<DocxCellBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocxVerticalMerge {
    None,
    Restart,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DocxCellBlock {
    Paragraph(String),
    Table(DocxTable),
}

fn parse_xml_tables<R: BufRead>(
    source: R,
    path: &str,
    options: DocxTextOptions,
) -> Result<Extraction<Vec<DocxTable>>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut tables = Vec::new();
    let mut table_stack = Vec::<DocxTableBuilder>::new();
    let mut paragraph = None::<String>;
    let mut in_text_node = false;
    let mut excluded_revision_depth = 0usize;
    let mut warnings = Vec::new();
    let mut decoded = String::new();
    let mut malformed = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if is_excluded_revision(element.name().as_ref(), options.revision_mode) {
                    mark_current_row_deleted(&mut table_stack);
                    excluded_revision_depth += 1;
                } else if name_eq(element.name().as_ref(), "tbl") {
                    table_stack.push(DocxTableBuilder::default());
                } else if name_eq(element.name().as_ref(), "gridCol") {
                    count_grid_column(&mut table_stack);
                } else if name_eq(element.name().as_ref(), "tr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.start_row();
                    }
                } else if name_eq(element.name().as_ref(), "trPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.row_properties_depth += 1;
                    }
                } else if name_eq(element.name().as_ref(), "tc") {
                    if let Some(table) = table_stack.last_mut() {
                        table.start_cell();
                    }
                } else if name_eq(element.name().as_ref(), "tcPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.cell_properties_depth += 1;
                    }
                } else if name_eq(element.name().as_ref(), "gridBefore")
                    || name_eq(element.name().as_ref(), "gridAfter")
                {
                    apply_row_grid_offset(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), "gridSpan") {
                    apply_cell_grid_span(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), "vMerge") {
                    apply_cell_vertical_merge(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), "p")
                    && current_table_has_cell(&table_stack)
                {
                    paragraph = Some(String::new());
                } else if excluded_revision_depth == 0
                    && paragraph.is_some()
                    && name_eq(element.name().as_ref(), "t")
                {
                    in_text_node = true;
                }
            }
            Ok(Event::Empty(element)) if excluded_revision_depth == 0 => {
                if is_excluded_revision(element.name().as_ref(), options.revision_mode) {
                    mark_current_row_deleted(&mut table_stack);
                } else if name_eq(element.name().as_ref(), "gridCol") {
                    count_grid_column(&mut table_stack);
                } else if name_eq(element.name().as_ref(), "gridBefore")
                    || name_eq(element.name().as_ref(), "gridAfter")
                {
                    apply_row_grid_offset(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), "gridSpan") {
                    apply_cell_grid_span(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), "vMerge") {
                    apply_cell_vertical_merge(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), "p") {
                    if let Some(cell) = current_cell_mut(&mut table_stack) {
                        cell.blocks.push(DocxCellBlock::Paragraph(String::new()));
                    }
                } else if let Some(paragraph) = paragraph.as_mut() {
                    if name_eq(element.name().as_ref(), "tab") {
                        paragraph.push('\t');
                    } else if name_eq(element.name().as_ref(), "br")
                        || name_eq(element.name().as_ref(), "cr")
                    {
                        paragraph.push('\n');
                    }
                }
            }
            Ok(Event::Text(value)) if in_text_node && excluded_revision_depth == 0 => {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                if let Some(paragraph) = paragraph.as_mut() {
                    paragraph.push_str(&decoded);
                }
            }
            Ok(Event::CData(value)) if in_text_node && excluded_revision_depth == 0 => {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                if let Some(paragraph) = paragraph.as_mut() {
                    paragraph.push_str(&decoded);
                }
            }
            Ok(Event::GeneralRef(value)) if in_text_node && excluded_revision_depth == 0 => {
                decoded.clear();
                append_decoded_xml_reference(value.as_ref(), &mut decoded);
                if let Some(paragraph) = paragraph.as_mut() {
                    paragraph.push_str(&decoded);
                }
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), "t") {
                    in_text_node = false;
                } else if is_excluded_revision(element.name().as_ref(), options.revision_mode) {
                    excluded_revision_depth = excluded_revision_depth.saturating_sub(1);
                    in_text_node = false;
                } else if name_eq(element.name().as_ref(), "p") {
                    if let Some(paragraph) = paragraph.take()
                        && let Some(cell) = current_cell_mut(&mut table_stack)
                    {
                        cell.blocks.push(DocxCellBlock::Paragraph(paragraph));
                    }
                } else if name_eq(element.name().as_ref(), "tcPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.cell_properties_depth = table.cell_properties_depth.saturating_sub(1);
                    }
                } else if name_eq(element.name().as_ref(), "tc") {
                    if let Some(table) = table_stack.last_mut() {
                        table.finish_cell();
                    }
                } else if name_eq(element.name().as_ref(), "trPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.row_properties_depth = table.row_properties_depth.saturating_sub(1);
                    }
                } else if name_eq(element.name().as_ref(), "tr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.finish_row();
                    }
                } else if name_eq(element.name().as_ref(), "tbl")
                    && let Some(table) = table_stack.pop()
                {
                    let table = table.finish(true);
                    if let Some(cell) = current_cell_mut(&mut table_stack) {
                        cell.blocks.push(DocxCellBlock::Table(table));
                    } else {
                        tables.push(table);
                    }
                }
            }
            Ok(Event::Eof) => {
                if !table_stack.is_empty() {
                    warnings.push(OutputWarning::malformed_xml(
                        path,
                        "unexpected EOF with open table",
                    ));
                    malformed = true;
                }
                break;
            }
            Err(source) => {
                warnings.push(OutputWarning::malformed_xml(path, source));
                malformed = true;
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    if malformed && let Some(table) = table_stack.into_iter().next() {
        let table = table.finish(false);
        if !table.rows.is_empty() {
            tables.push(table);
        }
    }

    Ok(Extraction::with_warnings(tables, warnings))
}

#[derive(Debug, Default)]
struct DocxTableBuilder {
    rows: Vec<DocxTableRow>,
    row: Option<DocxTableRow>,
    cell: Option<DocxTableCell>,
    grid_column_count: Option<usize>,
    row_properties_depth: usize,
    cell_properties_depth: usize,
}

impl DocxTableBuilder {
    fn start_row(&mut self) {
        self.row = Some(DocxTableRow {
            grid_before: 0,
            grid_after: 0,
            deleted: false,
            cells: Vec::new(),
        });
    }

    fn start_cell(&mut self) {
        if self.row.is_some() {
            self.cell = Some(DocxTableCell {
                grid_span: 1,
                v_merge: DocxVerticalMerge::None,
                blocks: Vec::new(),
            });
        }
    }

    fn finish_cell(&mut self) {
        if let Some(cell) = self.cell.take()
            && let Some(row) = self.row.as_mut()
        {
            row.cells.push(cell);
        }
        self.cell_properties_depth = 0;
    }

    fn finish_row(&mut self) {
        self.finish_cell();
        if let Some(row) = self.row.take()
            && !row.deleted
        {
            self.rows.push(row);
        }
        self.row_properties_depth = 0;
    }

    fn finish(self, complete: bool) -> DocxTable {
        DocxTable {
            complete,
            grid_column_count: self.grid_column_count,
            rows: self.rows,
        }
    }
}

fn current_table_has_cell(tables: &[DocxTableBuilder]) -> bool {
    tables.last().is_some_and(|table| table.cell.is_some())
}

fn current_cell_mut(tables: &mut [DocxTableBuilder]) -> Option<&mut DocxTableCell> {
    tables.last_mut()?.cell.as_mut()
}

fn count_grid_column(tables: &mut [DocxTableBuilder]) {
    let Some(table) = tables.last_mut() else {
        return;
    };
    if table.row.is_some() || table.cell.is_some() {
        return;
    }
    *table.grid_column_count.get_or_insert(0) += 1;
}

fn mark_current_row_deleted(tables: &mut [DocxTableBuilder]) {
    let Some(table) = tables.last_mut() else {
        return;
    };
    if table.row_properties_depth == 0 {
        return;
    }
    if let Some(row) = table.row.as_mut() {
        row.deleted = true;
    }
}

fn is_excluded_revision(name: &str, mode: DocxRevisionMode) -> bool {
    match mode {
        DocxRevisionMode::Final => name_eq(name, "del") || name_eq(name, "moveFrom"),
        DocxRevisionMode::Original => name_eq(name, "ins") || name_eq(name, "moveTo"),
        DocxRevisionMode::All => false,
    }
}

fn is_docx_text_element(name: &str, mode: DocxRevisionMode) -> bool {
    name_eq(name, "t") || (!matches!(mode, DocxRevisionMode::Final) && name_eq(name, "delText"))
}

fn apply_row_grid_offset(
    tables: &mut [DocxTableBuilder],
    element: &quick_xml::events::BytesStart<'_>,
    path: &str,
    warnings: &mut Vec<OutputWarning>,
) {
    let Some(table) = tables.last_mut() else {
        return;
    };
    if table.row_properties_depth == 0 {
        return;
    }
    let raw_value = attr_value(element, "val");
    let value = raw_value
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok());
    let Some(row) = table.row.as_mut() else {
        return;
    };
    let property = if name_eq(element.name().as_ref(), "gridBefore") {
        "gridBefore"
    } else {
        "gridAfter"
    };
    let value = match value {
        Some(value) => value,
        None => {
            warnings.push(OutputWarning::new(
                path,
                format!(
                    "row {} has invalid {property} value {}; using 0",
                    table.rows.len() + 1,
                    raw_value.as_deref().unwrap_or("<missing>")
                ),
            ));
            0
        }
    };
    if name_eq(element.name().as_ref(), "gridBefore") {
        row.grid_before = value;
    } else {
        row.grid_after = value;
    }
}

fn apply_cell_grid_span(
    tables: &mut [DocxTableBuilder],
    element: &quick_xml::events::BytesStart<'_>,
    path: &str,
    warnings: &mut Vec<OutputWarning>,
) {
    let Some(table) = tables.last_mut() else {
        return;
    };
    if table.cell_properties_depth == 0 {
        return;
    }
    let raw_value = attr_value(element, "val");
    let span = raw_value
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|span| *span > 0);
    let row_ordinal = table.rows.len() + 1;
    let cell_ordinal = table.row.as_ref().map_or(1, |row| row.cells.len() + 1);
    if let Some(cell) = table.cell.as_mut() {
        if let Some(span) = span {
            cell.grid_span = span;
        } else {
            warnings.push(OutputWarning::new(
                path,
                format!(
                    "row {row_ordinal} cell {cell_ordinal} has invalid gridSpan value {}; using 1",
                    raw_value.as_deref().unwrap_or("<missing>")
                ),
            ));
            cell.grid_span = 1;
        }
    }
}

fn apply_cell_vertical_merge(
    tables: &mut [DocxTableBuilder],
    element: &quick_xml::events::BytesStart<'_>,
    path: &str,
    warnings: &mut Vec<OutputWarning>,
) {
    let Some(table) = tables.last_mut() else {
        return;
    };
    if table.cell_properties_depth == 0 {
        return;
    }
    let raw_value = attr_value(element, "val");
    let row_ordinal = table.rows.len() + 1;
    let cell_ordinal = table.row.as_ref().map_or(1, |row| row.cells.len() + 1);
    if let Some(cell) = table.cell.as_mut() {
        cell.v_merge = match raw_value.as_deref() {
            Some(value) if value.eq_ignore_ascii_case("restart") => DocxVerticalMerge::Restart,
            Some(value) if value.eq_ignore_ascii_case("continue") => DocxVerticalMerge::Continue,
            None => DocxVerticalMerge::Continue,
            Some(value) => {
                warnings.push(OutputWarning::new(
                    path,
                    format!(
                        "row {row_ordinal} cell {cell_ordinal} has unknown vMerge value {value}; using none"
                    ),
                ));
                DocxVerticalMerge::None
            }
        };
    }
}

#[doc(hidden)]
pub fn fuzz_extract_text(xml: &[u8]) -> Result<()> {
    let _ = extract_xml_text(
        Cursor::new(xml),
        "word/document.xml",
        DocxTextOptions::default(),
    )?;
    let _ = parse_xml_tables(
        Cursor::new(xml),
        "word/document.xml",
        DocxTextOptions::default(),
    )?;
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
    use std::fs;
    use std::io::Cursor;
    use std::path::PathBuf;

    use super::{
        DocxCellBlock, DocxVerticalMerge, RelatedPartPlanEntry, RelatedRefKind, RelatedRefVariant,
        SectionReference, collect_section_references, extract_structured_text, extract_tables,
        extract_text, extract_xml_text, parse_xml_tables, plan_related_part_order,
    };
    use crate::OxdocError;
    use crate::models::{DocxRevisionMode, DocxTextOptions};
    use crate::parsers::Relationship;
    use crate::vfs::OoxmlPackage;

    fn fixture_path(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/docx")
            .join(path)
    }

    fn read_fixture(path: &str) -> String {
        fs::read_to_string(fixture_path(path)).unwrap()
    }

    fn only_paragraph_text(blocks: &[DocxCellBlock]) -> Vec<&str> {
        blocks
            .iter()
            .filter_map(|block| match block {
                DocxCellBlock::Paragraph(text) => Some(text.as_str()),
                DocxCellBlock::Table(_) => None,
            })
            .collect()
    }

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

        let result = extract_xml_text(
            Cursor::new(xml.as_bytes()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(result.value, "Hola\tMundo\nSegundo & final\n");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn applies_visible_content_policies() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
          <w:p><w:pPr><w:numPr/></w:pPr><w:r><w:t>listed</w:t></w:r></w:p>
          <w:p><w:r><w:rPr><w:vanish/></w:rPr><w:t>hidden</w:t></w:r><w:del><w:r><w:delText>old</w:delText></w:r></w:del><w:ins><w:r><w:t>new</w:t></w:r></w:ins></w:p>
        </w:body></w:document>"#;
        let default = extract_xml_text(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();
        assert_eq!(default.value, "listed\nhiddennew\n");

        let options = DocxTextOptions {
            include_hidden_text: false,
            include_comments: true,
            revision_mode: DocxRevisionMode::Original,
            include_related_parts: true,
            include_list_markers: true,
        };
        let result = extract_xml_text(Cursor::new(xml), "word/document.xml", options).unwrap();
        assert_eq!(result.value, "- listed\nold\n");
    }

    #[test]
    fn returns_partial_text_after_malformed_xml() {
        let xml = br#"<w:document><w:p><w:r><w:t>Hola</w:t></w:r></w:p><"#;

        let result = extract_xml_text(
            Cursor::new(xml.as_slice()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

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

        let result = extract_xml_text(
            Cursor::new(xml.as_bytes()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();
        let empty = extract_xml_text(
            Cursor::new(b"<w:document/>"),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

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

        let result = extract_xml_text(
            Cursor::new(xml.as_bytes()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

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

        let result = extract_xml_text(
            Cursor::new(xml.as_bytes()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

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

        let result = extract_xml_text(
            Cursor::new(xml.as_bytes()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

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

        let result = extract_xml_text(
            Cursor::new(xml.as_bytes()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

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

        let result = extract_xml_text(
            Cursor::new(xml.as_bytes()),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(result.value, "List item\n2026-04-14\nHidden text\n");
    }

    #[test]
    fn parses_table_rows_cells_paragraphs_and_raw_merge_properties() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:tbl>
                  <w:tr>
                    <w:trPr>
                      <w:gridBefore w:val="2"/>
                      <w:gridAfter w:val="1"/>
                    </w:trPr>
                    <w:tc>
                      <w:tcPr>
                        <w:gridSpan w:val="3"/>
                        <w:vMerge w:val="restart"/>
                      </w:tcPr>
                      <w:p><w:r><w:t>First paragraph</w:t></w:r></w:p>
                      <w:p/>
                      <w:p><w:r><w:t>Second</w:t><w:tab/><w:t>paragraph</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc>
                      <w:tcPr><w:vMerge/></w:tcPr>
                      <w:p><w:r><w:t>Continue</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc>
                      <w:p><w:r><w:t>Default properties</w:t></w:r></w:p>
                    </w:tc>
                  </w:tr>
                </w:tbl>
              </w:body>
            </w:document>
        "#;

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert!(result.warnings.is_empty());
        assert_eq!(result.value.len(), 1);
        let row = &result.value[0].rows[0];
        assert_eq!((row.grid_before, row.grid_after), (2, 1));
        assert_eq!(row.cells.len(), 3);
        assert_eq!(row.cells[0].grid_span, 3);
        assert_eq!(row.cells[0].v_merge, DocxVerticalMerge::Restart);
        assert_eq!(
            row.cells[0].blocks,
            vec![
                DocxCellBlock::Paragraph("First paragraph".to_owned()),
                DocxCellBlock::Paragraph(String::new()),
                DocxCellBlock::Paragraph("Second\tparagraph".to_owned()),
            ]
        );
        assert_eq!(row.cells[1].grid_span, 1);
        assert_eq!(row.cells[1].v_merge, DocxVerticalMerge::Continue);
        assert_eq!(row.cells[2].grid_span, 1);
        assert_eq!(row.cells[2].v_merge, DocxVerticalMerge::None);
    }

    #[test]
    fn retains_nested_tables_in_cell_block_order() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:tbl>
                  <w:tr>
                    <w:tc>
                      <w:p><w:r><w:t>Before</w:t></w:r></w:p>
                      <w:tbl>
                        <w:tr>
                          <w:tc><w:p><w:r><w:t>Nested</w:t></w:r></w:p></w:tc>
                        </w:tr>
                      </w:tbl>
                      <w:p><w:r><w:t>After</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc><w:p><w:r><w:t>Sibling</w:t></w:r></w:p></w:tc>
                  </w:tr>
                </w:tbl>
              </w:body>
            </w:document>
        "#;

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();
        let outer = &result.value[0];
        let first_cell = &outer.rows[0].cells[0];

        assert_eq!(result.value.len(), 1);
        assert_eq!(outer.rows[0].cells.len(), 2);
        assert_eq!(first_cell.blocks.len(), 3);
        assert_eq!(
            first_cell.blocks[0],
            DocxCellBlock::Paragraph("Before".to_owned())
        );
        let DocxCellBlock::Table(nested) = &first_cell.blocks[1] else {
            panic!("expected nested table block");
        };
        assert_eq!(nested.rows.len(), 1);
        assert_eq!(nested.rows[0].cells.len(), 1);
        assert_eq!(
            nested.rows[0].cells[0].blocks,
            vec![DocxCellBlock::Paragraph("Nested".to_owned())]
        );
        assert_eq!(
            first_cell.blocks[2],
            DocxCellBlock::Paragraph("After".to_owned())
        );
    }

    #[test]
    fn omits_deleted_text_from_table_paragraphs() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:tbl>
                  <w:tr>
                    <w:tc>
                      <w:p>
                        <w:r><w:t>Kept </w:t></w:r>
                        <w:del><w:r><w:t>deleted</w:t></w:r></w:del>
                        <w:ins><w:r><w:t>inserted</w:t></w:r></w:ins>
                      </w:p>
                    </w:tc>
                  </w:tr>
                </w:tbl>
              </w:body>
            </w:document>
        "#;

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(
            result.value[0].rows[0].cells[0].blocks,
            vec![DocxCellBlock::Paragraph("Kept inserted".to_owned())]
        );
    }

    #[test]
    fn parses_tables_from_related_part_xml() {
        let xml = r#"
            <w:hdr xmlns:w="w">
              <w:tbl>
                <w:tr>
                  <w:tc><w:p><w:r><w:t>Header cell</w:t></w:r></w:p></w:tc>
                </w:tr>
              </w:tbl>
            </w:hdr>
        "#;

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/header1.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert!(result.warnings.is_empty());
        assert_eq!(
            result.value[0].rows[0].cells[0].blocks,
            vec![DocxCellBlock::Paragraph("Header cell".to_owned())]
        );
    }

    #[test]
    fn fixture_table_semantics_match_structural_parser_contract() {
        let xml = read_fixture("table-semantics/document.xml");

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(result.warnings.len(), 2);
        assert!(
            result.warnings[0]
                .message
                .contains("invalid gridSpan value 0")
        );
        assert!(
            result.warnings[1]
                .message
                .contains("unknown vMerge value unexpected")
        );
        assert_eq!(result.value.len(), 1);
        let table = &result.value[0];
        assert_eq!(table.rows.len(), 3);
        assert_eq!(
            (table.rows[0].grid_before, table.rows[0].grid_after),
            (1, 0)
        );
        assert_eq!(
            (table.rows[2].grid_before, table.rows[2].grid_after),
            (0, 2)
        );
        assert_eq!(table.rows[0].cells[0].grid_span, 2);
        assert_eq!(table.rows[0].cells[1].v_merge, DocxVerticalMerge::Restart);
        assert_eq!(table.rows[1].cells[1].v_merge, DocxVerticalMerge::Continue);
        assert_eq!(table.rows[2].cells[1].grid_span, 1);
        assert_eq!(table.rows[2].cells[1].v_merge, DocxVerticalMerge::None);
        assert_eq!(
            only_paragraph_text(&table.rows[0].cells[0].blocks),
            vec!["Alpha", "", "Beta"]
        );
        assert_eq!(
            only_paragraph_text(&table.rows[2].cells[0].blocks),
            vec!["Visible before inserted moved here"]
        );

        let DocxCellBlock::Table(nested) = &table.rows[1].cells[0].blocks[1] else {
            panic!("expected nested table block");
        };
        assert_eq!(
            only_paragraph_text(&nested.rows[0].cells[0].blocks),
            vec!["Nested cell"]
        );
    }

    #[test]
    fn fixture_malformed_table_returns_closed_prefix_and_warning() {
        let xml = read_fixture("malformed-table/document.xml");

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(result.value.len(), 1);
        assert_eq!(result.value[0].rows.len(), 1);
        assert_eq!(result.value[0].rows[0].cells.len(), 2);
        assert_eq!(
            only_paragraph_text(&result.value[0].rows[0].cells[0].blocks),
            vec!["Complete left"]
        );
        assert_eq!(
            only_paragraph_text(&result.value[0].rows[0].cells[1].blocks),
            vec!["Complete right"]
        );
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn fixture_related_part_tables_parse_independently() {
        let parts = [
            ("related-parts/package/word/document.xml", "Main table"),
            ("related-parts/package/word/comments.xml", "Comment table"),
            ("related-parts/package/word/header1.xml", "Header table"),
            ("related-parts/package/word/footnotes.xml", "Footnote table"),
            ("related-parts/package/word/footer1.xml", "Footer table"),
            ("related-parts/package/word/endnotes.xml", "Endnote table"),
        ];

        for (path, expected) in parts {
            let xml = read_fixture(path);
            let result =
                parse_xml_tables(Cursor::new(xml), path, DocxTextOptions::default()).unwrap();

            assert_eq!(result.value.len(), 1, "{path}");
            assert_eq!(
                only_paragraph_text(&result.value[0].rows[0].cells[0].blocks),
                vec![expected],
                "{path}"
            );
            assert!(result.warnings.is_empty(), "{path}");
        }
    }

    #[test]
    fn returns_completed_tables_and_warning_after_malformed_xml() {
        let xml = br#"
            <w:document xmlns:w="w">
              <w:body>
                <w:tbl>
                  <w:tr>
                    <w:tc><w:p><w:r><w:t>Complete</w:t></w:r></w:p></w:tc>
                  </w:tr>
                </w:tbl>
                <w:tbl>
                  <w:tr>
                    <w:tc><w:p><w:r><w:t>Completed row</w:t></w:r></w:p></w:tc>
                  </w:tr>
                  <
        "#;

        let result = parse_xml_tables(
            Cursor::new(xml.as_slice()),
            "word/header1.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(result.value.len(), 2);
        assert_eq!(
            result.value[0].rows[0].cells[0].blocks,
            vec![DocxCellBlock::Paragraph("Complete".to_owned())]
        );
        assert_eq!(
            result.value[1].rows[0].cells[0].blocks,
            vec![DocxCellBlock::Paragraph("Completed row".to_owned())]
        );
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].path, "word/header1.xml");
        assert!(
            result.warnings[0]
                .message
                .starts_with("stopped after malformed XML:")
        );
    }

    #[test]
    fn parses_grid_and_merge_attributes_with_warnings() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
          <w:tbl>
            <w:tblGrid><w:gridCol></w:gridCol><w:gridCol></w:gridCol></w:tblGrid>
            <w:trPr><w:gridBefore w:val="1"></w:gridBefore></w:trPr>
            <w:tcPr><w:gridSpan w:val="2"></w:gridSpan><w:vMerge w:val="restart"></w:vMerge></w:tcPr>
            <w:tr>
              <w:gridCol></w:gridCol>
              <w:trPr>
                <w:gridBefore w:val="2"></w:gridBefore>
                <w:gridAfter w:val="oops"></w:gridAfter>
              </w:trPr>
              <w:tc>
                <w:tcPr>
                  <w:gridSpan w:val="0"></w:gridSpan>
                  <w:gridSpan w:val="3"></w:gridSpan>
                  <w:vMerge w:val="weird"></w:vMerge>
                </w:tcPr>
                <w:p><w:r><w:t>Cell</w:t><w:br/><w:cr/></w:r></w:p>
              </w:tc>
              <w:tc><w:p><w:t><![CDATA[raw]]></w:t></w:p></w:tc>
              <w:tc><w:p><w:t>Q&#68;R</w:t></w:p></w:tc>
            </w:tr>
          </w:tbl>
        </w:body></w:document>"#;

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        let table = &result.value[0];
        assert_eq!(table.grid_column_count, Some(2));
        assert_eq!(table.rows.len(), 1);
        let row = &table.rows[0];
        assert_eq!((row.grid_before, row.grid_after), (2, 0));
        assert_eq!(row.cells.len(), 3);
        assert_eq!(row.cells[0].grid_span, 3);
        assert_eq!(row.cells[0].v_merge, DocxVerticalMerge::None);
        assert_eq!(
            row.cells[0].blocks,
            vec![DocxCellBlock::Paragraph("Cell\n\n".to_owned())]
        );
        assert_eq!(
            row.cells[1].blocks,
            vec![DocxCellBlock::Paragraph("raw".to_owned())]
        );
        assert_eq!(
            row.cells[2].blocks,
            vec![DocxCellBlock::Paragraph("QDR".to_owned())]
        );
        let messages: Vec<&str> = result
            .warnings
            .iter()
            .map(|warning| warning.message.as_str())
            .collect();
        assert_eq!(messages.len(), 3, "unexpected warnings: {messages:?}");
        assert!(messages[0].contains("invalid gridAfter value oops; using 0"));
        assert!(messages[1].contains("invalid gridSpan value 0; using 1"));
        assert!(messages[2].contains("unknown vMerge value weird; using none"));
    }

    #[test]
    fn ignores_table_properties_outside_valid_contexts() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
          <w:gridCol></w:gridCol>
          <w:del></w:del>
          <w:gridBefore w:val="2"></w:gridBefore>
          <w:gridAfter w:val="1"></w:gridAfter>
          <w:gridSpan w:val="2"></w:gridSpan>
          <w:vMerge></w:vMerge>
          <w:tbl>
            <w:gridBefore w:val="2"></w:gridBefore>
            <w:gridAfter w:val="1"></w:gridAfter>
            <w:gridSpan w:val="2"></w:gridSpan>
            <w:vMerge w:val="restart"></w:vMerge>
          </w:tbl>
        </w:body></w:document>"#;

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(result.value.len(), 1);
        assert!(result.value[0].rows.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn keeps_all_revision_content_in_all_mode() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
          <w:p><w:ins><w:r><w:t>I</w:t></w:r></w:ins><w:del><w:r><w:delText>D</w:delText></w:r></w:del></w:p>
        </w:body></w:document>"#;

        let all = extract_xml_text(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions {
                revision_mode: DocxRevisionMode::All,
                ..DocxTextOptions::default()
            },
        )
        .unwrap();
        assert_eq!(all.value, "ID\n");

        let final_mode = extract_xml_text(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();
        assert_eq!(final_mode.value, "I\n");
    }

    #[test]
    fn tolerates_cell_and_paragraph_ends_outside_expected_contexts() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
          <w:tc></w:tc>
          <w:ins><w:p><w:r><w:t>Z</w:t></w:r></w:p></w:ins>
          <w:tbl><w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc></w:tbl>
          <w:p><w:r><w:t>B<![CDATA[]]></w:t></w:r></w:p>
        </w:body></w:document>"#;

        let original = extract_xml_text(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions {
                revision_mode: DocxRevisionMode::Original,
                ..DocxTextOptions::default()
            },
        )
        .unwrap();
        assert_eq!(original.value, "A\nB\n");

        let with_rows = extract_xml_text(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();
        assert_eq!(with_rows.value, "Z\nA\nB\n");
    }

    #[test]
    fn paragraph_texts_filter_skips_nested_tables() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
          <w:tbl>
            <w:tr>
              <w:tc>
                <w:p><w:r><w:t>Before</w:t></w:r></w:p>
                <w:tbl>
                  <w:tr><w:tc><w:p><w:r><w:t>Nested</w:t></w:r></w:p></w:tc></w:tr>
                </w:tbl>
                <w:p><w:r><w:t>After</w:t></w:r></w:p>
              </w:tc>
            </w:tr>
          </w:tbl>
        </w:body></w:document>"#;

        let result = parse_xml_tables(
            Cursor::new(xml),
            "word/document.xml",
            DocxTextOptions::default(),
        )
        .unwrap();

        assert_eq!(
            only_paragraph_text(&result.value[0].rows[0].cells[0].blocks),
            vec!["Before", "After"]
        );
    }

    #[test]
    fn fuzz_helper_accepts_wellformed_xml() {
        let xml = b"<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>x</w:t></w:r></w:p></w:body></w:document>";

        super::fuzz_extract_text(xml).unwrap();
    }

    fn ooxml_package(entries: &[(&str, &str)]) -> OoxmlPackage<Cursor<Vec<u8>>> {
        use std::io::Write;
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in entries {
            zip.start_file((*name).to_owned(), SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        OoxmlPackage::new(zip.finish().unwrap()).unwrap()
    }

    fn relationship(id: &str, kind: &str, target: &str) -> Relationship {
        Relationship {
            id: Some(id.to_owned()),
            target: target.to_owned(),
            relationship_type: Some(kind.to_owned()),
            target_mode: None,
        }
    }

    const HEADER_TYPE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";
    const FOOTER_TYPE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer";

    fn collect_rids(sections: &[Vec<SectionReference>]) -> Vec<Vec<Option<&str>>> {
        sections
            .iter()
            .map(|section| {
                section
                    .iter()
                    .map(|reference| reference.rid.as_deref())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn collects_sections_in_document_order_with_variant_ranking() {
        let xml = r#"<w:document xmlns:w="w" xmlns:r="r"><w:body>
            <w:p>
              <w:pPr>
                <w:sectPr>
                  <w:headerReference w:type="default" r:id="rDefault"/>
                  <w:headerReference w:type="first" r:id="rFirst"/>
                  <w:footerReference r:id="rFooter"/>
                  <w:headerReference w:type="even" r:id="rEven"/>
                </w:sectPr>
              </w:pPr>
              <w:r><w:t>Body</w:t></w:r>
            </w:p>
            <w:sectPr>
              <w:footerReference w:type="default" r:id="rFooter2"/>
              <w:headerReference r:id="rHeader2"/>
            </w:sectPr>
          </w:body></w:document>"#;

        let sections = collect_section_references(Cursor::new(xml));

        assert_eq!(
            collect_rids(&sections),
            vec![
                vec![
                    Some("rDefault"),
                    Some("rFirst"),
                    Some("rFooter"),
                    Some("rEven")
                ],
                vec![Some("rFooter2"), Some("rHeader2")],
            ]
        );
        assert_eq!(sections[0][0].kind, RelatedRefKind::Header);
        assert_eq!(sections[0][0].variant, RelatedRefVariant::Default);
        assert_eq!(sections[0][1].variant, RelatedRefVariant::First);
        assert_eq!(sections[0][2].kind, RelatedRefKind::Footer);
        assert_eq!(sections[0][3].variant, RelatedRefVariant::Even);
        assert_eq!(sections[1][0].kind, RelatedRefKind::Footer);
        assert_eq!(sections[1][1].kind, RelatedRefKind::Header);

        let rels = [
            relationship("rFooter2", FOOTER_TYPE, "footer2.xml"),
            relationship("rDefault", HEADER_TYPE, "header-default.xml"),
            relationship("rFirst", HEADER_TYPE, "header-first.xml"),
            relationship("rEven", HEADER_TYPE, "header-even.xml"),
            relationship("rFooter", FOOTER_TYPE, "footer1.xml"),
            relationship("rHeader2", HEADER_TYPE, "header2.xml"),
        ];
        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let plan = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions::default(),
            &mut warnings,
        )
        .unwrap();

        // headers before footers, first/even/default within each kind,
        // section order preserved.
        assert_eq!(
            plan.iter()
                .map(|entry| (entry.index, entry.variant))
                .collect::<Vec<_>>(),
            vec![
                (2, Some(RelatedRefVariant::First)),
                (3, Some(RelatedRefVariant::Even)),
                (1, Some(RelatedRefVariant::Default)),
                (4, Some(RelatedRefVariant::Default)),
                (5, Some(RelatedRefVariant::Default)),
                (0, Some(RelatedRefVariant::Default)),
            ]
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn treats_missing_and_unrecognized_type_as_default() {
        let xml = r#"<w:document xmlns:w="w" xmlns:r="r"><w:body>
            <w:sectPr>
              <w:headerReference w:type="title" r:id="rTitle"/>
              <w:headerReference r:id="rPlain"/>
              <w:footerReference w:type="even" r:id="rEven"/>
            </w:sectPr>
          </w:body></w:document>"#;

        let sections = collect_section_references(Cursor::new(xml));

        assert_eq!(sections.len(), 1);
        assert_eq!(
            sections[0]
                .iter()
                .map(|reference| (reference.kind, reference.variant))
                .collect::<Vec<_>>(),
            vec![
                (RelatedRefKind::Header, RelatedRefVariant::Default),
                (RelatedRefKind::Header, RelatedRefVariant::Default),
                (RelatedRefKind::Footer, RelatedRefVariant::Even),
            ]
        );
    }

    #[test]
    fn keeps_reference_without_rid_as_none() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
            <w:sectPr><w:footerReference/></w:sectPr>
          </w:body></w:document>"#;

        let sections = collect_section_references(Cursor::new(xml));

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].len(), 1);
        assert_eq!(sections[0][0].kind, RelatedRefKind::Footer);
        assert_eq!(sections[0][0].variant, RelatedRefVariant::Default);
        assert_eq!(sections[0][0].rid, None);
    }

    #[test]
    fn stops_collection_gracefully_on_malformed_xml() {
        let xml = br#"<w:document><w:body><w:p><w:pPr><w:sectPr><w:headerReference r:id="rH"/></w:sectPr></w:pPr></w:p><w:p><w:r><w:t>"#;

        let sections = collect_section_references(Cursor::new(xml.as_slice()));

        // Deterministic prefix: the first section is still collected.
        assert_eq!(sections.len(), 1);
        assert_eq!(collect_rids(&sections), vec![vec![Some("rH")]]);
    }

    #[test]
    fn plans_orphans_and_notes_in_relationship_order() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
            <w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rH1"/></w:sectPr></w:pPr></w:p>
            <w:sectPr/>
          </w:body></w:document>"#;
        let rels = [
            relationship(
                "rFootnotes",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes",
                "footnotes.xml",
            ),
            relationship("rHOrphan", HEADER_TYPE, "header-orphan.xml"),
            relationship(
                "rComments",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments",
                "comments.xml",
            ),
            relationship("rH1", HEADER_TYPE, "header1.xml"),
            relationship(
                "rEndnotes",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes",
                "endnotes.xml",
            ),
        ];
        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let plan = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions::default(),
            &mut warnings,
        )
        .unwrap();

        // Referenced header first, orphan header next in rels order, then
        // footnotes/comments/endnotes in rels order.
        assert_eq!(
            plan.iter().map(|entry| entry.index).collect::<Vec<_>>(),
            vec![3, 1, 0, 2, 4]
        );
        assert!(warnings.is_empty());

        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let plan = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions {
                include_comments: false,
                ..DocxTextOptions::default()
            },
            &mut warnings,
        )
        .unwrap();

        assert_eq!(
            plan.iter().map(|entry| entry.index).collect::<Vec<_>>(),
            vec![3, 1, 0, 4]
        );
    }

    #[test]
    fn plan_skips_missing_and_unknown_reference_ids_with_warnings() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
            <w:sectPr>
              <w:headerReference/>
              <w:footerReference r:id="rGhost"/>
              <w:headerReference w:type="default" r:id="rH1"/>
            </w:sectPr>
          </w:body></w:document>"#;
        let rels = [relationship("rH1", HEADER_TYPE, "header1.xml")];
        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let plan = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions::default(),
            &mut warnings,
        )
        .unwrap();

        assert_eq!(
            plan.iter().map(|entry| entry.index).collect::<Vec<_>>(),
            vec![0]
        );
        let messages: Vec<&str> = warnings
            .iter()
            .map(|warning| warning.message.as_str())
            .collect();
        assert_eq!(
            messages,
            vec![
                "skipped DOCX headerReference: missing r:id",
                "skipped DOCX footerReference rGhost: unknown relationship id",
            ]
        );
        assert!(
            warnings
                .iter()
                .all(|warning| warning.path == "word/_rels/document.xml.rels")
        );
    }

    #[test]
    fn plan_dedups_by_resolved_path_at_first_reference() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
            <w:p>
              <w:pPr>
                <w:sectPr>
                  <w:headerReference w:type="default" r:id="rShared"/>
                  <w:headerReference w:type="default" r:id="rShared"/>
                  <w:headerReference w:type="default" r:id="rA"/>
                  <w:headerReference w:type="default" r:id="rB"/>
                </w:sectPr>
              </w:pPr>
            </w:p>
            <w:sectPr>
              <w:headerReference w:type="default" r:id="rShared"/>
              <w:headerReference w:type="default" r:id="rAlias"/>
            </w:sectPr>
          </w:body></w:document>"#;
        let rels = [
            relationship("rShared", HEADER_TYPE, "shared.xml"),
            relationship("rAlias", HEADER_TYPE, "shared.xml"),
            relationship("rA", HEADER_TYPE, "a.xml"),
            relationship("rB", HEADER_TYPE, "b.xml"),
        ];
        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let plan = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions::default(),
            &mut warnings,
        )
        .unwrap();

        // Duplicate rid in one section, shared part across sections, and two
        // rids aliasing one path all dedup by resolved path at first reference.
        assert_eq!(
            plan.iter().map(|entry| entry.index).collect::<Vec<_>>(),
            vec![0, 2, 3]
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn label_maps_variants_to_public_names() {
        assert_eq!(RelatedRefVariant::First.label(), "first");
        assert_eq!(RelatedRefVariant::Even.label(), "even");
        assert_eq!(RelatedRefVariant::Default.label(), "default");
    }

    #[test]
    fn labels_plan_entries_with_section_reference_variants() {
        let xml = r#"<w:document xmlns:w="w" xmlns:r="r"><w:body>
            <w:p>
              <w:pPr>
                <w:sectPr>
                  <w:headerReference w:type="first" r:id="rFirst"/>
                  <w:headerReference w:type="even" r:id="rEven"/>
                  <w:footerReference w:type="default" r:id="rFooter"/>
                </w:sectPr>
              </w:pPr>
            </w:p>
          </w:body></w:document>"#;
        let rels = [
            relationship("rFirst", HEADER_TYPE, "header1.xml"),
            relationship("rEven", HEADER_TYPE, "header2.xml"),
            relationship("rFooter", FOOTER_TYPE, "footer1.xml"),
            relationship("rOrphan", HEADER_TYPE, "header3.xml"),
            relationship(
                "rNotes",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes",
                "footnotes.xml",
            ),
        ];
        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let plan = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions::default(),
            &mut warnings,
        )
        .unwrap();

        // Section-referenced entries carry the sorted reference's variant;
        // orphan and notes entries carry none.
        assert_eq!(
            plan,
            vec![
                RelatedPartPlanEntry {
                    index: 0,
                    variant: Some(RelatedRefVariant::First)
                },
                RelatedPartPlanEntry {
                    index: 1,
                    variant: Some(RelatedRefVariant::Even)
                },
                RelatedPartPlanEntry {
                    index: 2,
                    variant: Some(RelatedRefVariant::Default)
                },
                RelatedPartPlanEntry {
                    index: 3,
                    variant: None
                },
                RelatedPartPlanEntry {
                    index: 4,
                    variant: None
                },
            ]
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn plans_deduped_part_with_first_reference_variant() {
        let xml = r#"<w:document xmlns:w="w" xmlns:r="r"><w:body>
            <w:p>
              <w:pPr>
                <w:sectPr>
                  <w:headerReference w:type="default" r:id="rShared"/>
                </w:sectPr>
              </w:pPr>
            </w:p>
            <w:sectPr>
              <w:headerReference w:type="first" r:id="rShared"/>
            </w:sectPr>
          </w:body></w:document>"#;
        let rels = [relationship("rShared", HEADER_TYPE, "shared.xml")];
        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let plan = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions::default(),
            &mut warnings,
        )
        .unwrap();

        // One deduped part referenced as two variants: the first reference's
        // variant wins.
        assert_eq!(
            plan,
            vec![RelatedPartPlanEntry {
                index: 0,
                variant: Some(RelatedRefVariant::Default)
            }]
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn plan_propagates_suspicious_target() {
        let xml = r#"<w:document xmlns:w="w"><w:body>
            <w:sectPr><w:headerReference w:type="default" r:id="rExt"/></w:sectPr>
          </w:body></w:document>"#;
        let rels = [Relationship {
            id: Some("rExt".to_owned()),
            target: "https://example.invalid/header.xml".to_owned(),
            relationship_type: Some(HEADER_TYPE.to_owned()),
            target_mode: Some("External".to_owned()),
        }];
        let mut package = ooxml_package(&[
            ("word/document.xml", xml),
            ("word/_rels/document.xml.rels", r#"<Relationships/>"#),
        ]);
        let mut warnings = Vec::new();
        let err = plan_related_part_order(
            &mut package,
            "word/document.xml",
            &rels,
            "word/_rels/document.xml.rels",
            DocxTextOptions::default(),
            &mut warnings,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            OxdocError::SuspiciousRelationshipTarget { .. }
        ));
    }

    #[test]
    fn fixture_related_parts_oracle_orders_sections_across_all_paths() {
        let part_names = [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/document.xml",
            "word/_rels/document.xml.rels",
            "word/comments.xml",
            "word/endnotes.xml",
            "word/footer1.xml",
            "word/footnotes.xml",
            "word/header1.xml",
        ];
        let contents: Vec<String> = part_names
            .iter()
            .map(|name| read_fixture(&format!("related-parts/package/{name}")))
            .collect();
        let entries: Vec<(&str, &str)> = part_names
            .iter()
            .zip(contents.iter().map(String::as_str))
            .map(|(name, content)| (*name, content))
            .collect();

        let expected: serde_json::Value =
            serde_json::from_str(&read_fixture("related-parts/expected.json")).unwrap();
        let expected_parts: Vec<(String, String)> = expected["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| {
                (
                    part["part_type"].as_str().unwrap().to_owned(),
                    part["part_path"].as_str().unwrap().to_owned(),
                )
            })
            .collect();

        // Flat text: join of the oracle part texts in oracle order.
        let mut package = ooxml_package(&entries);
        let text = extract_text(&mut package, DocxTextOptions::default()).unwrap();
        let expected_text: String = expected["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| format!("{}\n", part["text"].as_str().unwrap()))
            .collect();
        assert_eq!(text.value, expected_text);
        assert!(text.warnings.is_empty());

        // Structured blocks: part_type/part_path sequence matches the oracle.
        let mut package = ooxml_package(&entries);
        let structured = extract_structured_text(&mut package, DocxTextOptions::default()).unwrap();
        let blocks: Vec<(String, String)> = structured
            .value
            .blocks
            .iter()
            .map(|block| (block.part_type.clone(), block.part_path.clone()))
            .collect();
        assert_eq!(blocks, expected_parts);
        assert!(structured.warnings.is_empty());

        // Tables: part_type/part_path sequence matches the oracle.
        let mut package = ooxml_package(&entries);
        let tables = extract_tables(&mut package, DocxTextOptions::default()).unwrap();
        let table_parts: Vec<(String, String)> = tables
            .value
            .tables
            .iter()
            .map(|table| (table.part_type.clone(), table.part_path.clone()))
            .collect();
        assert_eq!(table_parts, expected_parts);
        assert_eq!(
            table_ordinals(&tables.value.tables),
            vec![1; expected_parts.len()]
        );
        assert!(tables.warnings.is_empty());
    }

    fn table_ordinals(tables: &[crate::models::DocxTable]) -> Vec<usize> {
        tables.iter().map(|table| table.table_ordinal).collect()
    }
}
