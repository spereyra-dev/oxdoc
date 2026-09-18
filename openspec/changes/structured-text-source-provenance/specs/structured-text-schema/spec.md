# Structured Text Schema Specification

## Purpose

Define the versioned public contract for `oxdoc extract text --format
structured-json`: the JSON Schema files that govern the payload, the payload's
version marker, the semantics of block fields (`part_type`, `part_path`,
`ordinal`, `variant`, `text`) for DOCX and PPTX, the guarantee that provenance
metadata never leaks into plain-text output, and the schema versioning policy
that any new output field requires a new schema version.

## Requirements

### Requirement: Structured-text schema version 2

The repository MUST publish
`schemas/v2/oxdoc-structured-text.schema.json` describing the structured-json
output, with an exact mirrored copy at
`docs/schemas/v2/oxdoc-structured-text.schema.json`. The schema MUST use JSON
Schema draft 2020-12, `$id`
`https://github.com/spereyra-dev/oxdoc/schemas/v2/oxdoc-structured-text.schema.json`,
and `additionalProperties: false`. The top level MUST require `schema_version`
(constant `2`, integer), `file`, `document_type` (enum `["docx", "pptx"]`), and
`blocks`. Each block MUST require `part_type`, `part_path`, `ordinal`
(minimum `1`), and `text`, and MUST allow an optional `variant` with enum
`["first", "even", "default"]`. The existing `schemas/v1/**` files MUST remain
frozen and unmodified as the validatable contract for previously published
outputs.

#### Scenario: Snapshot validates against v2 schema

- GIVEN the DOCX and PPTX structured-json CLI snapshots
- WHEN schema validation runs
- THEN both snapshots validate against
  `schemas/v2/oxdoc-structured-text.schema.json`.

#### Scenario: Mirror lockstep

- GIVEN `schemas/v2/` and `docs/schemas/v2/`
- WHEN `make docs-schemas-check` runs
- THEN the v2 directories are verified identical in addition to the existing v1
  check.

#### Scenario: v1 stays frozen

- GIVEN the change diff
- WHEN the v1 schema directory is inspected
- THEN no file under `schemas/v1/**` or `docs/schemas/v1/**` is modified.

#### Scenario: v1-strict consumers must migrate

- GIVEN a v2 payload containing `schema_version: 2`
- WHEN it is validated against
  `schemas/v1/oxdoc-structured-text.schema.json` with its
  `additionalProperties: false`
- THEN validation fails, and the v1 schema remains published unchanged for
  validating previously captured v1 outputs.

### Requirement: Schema versioning policy

New output fields in the structured-json contract MUST be introduced through a
new schema version instead of silently widening the current contract, mirroring
the policy already stated for other machine-readable outputs in
`docs/json-output.md`. The version policy and the structured-json semantics
(payload version field, optional variant, global ordinal, slide/notes meaning)
MUST be documented in `docs/json-output.md`.

#### Scenario: New field requires new version

- GIVEN a future change that wants to add a field to the structured-json output
- WHEN the contract is updated
- THEN a new schema version directory (for example `schemas/v3/`) is published
  and the v2 schema remains as-is.

### Requirement: Payload carries schema_version 2

The CLI structured-json payload MUST carry a top-level `schema_version` field
with integer value `2`, for single-input, stdin, and multi-input payloads alike;
in multi-input mode every element MUST carry the marker. The v1 structured-text
snapshot contract MUST remain untouched for outputs validated as v1. Consumers
using strict v1 validators MUST be guided to
`schemas/v2/oxdoc-structured-text.schema.json` in `docs/json-output.md` and
`CHANGELOG.md`. The core `StructuredText` model MUST remain unversioned: the
version marker is a CLI emission concern, and Rust library callers that
serialize `StructuredText` directly keep the unversioned body (a documented,
deliberate asymmetry).

#### Scenario: Single-input payload is versioned

- GIVEN `oxdoc extract text report.docx --format structured-json`
- WHEN the payload is emitted
- THEN it begins with `schema_version: 2` followed by `file`, `document_type`,
  and `blocks`.

#### Scenario: Multi-input payloads are each versioned

- GIVEN `oxdoc extract text a.docx b.pptx --format structured-json`
- WHEN the payload array is emitted
- THEN every element carries `schema_version: 2`.

#### Scenario: Library serialization stays unversioned

- GIVEN a Rust caller serializing `StructuredText` directly
- WHEN the value is serialized
- THEN the output has no `schema_version` field, and this asymmetry is
  documented in `docs/json-output.md`.

### Requirement: Plain-text and tables output never carry provenance metadata

Variant labels, part paths, and ordinals MUST NOT appear in `--format text` or
tables output. Existing plain-text output MUST remain byte-identical, locked by
regression assertions over the same fixtures used for variant testing and by the
existing plain-text snapshots.

#### Scenario: Variant data does not leak into flat text

- GIVEN the section-order DOCX fixture whose headers carry `first`/`even`/
  `default` variants
- WHEN `oxdoc extract text ... --format text` runs
- THEN the output is byte-identical to the previous release, with no variant
  strings, part paths, or ordinals anywhere in the text.

#### Scenario: Tables output unchanged

- GIVEN the same fixtures
- WHEN tables extraction runs
- THEN tables output is byte-identical to the previous release.

### Requirement: PPTX structured blocks are labeled slide and notes

For PPTX documents, structured-json blocks MUST be labeled `part_type: "slide"`
for slide text and `part_type: "notes"` for speaker notes extracted from the
slide's `ppt/notesSlides/notesSlideN.xml` part. Notes blocks MUST appear after
their slide's block. Slide order MUST follow presentation slide order
(`p:sldIdLst`) and MUST be stable. This behavior MUST be locked by a
schema-validated PPTX structured snapshot test, and `docs/formats/pptx.md` MUST
document the structured-json output: slide vs notes meaning, the notes part
path, ordinal semantics, and that PPTX blocks never carry `variant`.

#### Scenario: Slide and notes blocks for a two-slide deck

- GIVEN a PPTX with two slides where slide 2 links a notesSlide
- WHEN structured extraction runs
- THEN the output contains a `slide` block for each slide in presentation order
  and a `notes` block for slide 2 after its slide block, with `part_path`
  pointing at the slide part and the `ppt/notesSlide2.xml` part respectively.

#### Scenario: PPTX snapshot locks the contract

- GIVEN the PPTX text fixture corpus
- WHEN the CLI structured-json output is tested
- THEN it is compared byte-for-byte against a committed snapshot that validates
  against the v2 schema.

### Requirement: Global ordinal semantics are documented and stable

The `ordinal` field MUST be a 1-based global output-order index across the
flattened block list — not a paragraph ordinal and not scoped per part or per
slide. Ordinals MUST be contiguous in output order (`ordinal == 1..=blocks.len()`).
This semantic MUST be stated in the v2 schema description,
`docs/json-output.md`, and `docs/cli.md`. Per-slide ordinal scoping is deferred
to a future change and MUST NOT be introduced here.

#### Scenario: Ordinals are contiguous in output order

- GIVEN the multi-section DOCX fixture with related parts and comments
- WHEN structured extraction runs
- THEN the blocks' `ordinal` values are exactly `1..=blocks.len()` in output
  order, spanning main, headers, footers, and comments.
