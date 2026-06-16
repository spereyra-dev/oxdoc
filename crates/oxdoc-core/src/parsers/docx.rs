use std::io::Cursor;
use std::io::{BufRead, BufReader, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{
    DocxTableBlock as PublicDocxTableBlock, DocxTableCell as PublicDocxTableCell,
    DocxTableRow as PublicDocxTableRow, DocxTables, Extraction, OutputWarning, StructuredText,
    TextBlock,
};
use crate::parsers::find_office_document_path;
use crate::parsers::{
    append_decoded_xml_reference, append_decoded_xml_text, attr_value, name_eq, parent_dir,
    parse_relationships, rels_path_for, resolve_relationship_target,
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

pub(crate) fn extract_tables<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
) -> Result<Extraction<DocxTables>> {
    let document_path = find_office_document_path(package, "word/document.xml")?;
    let document = extract_part_tables(package, &document_path, "main")?;
    let mut tables = public_tables_for_part(document.value, "main", &document_path);
    let mut warnings = document.warnings;

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
        match extract_part_tables(package, &part_path, part_type) {
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
) -> Result<Extraction<String>> {
    package.with_entry(path, |entry| {
        let reader = BufReader::new(entry);
        extract_xml_text(reader, path)
    })
}

fn extract_part_tables<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    path: &str,
    _part_type: &str,
) -> Result<Extraction<Vec<DocxTable>>> {
    package.with_entry(path, |entry| {
        let reader = BufReader::new(entry);
        parse_xml_tables(reader, path)
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
    let mut decoded = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if is_deleted_revision(element.name().as_ref()) {
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
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                push_text(&mut text, &decoded, &mut pending_cell_paragraph_separator);
            }
            Ok(Event::CData(value)) if in_text_node => {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                push_text(&mut text, &decoded, &mut pending_cell_paragraph_separator);
            }
            Ok(Event::GeneralRef(value)) if in_text_node => {
                decoded.clear();
                append_decoded_xml_reference(value.as_ref(), &mut decoded);
                push_text(&mut text, &decoded, &mut pending_cell_paragraph_separator);
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text_node = false;
                } else if is_deleted_revision(element.name().as_ref()) {
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

fn parse_xml_tables<R: BufRead>(source: R, path: &str) -> Result<Extraction<Vec<DocxTable>>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut tables = Vec::new();
    let mut table_stack = Vec::<DocxTableBuilder>::new();
    let mut paragraph = None::<String>;
    let mut in_text_node = false;
    let mut deleted_revision_depth = 0usize;
    let mut warnings = Vec::new();
    let mut decoded = String::new();
    let mut malformed = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if is_deleted_revision(element.name().as_ref()) {
                    mark_current_row_deleted(&mut table_stack);
                    deleted_revision_depth += 1;
                } else if name_eq(element.name().as_ref(), b"tbl") {
                    table_stack.push(DocxTableBuilder::default());
                } else if name_eq(element.name().as_ref(), b"gridCol") {
                    count_grid_column(&mut table_stack);
                } else if name_eq(element.name().as_ref(), b"tr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.start_row();
                    }
                } else if name_eq(element.name().as_ref(), b"trPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.row_properties_depth += 1;
                    }
                } else if name_eq(element.name().as_ref(), b"tc") {
                    if let Some(table) = table_stack.last_mut() {
                        table.start_cell();
                    }
                } else if name_eq(element.name().as_ref(), b"tcPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.cell_properties_depth += 1;
                    }
                } else if name_eq(element.name().as_ref(), b"gridBefore")
                    || name_eq(element.name().as_ref(), b"gridAfter")
                {
                    apply_row_grid_offset(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), b"gridSpan") {
                    apply_cell_grid_span(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), b"vMerge") {
                    apply_cell_vertical_merge(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), b"p")
                    && current_table_has_cell(&table_stack)
                {
                    paragraph = Some(String::new());
                } else if deleted_revision_depth == 0
                    && paragraph.is_some()
                    && name_eq(element.name().as_ref(), b"t")
                {
                    in_text_node = true;
                }
            }
            Ok(Event::Empty(element)) if deleted_revision_depth == 0 => {
                if is_deleted_revision(element.name().as_ref()) {
                    mark_current_row_deleted(&mut table_stack);
                } else if name_eq(element.name().as_ref(), b"gridCol") {
                    count_grid_column(&mut table_stack);
                } else if name_eq(element.name().as_ref(), b"gridBefore")
                    || name_eq(element.name().as_ref(), b"gridAfter")
                {
                    apply_row_grid_offset(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), b"gridSpan") {
                    apply_cell_grid_span(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), b"vMerge") {
                    apply_cell_vertical_merge(&mut table_stack, &element, path, &mut warnings);
                } else if name_eq(element.name().as_ref(), b"p") {
                    if let Some(cell) = current_cell_mut(&mut table_stack) {
                        cell.blocks.push(DocxCellBlock::Paragraph(String::new()));
                    }
                } else if let Some(paragraph) = paragraph.as_mut() {
                    if name_eq(element.name().as_ref(), b"tab") {
                        paragraph.push('\t');
                    } else if name_eq(element.name().as_ref(), b"br")
                        || name_eq(element.name().as_ref(), b"cr")
                    {
                        paragraph.push('\n');
                    }
                }
            }
            Ok(Event::Text(value)) if in_text_node && deleted_revision_depth == 0 => {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                if let Some(paragraph) = paragraph.as_mut() {
                    paragraph.push_str(&decoded);
                }
            }
            Ok(Event::CData(value)) if in_text_node && deleted_revision_depth == 0 => {
                decoded.clear();
                append_decoded_xml_text(value.as_ref(), &mut decoded);
                if let Some(paragraph) = paragraph.as_mut() {
                    paragraph.push_str(&decoded);
                }
            }
            Ok(Event::GeneralRef(value)) if in_text_node && deleted_revision_depth == 0 => {
                decoded.clear();
                append_decoded_xml_reference(value.as_ref(), &mut decoded);
                if let Some(paragraph) = paragraph.as_mut() {
                    paragraph.push_str(&decoded);
                }
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text_node = false;
                } else if is_deleted_revision(element.name().as_ref()) {
                    deleted_revision_depth = deleted_revision_depth.saturating_sub(1);
                    in_text_node = false;
                } else if name_eq(element.name().as_ref(), b"p") {
                    if let Some(paragraph) = paragraph.take()
                        && let Some(cell) = current_cell_mut(&mut table_stack)
                    {
                        cell.blocks.push(DocxCellBlock::Paragraph(paragraph));
                    }
                } else if name_eq(element.name().as_ref(), b"tcPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.cell_properties_depth = table.cell_properties_depth.saturating_sub(1);
                    }
                } else if name_eq(element.name().as_ref(), b"tc") {
                    if let Some(table) = table_stack.last_mut() {
                        table.finish_cell();
                    }
                } else if name_eq(element.name().as_ref(), b"trPr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.row_properties_depth = table.row_properties_depth.saturating_sub(1);
                    }
                } else if name_eq(element.name().as_ref(), b"tr") {
                    if let Some(table) = table_stack.last_mut() {
                        table.finish_row();
                    }
                } else if name_eq(element.name().as_ref(), b"tbl")
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

fn is_deleted_revision(name: &[u8]) -> bool {
    name_eq(name, b"del") || name_eq(name, b"moveFrom")
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
    let raw_value = attr_value(element, b"val");
    let value = raw_value
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok());
    let Some(row) = table.row.as_mut() else {
        return;
    };
    let property = if name_eq(element.name().as_ref(), b"gridBefore") {
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
    if name_eq(element.name().as_ref(), b"gridBefore") {
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
    let raw_value = attr_value(element, b"val");
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
    let raw_value = attr_value(element, b"val");
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
    let _ = extract_xml_text(Cursor::new(xml), "word/document.xml")?;
    let _ = parse_xml_tables(Cursor::new(xml), "word/document.xml")?;
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

    use super::{DocxCellBlock, DocxVerticalMerge, extract_xml_text, parse_xml_tables};

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

        let result = parse_xml_tables(Cursor::new(xml), "word/document.xml").unwrap();

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

        let result = parse_xml_tables(Cursor::new(xml), "word/document.xml").unwrap();
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

        let result = parse_xml_tables(Cursor::new(xml), "word/document.xml").unwrap();

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

        let result = parse_xml_tables(Cursor::new(xml), "word/header1.xml").unwrap();

        assert!(result.warnings.is_empty());
        assert_eq!(
            result.value[0].rows[0].cells[0].blocks,
            vec![DocxCellBlock::Paragraph("Header cell".to_owned())]
        );
    }

    #[test]
    fn fixture_table_semantics_match_structural_parser_contract() {
        let xml = read_fixture("table-semantics/document.xml");

        let result = parse_xml_tables(Cursor::new(xml), "word/document.xml").unwrap();

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

        let result = parse_xml_tables(Cursor::new(xml), "word/document.xml").unwrap();

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
            let result = parse_xml_tables(Cursor::new(xml), path).unwrap();

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

        let result = parse_xml_tables(Cursor::new(xml.as_slice()), "word/header1.xml").unwrap();

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
}
