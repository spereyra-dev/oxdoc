# Exploration — pptx-slide-scoped-json-jsonl (#180)

Explored against: `main`; OpenSpec store; canonical specs
`docx-extraction` + `structured-text-schema`. Upstream state: #179
(`structured-text-source-provenance`) archived; v2 structured-text contract live.

## 1. Goal restated

Expose presentation text **one slide at a time** (body text separated from
speaker notes) under a versioned JSON contract with JSON + JSONL emission,
stable slide identifiers/ordinals, and fixtures for no-notes, notes,
missing-target, and malformed-XML cases — **without** touching existing
text/plain/structured-json output.

## 2. Existing parser structures (crates/oxdoc-core/src/parsers/pptx.rs)

- `parse_slide_relation_ids(xml, path)` walks `p:sldIdLst` in presentation
  order and returns **only the `r:id` strings**. The `p:sldId/@id` numeric
  attribute (`id="256"`, `257` in the fixture) is read in
  `relationship_id_value`… actually it is NOT read: only `r:id` is extracted.
  A slide-stable identifier for #180 either needs to parse `@id` here or be
  derived (e.g., 1-based presentation ordinal). **Open design decision.**
- Warning in this path: `ignored presentation slide without relationship id`
  when a `p:sldId` lacks `r:id`; malformed presentation XML → `W001
  malformed_xml` with partial id list.
- `extract_text` / `extract_structured_text` share the loop: resolve rel id →
  `resolve_relationship_target` → `read_text_part` (slide), then notes via
  `read_notes_*_for_slide` (sorted by rel id; missing slide `.rels` file is
  **silently empty**; MissingPart of the rels file tolerated).
- **Hard-error paths to be aware of** (matter for the "missing targets" AC):
  - a dangling slide `r:id` (not in presentation rels) → hard
    `OxdocError::MissingPart`, whole extraction fails (no warning);
  - a notesSlide rel whose target part is absent from the package → hard
    `MissingPart` (only the DOCX path converts missing parts to warnings).
  #180's "missing targets" fixture must either lock this error behavior or
  define a new warning; that is a spec-level decision.
- `extract_structured_text` emits `TextBlock { part_type: "slide"|"notes",
  part_path, ordinal (global, renumbered), text }`. `renumber_blocks` keeps
  `ordinal == 1..=blocks.len()`. This is the #179 contract that #180 must NOT
  duplicate or modify.

## 3. Core public API surface (crates/oxdoc-core/src/lib.rs)

- PPTX entry points: `extract_pptx_text[_from_reader[_with_limits]]` and
  `extract_pptx_structured_text[_from_reader[_with_limits]]`.
- #180 needs a new per-slide extraction (e.g.,
  `Extraction<Vec<PptxSlideText>>` with slide id/ordinal/slide text/notes
  text), plus stdin/reader variants to match CLI stdin support. New model
  struct(s) belong in `models.rs` next to `StructuredText`.
- `TextBlock` gains **no** new fields (docx-extraction spec forbids; per-slide
  scoping must live in the new contract, not in `StructuredText`).

## 4. CLI plumbing (crates/oxdoc-cli/src/main.rs)

- `TextFormat { Text, Json, Jsonl, StructuredJson }` on
  `extract text FILE... --format`. Formats are document-type-restricted
  inline: `extract_structured_text` rejects XLSX;
  `extract_docx_tables` rejects PPTX ("cannot extract tables from a PPTX
  presentation"). Two viable shapes for #180:
  - **A. new subcommand** `extract slides FILE --format json|jsonl` (mirrors
    `extract tables`/`extract rows`, which take a single file and are
    type-restricted); or
  - **B. new `TextFormat` variants** (`slides-json`, `slides-jsonl`) on
    `extract text`, keeping multi-input semantics.
  `extract rows` precedent: single file, streaming records. `extract text`
  precedent: multi-file with array/pretty JSON. **Open decision**; option A
  matches "one document, per-slide streaming" and the rows/tables command
  shape, option B reuses the existing `-`/stdin and multi-file machinery.
- JSONL precedents:
  - `TextJsonlRecord`: **no schema_version**, fields `file`,
    `document_type`, optional `text`/`error`, optional embedded `warnings`
    (in-record, so batch consumers index them); one record per input file;
    per-file failures become error lines, never abort the batch.
  - `AuditJsonlRecord`: `schema_version: 1`, one record per file, embedded
    warnings.
  - `RowsJsonlRecord`: `schema_version: 1`, one record per **row** of a
    single input; recoverable warnings go to **stderr** so stdout stays a
    valid JSONL stream.
  A slide-scoped JSONL is closest to rows-jsonl (stream within one document)
  but the "streaming batch" text-jsonl precedent embeds warnings. **Open
  decision:** one line per slide (`{schema_version, file, slide_id,
  slide_ordinal, text, notes?}` shape TBD) with either embedded or
  stderr-only warnings. Existing lines with errors: text-jsonl and
  audit-jsonl emit an `error` object line; rows-jsonl has no per-line error
  contract.
- `OutputWriter` + `--output` exist; `emit_skipped_input_warning`,
  `emit_warnings` (text/json/none), `--warnings json` one-line-per-warning
  stderr channel all reusable.
- JSON pretty emission uses `serde_json::to_writer_pretty` + trailing
  newline; empty-batch behavior: "no input files were processed
  successfully" error for json/structured-json/text.

## 5. Schemas and versioning policy

- Policy (docs/json-output.md + structured-text-schema spec): new output
  fields → **new schema version**; frozen old versions stay validatable.
  This is a **new contract**, not a widening of structured-text, so a fresh
  `schemas/v1/oxdoc-pptx-slides.schema.json` (draft 2020-12, stable `$id`
  `https://github.com/spereyra-dev/oxdoc/schemas/v1/oxdoc-pptx-slides.schema.json`,
  `additionalProperties: false`) is the policy-compliant route, exactly like
  `oxdoc-xlsx-rows-jsonl.schema.json` did for rows-jsonl.
- Mandatory chores attached to a new schema file:
  - mirrored copy `docs/schemas/v1/…` (Makefile `docs-schemas-check` does
    `diff -ru schemas/v1 docs/schemas/v1`, so the new file is auto-covered);
  - `crates/oxdoc-core/tests/schema.rs` `SCHEMA_VERSIONS` table: one-line
    addition to the v1 list; validation harness (`validate_object`,
    `read_json_schema(version, name)`) supports integer minimums but note it
    only checks declared fields/types (no const/enum enforcement except
    where locally asserted);
  - `docs/json-output.md` table row + semantics section;
  - `docs/formats/pptx.md` new section; `docs/cli.md`; `CHANGELOG.md`.
- Documented asymmetry to preserve: `StructuredText` library type stays
  unversioned; version markers are CLI emission concerns. The new slide model
  should follow the same rule.

## 6. Fixtures & snapshots (tests/fixtures)

- `corpus/pptx/basic/`: 1 slide, **no notes** → covers "no notes" AC as-is.
- `corpus/pptx/text/`: 2 slides in non-file-name order (sldIdLst lists rId2
  first → slide2 emitted first), slide 2 links `notesSlide2.xml` → covers
  "notes" + order-stability AC as-is.
- Snapshots: `pptx_text.txt`, `cli_structured_text_pptx_json.json` (v2
  payload). CLI tests: `extracts_pptx_text_as_structured_json` (byte
  snapshot + `file` assert), schema.rs
  `representative_structured_text_pptx_json_matches_schema`.
- **Missing**: "missing targets" fixture (dangling slide rel / notes rel to
  absent part — see §2 hard-error list; today these are hard errors, not
  warnings) and "malformed XML" fixture (no pptx corpus fixture has malformed
  slide XML today; unit tests cover it synthetically). New corpus fixtures
  under `corpus/pptx/…` follow the `fixtures::build_package(dir, name)`
  pattern. Note: `tests/fixtures/provenance/pptx-*.md` provenance + digest
  checks exist (`make compatibility-corpus-check`) — new corpus dirs may need
  provenance entries; existing corpus edits require updated digests.

## 7. Gaps vs acceptance criteria

| AC | Status |
| --- | --- |
| Slide order + stable identifiers/ordinals | Order: done in parser. Identifiers: `p:sldId/@id` not parsed today; ordinals are global-block, not per-slide → new work |
| JSON + JSONL contracts without breaking text output | No slide-scoped format exists; JSONL shape + warning placement undecided; existing text/json/jsonl/structured-json outputs must stay byte-identical (snapshot-locked) |
| Versioned schemas + documented warning behavior | New schema file + mirror + docs needed; warning behavior of the new format (embedded vs stderr) needs a spec scenario; missing-target behavior is currently hard-error and must be pinned or changed explicitly |
| Fixtures (no notes / notes / missing targets / malformed XML) | 2 of 4 exist; missing-target and malformed-XML corpus fixtures must be added (with provenance/digest updates) |

## 8. What #180 must NOT duplicate from #179

- `--format structured-json` and the v2 schema stay untouched: no new block
  fields, no per-slide ordinals in structured-json (the canonical
  structured-text-schema spec **explicitly** says "Per-slide ordinal scoping is
  deferred to a future change and MUST NOT be introduced here" — #180 is that
  future change, but the deferral is satisfied by the *new* contract, not by
  editing structured-json semantics).
- `TextBlock`/`StructuredText` shapes unchanged; DOCX paths untouched.
- Plain text and tables outputs byte-identical (regression scenarios + existing
  snapshots already lock this).
- v1 frozen schemas and the v2 structured-text schema unmodified.

## 9. Risks / decisions to surface in the proposal

1. Slide identity source: `p:sldId/@id` (numeric, canonical) vs 1-based
   presentation ordinal vs both. Recommend both (`slide_id` + `slide_ordinal`)
   since `@id` is parsed-adjacent and `@id` guarantees "stable identifiers".
2. CLI shape: new subcommand vs new TextFormat variants (§4) — affects stdin,
   multi-input, and `--output` semantics.
3. JSONL record granularity and warning placement (stderr-only like rows vs
   embedded like text/audit jsonl); whether an error-line contract exists.
4. Missing-target behavior: hard error today; AC wording ("warnings… missing
   targets") suggests per-slide JSON/JSONL may need warnings or per-record
   error lines instead of aborting extraction.
5. Review budget: schema file + mirror (~120 lines JSON), parser work (~60),
   CLI (~80), tests (~100–150), fixtures + docs (~80) — roughly at the 400-line
   ask-on-risk threshold; likely a size conversation at proposal time.
6. Provenance/digest gate for new corpus fixtures.

## 10. Recommended next step

Proceed to proposal/spec with decisions on §9 items 1–4; draft delta spec as a
**new domain** (e.g., `pptx-slide-extraction`) since `openspec/specs` has no
PPTX domain yet, while referencing `structured-text-schema` only for the
frozen-v2 boundary.
