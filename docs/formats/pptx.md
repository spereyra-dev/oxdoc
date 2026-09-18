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

## Structured JSON

```bash
oxdoc extract text deck.pptx --format structured-json
```

Structured output keeps each non-empty text part as a separate block:

```json
{
  "schema_version": 2,
  "file": "deck.pptx",
  "document_type": "pptx",
  "blocks": [
    {
      "part_type": "slide",
      "part_path": "ppt/slides/slide2.xml",
      "ordinal": 1,
      "text": "First Slide\n"
    },
    {
      "part_type": "notes",
      "part_path": "ppt/notesSlides/notesSlide2.xml",
      "ordinal": 2,
      "text": "Speaker note\n"
    }
  ]
}
```

- `slide` blocks carry the text of one slide; `notes` blocks carry that
  slide's speaker notes, extracted from the linked
  `ppt/notesSlides/notesSlideN.xml` part. A `notes` block follows its slide.
- Slide order follows `p:sldIdLst` in `ppt/presentation.xml`, not slide part
  file names: a deck whose `sldIdLst` lists slide2 before slide1 emits the
  slide2 block first even though its file name sorts later.
- `ordinal` is a 1-based global output-order index across the flattened block
  list — not scoped per part or per slide.
- PPTX blocks never carry a `variant` field; that label is a DOCX
  header/footer concept.

## Non-Goals

- Rendering slides.
- Preserving shape positions, visual layering, fonts, colors, or animations.
- Synthesizing bullets, numbering, or speaker timing.
- Extracting embedded media or chart data.
