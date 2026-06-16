# DOCX Text Extraction

DOCX files are OOXML ZIP packages. The primary text content usually lives in `word/document.xml`, but `oxdoc` first checks `_rels/.rels` for the office document relationship and follows it when present.

## Current Behavior

The current parser:

- Reads the main document XML part.
- Follows main-document relationships for headers, footers, footnotes, endnotes, and comments.
- Extracts text from `<w:t>` nodes.
- Converts `<w:tab/>` into a tab character.
- Converts `<w:br/>` and `<w:cr/>` into line breaks.
- Adds a line break at the end of each paragraph.
- Separates table cells with tabs and table rows with line breaks.
- Flattens nested tables into the containing cell while preserving the outer row separators.
- Omits text inside deleted revision ranges (`<w:del>`) and keeps inserted revision text (`<w:ins>`).
- Emits recoverable malformed XML issues as warnings with partial text when possible.

## Logical Text Contract

`oxdoc` emits useful plain text, not rendered Word layout. The current contract is:

| DOCX construct | Output behavior |
| --- | --- |
| Paragraphs | A single `\n` is emitted at the end of each paragraph that closes outside a table cell. Empty paragraphs collapse with adjacent line breaks instead of producing repeated blank lines. |
| Table cells | Cells in the same row are separated with `\t`. |
| Table rows | Completed rows are separated with `\n`. |
| Multiple paragraphs inside one table cell | Paragraphs are joined with one space before the next cell separator. |
| Nested tables | Nested table text is flattened into the containing cell while preserving the outer row shape. |
| Tabs | `<w:tab/>` emits `\t`. |
| Line and carriage breaks | `<w:br/>` and `<w:cr/>` emit `\n`. |
| Lists | Numbering and bullet markers are not synthesized from `w:numPr`; only literal run text is emitted. |
| Inserted revisions | Text inside `<w:ins>` is emitted. |
| Deleted revisions | Text inside `<w:del>` is omitted. |
| Fields | Field instructions such as `<w:instrText>` are omitted; visible cached/result text in `<w:t>` is emitted. |
| Hidden text | Hidden run properties such as `<w:vanish/>` are not interpreted yet, so hidden `<w:t>` text is emitted when present in `word/document.xml`. |
| Hyperlinks | Visible run text inside hyperlinks is emitted; relationship targets are not emitted. |
| Related text parts | The main document body is emitted first, then headers, footers, footnotes, endnotes, and comments are appended in `word/_rels/document.xml.rels` relationship order. Missing related parts are skipped with a warning. |
| Text boxes and drawings | Visible text is emitted when it appears in `*:t` text nodes in parsed DOCX parts. Layout and drawing geometry are not interpreted. |

These rules are intentionally stable for scripts. Breaking changes to this contract should be called out in release notes.

## Structural Table Model

The plain-text contract above remains unchanged. `oxdoc extract tables` and the
Rust `extract_docx_tables` API expose a structural view rather than infer rows
and cells from tab-separated text.

The public shape is:

```text
DocxTable {
  part_type,
  part_path,
  table_ordinal,
  complete,
  grid_column_count,
  rows: [DocxTableRow],
}

DocxTableRow {
  row_ordinal,
  grid_before,
  grid_after,
  complete,
  cells: [DocxTableCell],
}

DocxTableCell {
  cell_ordinal,
  grid_start,
  grid_span,
  vertical_merge: none | restart | continue,
  complete,
  blocks: [Paragraph | Table],
}

Paragraph {
  text,
}
```

`table_ordinal`, `row_ordinal`, and `cell_ordinal` are 1-based encounter
ordinals. `grid_start` is a 0-based logical grid-column offset. The model is
structural, not a rectangular matrix: it does not synthesize cells for spans,
vertical merges, or omitted leading and trailing grid columns.

### Decisions

| Construct | Structural decision |
| --- | --- |
| Paragraphs | Preserve every direct child paragraph of a cell as an ordered `Paragraph` block, including empty paragraphs. Paragraph text follows the existing visible-text rules, but paragraph boundaries are not collapsed or converted to spaces. |
| `w:gridSpan` | Store the declared positive span as `grid_span`; absence means `1`. Advance the next cell's `grid_start` by that span. Do not create covered placeholder cells. A missing, zero, negative, or non-integer value produces a warning and falls back to `1`. |
| `w:vMerge` | Preserve raw merge state per cell. No element means `none`; `<w:vMerge w:val="restart"/>` means `restart`; `<w:vMerge/>` and `<w:vMerge w:val="continue"/>` mean `continue`. Unknown values produce a warning and are preserved as `none` for this version. |
| Row spans | Do not normalize vertical merges into a computed `row_span`. Raw restart/continue states are the stable source representation because malformed and producer-specific merge chains cannot always be resolved unambiguously. A future convenience layer may derive normalized spans without changing this model. |
| `w:gridBefore` / `w:gridAfter` | Store the declared non-negative counts on the row; absence means `0`. The first cell starts at `grid_before`. `grid_after` records omitted trailing columns and does not synthesize cells. Invalid values warn and fall back to `0`. |
| Grid width | `grid_column_count` is the number of `w:gridCol` children in `w:tblGrid`, when present. Row occupancy is `grid_before + sum(grid_span) + grid_after`; disagreement with the table grid is retained and warned about, not repaired. |
| Nested blocks | A cell's direct `w:p` and `w:tbl` children become `blocks` in exact document order. A nested table appears only inside its containing cell; it is not also emitted as a top-level table. |
| Revisions | Apply the visible-content policy before building structure: run content inside `w:del` and `w:moveFrom` is omitted; content inside `w:ins` and `w:moveTo` is retained. A row marked deleted by `w:trPr/w:del` is omitted. Property history such as `w:tblPrChange`, `w:trPrChange`, and `w:tcPrChange` is metadata and does not replace the current properties. |
| Malformed XML | Return warnings and the largest deterministic prefix. Fully closed tables are retained. If a table is open at the failure point, emit it with `complete: false`, retaining only fully closed rows; an open row or cell is discarded. Closed descendants of a discarded open row are not promoted. |
| Related parts | Discover the main part through the root office-document relationship. Emit main-part tables first, followed by tables from headers, footers, footnotes, endnotes, and comments in `document.xml.rels` relationship order. Every table carries `part_type` and normalized package `part_path`. Missing or malformed related parts warn and do not suppress tables already extracted from other parts. |

Table order within each part is depth-first document order. Because nested
tables live in cell `blocks`, top-level `table_ordinal` counts only tables whose
nearest table ancestor is absent. A nested table has its own row and cell
ordinals, scoped to that table, but no independent top-level ordinal.

The hand-authored source fixtures under `tests/fixtures/docx/` encode these
decisions as XML plus JSON result oracles. They are intentionally unpackaged so
parser and API tests can feed the XML directly or deterministically ZIP the
complete related-parts package tree.

## Example

```bash
oxdoc extract text contrato.docx
```

Output:

```text
Este es el texto plano extraido.
Otra linea del documento.
```

## JSON Example

```bash
oxdoc extract text contrato.docx --format json
```

```json
{
  "file": "contrato.docx",
  "text": "Este es el texto plano extraido.\n"
}
```

## Planned Improvements

- Section-aware ordering for headers and footers.
- Optional policy controls for hidden text, generated list markers, comments, related parts, and more detailed revision semantics.

## Non-Goals

- Rendering pages.
- Preserving line wrapping from Word.
- Preserving fonts, colors, margins, or pagination.
