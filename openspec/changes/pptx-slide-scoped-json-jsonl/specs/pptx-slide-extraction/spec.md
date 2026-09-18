# PPTX Slide Extraction Specification

## Purpose

Define the slide-scoped PPTX contract introduced by change
`pptx-slide-scoped-json-jsonl` (GitHub issue #180): one record per slide with
stable slide identity (`p:sldId/@id` + 1-based `p:sldIdLst` position), body
text and speaker notes as separate fields, JSON and JSONL emission through a
new `oxdoc extract slides` subcommand, a new versioned JSON Schema contract
(`schema_version: 1`), per-slide skip-with-warning failure granularity, and the
fixtures/docs that lock the contract. The frozen structured-text v2 contract
and all existing PPTX output are explicitly out of this contract's blast
radius: per-slide scoping is delivered by this NEW domain, not by editing
`structured-json` semantics (satisfying the deferral in the canonical
`structured-text-schema` spec without modifying it).

## Requirements

### Requirement: Slide identity from p:sldId

The system MUST identify each extracted slide by `slide_id`, the value of the
`p:sldId/@id` attribute (OOXML `xsd:unsignedInt`, emitted as a JSON integer).
When a `p:sldId` element has no `@id` attribute (or the attribute is not
parsable as an unsigned integer), the system MUST still extract that slide and
MUST omit the `slide_id` key from the emitted record; the record remains a
first-class slide identified by `slide_ordinal` and `slide_path`. Existing
text and structured extraction paths MUST continue to extract such a slide
exactly as they do today (unchanged behavior; no new warning on any path for
an `@id`-less `p:sldId`).

#### Scenario: Stable slide ids in presentation order

- GIVEN the `corpus/pptx/text` fixture whose `p:sldIdLst` lists the `r:id`
  values in the order `rId2`, `rId1` with `@id` values `256` and `257`
- WHEN slide extraction runs
- THEN the first emitted record carries `slide_id` `256` and the second
  carries `slide_id` `257`, independent of slide part file names.

#### Scenario: p:sldId without @id still extracts

- GIVEN a presentation whose `p:sldIdLst` contains a `p:sldId` with a valid
  `r:id` but no `@id` attribute
- WHEN slide extraction runs
- THEN a record for that slide is emitted with the `slide_id` key omitted,
  `slide_ordinal` and `slide_path` present, and no warning is emitted for the
  missing `@id`; text and structured extraction of the same slide are
  unchanged.

### Requirement: 1-based slide_ordinal in p:sldIdLst order

The system MUST emit slides in `p:sldIdLst` presentation order and MUST set
`slide_ordinal` to the 1-based position of the `p:sldId` element within
`p:sldIdLst` — not a renumbered index over emitted records and not a global
block ordinal. When a slide is skipped (per the skip-with-warning rules), the
remaining records MUST keep their presentation positions, so the emitted
ordinal sequence MAY contain gaps (for example `1, 3`). This semantics MUST be
stated in the schema description, `docs/json-output.md`, `docs/cli.md`, and
`docs/formats/pptx.md`. Output MUST be deterministic for identical input:
slide records are ordered by `p:sldIdLst` position and notes relationships
keep their existing rel-id sort order.

#### Scenario: Inverted file-name order follows sldIdLst

- GIVEN the `corpus/pptx/text` fixture where `sldIdLst` lists `rId2` (slide
  part `slide2.xml`) before `rId1` (slide part `slide1.xml`)
- WHEN slide extraction runs
- THEN the first record has `slide_ordinal` `1` and `slide_path`
  `ppt/slides/slide2.xml`, and the second record has `slide_ordinal` `2` and
  `slide_path` `ppt/slides/slide1.xml`.

#### Scenario: Ordinal gaps after a skip

- GIVEN a presentation with three `p:sldId` entries where the second slide is
  skipped for a missing target
- WHEN slide extraction runs
- THEN the emitted records carry `slide_ordinal` values `1` and `3` (not
  renumbered `1` and `2`).

### Requirement: slide_path provenance

Every emitted slide record MUST carry `slide_path`, the resolved package part
path of the slide (for example `ppt/slides/slide2.xml`), so consumers never
have to reverse-engineer part-name conventions.

#### Scenario: Slide path matches the resolved part

- GIVEN any PPTX package processed by slide extraction
- WHEN a slide record is emitted
- THEN `slide_path` is the package path of the slide part whose text was
  extracted, identical to the `part_path` the structured path would report for
  that slide's text.

### Requirement: One record per slide with separate body text and notes

The system MUST emit exactly one record per non-skipped slide, carrying:

- `slide_id` (integer; key omitted when `p:sldId/@id` is absent/unparsable),
- `slide_ordinal` (integer, minimum `1`),
- `slide_path` (string),
- `text` (string; slide body text, possibly `""` for a textless slide — the
  record set stays aligned with the presentation), and
- `notes` (optional string; speaker notes from the slide's
  `ppt/notesSlides/notesSlideN.xml` part).

Body text and speaker notes MUST be fields of the same slide record, replacing
any positional reconstruction across separate blocks. `notes` MUST be omitted
when the slide has no notes relationship, when the notes target part is
missing (skip-with-warning below), or when the slide's `.rels` file is absent;
`notes` MUST be present (possibly `""`) when a notes part was read
successfully. The core model MUST NOT add fields to `TextBlock` or
`StructuredText`; the slide model is additive and unversioned at the library
level (the version marker is a CLI emission concern).

#### Scenario: Notes and no-notes side by side

- GIVEN slide extraction over `corpus/pptx/text` (slide with notes) and
  `corpus/pptx/basic` (slide without notes)
- WHEN records are emitted
- THEN the notes-bearing slide record carries a `notes` string and the
  no-notes slide record has no `notes` key; both carry non-empty `text`.

#### Scenario: Textless slide still emits a record

- GIVEN a slide whose body contains no extractable text
- WHEN slide extraction runs
- THEN the record is emitted with `"text": ""` and the slide is not dropped
  from the record set.

### Requirement: Per-slide skip-with-warning for missing targets

Missing per-slide targets MUST degrade to a per-slide skip with an exact
warning, never abort the whole extraction. The system MUST emit the warning
`skipped PPTX slide {rid}: unknown relationship id` (warning path
`ppt/presentation.xml`) when a slide `r:id` is absent from
`ppt/_rels/presentation.xml.rels`, and skip only that slide. A missing
`ppt/_rels/presentation.xml.rels` part MUST be treated as an empty
relationship map with no warning of its own (mirroring the DOCX rule), so
every slide reference degrades into the warning above. The system MUST emit
`skipped related PPTX slide part {path}: missing part` (warning path = the
absent part) when a resolved slide target is absent from the package, and skip
only that slide. The system MUST emit `skipped related PPTX notes part {path}:
missing part` (warning path = the absent part) when a resolved notes target is
absent, and MUST still emit that slide's record with `notes` omitted.
Extraction MUST continue after any of these warnings.

#### Scenario: Dangling slide relationship id

- GIVEN the `corpus/pptx/missing-target` fixture with a slide `r:id` absent
  from the presentation rels
- WHEN slide extraction runs
- THEN the warning `skipped PPTX slide {rid}: unknown relationship id` is
  emitted, the affected slide is absent from the records, and the intact
  slides are extracted.

#### Scenario: Missing slide part target

- GIVEN a slide relationship that resolves to a part absent from the package
- WHEN slide extraction runs
- THEN the warning `skipped related PPTX slide part {path}: missing part` is
  emitted with the absent part's path, the slide is skipped, and other slides
  are extracted.

#### Scenario: Missing notes part target

- GIVEN a notes relationship that resolves to a part absent from the package
- WHEN slide extraction runs
- THEN the warning `skipped related PPTX notes part {path}: missing part` is
  emitted and the slide record is still emitted with the `notes` key omitted.

#### Scenario: Missing presentation rels file

- GIVEN a package with no `ppt/_rels/presentation.xml.rels` part
- WHEN slide extraction runs
- THEN each slide `r:id` produces only the
  `skipped PPTX slide {rid}: unknown relationship id` warning, no
  missing-rels warning is emitted, and the extraction succeeds with no slide
  records.

### Requirement: Malformed slide XML keeps recoverable partial text

Malformed XML inside a readable slide or notes part MUST keep the existing
recoverable behavior: the system MUST emit the record with whatever partial
text could be recovered, plus the existing `W001 malformed_xml` warning; the
record MUST NOT be silently dropped. Malformed `ppt/presentation.xml` keeps
its existing `W001` warning with partial id-list behavior. Skip-with-warning
applies only to targets that cannot be read at all, not to readable-but-
malformed parts.

#### Scenario: Truncated slide part keeps partial text

- GIVEN the `corpus/pptx/malformed-xml` fixture whose second slide part is
  truncated mid-XML
- WHEN slide extraction runs
- THEN a record for that slide is emitted with the recovered partial text, the
  `W001 malformed_xml` warning is emitted, and the first slide's record is
  unaffected.

### Requirement: Security and hard-error boundaries are preserved

The system MUST keep `SuspiciousRelationshipTarget` (external target mode,
escaping target, NUL in target) a HARD error in the slide extraction path —
per-slide skip-with-warning MUST NOT downgrade it. A missing or malformed
`ppt/presentation.xml` main part MUST remain a hard error: without the main
part there is nothing to enumerate. No limits or security-policy behavior may
change.

#### Scenario: Suspicious relationship target

- GIVEN a slide relationship whose target mode is external (or escapes the
  package, or contains a NUL)
- WHEN slide extraction runs
- THEN `SuspiciousRelationshipTarget` is raised as a hard error and no
  slide records are emitted for the document.

#### Scenario: Missing presentation part

- GIVEN a PPTX package without a readable `ppt/presentation.xml`
- WHEN slide extraction runs
- THEN extraction fails with the existing hard-error behavior (no partial
  slide record set is emitted).

### Requirement: Existing PPTX extraction stays byte-identical

The shared slide-reference parsing refactor MUST be output-neutral for the
existing paths: `extract text --format text|json|jsonl|structured-json`,
`extract tables`, `extract rows`, `extract csv`, `info`, and `audit` output
MUST remain byte-identical, locked by the existing snapshots (`pptx_text.txt`,
`cli_structured_text_pptx_json.json`, `cli_structured_text_json.json`,
`pptx_basic_info.json`, tables/rows snapshots). `extract_pptx_text` and
`extract_pptx_structured_text` MUST keep their hard `MissingPart` failure for
a dangling slide `r:id` or an absent slide/notes part — the skip-with-warning
leniency is scoped to the slide extraction contract only, and a regression
test MUST assert the old paths still fail with `MissingPart` on the new
missing-target fixture. A `p:sldId` with a valid `r:id` but no `@id` MUST
still be extracted by the text and structured paths exactly as today.

#### Scenario: Old paths hard-error on missing targets

- GIVEN the `corpus/pptx/missing-target` fixture
- WHEN `extract_pptx_text` or `extract_pptx_structured_text` runs
- THEN extraction fails with `MissingPart`, exactly as before this change.

#### Scenario: Pre-existing snapshots unchanged

- GIVEN all existing PPTX-related snapshots and fixtures (no corpus tree is
  edited)
- WHEN the full test suite runs
- THEN every pre-existing snapshot comparison passes byte-identically and no
  existing fixture digest changes.

### Requirement: JSON document contract

`oxdoc extract slides <FILE> --format json` MUST emit a single
pretty-printed JSON document with a trailing newline containing:

- `schema_version` (integer, constant `1`),
- `file` (the input file label, `<stdin>` for stdin, matching the repo-wide
  `display_file_name` precedent),
- `document_type` (string, constant `"pptx"`),
- `slides` (array of slide records in `p:sldIdLst` order), and
- `warnings` (array of warning objects; MUST be present and `[]` when empty,
  never omitted; embedded warnings are never suppressed by `--quiet`).

Skipped slides MUST simply be absent from `slides`; a deck whose slides are
all skipped MUST still produce a valid payload with `"slides": []` and the
warnings — it is NOT the "no input files were processed successfully" error
(the document was processed successfully). Embedded warnings MUST also be
mirrored to stderr subject to the global `--warnings`/`--quiet` flags,
following the DOCX tables/audit precedent.

#### Scenario: JSON payload shape

- GIVEN `oxdoc extract slides deck.pptx --format json` on a two-slide deck
  with one notes-bearing slide
- WHEN the payload is emitted
- THEN it is a single pretty-printed document whose top-level keys are
  `schema_version` (`1`), `file`, `document_type` (`"pptx"`), `slides` (two
  records), and `warnings` (`[]`), with `notes` present on exactly the
  notes-bearing slide record.

#### Scenario: All slides skipped still yields a payload

- GIVEN a deck where every slide is skipped for a missing target
- WHEN `--format json` runs
- THEN the exit succeeds and the payload contains `"slides": []` plus the
  skip warnings.

### Requirement: JSONL stream contract

`oxdoc extract slides <FILE> --format jsonl` MUST emit exactly one compact
JSON object per non-skipped slide on stdout, one per line in `p:sldIdLst`
order, with each record carrying `schema_version` (`1`), `file`,
`slide_ordinal`, `slide_path`, `text`, and `slide_id` (omitted when absent),
plus `notes` under the presence rule above; there is no wrapping array. The
`file` field follows the repo-wide `display_file_name` label rule, so stdin is
labelled `<stdin>`. Recoverable warnings MUST go to stderr only (rows-jsonl
precedent) so stdout
remains a valid JSONL record stream; there is no per-line error contract
because skips are non-fatal. Skipped slides are detectable only through the
stderr warning channel.

#### Scenario: Valid stream under warnings

- GIVEN `oxdoc extract slides deck.pptx --format jsonl` on the missing-target
  fixture
- WHEN extraction completes
- THEN stdout parses line-by-line as JSON for every emitted slide (no warning
  or error text on stdout), all warnings appear on stderr, and the emitted
  ordinals show the expected gap.

#### Scenario: One record per slide, same order as JSON

- GIVEN the same deck extracted with `--format json` and `--format jsonl`
- WHEN both outputs are compared
- THEN the JSONL records appear in the same `p:sldIdLst` order with the same
  field values as the entries of the JSON `slides` array.

### Requirement: Versioned slides schema contract

The repository MUST publish a NEW contract
`schemas/v1/oxdoc-pptx-slides.schema.json` with an exact mirrored copy at
`docs/schemas/v1/oxdoc-pptx-slides.schema.json` (covered by
`make docs-schemas-check` lockstep). The schema MUST use JSON Schema draft
2020-12, the stable `$id`
`https://github.com/spereyra-dev/oxdoc/schemas/v1/oxdoc-pptx-slides.schema.json`,
and `additionalProperties: false`, with a top-level `oneOf` over:

- the document payload (requires `schema_version` const `1`, `file`,
  `document_type` const `"pptx"`, `slides`, `warnings`), and
- the JSONL record (requires `schema_version` const `1`, `file`,
  `slide_ordinal` minimum `1`, `slide_path`, `text`; `slide_id` and `notes`
  optional).

Both shapes MUST share a `$defs.slide` definition for the slide object so JSON
and JSONL cannot drift. `slide_id` MUST be an optional integer,
`slide_ordinal` an integer with `minimum: 1`, and `slide_path`/`text`/`notes`
strings. The change MUST NOT modify `schemas/v1/**`,
`schemas/v2/oxdoc-structured-text.schema.json`, their `docs/schemas` mirrors,
or any `TextBlock`/`StructuredText` field — this is a new contract, not a
widening of structured-text. A negative test MUST assert that a slides payload
does NOT validate against the structured-text schema (v2 and v1), and the new
schema MUST be registered in the `schema.rs` version list and validated
against representative JSON and JSONL snapshots.

#### Scenario: Snapshots validate against the new schema

- GIVEN the committed snapshots `cli_pptx_slides_json.json` and
  `cli_pptx_slides_jsonl.jsonl`
- WHEN schema validation runs
- THEN both validate against
  `schemas/v1/oxdoc-pptx-slides.schema.json` and the mirror is identical
  (`make docs-schemas-check` passes).

#### Scenario: Slides payload is not a structured-text payload

- GIVEN the `cli_pptx_slides_json.json` snapshot
- WHEN it is validated against
  `schemas/v2/oxdoc-structured-text.schema.json` (and the v1 equivalent)
- THEN validation fails (no `blocks`/`schema_version: 2` overlap is accepted).

#### Scenario: Structured-text schemas stay frozen

- GIVEN the change diff
- WHEN every file under `schemas/v1/**`, `schemas/v2/**`, and their
  `docs/schemas` mirrors except the new `oxdoc-pptx-slides.schema.json` files
  is inspected
- THEN no existing schema file is modified.

### Requirement: extract slides CLI subcommand

The CLI MUST provide `oxdoc extract slides <FILE> --format json|jsonl` as a
new subcommand (not new `TextFormat` variants), with `json` as the default
format and a single positional file that accepts `-` for stdin (same stdin
handling as `extract rows`). The command MUST reject non-PPTX inputs (DOCX,
XLSX) with a `CliError::InvalidArgument` message in the same style as the
neighbouring type-restricted commands. The command MUST NOT offer
`--output`/`--output-dir` or multi-input batch in this change (deferred, not
rejected), and exit codes MUST follow existing CLI conventions: success
(including all-slides-skipped with warnings) is `0`; hard errors (missing
presentation part, suspicious target, wrong type argument) are non-zero.

#### Scenario: JSON is the default format

- GIVEN `oxdoc extract slides deck.pptx` with no `--format` flag
- WHEN the command runs
- THEN the output is the JSON document payload described above.

#### Scenario: Stdin input

- GIVEN `oxdoc extract slides - --format jsonl` with a PPTX piped on stdin
- WHEN the command runs
- THEN slide records are emitted exactly as for a file argument, and the JSONL
  `file` field labels the input as `<stdin>`.

#### Scenario: DOCX input rejected

- GIVEN `oxdoc extract slides report.docx`
- WHEN the command runs
- THEN the command fails with an invalid-argument error consistent with the
  existing type-restricted commands and no slide output is produced.

### Requirement: Fixtures and provenance

The change MUST reuse the existing `tests/fixtures/corpus/pptx/basic/`
corpus (one slide, no notes) and `tests/fixtures/corpus/pptx/text/` corpus
(two slides in inverted `sldIdLst` order, slide 2 with notes) without
modification, and MUST add two new runtime-zipped corpus trees:

- `tests/fixtures/corpus/pptx/missing-target/` — slides exercising a dangling
  slide `r:id`, an absent notes target part, and an intact slide; and
- `tests/fixtures/corpus/pptx/malformed-xml/` — slides where one slide part is
  truncated mid-XML so partial text plus `W001` is exercised.

Each new corpus tree MUST have a provenance note under
`tests/fixtures/provenance/` (`pptx-missing-target.md`,
`pptx-malformed-xml.md`) following the `docx-section-order.md` label set
(`Source:`, `Producer:`, `Redistribution:`, `Purpose:`, plus
`Archive generation:`/`Sanitization:` as applicable), and both names MUST be
added to the fixture-provenance presence checks in the core and CLI test
suites. The compatibility-corpus gate MUST stay green: no
`tests/fixtures/compatibility-matrix.json` entry and no existing fixture
digest is added or changed (runtime-zipped corpus trees carry no digest).

#### Scenario: New corpora drive the new behaviors

- GIVEN the new `missing-target` and `malformed-xml` corpus packages built at
  test runtime
- WHEN the slide extraction test suite runs
- THEN the dangling-rel, missing-slide-part, missing-notes, and
  truncated-XML scenarios are each exercised through the fixtures (not only
  synthetic inline XML), including the old-paths-still-`MissingPart`
  regression.

#### Scenario: Provenance and corpus gates pass

- GIVEN the two new provenance notes and unchanged compatibility matrix
- WHEN `make compatibility-corpus-check` and the provenance presence tests run
- THEN both pass, with no existing digest modified and no existing corpus tree
  edited.

### Requirement: Warning texts are stable and documented

The new recoverable warnings MUST use these exact, stable wordings (locked by
the scenarios above and by tests):

- `skipped PPTX slide {rid}: unknown relationship id` (path
  `ppt/presentation.xml`),
- `skipped related PPTX slide part {path}: missing part` (path = absent slide
  part), and
- `skipped related PPTX notes part {path}: missing part` (path = absent notes
  part).

Malformed XML continues to use the existing `W001 malformed_xml` warning. No
warning is introduced for a `p:sldId` without `@id` on any path. These texts
and the channel rules (JSONL stderr-only; JSON embedded plus stderr) MUST be
documented in `docs/json-output.md` and `docs/formats/pptx.md`.

#### Scenario: Warning wording is byte-stable

- GIVEN the missing-target fixture processed by slide extraction
- WHEN the emitted warnings are compared
- THEN each warning matches the exact wording above with the actual
  relationship id or part path substituted and no extra text.

### Requirement: Documentation and changelog

The change MUST document the new contract in:

- `docs/formats/pptx.md` — a new "Slide-scoped JSON / JSONL" section (record
  fields, identity, ordinal gaps, notes rule, warning channels);
- `docs/json-output.md` — the schema table row for
  `schemas/v1/oxdoc-pptx-slides.schema.json`, slide payload/JSONL semantics,
  the warning-channel statement, and the version-policy note that this is a
  new contract rather than a structured-text widening; and
- `docs/cli.md` — `extract slides` usage, an example, and the ordinal/skip
  semantics.

Additionally `README.md` MUST gain a command example alongside the existing
tables/rows examples, and `CHANGELOG.md` MUST record the new subcommand and
the new schema contract. No existing documentation of `extract text`,
`structured-json`, or the v2 schema may be altered in meaning.

#### Scenario: Docs describe the contract

- GIVEN a reader of `docs/formats/pptx.md`, `docs/json-output.md`, and
  `docs/cli.md`
- WHEN they look up slide-scoped extraction
- THEN they find the subcommand usage, both format shapes, the
  `slide_id`/`slide_ordinal` rules (including gaps after skips), the
  `notes` presence rule, the exact warning wordings with their channels, and
  the schema versioning note.
