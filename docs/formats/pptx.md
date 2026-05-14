# PPTX Text Extraction

PPTX files are OOXML ZIP packages. `oxdoc` reads `ppt/presentation.xml`, follows the presentation relationships in slide order, extracts drawing text from slide text boxes, and includes speaker notes when a slide links to a notes part.

## Current Behavior

The current parser:

- Resolves the presentation part through `_rels/.rels` when present.
- Preserves slide order from `p:sldIdLst`.
- Extracts text from DrawingML `<a:t>` nodes.
- Converts `<a:tab/>` into a tab character.
- Converts `<a:br/>` and `<a:cr/>` into line breaks.
- Adds a line break at the end of each text paragraph.
- Extracts linked speaker notes after the slide text.
- Emits recoverable malformed XML issues as warnings with partial text when possible.

## Example

```bash
oxdoc extract text deck.pptx
```

Output:

```text
First Slide
Speaker note
Second Slide
```

## JSON Example

```bash
oxdoc extract text deck.pptx --format json
```

```json
{
  "file": "deck.pptx",
  "text": "First Slide\nSpeaker note\n"
}
```

Warnings are still written to stderr when JSON output is selected. They are not embedded in the JSON payload.

## Non-Goals

- Rendering slides.
- Preserving shape positions, visual layering, fonts, colors, or animations.
- Synthesizing bullets, numbering, or speaker timing.
- Extracting embedded media or chart data.
