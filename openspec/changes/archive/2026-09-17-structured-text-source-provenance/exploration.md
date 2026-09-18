# Exploration — expose source-aware structured text for DOCX and PPTX (issue #179)

- Change id: `structured-text-source-provenance` (proposed)
- Status: explored (SDD explore phase, artifact store: openspec, branch `main`)
- Inputs: GitHub issue #179 acceptance criteria (verbatim, below); code on `main`
  after the archived change `docx-section-order-related-parts` (#177, PR #199-era).

## Acceptance criteria (verbatim)

1. Version a JSON schema describing part/slide, paragraph/block ordinal, and text.
2. Keep existing plain-text output unchanged.
3. Support DOCX related parts and PPTX slide text versus speaker notes.
4. Add schema snapshots, fixtures, CLI tests, and documentation.

## 1. Current StructuredText/TextBlock model

`crates/oxdoc-core/src/models.rs` (~line 187):

- `StructuredText { document_type: String, blocks: Vec<TextBlock> }` (Serialize).
- `TextBlock { part_type: String, part_path: String, ordinal: usize, text: String }`
  with `TextBlock::new(part_type, part_path, ordinal, text)`.
- `ordinal` is a **global 1-based output-order index**, not a per-part paragraph
  ordinal. Blocks are one per *part* (whole-part text), not per paragraph.
- Public API surface (`lib.rs`): `extract_{docx,pptx}_structured_text[_from_reader]`
  plus `_with_options` / `_with_limits` variants for DOCX.

## 2. DOCX structured extraction state

`crates/oxdoc-core/src/parsers/docx.rs::extract_structured_text` (line ~77):

- Pushes a `main` block from `word/document.xml` first, then walks the shared
  section-aware plan (`plan_related_part_order`) for related parts when
  `options.include_related_parts`; missing rels returns main-only silently.
- Part labels come from `related_docx_text_part_type` (line ~259):
  `"header" | "footer" | "footnotes" | "endnotes" | "comments"` (comments only
  when `include_comments`). **No variant discrimination**: first/even/default
  headers all map to `"header"`; variant info is only implicit in `part_path`
  and in the section-aware ordering (first, even, default) fixed by #177.
- `push_text_block` (line ~509) skips empty text and numbers ordinals by
  encounter order; warnings follow #177 spec exactly.
- Core tests: `crates/oxdoc-core/tests/api.rs::orders_docx_structured_blocks_by_section`
  (line ~637), `extracts_docx_structured_text_blocks_with_related_part_sources`
  (line ~1023), `..._without_relationships_from_reader` (line ~1069).

## 3. PPTX structured extraction state (already exists)

`crates/oxdoc-core/src/parsers/pptx.rs::extract_structured_text` (line ~52):

- Iterates `p:sldIdLst` slide ids in presentation order; per slide pushes a
  `"slide"` block from the slide part, then resolves `/notesSlide`
  relationships from `ppt/slides/_rels/slideN.xml.rels` (sorted by rel id) and
  pushes `"notes"` blocks after the slide block; `renumber_blocks` keeps
  global ordinals contiguous.
- Malformed slide/notes XML → W001 warning with partial text; missing slide
  rels file → no notes, no warning; escaping notes targets are hard errors
  (covered by `api.rs::rejects_pptx_notes_relationship_targets_that_escape_package_root`).
- Core test: `api.rs::extracts_pptx_structured_text_blocks_with_notes_sources`
  (line ~1089). Parser unit tests cover text extraction, not structured shape.

## 4. CLI state

`crates/oxdoc-cli/src/main.rs`:

- `TextFormat::{Text, Json, StructuredJson, Jsonl}` (line ~218);
  `StructuredJson` dispatches to `extract_structured_text` which routes
  DOCX → `extract_docx_structured_text_with_options`, PPTX →
  `extract_pptx_structured_text`, XLSX rejected (test
  `rejects_structured_text_from_xlsx`). Stdin/multi-input batching and
  skipped-input warnings are implemented and tested (`cli.rs` lines ~394–460,
  ~2581).
- Output payload `TextStructuredPayload { file, #[serde(flatten)] structured }`
  (line ~1469). **No `schema_version` field** — unlike tables/rows-jsonl/audit
  payloads which carry `schema_version: 1`.

## 5. Schema + snapshot state

- `schemas/v1/oxdoc-structured-text.schema.json` (draft 2020-12, `$id`
  `.../schemas/v1/...`, `additionalProperties: false`): requires
  `file`, `document_type` (enum `["docx","pptx"]`), `blocks[]` with
  `part_type` (free string), `part_path`, `ordinal` (min 1), `text`. The
  description enumerates `main, header, footer, footnotes, endnotes, comments,
  slide, notes` but there is **no `enum` constraint** and **no variant field**.
- Mirror copy in `docs/schemas/v1/`; kept in sync by `Makefile` target
  (`diff -ru schemas/v1 docs/schemas/v1`). Both dirs must be updated together.
- `crates/oxdoc-core/tests/schema.rs::representative_structured_text_json_matches_schema`
  validates snapshot `tests/fixtures/snapshots/cli_structured_text_json.json` —
  which is **DOCX-only** (`main` + `comments`). There is **no PPTX structured
  snapshot**. `schemas_have_stable_public_metadata` pins `$schema`/`$id`/
  `additionalProperties` for all nine v1 schemas.
- Policy in `docs/json-output.md`: "New output fields are introduced through a
  new schema version instead of silently widening the current contract."

## 6. Fixture state

- Hand-authored corpus `tests/fixtures/corpus/`: `docx/section-order/` has
  first/even/default/orphan/titled header parts (ideal for variant-label
  scenarios); `pptx/text/` has slides + `notesSlide2.xml` linked from slide2
  (slide-vs-notes fixture already exists). `docx/related-parts/` covers
  related parts. Application-generated files exist for docx/pptx/xlsx with
  provenance notes under `tests/fixtures/provenance/`.

## 7. Gap analysis vs acceptance criteria

| Criterion | State | Gap |
| --- | --- | --- |
| 1. Versioned JSON schema | Schema exists (v1) but output payload has no version marker; schema has no `enum` for `part_type` and no variant field | Decide: add optional variant field + (per policy) `schemas/v2`, or clarify v1 suffices; decide on `schema_version` in payload |
| 2. Plain text unchanged | Structured paths are separate; plain-text snapshots (`pptx_text.txt`, etc.) lock behavior | Add explicit regression scenarios asserting plain-text byte-stability alongside new structured scenarios |
| 3. DOCX related parts / PPTX slide vs notes | Fully implemented (labels `slide` vs `notes`; DOCX related-part labels) | **Header/footer variant labeling (first/even/default) deferred here from #177** — the open functional work |
| 4. Snapshots, fixtures, CLI tests, docs | DOCX-only snapshot; pptx structured CLI test asserts minimal fields; `docs/formats/pptx.md` has no structured-json section; `docs/formats/docx.md` documents structured only via API/options text | Add PPTX structured snapshot; variant-labeled fixture expectations; pptx.md structured section; cli.md/json-output.md wording for variants |

## 8. Deferred-work note (from #177 spec)

`openspec/specs/docx-extraction/spec.md` requirement "No public output schema
changes" explicitly states: "MUST NOT emit variant labels. Variant labeling is
deferred to a future change." Issue #179 is that future change, so its delta
spec must modify/amend that requirement (a versioned, intentional schema
change) rather than silently add fields.

## 9. Adjacent issues (out of scope)

- #180 slide-scoped JSON/JSONL: overlaps with per-slide granularity and
  `schema_version` questions; keep #179 schema-level, record-level concerns
  for #180.
- #182 xlsx formulas: unrelated (no shared parser files).
