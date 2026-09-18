# Proposal — slide-scoped PPTX JSON/JSONL with stable slide identity (issue #180)

- Change id: `pptx-slide-scoped-json-jsonl`
- Status: proposed (SDD propose phase, artifact store: openspec)
- Inputs: `openspec/changes/pptx-slide-scoped-json-jsonl/exploration.md`,
  GitHub issue #180 acceptance criteria (as quoted by the parent context),
  parent-resolved product decisions (authoritative, traceability table below),
  `openspec/config.yaml`, canonical specs
  `openspec/specs/docx-extraction/spec.md` and
  `openspec/specs/structured-text-schema/spec.md`, and the versioning policy in
  `docs/json-output.md`.
- Delivery: `ask-on-risk` · review budget 400 changed lines · strict TDD
  (`cargo test`) · 95% line-coverage gate.

## Intent / problem

`oxdoc extract text --format structured-json` (schema v2, #179) made slide text
and speaker notes *visible*, but the contract is still **part-scoped**, not
**slide-scoped**:

1. **No slide identity.** PPTX output identifies slides only by
   `part_path` (`ppt/slides/slide2.xml`) plus a **global** block ordinal that
   counts every emitted block, including `notes` blocks and — for mixed inputs —
   other documents' blocks. A consumer cannot say "give me slide 3 of this deck"
   without reverse-engineering part-name conventions, which the schema never
   promises. The canonical `structured-text-schema` spec explicitly deferred
   per-slide ordinal scoping to a future change; #180 is that change.
2. **Body text and notes are separate blocks, not one slide object.** Notes
   arrive as a sibling block, so joining "slide N's text with slide N's notes"
   is a positional reconstruction across two block kinds. Integrations
   (translation pipelines, deck review tools, LLM ingestion) repeatedly want one
   record per slide with both fields.
3. **No streaming shape for decks.** `extract rows --format jsonl` proves the
   streaming precedent for record-per-record extraction inside **one** document,
   with warnings on stderr so stdout stays a valid record stream. PPTX has no
   equivalent: `extract text --format jsonl` emits one record per *file*, and
   `structured-json` is a single pretty-printed document. A 300-slide deck must
   be buffered whole.
4. **Missing per-slide targets are all-or-nothing.** Today a dangling slide
   `r:id` or an absent notes part raises a hard `OxdocError::MissingPart` that
   aborts the whole extraction, so one broken slide reference loses the other
   99 slides' usable text. The new slide-scoped contract needs documented
   skip-with-warning granularity per slide.

Users and situations: automation that ingests decks into text/embeddings store,
localization teams extracting per-slide text + notes, review tooling that needs
a stable handle per slide, and any pipeline that reads with `jq`-style line
processing. The gap today is that these consumers must parse
`structured-json` positionally and accept whole-document failure.

The value is that PPTX gains a first-class, versioned, per-slide contract with
stable identity, streaming, and bounded failure — **without** touching the frozen
v2 structured-text contract or any existing byte-locked output.

## Solution shape

### 1. Core API: one per-slide extraction path

New model in `crates/oxdoc-core/src/models.rs`, next to `StructuredText`
(unversioned, like every other core model — the version marker stays a CLI
emission concern):

```rust
pub struct PptxSlideText {
    pub slide_id: u32,          // p:sldId/@id (stable within the presentation)
    pub slide_ordinal: usize,   // 1-based index in p:sldIdLst
    pub slide_path: String,     // resolved slide part path
    pub text: String,           // slide body text; may be "" for a textless slide
    pub notes: Option<String>,  // speaker notes; None when the slide links no notes part
}
```

New `crates/oxdoc-core/src/lib.rs` entry points, mirroring the existing
`extract_pptx_*` family (path / reader / reader-with-limits), so stdin and the
`CliLimits` plumbing keep working:

```rust
pub fn extract_pptx_slides(path) -> Result<Extraction<Vec<PptxSlideText>>>
pub fn extract_pptx_slides_from_reader<R: Read + Seek>(reader) -> Result<…>
pub fn extract_pptx_slides_from_reader_with_limits<R: Read + Seek>(reader, limits) -> Result<…>
```

`Extraction<Vec<PptxSlideText>>` carries `warnings` for skipped slides, exactly
like every other lib path. `StructuredText` and `TextBlock` gain **no** fields;
the `structured-text-schema` v2 boundary is untouched.

### 2. Parser semantics (`crates/oxdoc-core/src/parsers/pptx.rs`)

- `parse_slide_relation_ids` becomes a richer shared parse that returns, per
  `p:sldId` in `p:sldIdLst` order, the `r:id` **and** the optional `p:sldId/@id`
  (unprefixed attribute; the existing `relationship_id_value` helper deliberately
  matches only namespaced `r:id`, so `@id` needs its own unprefixed lookup).
  The existing path/structured paths consume only the `r:id` field, so their
  output and warning stream stay byte-identical — including the case of a
  `p:sldId` that has a valid `r:id` but **no** `@id`, which text/structured must
  keep extracting exactly as today.
- `slide_id` parsing rules: `@id` must parse as `u32`. A `p:sldId` without a
  parsable `@id` is skipped **by the slides contract only** with the new warning
  `ignored presentation slide without slide id` (path `ppt/presentation.xml`);
  existing paths are unaffected.
- `slide_ordinal` is the 1-based position of the `p:sldId` element in
  `p:sldIdLst`, so ordinals across the presentation are always `1..=N`. When a
  slide is skipped, the remaining records keep their presentation positions and
  the emitted set has gaps; this is documented and asserted by a test.
- Per-slide failure handling (this contract only):
  - slide `r:id` absent from `ppt/_rels/presentation.xml.rels` → warning
    `skipped PPTX slide {rid}: unknown relationship id` (path
    `ppt/presentation.xml`), slide skipped, extraction continues;
  - a missing `ppt/_rels/presentation.xml.rels` part is treated as an empty
    relationship map (no warning of its own, mirroring the DOCX "missing rels
    file yields no warning" rule), so every slide reference degrades into the
    warning above instead of aborting the document;
  - resolved slide target absent from the package → warning
    `skipped related PPTX slide part {path}: missing part` (path = the absent
    part), slide skipped; wording deliberately mirrors the DOCX
    `skipped related DOCX text part {path}: missing part`;
  - resolved notes target absent from the package → warning
    `skipped related PPTX notes part {path}: missing part` (path = the absent
    part), the slide record is still emitted with `notes` omitted;
  - malformed XML **inside a readable target** keeps the existing recoverable
    behavior: partial text plus the existing `W001 malformed_xml` warning, and
    the slide record is still emitted (never silently dropped). Malformed
    `ppt/presentation.xml` keeps its current `W001` + partial-id-list behavior.
  - a malformed or missing `ppt/presentation.xml` itself stays a hard error:
    without the main part there is nothing to enumerate.
- Security and determinism invariants are preserved: `SuspiciousRelationshipTarget`
  (external target mode, escaping target, NUL) stays a **hard error** in the
  slides path too — skip-with-warning never downgrades it. Notes relationships
  keep their existing rel-id sort order, and `slide_id`/`slide_ordinal` ordering
  guarantees deterministic output for identical input.
- Existing PPTX text extraction (`extract_pptx_text`) and structured extraction
  keep the hard `MissingPart` behavior for dangling/absent targets: the new
  leniency is scoped to the new contract, and a regression test asserts the old
  path still fails on the new missing-target fixture.

### 3. CLI: new `extract slides` subcommand

Parent-resolved shape: a new subcommand, **not** new `TextFormat` variants.

```bash
oxdoc extract slides deck.pptx --format json    # single document payload (default)
oxdoc extract slides deck.pptx --format jsonl   # one record per slide, streamed
oxdoc extract slides - --format jsonl           # stdin, same as `extract rows`
```

- `ExtractCommand::Slides { file: PathBuf, format: SlidesFormat }` with
  `#[derive(ValueEnum)] enum SlidesFormat { Json, Jsonl }`, default `Json`;
  single positional file (accepts `-` for stdin), matching `extract tables` /
  `extract rows`.
- `extract slides` rejects DOCX and XLSX with the same style of
  `CliError::InvalidArgument` message used by the neighbouring type-restricted
  commands; no `--output`/`--output-dir` in this change (matches `extract
  tables`/`extract rows`; writing streaming output to a file is deferred).
- JSON payload (single document, pretty-printed, trailing newline):

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
    },
    {
      "slide_id": 257,
      "slide_ordinal": 2,
      "slide_path": "ppt/slides/slide1.xml",
      "text": "Second Slide\n"
    }
  ],
  "warnings": []
}
```

- JSONL payload (compact, one line per slide, no wrapping array):

```json
{"schema_version":1,"file":"deck.pptx","slide_id":256,"slide_ordinal":1,"slide_path":"ppt/slides/slide2.xml","text":"First Slide\n","notes":"Speaker note\n"}
{"schema_version":1,"file":"deck.pptx","slide_id":257,"slide_ordinal":2,"slide_path":"ppt/slides/slide1.xml","text":"Second Slide\n"}
```

- One record per **non-skipped** slide, in `p:sldIdLst` order. A textless slide
  emits `"text": ""` (the record set stays aligned with the presentation);
  `notes` is omitted when the slide has no notes relationship, and is present
  (possibly `""`) when it has one that was read successfully.
- Warning channels (parent decision 3 + repo precedent): **JSONL warnings go to
  stderr only** (rows-jsonl precedent) so stdout stays a valid record stream and
  there is no error-line contract; **JSON embeds a top-level `warnings` array**
  (DOCX tables / audit precedent) *and* mirrors them to stderr subject to the
  global `--warnings`/`--quiet` flags. Embedded warnings are never suppressed by
  `--quiet`. `warnings` is `[]`, never omitted, in the JSON payload.
- A deck whose slides are all skipped still emits a valid JSON payload with
  `"slides": []` and the warnings; it is **not** the
  "no input files were processed successfully" error (the document was processed).
- Skipped slides are simply absent from the JSONL stream; consumers detect them
  from the stderr warning channel or, in JSON, from `warnings`.

### 4. Versioned schema: a new v1 contract, not a structured-text bump

- New file `schemas/v1/oxdoc-pptx-slides.schema.json` + exact mirror
  `docs/schemas/v1/oxdoc-pptx-slides.schema.json` (`make docs-schemas-check`
  already diffs `schemas/v1` ↔ `docs/schemas/v1`, so the mirror is auto-covered).
- JSON Schema draft 2020-12, stable `$id`
  `https://github.com/spereyra-dev/oxdoc/schemas/v1/oxdoc-pptx-slides.schema.json`,
  `additionalProperties: false`, top-level `oneOf`:
  - **document payload**: requires `schema_version` (`const: 1`), `file`,
    `document_type` (`const: "pptx"`), `slides`, `warnings`;
  - **JSONL record**: requires `schema_version` (`const: 1`), `file`,
    `slide_id`, `slide_ordinal`, `slide_path`, `text`, optional `notes`.
  Both shapes share a `$defs.slide` for the `slide_id` / `slide_ordinal` /
  `slide_path` / `text` / optional `notes` object, so JSON and JSONL cannot drift.
- `slide_id` is an integer (OOXML `p:sldId/@id` is `xsd:unsignedInt`),
  `slide_ordinal` an integer with `minimum: 1`, `slide_path`/`text`/`notes` strings.
- **Version policy:** this is a *new* contract, so it starts at its own
  `schema_version: 1`. `schemas/v1/oxdoc-structured-text.schema.json` and
  `schemas/v2/oxdoc-structured-text.schema.json` are **not modified**; v2
  structured-text stays frozen and no new block field is introduced there.
- `crates/oxdoc-core/tests/schema.rs`: add the schema to the v1
  `SCHEMA_VERSIONS` list and add validation tests for a representative JSON
  payload and a representative JSONL record (snapshots below).

### 5. Fixtures, snapshots, provenance

- Reuse without modification: `tests/fixtures/corpus/pptx/basic/` (one slide, no
  notes → "no notes" case) and `tests/fixtures/corpus/pptx/text/` (two slides in
  `sldIdLst` order `rId2`, `rId1`; slide 2 links `notesSlide2.xml` → notes case
  **and** order-stability case, where slide2 is emitted first because of
  `sldIdLst` order despite its later file name).
- New corpus source trees (hand-authored, runtime-zipped by
  `fixtures::build_package`, no binary checked in):
  - `tests/fixtures/corpus/pptx/missing-target/` — three slides where one slide's
    `r:id` is dangling, one slide's notes target part is absent, and one slide is
    intact. Exercises per-slide skip-with-warning **and** pins that
    `extract_pptx_text` still fails with `MissingPart` on the same package
    (existing-behavior regression).
  - `tests/fixtures/corpus/pptx/malformed-xml/` — two slides where the second
    slide part is truncated mid-XML, so the record keeps recovered partial text
    plus `W001`. Promotes today's inline `create_ooxml` malformed fixture into a
    durable corpus package (the existing inline test stays as-is).
- New provenance notes `tests/fixtures/provenance/pptx-missing-target.md` and
  `tests/fixtures/provenance/pptx-malformed-xml.md`, following the
  `docx-section-order.md` label set (`Source:`, `Producer:`, `Redistribution:`,
  `Purpose:`, plus `Archive generation:`/`Sanitization:`), and add both names to
  the `fixture_provenance_notes_are_present` lists in
  `crates/oxdoc-core/tests/api.rs` and `crates/oxdoc-cli/tests/cli.rs`.
- `make compatibility-corpus-check` must stay green: `tests/fixtures/compatibility-matrix.json`
  only tracks checked-in binary fixtures under `tests/fixtures/files/` with a
  `sha256`, so runtime-zipped corpus trees need no manifest entry — the change
  adds none, and the new provenance notes satisfy the label contract for any
  future manifest entry. No existing digest is touched (no existing corpus tree
  is edited).
- New snapshots: `tests/fixtures/snapshots/cli_pptx_slides_json.json` (from
  `corpus/pptx/text`) and `tests/fixtures/snapshots/cli_pptx_slides_jsonl.jsonl`,
  both byte-compared by CLI tests and validated by `schema.rs`.

### 6. Acceptance-criteria mapping

| Issue #180 acceptance criterion | How this proposal satisfies it |
| --- | --- |
| Slide order and stable identifiers/ordinals, one record per slide | `p:sldId` elements are walked in `p:sldIdLst` order; each record carries `slide_id` (`p:sldId/@id`) and `slide_ordinal` (1-based `sldIdLst` position); the existing inverted fixture (`corpus/pptx/text`, `sldIdLst` = `rId2`, `rId1`) locks order stability. |
| JSON + JSONL contracts without breaking existing text/structured output | New `extract slides --format json\|jsonl` plus the new `schemas/v1/oxdoc-pptx-slides.schema.json` contract; `extract text` (text/json/jsonl/structured-json), `extract tables`, plain text, and all v1/v2 schemas are untouched and snapshot-locked. |
| Body text and speaker notes as separate per-slide fields | Each record has `text` (slide body) and optional `notes` (speaker notes from `ppt/notesSlides/notesSlideN.xml`), replacing positional block reconstruction. |
| Documented warning behavior, including missing targets | Exact warning texts are locked by requirement scenarios; malformed/missing per-slide targets become skip-with-warning in the new contract only; warnings are stderr-only for JSONL and both embedded and stderr-emitted for JSON; existing PPTX text extraction keeps its hard `MissingPart` behavior. |
| Versioned schema, no silent widening | Own contract at `schema_version: 1`, `additionalProperties: false`, `$id`, docs mirror, docs table row; structured-text v2 and every `schemas/v1/**` structured-text file stay frozen. |
| Fixtures for no-notes, notes, missing targets, malformed XML | `corpus/pptx/basic` (no notes) and `corpus/pptx/text` (notes, inverted order) reused; new `corpus/pptx/missing-target` and `corpus/pptx/malformed-xml` trees with provenance notes; compatibility-corpus check stays green. |
| Documentation | `docs/json-output.md` (schema row + slide semantics + JSONL contract), `docs/cli.md`, `docs/formats/pptx.md`, `README.md` command examples, `CHANGELOG.md`. |

## Backwards compatibility

- **Existing outputs are byte-identical.** No change to `extract text` in any
  format (text/json/jsonl/structured-json), `extract tables`, `extract rows`,
  `extract csv`, `info`, or `audit`. Every existing snapshot
  (`pptx_text.txt`, `cli_structured_text_pptx_json.json`,
  `cli_structured_text_json.json`, `pptx_basic_info.json`, tables/rows
  snapshots) stays unchanged, and the PR must not touch them.
- **Existing error behavior preserved.** `extract_pptx_text` /
  `extract_pptx_structured_text` keep the hard `MissingPart` failure for a
  dangling slide `r:id` or an absent notes part. The new skip-with-warning is
  scoped to the `extract slides` contract and is documented as an intentional
  asymmetry (a new test asserts the old path still errors on the new
  missing-target fixture).
- **Schemas.** `schemas/v1/**` and `schemas/v2/oxdoc-structured-text.schema.json`
  are unmodified; the change *adds* one v1 file, so nothing that validated before
  stops validating. v1 consumers of structured-json see no change at all.
- **Rust API is additive.** New public type `PptxSlideText`, three new
  functions, no signature changes to existing functions; `TextBlock`,
  `StructuredText`, `DocxTables`, `DocxTextOptions`, and `Extraction<T>` are
  unchanged. The only internal refactor (a richer shared slide-reference parse)
  is output-neutral for existing paths and is pinned by their existing tests.
- **CLI is additive.** A new subcommand; no existing flag, default, or output
  changes. New output is only produced when `extract slides` is invoked.

## Scope / affected areas

Production:

- `crates/oxdoc-core/src/models.rs` — `PptxSlideText` (+ serde derives,
  `skip_serializing_if` for `notes`).
- `crates/oxdoc-core/src/parsers/pptx.rs` — slide-reference struct with `@id` +
  `r:id`, shared parse refactor, `extract_slides` (skip-with-warning loop, notes
  resolution), new warning texts; existing entry points read only `r:id`.
- `crates/oxdoc-core/src/lib.rs` — `extract_pptx_slides[_from_reader[_with_limits]]`
  + `PptxSlideText` re-export.
- `crates/oxdoc-cli/src/main.rs` — `ExtractCommand::Slides`, `SlidesFormat`,
  `extract_slides_command`, `SlidesJsonPayload` / `SlidesJsonlRecord`, dispatch.
- `schemas/v1/oxdoc-pptx-slides.schema.json` + `docs/schemas/v1/…` (new, mirrored).

Tests / fixtures:

- `crates/oxdoc-core/tests/api.rs` — per-slide API tests (order, ids/ordinals,
  notes/no-notes, skip-with-warning, gaps, malformed partial text, stdin/reader
  path, suspicious-target still hard-error, existing text path still `MissingPart`),
  provenance list update.
- `crates/oxdoc-cli/tests/cli.rs` — `extract slides` json/jsonl tests, snapshot
  byte-comparison, stderr-only JSONL warning assertion, JSON embedded `warnings`,
  stdin `-`, type rejection (docx/xlsx), provenance list update.
- `crates/oxdoc-core/tests/schema.rs` — schema registration + validation of the
  JSON payload and JSONL record snapshots.
- `tests/fixtures/corpus/pptx/missing-target/**`,
  `tests/fixtures/corpus/pptx/malformed-xml/**`, provenance notes, snapshots
  `cli_pptx_slides_json.json` / `cli_pptx_slides_jsonl.jsonl`.

Docs:

- `docs/json-output.md` — schema table row, slide payload/JSONL semantics,
  warning-channel statement, version-policy note ("new contract, not a
  structured-text widen").
- `docs/cli.md` — `extract slides` usage, example, ordinal/skip semantics.
- `docs/formats/pptx.md` — new "Slide-scoped JSON / JSONL" section.
- `README.md` — command example alongside the existing tables/rows examples.
- `CHANGELOG.md` — new subcommand + new schema contract entry.

## Non-goals (explicit)

1. **Any change to `--format structured-json`** or to
   `schemas/v2/oxdoc-structured-text.schema.json`; no new `TextBlock` field, no
   slide-scoped ordinal inside structured-json.
2. **Any change to `TextBlock`, `StructuredText`, `DocxTextOptions`,
   `DocxTable`, or `DocxTables`.**
3. **Any change to plain-text or existing JSON output.** `extract text --format
   text|json|jsonl|structured-json`, `extract tables`, `extract rows`,
   `extract csv`, `info`, `audit` stay byte-identical; the PPTX text path keeps
   its hard `MissingPart` behavior.
4. **DOCX paths, XLSX paths, `oxdoc-tabular`, and the Python wrapper** are out
   of scope (the Python package may later surface the new API, but not here).
5. **No recalculation, no rendering, no slide thumbnails, no media/chart/embedded
   object extraction**, no shape geometry, no fonts, colors, bullets, numbering,
   or timing synthesis.
6. **No per-paragraph or per-shape granularity** for slides; the record
   granularity is one slide.
7. **No `--output`/`--output-dir`, no multi-input batch, no per-line error-line
   contract** for `extract slides` in this change (deferred, not rejected).
8. **No new dependency**, no changes to limits/security policy, no
   `size:exception` and no chain strategy assumed by this proposal.

## Alternatives considered (rejected)

1. **New `TextFormat` variants (`slides-json`, `slides-jsonl`) on `extract text`
   instead of a subcommand** (exploration §4 option B). Rejected — the parent
   resolved the shape: `extract text` carries multi-file semantics, an
   array-of-payloads JSON output, DOCX option flags, and a `-`/stdin batch
   contract that do not apply to a single-document, per-slide stream. Adding
   variants would make `TextFormat` values mean different document-type and
   multiplicity rules, and would force the "multi-input slides" question
   (`slides` array per file? which ordinal scope?) into an existing, frozen CLI
   surface. The `extract rows` precedent — one file, streaming records,
   type-restricted — is the closer match.
2. **Embed warnings in every JSONL record (text-jsonl / audit-jsonl precedent)
   instead of stderr-only.** Rejected — the parent resolved stderr-only for the
   new contract; embedding per-record warnings duplicates identical skip
   information up to N-1 times, blurs the record schema with operational
   metadata, and no error-line contract is needed because skips are
   non-fatal. Rows-jsonl already establishes the stderr rule for a
   record-per-unit stream inside one document.
3. **Stderr-only warnings for the JSON payload as well (fully symmetric
   channels).** Rejected as the default — DOCX tables JSON and audit JSON embed
   recoverable warnings so a single-document consumer does not have to parse
   stderr, and skipped slides are data-shaped information about the payload
   (`slides` is short). JSON therefore embeds `warnings` like tables; the
   documented asymmetry with JSONL follows existing precedent instead of
   inventing a new rule.
4. **Widen structured-text to v3 with slide-scoped ordinals and a `notes`
   pairing.** Rejected — the frozen-v2 spec and #179 explicitly deferred
   slide scoping to a *new* contract; a v3 would invalidate every existing
   structured-json snapshot and consumer for a capability that only PPTX
   needs, and would put two unrelated concerns (DOCX provenance, PPTX slide
   scoping) behind one version marker.
5. **Emit `slide_id` as a string.** Rejected — OOXML types `@id` as
   `xsd:unsignedInt`, so an integer preserves the source type, sorts
   numerically in `jq`, and avoids inventing a formatting rule. Consumers that
   need a string key can stringify it.
6. **Let a `p:sldId` without `@id` emit a record with a fallback identifier**
   (e.g. reuse the ordinal, or emit `slide_id: null`). Rejected — a fallback
   silently overloads two different identity concepts in one field, and a
   nullable required field pushes `null` handling into every consumer. Skipping
   with `ignored presentation slide without slide id` mirrors the existing
   sibling rule (`ignored presentation slide without relationship id`) and keeps
   the field contract crisp. Existing text/structured paths still extract such a
   slide, so no regression.
7. **Keep the hard `MissingPart` error and document it for the new contract
   too.** Rejected — it defeats the AC's missing-target case and makes one bad
   slide reference destroy a whole deck's extraction; skip-with-warning is the
   documented granularity, scoped strictly to the new contract.
8. **Downgrade `SuspiciousRelationshipTarget` to a warning for the slides
   path.** Rejected — external/escaping/NUL targets are a security boundary in
   every existing path; per-slide leniency must not weaken it.
9. **Make `slide_ordinal` contiguous over *emitted* records instead of
   presentation position.** Rejected — an ordinal that shifts when a slide is
   skipped is not a stable identity, and it would disagree with the parent
   decision that ordinals are per-presentation contiguous `1..=N`.
10. **Copy the existing corpus trees into new slide-specific fixtures.**
    Rejected — duplicates hand-authored OOXML, multiplies digest/provenance
    maintenance, and loses the "reuse the existing fixtures" instruction;
    reuse also proves the inverted-order deck through the new contract.

## Risks

- **Review budget (highest).** Honest sizing puts this change well over 400
  lines (see estimate), driven by two schema copies, two new corpus trees, and
  two snapshots. Mitigation: delivery is `ask-on-risk`; the apply phase must
  pause for a delivery decision before producing an oversized diff. Recommended
  split into four reviewable work units (below). No chain strategy or
  `size:exception` is assumed by this proposal.
- **Shared parser refactor leaking into existing outputs.** Widening the shared
  slide-reference parse touches the text and structured paths. Mitigation: the
  refactor keeps `r:id` handling identical, existing tests and snapshots stay
  untouched, and a dedicated test pins the "`r:id` without `@id` still extracts
  in text mode" case that a naive refactor would break.
- **Skip-with-warning could mask real regressions.** A deck that silently loses
  slides is worse than a failure if the warnings are not seen. Mitigation: JSON
  embeds `warnings`; JSONL documents the stderr channel; warning texts are
  spec-locked; the missing-target fixture asserts warnings for both the dangling
  rel and the absent notes part.
- **Ordinal ambiguity ("contiguous" read two ways).** Mitigation: the schema
  description, `docs/json-output.md`, `docs/cli.md`, and `docs/formats/pptx.md`
  all state "1-based position in `p:sldIdLst`; skipped slides leave gaps", with a
  test asserting a gap.
- **Malformed-XML nuance vs. the skip wording.** The parent decision speaks of
  malformed targets becoming skip-with-warning; this proposal keeps the existing
  recoverable-partial-text + `W001` behavior for malformed XML *inside* a
  readable part (and still emits the record) because dropping already-recoverable
  text would be a regression in information. This is called out in the question
  round for confirmation.
- **Fixture provenance / corpus gate.** New corpus trees need provenance notes
  and must not disturb `make compatibility-corpus-check` or existing digests.
  Mitigation: no existing fixture or manifest entry is edited; new trees are
  runtime-zipped source only.
- **Schema/test drift between JSON and JSONL.** Two shapes in one schema file
  can diverge. Mitigation: a shared `$defs.slide`, plus `schema.rs` validating
  both snapshots.
- **Coverage gate (95%).** New parser branches (skip paths, notes-missing,
  malformed) must all be exercised. Mitigation: the missing-target and
  malformed-xml fixtures exist precisely to cover those branches in integration
  tests, not only in unit tests.

## Rollback

Delete the new subcommand and payload types from `oxdoc-cli`, the
`extract_pptx_slides*` functions and `PptxSlideText` from `oxdoc-core` (and
revert the shared slide-reference refactor to the `r:id`-only helper), delete
`schemas/v1/oxdoc-pptx-slides.schema.json` plus its `docs/schemas/v1` mirror, the
two new corpus trees and provenance notes, the two new snapshots, and the
`json-output.md` / `cli.md` / `formats/pptx.md` / `README.md` / `CHANGELOG.md`
edits. No existing contract, snapshot, digest, or public signature is modified by
this change, so rollback restores the previous state exactly; nothing validates
against the new schema before it is published, so no captured output becomes
invalid. No data migration and no persisted state.

## Size estimate (vs 400-line review budget)

Raw line counts, then an honest **1.5–2× multiplier on test/fixture weight** —
the post-#177 lesson: hand-authored fixtures, snapshots, and schema-validation
tests reliably come in far above their first estimate.

| Work unit | Content | Raw lines | Realistic (÷ multiplier) |
| --- | --- | --- | --- |
| WU1 — fixtures + provenance | `corpus/pptx/missing-target` + `corpus/pptx/malformed-xml` trees, 2 provenance notes, provenance-list tests | ~110 | **~165–220** |
| WU2 — core model + parser + API | `PptxSlideText`, shared slide-reference parse with `@id`, `extract_slides` skip/warn loop, notes resolution, 3 lib wrappers, core API tests | ~230 (+tests raw ~130) | **~425–490** |
| WU3 — schema + validation | `schemas/v1/oxdoc-pptx-slides.schema.json` + `docs/schemas/v1` mirror, `schema.rs` registration/validation, 2 snapshots | ~300 | **~330–400** |
| WU4 — CLI + docs | `extract slides` subcommand, payload/record structs, JSON/JSONL emission, CLI tests, `json-output.md`, `cli.md`, `formats/pptx.md`, `README.md`, `CHANGELOG.md` | ~300 (+tests raw ~120) | **~390–480** |
| **Total** | | **~1,070** | **~1,300–1,600** |

The estimate is **~3× the 400-line budget**, so this change cannot ship as one
PR without an explicit exception. Because the delivery strategy is
`ask-on-risk`, the apply phase must **pause and ask the user for a delivery
decision** — this proposal recommends chaining, with these review slices (each
in dependency order, each independently reviewable and under 400 lines *after*
the multiplier is applied only if the user accepts tighter slices):

- **WU1 — fixtures and provenance** (no production code): new corpus trees +
  provenance + list updates. Unblocks all later tests.
- **WU2 — core per-slide API**: model, parser, lib entry points, core tests
  (including the "existing text path still hard-errors" regression).
- **WU3 — schema contract**: schema + mirror + `schema.rs` + snapshots
  (validated against WU2's real output).
- **WU4 — CLI subcommand + docs**: `extract slides`, CLI tests, docs,
  CHANGELOG.

Honesty note: WU2 and WU4 sit at or just above 400 realistic lines, so the
delivery decision may be to split further (schema vs. snapshots; CLI vs. docs)
or to accept a `size:exception` — either way that is a user/parent decision, not
an assumption of this proposal. The 95% coverage gate is unaffected in principle;
every new production branch lands with its test in the same work unit.

## Success criteria

1. `oxdoc extract slides <FILE> --format json` emits the versioned document
   payload with `schema_version: 1`, `file`, `document_type: "pptx"`,
   `slides`, and `warnings`, in `p:sldIdLst` order, one object per non-skipped
   slide carrying `slide_id`, `slide_ordinal`, `slide_path`, `text`, and
   `notes` (omitted when the slide links no notes part).
2. `oxdoc extract slides <FILE> --format jsonl` emits exactly one compact JSON
   object per line for the same slides in the same order, with warnings on
   stderr only so stdout is a valid JSONL stream; `-` (stdin) works.
3. `slide_id` equals `p:sldId/@id` and `slide_ordinal` is the 1-based
   `p:sldIdLst` position; both are asserted on `corpus/pptx/text` (inverted
   order: slide2 first, `slide_id` 256/257) and on `corpus/pptx/basic` (no notes).
4. Missing and malformed per-slide targets produce the exact documented warnings
   and skip only the affected slide: `corpus/pptx/missing-target` yields the
   dangling-`r:id` and missing-notes-part warnings with the remaining slides
   intact, and `corpus/pptx/malformed-xml` yields recovered partial text plus
   `W001` for the truncated slide.
5. `extract_pptx_text` and `extract_pptx_structured_text` still fail with
   `MissingPart` on the missing-target fixture, proving the leniency is scoped to
   the new contract; `SuspiciousRelationshipTarget` remains a hard error.
6. All pre-existing snapshots and outputs are byte-identical: `pptx_text.txt`,
   `cli_structured_text_pptx_json.json`, `cli_structured_text_json.json`,
   `pptx_basic_info.json`, DOCX tables, XLSX rows, audit, info, and plain text.
7. `schemas/v1/oxdoc-pptx-slides.schema.json` and its `docs/schemas/v1` mirror
   exist and are identical (`make docs-schemas-check`), validate both new
   snapshots through `crates/oxdoc-core/tests/schema.rs`, and are linked from
   `docs/json-output.md`; no other schema file (v1 or v2) is modified.
8. `docs/json-output.md`, `docs/cli.md`, `docs/formats/pptx.md`, `README.md`, and
   `CHANGELOG.md` document the subcommand, the payload/record shapes, the ordinal
   rule (including gaps after skips), the notes-field rule, and the warning
   channels.
9. `make compatibility-corpus-check` passes with the new provenance notes and no
   digest changes.
10. `cargo test --workspace`, `cargo fmt --all -- --check`,
    `cargo clippy --workspace --all-targets -- -D warnings`, and the 95%
    line-coverage gate pass.
11. Realized changed lines stay within 400 lines per PR, **or** the apply phase
    explicitly asked for a delivery decision (`ask-on-risk`) before producing an
    oversized or multi-area diff.

## Parent decision traceability

| Parent decision | Where honored |
| --- | --- |
| 1. `p:sldId/@id` as stable `slide_id` + 1-based `slide_ordinal` from `p:sldIdLst`; ordinals per-presentation 1-based contiguous | Solution shape §1–§2; Alternatives 5, 6, 9; Success criteria 3; question-round assumptions on `@id`-less slides and gaps |
| 2. New `extract slides` subcommand with `--format json` (single doc) / `--format jsonl` (one record per slide); `extract text`/`structured-json` untouched | Solution shape §3; Alternatives 1; Non-goals 1–3; Success criteria 1, 2, 6 |
| 3. JSONL warnings stderr-only; per-slide body/notes fields; malformed/missing per-slide targets skip-with-warning per slide (new contract only); existing PPTX text behavior byte-identical with its `MissingPart` hard error | Solution shape §2–§3; Alternatives 2, 3, 7; Risks (malformed nuance); Success criteria 4, 5 |
| 4. New contract `schemas/v1/oxdoc-pptx-slides.schema.json` + `docs/schemas/v1` mirror; payload's own `schema_version: 1`; structured-text v2 frozen | Solution shape §4; Backwards compatibility; Alternatives 4; Success criteria 1, 7 |
| 5. Reuse `corpus/pptx/basic` and `corpus/pptx/text`; add missing-target and malformed-XML fixtures with provenance and compatibility-corpus-check updates | Solution shape §5; Alternatives 10; Success criteria 4, 9 |
| 6. Non-goals: structured-json, `TextBlock`, plain text, DOCX paths, XLSX, recalculation, thumbnails/media | Non-goals 1–5; Backwards compatibility; Success criterion 6 |

## Proposal question round

The product decisions above are parent-resolved and treated as authoritative;
this section does not re-open them. These are the **remaining refinements and
assumptions** that the spec phase should not silently bake in — they can be
answered, corrected, skipped, or followed by a second round:

1. **Malformed-XML wording vs. behavior.** Parent decision 3 groups "malformed
   or missing per-slide targets" as skip-with-warning. This proposal keeps the
   existing recoverable behavior for malformed XML *inside* a readable part
   (partial text + `W001`, record still emitted) and reserves "skip" for targets
   that cannot be read at all (dangling `r:id`, absent slide/notes part, absent
   presentation rels). Confirm, or should a truncated slide part also produce no
   record (dropping the recoverable partial text)?
2. **`p:sldId` without `@id`.** The slides contract skips it with
   `ignored presentation slide without slide id`, while text/structured keep
   extracting it (so nothing existing changes). Alternative: emit the record with
   `slide_id` omitted and `slide_ordinal` as the only key. Which identity rule
   should the schema lock?
3. **Ordinal gaps after a skip.** `slide_ordinal` stays the presentation position
   (`1..=N` over all `p:sldId` entries), so skipped slides leave gaps in the
   emitted sequence. Confirm that consumers should see
   `1, 3` rather than a renumbered `1, 2`.
4. **JSON warning channel.** JSON embeds a top-level `warnings` array (tables
   precedent) *in addition to* stderr, while JSONL is stderr-only as decided.
   Confirm the asymmetry, or should JSON also be stderr-only (smaller payload,
   silent skips to a single-document consumer)?
5. **Textless slides emit `"text": ""`.** Alternative: omit such slides from the
   stream entirely, or emit records only for slides with non-empty body text
   (matching the structured path's "non-empty block" rule). Confirm the
   record-per-slide reading of "one record per slide".
6. **`notes` presence rule.** `notes` is omitted when the slide links no notes
   part, but present (possibly `""`) when a notes part was read successfully.
   Alternative: omit whenever the extracted notes text is empty. Confirm.
7. **No `--output` / no multi-input for `extract slides`** in this change
   (matching `extract tables`/`extract rows`). Confirm that deferring them is
   acceptable, or whether `--output` should land now.
