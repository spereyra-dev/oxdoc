# XLSX Formula Provenance Specification

## Purpose

Make the typed XLSX rows contract self-describing about formula provenance:
the stored formula expression text, whether the workbook carried a cached value
for the formula cell, and an explicit guarantee that formulas are never
recalculated. The contract is delivered additively behind a versioned rows-jsonl
schema (v2) and a versioned, documented `XlsxCell` library API change, without
touching plain-text/CSV output, `oxdoc-tabular` behavior, DOCX/PPTX paths, or
the frozen v1 schema.

Domains inferred from the proposal (no `Capabilities` section present; no
canonical XLSX domain exists yet): this is a new domain, `xlsx-formula-provenance`.

## Requirements

### Requirement: Formula expression capture

The XLSX parser MUST capture the text between `<f>` and `</f>` on worksheet
cells using the same text decoders used for cell values (`Event::Text`,
`Event::CData`, and `Event::GeneralRef` decoded exactly as `<v>`/`<t>` text is
decoded, so entity references and CDATA resolve identically). Capture MUST
handle `<f>` as `Event::Start` with following text, and `Event::Empty`
(self-closing) elements. Formula text MUST be accumulated in a buffer disjoint
from the value and inline-text buffers: formula text MUST never append to the
cell value, and value text MUST never append to the formula expression. The
`<f>` attributes `t` and `si` MUST be read by local name via the existing
`attr_value` helper, so worksheet-namespace prefixes on the sheet element are
irrelevant. Other `<f>` attributes (`aca`, `dt2D`, `dtr`, `cm`, `ref`) MUST
remain ignored. Cells that contain no `<f>` element MUST be unchanged: no
formula state, no synthesized expression, and byte-identical serialization
(apart from the payload envelope version).

#### Scenario: Cached formula captures expression and value

- GIVEN a corpus cell `<c r="B2"><f>SUM(B1:B1)</f><v>42</v></c>`
- WHEN typed rows are extracted
- THEN the cell reports the expression `SUM(B1:B1)`, cache presence `true`, and
  the stored cached number value unchanged.

#### Scenario: Entity and CDATA decoding in formulas

- GIVEN a formula whose text contains `&quot;`, `&amp;`, `&lt;`, a numeric
  character reference, or a CDATA section
- WHEN the cell is parsed
- THEN the captured expression equals the decoded text, identically to how the
  same sequences decode inside `<v>`/`<t>`.

#### Scenario: Non-formula cells are untouched

- GIVEN a cell with no `<f>` element (including an error cell with no formula)
- WHEN typed rows are extracted
- THEN the cell has no formula state, `has_formula` is `false`, and its
  serialization is unchanged from v1 apart from the envelope version.

### Requirement: Cache-presence provenance (`formula_cached`)

The parser MUST record whether the formula cell contained a `<v>` element as a
pure XML-presence fact: `<v>` present on `Event::Start` or `Event::Empty`
(including `<v/>`, a present-but-empty cache) means the cache existed. Cache
presence MUST NOT depend on the success of value type resolution: a
`<c t="s"><f>…</f><v>999</v></c>` whose shared string index is out of bounds
still reports the cache as present (the existing shared-string warning
continues to explain the resolution failure). A formula cell with no `<v>`
reports no cache. The three cases MUST be mutually distinguishable in output:
cached formula (value + cache flag true), uncached formula (`kind: "blank"`,
`has_formula: true`, expression present, cache flag false), and
empty-but-cached (`kind: "blank"`, cache flag true).

#### Scenario: Uncached formula is distinguishable from a blank cell

- GIVEN a corpus cell `<c r="C2"><f>SUM(C1:C1)</f></c>` with no `<v>`
- WHEN typed rows are extracted
- THEN the cell reports `kind: "blank"`, `has_formula: true`, the expression,
  and cache presence `false` — distinguishable from a plain blank cell.

#### Scenario: Empty-but-cached value keeps cached semantics

- GIVEN a corpus cell `<c r="D2"><f>IF(1=1,"","x")</f><v></v></c>`
- WHEN typed rows are extracted
- THEN the cell reports `kind: "blank"` with cache presence `true`, preserving
  the cached-value semantics instead of collapsing to the uncached case.

#### Scenario: Cache flag survives failed type resolution

- GIVEN a shared-string formula cell whose `<v>` index is out of bounds
- WHEN typed rows are extracted
- THEN cache presence is `true` and the existing shared-string warning is
  emitted unchanged.

### Requirement: Core API — XlsxFormula and XlsxCell.formula

The library MUST expose a new `#[non_exhaustive]` type `XlsxFormula` with
fields `expression: String` and `cached: bool`, and `XlsxCell` MUST gain a new
field `formula: Option<XlsxFormula>`. The documented invariant MUST hold:
`has_formula == false` implies `formula == None`; the converse does not hold
(a cell whose expression is unresolved reports `has_formula: true` with
`formula: None`). `has_formula` MUST keep its existing meaning ("the cell
contains an `<f>` element") and `XlsxCellValue` variants MUST keep their
existing semantics — the formula never influences the value, and an uncached
formula stays `Blank`. `XlsxCell` MUST NOT be marked `#[non_exhaustive]` in
this change. The field addition is a source break for external struct literals;
it MUST be versioned in `CHANGELOG.md` under **Changed** with migration
guidance: add `formula: None` to struct literals, or read the field when
constructing parser output. In-tree call sites across `oxdoc-core`,
`oxdoc-cli`, and `oxdoc-tabular` (test constructors included) MUST be updated
in the same work unit so the workspace compiles throughout.

#### Scenario: Migration is mechanical for struct literals

- GIVEN an external consumer constructing `XlsxCell` with a struct literal
- WHEN the consumer upgrades
- THEN compilation requires exactly one added field, `formula: None`, as
  documented in `CHANGELOG.md`.

#### Scenario: Invariant holds in both directions where guaranteed

- GIVEN any parsed cell with `has_formula == false`
- WHEN the cell is inspected
- THEN `formula` is `None`; a cell with `has_formula == true` MAY have
  `formula == None` only when its expression is unresolved.

### Requirement: Shared formula resolution — master-first, bounded

The parser MUST resolve shared formulas with a per-worksheet map from the raw
`si` attribute text to expression text, without numeric parsing of `si`. When a
master `<f t="shared" si="N">EXPR</f>` is parsed, the cell's own expression
MUST be `EXPR`, and `N → EXPR` MUST be registered. The first registration for
an `si` MUST win; later duplicates MUST NOT overwrite it. A master whose text
is empty MUST register nothing. When a slave `<f t="shared" si="N"/>` is
parsed and `N` is registered, the slave's expression MUST be the master's text
copied verbatim — no reference translation, no rewriting, no evaluation. When
`N` is not registered (including a slave that appears before its master in the
single-pass stream), the slave MUST report `has_formula: true` with
`formula: None` and emit one warning per affected cell at the worksheet path
with the exact wording
`unresolved shared formula index '{si}': formula expression omitted`.
The cell, its `kind`, and its cached value MUST still be emitted.

#### Scenario: Slave resolves to master text verbatim

- GIVEN a master `<f t="shared" si="0" ref="A2:A4">SUM(B2:B4)</f>` followed by
  a cached slave `<f t="shared" si="0"/>` and an uncached slave
- WHEN typed rows are extracted
- THEN both slaves carry the expression `SUM(B2:B4)` verbatim, each with its
  own cache-presence value, and the cached slave still emits its cached value.

#### Scenario: Dangling si warns per cell and still emits the cell

- GIVEN a slave `<f t="shared" si="9"/>` with no registered master
- WHEN typed rows are extracted
- THEN the cell is emitted with `has_formula: true`, no expression, its cached
  value intact, and stderr contains exactly one line
  `unresolved shared formula index '9': formula expression omitted` for that
  cell.

#### Scenario: Slave before master is unresolved

- GIVEN a slave whose `si` master appears later in the worksheet stream
- WHEN typed rows are extracted
- THEN the slave is treated as unresolved (per-cell warning, no expression) and
  the later master still registers and captures its own text.

#### Scenario: First registration wins

- GIVEN two masters carrying the same `si` with different texts
- WHEN a slave resolves
- THEN it resolves to the text of the first master registered in the stream.

### Requirement: Bounded shared-formula table with injectable limit

The shared-formula map MUST be bounded: a crate-internal default of 1 MiB
(consistent with existing resource-limit conventions), checked with saturating
arithmetic before insertion (expression length plus a fixed per-entry overhead).
When an insertion would exceed the bound, the entry MUST NOT be recorded and a
warning MUST be emitted at the worksheet path with the exact wording
`shared formula table limit reached: expressions beyond it are omitted`,
latched to once per worksheet. Cells whose expressions are then unresolvable
MUST still emit the per-cell unresolved warning. The bound MUST be injectable
through a crate-internal parser seam (mirroring the shared-string
`parse_with_memory_limit` precedent) so the overflow path is unit-testable
with a tiny limit; `OoxmlLimits` and the public API MUST NOT change. The table
MUST NOT spill to temporary files.

#### Scenario: Overflow latches once per worksheet

- GIVEN a worksheet whose shared-formula entries exceed an injectable tiny
  limit
- WHEN the sheet is parsed
- THEN exactly one warning
  `shared formula table limit reached: expressions beyond it are omitted`
  appears for the worksheet, affected slaves still warn per cell, and cached
  values are still emitted.

#### Scenario: Default bound follows existing memory conventions

- GIVEN the default configuration
- WHEN the parser runs
- THEN the shared-formula table bound is 1 MiB and `OoxmlLimits` is unchanged.

### Requirement: Array formulas — master text only, region cells untouched

An array formula master `<f t="array" ref="…">EXPR</f>` MUST capture `EXPR`
and its own cache flag exactly like a normal formula. Cells of the `ref` region
that contain no `<f>` element of their own MUST remain `has_formula: false`
with no formula state — the system MUST NOT synthesize formulas for cells whose
XML carries none. Any other `<f>` with text (for example `t="dataTable"`) MUST
capture its text as-is under the same rule; `t="shared"` with `si` is the only
special case.

#### Scenario: Array master captures expression, region cell stays non-formula

- GIVEN a master `<f t="array" ref="F2:F3">SUM(G2:G3)</f>` and a region cell
  carrying only `<v>`
- WHEN typed rows are extracted
- THEN the master carries the expression and its cache flag, and the region
  cell remains `has_formula: false` with no formula state.

### Requirement: Warning classification, channel, and no-recalculation guarantee

Both new warning wordings MUST classify as `custom`/`W999` through the existing
message-sniffing warning classification (no new `WarningCode` variant; the
enum stays exhaustive and unchanged). Rows warnings MUST continue to go to
stderr so stdout remains a valid JSONL stream. The system MUST NEVER
recalculate formulas: no evaluation, no dependency graph, no reference
translation for shared slaves; `formula` is stored text, never a computed
result, and no value is ever derived from an expression. The no-recalculation
guarantee MUST be stated in the v2 schema field descriptions and in the
documentation surfaces listed below.

#### Scenario: Warnings stay on stderr

- GIVEN an extraction that emits an unresolved shared-formula warning
- WHEN `extract rows --format jsonl` runs
- THEN stdout remains valid JSONL and the warning appears on stderr.

#### Scenario: Formulas are never recalculated

- GIVEN corpus fixtures containing `SUM`/`1/0`-style expressions
- WHEN rows are extracted
- THEN every emitted `raw` equals the workbook's stored `<v>` (or is absent for
  uncached formulas) and no expression ever produces a computed number.

### Requirement: Rows-jsonl schema version 2

The repository MUST publish
`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` (JSON Schema draft 2020-12,
stable `$id`, `additionalProperties: false`, `const: 2` on `schema_version`)
with an exact mirrored copy at
`docs/schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`, verified identical by
`make docs-schemas-check`. The v2 schema MUST declare the optional cell fields
`formula` (string) and `formula_cached` (boolean) on all five cell variants
(`blank`, `string`, `boolean`, `number`, `error`) through the shared
`$defs.cellBaseProperties`, and MUST declare presence coupling so that
`formula_cached` is required exactly when `formula` is present (via
`allOf`/`if`/`then` on the shared cell shape). All v1 field names, types, `kind`
semantics, required fields, sparse-cell rules, and the
`not: {required: [sheet_name, sheet_index]}` constraint MUST be unchanged.
The v1 schema and its docs mirror MUST remain frozen and unmodified and keep
validating previously captured v1 payloads. A v2 payload for a formula-free
workbook MUST differ from its v1 payload only by `schema_version`.

#### Scenario: Representative v2 record validates

- GIVEN a v2 record with a cached-formula number cell, an uncached formula
  `blank` cell, an error formula cell (`t="e"` keeping its cached error value
  as `kind: "error"` with `raw` the stored error string and the formula text
  captured), and a shared slave
- WHEN schema validation runs
- THEN the record validates against the v2 schema with the declared fields and
  the presence-coupling invariant holds.

#### Scenario: Presence coupling is enforced

- GIVEN a cell carrying `formula_cached` without `formula`
- WHEN schema validation (or the direct Rust assertion if the hand-rolled
  harness cannot evaluate `allOf`/`if`/`then`) runs
- THEN validation fails; any harness limitation MUST be stated explicitly or
  the harness extended, never silently dropped.

#### Scenario: v1 payload fails v2 validation

- GIVEN a v1-shaped payload (`schema_version: 1`, no `formula` fields)
- WHEN it is validated against the v2 schema
- THEN validation fails — the documented, intentional break strict v1
  consumers migrate for.

#### Scenario: v1 stays frozen and mirror stays identical

- GIVEN the change diff
- WHEN inspected
- THEN no file under `schemas/v1/**` or `docs/schemas/v1/**` is modified, and
  `make docs-schemas-check` verifies the v2 schema and its docs mirror are
  identical.

#### Scenario: Snapshot byte-compare locks v2 output

- GIVEN the new JSONL snapshot
  `tests/fixtures/snapshots/cli_xlsx_rows_v2_jsonl.jsonl` built from the
  formulas corpus tree
- WHEN the CLI rows test runs
- THEN output is byte-compared against the snapshot and carries
  `schema_version: 2`.

### Requirement: CLI emission of formula fields

`extract rows --format jsonl` MUST emit `schema_version: 2` and map
`XlsxCell.formula` into the two optional cell fields `formula` and
`formula_cached`, both present or both omitted together, with `formula_cached`
reporting the cache-presence flag. Field order MUST keep `has_formula` in place
and append the new fields, so documented examples gain two trailing keys. There
MUST be no `--schema-version` flag and no dual emission: the version moves,
the previous contract stays frozen. The existing `InvalidArgument` catch-all
for unknown `kind` values MUST be unchanged. The Python wrapper MUST require no
code change and a test MUST pin that a v2 record carrying the new fields passes
through `extract_rows` unchanged.

#### Scenario: Cached formula cell carries both fields

- GIVEN a cached formula number cell
- WHEN rows-jsonl v2 is emitted
- THEN the cell carries `formula` (the stored expression) and
  `formula_cached: true` alongside its unchanged `kind` and value fields.

#### Scenario: Uncached formula cell carries expression without cache

- GIVEN an uncached formula cell
- WHEN rows-jsonl v2 is emitted
- THEN the cell carries `formula` and `formula_cached: false` with
  `kind: "blank"` and no `raw`.

#### Scenario: Python pass-through is unchanged

- GIVEN a v2 record with `formula`/`formula_cached`
- WHEN the Python `extract_rows` returns it
- THEN the record passes through verbatim (pinned by test, no wrapper change).

### Requirement: Documentation and CHANGELOG versioning

The change MUST be documented in: `docs/json-output.md` (v2 schema row, frozen
v1 note, updated sample record, no-recalculation guarantee, shared-formula
provenance rule, uncached-formula case), `docs/formats/xlsx.md` (explicit
never-recalculated statement; uncached formula yields an empty CSV field,
unchanged), `docs/cli.md` (`schema_version: 2`, new optional fields, warning
wordings), `docs/library-api.md` (`XlsxFormula`, the `has_formula`/`formula`
relationship, the `formula: None` migration note), and `README.md` (rows
example gains the new fields). `CHANGELOG.md` MUST record under **Added** the
new cell fields, expression capture, bounded shared-formula resolution, and the
no-recalculation statement, and under **Changed** the rows-jsonl v2 move with
the strict-v1-validator migration note (mirroring the structured-text v2
wording) and the `XlsxCell` source break with the `formula: None` migration
guidance.

#### Scenario: Docs state the no-recalculation guarantee

- GIVEN the documentation surfaces listed above
- WHEN inspected
- THEN each states that formulas are never recalculated and that emitted values
  are the workbook's stored values.

#### Scenario: CHANGELOG versions both breaks

- GIVEN `CHANGELOG.md`
- WHEN inspected
- THEN it documents the v1→v2 payload migration for strict validators and the
  `XlsxCell` struct-literal migration (`formula: None`).

### Requirement: Fixture corpus with provenance

The repository MUST add two hand-authored runtime-zipped corpus trees with
formula provenance: `tests/fixtures/corpus/xlsx/formulas/` covering a cached
numeric formula, an uncached formula, an empty-but-cached value, an error
formula (`t="e"` with cached error kept as the value), a string-result formula,
a shared-string formula cell, entity-decoding formula text, and an error cell
with no `<f>` (control); and `tests/fixtures/corpus/xlsx/shared-formulas/`
covering a cached master plus cached and uncached slaves, a dangling `si`
slave, slave-before-master ordering, an array master with a region cell holding
only `<v>`, and `si` attributes written under a namespace-prefixed sheet
element. Both trees MUST have provenance notes under `tests/fixtures/provenance/`
with the required label set, listed in the provenance-presence test lists in
`oxdoc-core` and `oxdoc-cli`. No compatibility-matrix entry, digest, or
`tests/fixtures/files/**` change is required for runtime-zipped trees, and
`make compatibility-corpus-check` MUST stay green. Overflow of the
shared-formula table MUST NOT be a corpus fixture; it is covered by a unit test
through the injectable limit seam. An openpyxl-generated formula workbook for
the compatibility matrix is deferred and MUST be documented as deferred.

#### Scenario: Corpus covers the formula case matrix

- GIVEN the two corpus trees
- WHEN fixtures are built and typed rows are extracted
- THEN each documented case (cached, uncached, empty-cached, error, shared
  master/slave cached and uncached, dangling `si`, ordering, array, prefixed
  attributes, non-formula control) produces its specified record shape.

#### Scenario: Provenance and corpus gates stay green

- GIVEN the new trees and provenance notes
- WHEN `make compatibility-corpus-check` and the provenance-presence tests run
- THEN they pass with no manifest entry, digest, snapshot-of-binary, or
  `tests/fixtures/files/**` change.
