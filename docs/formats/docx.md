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
