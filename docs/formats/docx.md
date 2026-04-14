# DOCX Text Extraction

DOCX files are OOXML ZIP packages. The primary text content usually lives in `word/document.xml`, but `oxdoc` first checks `_rels/.rels` for the office document relationship and follows it when present.

## Current Behavior

The MVP parser:

- Reads the main document XML part.
- Extracts text from `<w:t>` nodes.
- Converts `<w:tab/>` into a tab character.
- Converts `<w:br/>` and `<w:cr/>` into line breaks.
- Adds a line break at the end of each paragraph.
- Separates table cells with tabs and table rows with line breaks.
- Flattens nested tables into the containing cell while preserving the outer row separators.
- Omits text inside deleted revision ranges (`<w:del>`) and keeps inserted revision text (`<w:ins>`).
- Emits recoverable malformed XML issues as warnings with partial text when possible.

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

- Headers and footers.
- Footnotes and endnotes.
- Comments.
- Hyperlink text.
- Text boxes and drawing text.
- More detailed revision semantics outside deleted and inserted text.

## Non-Goals

- Rendering pages.
- Preserving line wrapping from Word.
- Preserving fonts, colors, margins, or pagination.
