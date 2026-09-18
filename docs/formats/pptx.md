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

## Slide-scoped JSON / JSONL

```bash
oxdoc extract slides deck.pptx
oxdoc extract slides deck.pptx --format jsonl
```

`oxdoc extract slides` emits one record per slide with body text and speaker
notes as fields of the same record, so consumers never reconstruct slides from
positional blocks. `json` is the default format and emits a single
pretty-printed document; `--format jsonl` emits one compact record per line.
Both shapes validate against
[`schemas/v1/oxdoc-pptx-slides.schema.json`](../schemas/v1/oxdoc-pptx-slides.schema.json)
(`schema_version: 1`).

```json
{
  "schema_version": 1,
  "file": "deck.pptx",
  "document_type": "pptx",
  "slides": [
    {
      "slide_id": 256,
      "slide_ordinal": 1,
      "slide_path": "ppt/slides/slide2.xml",
      "text": "First Slide\n",
      "notes": "Speaker note\n"
    }
  ],
  "warnings": []
}
```

Record fields:

- `slide_id` is the `p:sldId/@id` attribute value as a JSON integer. When a
  `p:sldId` has no `@id` (or it is not an unsigned integer), the slide is
  still extracted and the `slide_id` key is omitted — never `null`.
- `slide_ordinal` is the 1-based position of the `p:sldId` element within
  `p:sldIdLst` in `ppt/presentation.xml` — not a renumbered index over the
  emitted records. When a slide is skipped (below), the remaining records keep
  their presentation positions, so the emitted ordinals may contain gaps: a
  deck whose second slide is skipped emits ordinals `1` and `3`, and a
  four-entry `sldIdLst` whose two middle slides are skipped emits `1` and `4`.
- `slide_path` is the resolved package part path of the slide (for example
  `ppt/slides/slide2.xml`), so consumers never reverse-engineer part-name
  conventions. Slide order follows `p:sldIdLst`, not slide part file names.
- `text` is the slide body text and may be `""` for a textless slide; the
  record is still emitted so the record set stays aligned with the deck.
- `notes` holds the slide's speaker notes from its
  `ppt/notesSlides/notesSlideN.xml` part. It is present (possibly `""`) when a
  notes part was read successfully and omitted when the slide has no notes
  relationship, the notes target part is missing, or the slide's `.rels` file
  is absent — the key is omitted, never `null`.

Skips degrade per slide and never abort the extraction. Exactly three warning
wordings can skip or amend slide records:

- `skipped PPTX slide {rid}: unknown relationship id` (path
  `ppt/presentation.xml`) — the slide `r:id` is absent from
  `ppt/_rels/presentation.xml.rels`; that slide is skipped. A missing
  presentation rels part is treated as an empty relationship map with no
  warning of its own, so every slide reference produces this warning.
- `skipped related PPTX slide part {path}: missing part` (path = the absent
  slide part) — the resolved slide target is absent from the package; that
  slide is skipped.
- `skipped related PPTX notes part {path}: missing part` (path = the absent
  notes part) — the resolved notes target is absent; the slide record is still
  emitted with `notes` omitted.

Malformed slide XML keeps the existing recoverable `W001 malformed_xml`
warning plus whatever partial text could be recovered; the record is not
dropped. A `p:sldId` without `@id` never produces a warning.

Warning channels differ by format:

- `--format json` embeds the warnings in the payload's top-level `warnings`
  array (present and `[]` when empty, never omitted, never suppressed by
  `--quiet`) and mirrors them to stderr subject to the global
  `--warnings`/`--quiet` flags.
- `--format jsonl` writes warnings to stderr only, so stdout remains a valid
  JSONL record stream; skipped slides are detectable only through stderr.

A deck whose slides are all skipped still exits `0` with a valid payload:
`--format json` emits `"slides": []` plus the embedded warnings, and
`--format jsonl` emits no records with the warnings on stderr.

## Non-Goals

- Rendering slides.
- Preserving shape positions, visual layering, fonts, colors, or animations.
- Synthesizing bullets, numbering, or speaker timing.
- Extracting embedded media or chart data.
