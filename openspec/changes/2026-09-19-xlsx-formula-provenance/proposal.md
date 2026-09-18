# Proposal — XLSX formula provenance in typed rows (issue #182)

- Change id: `2026-09-19-xlsx-formula-provenance`
- Status: proposed (SDD propose phase, artifact store: openspec)
- Inputs: `openspec/changes/2026-09-19-xlsx-formula-provenance/exploration.md`,
  GitHub issue #182 acceptance criteria (as quoted by the parent context),
  parent-resolved product decisions (authoritative; traceability table below),
  `openspec/config.yaml`, versioning policy in `docs/json-output.md`, and the
  precedents `structured-text` v2 (`schemas/v2` + frozen v1), `pptx slides` v1
  (new contract), and the 400-line budget with maintainer-approved stacked-PR
  chaining established by #180.
- Delivery: `ask-on-risk` · review budget 400 changed lines · strict TDD
  (`cargo test`) · 95% line-coverage gate · `make compatibility-corpus-check`.

## Intent / problem

`oxdoc extract rows --format jsonl` and `visit_xlsx_rows*` publish **typed cell
values plus one boolean**. That boolean (`has_formula`) is the entire formula
provenance the contract offers, and it is not enough:

1. **The expression is lost.** `<f>` elements are detected only to set
   `CellState.has_formula`; the text between `<f>` and `</f>` is never captured
   (`crates/oxdoc-core/src/parsers/xlsx.rs`). A consumer can see *that* a cell is
   a formula but never *which* formula, so audit, lineage, and spreadsheet-QA
   workflows must re-open the workbook with a second tool and re-implement
   shared-formula resolution to recover it.
2. **Cached and uncached formulas are indistinguishable in the failing case.**
   A cell with `<f>` and no `<v>` collapses to `kind: "blank"`,
   `has_formula: true`. So does `<c><f>X</f><v></v></c>`, where a cached value
   genuinely exists but is empty. Worse, an uncached formula is *identical to a
   blank cell apart from one boolean*, so "this column has never been computed"
   and "this column is empty" cannot be separated. Downstream pipelines silently
   treat never-evaluated formulas as empty data.
3. **Shared and array formulas are opaque.** In real Excel workbooks most
   formulas are stored once as a `<f t="shared" si="N" ref="...">EXPR</f>`
   master, with `N-1` slave cells carrying only `<f t="shared" si="N"/>` and a
   cached `<v>`. Today every slave reports `has_formula: true` with no text and
   no indication that its expression lives elsewhere. Slaves written without a
   cache (openpyxl-style, or a file saved with "calculate on open") become
   `kind: "blank"` — a full data-integrity hazard for ingestion.
4. **The no-recalculation promise is implicit.** "Formulas are never
   recalculated" exists in `models.rs` doc comments and one line of
   `docs/formats/xlsx.md`, but the versioned JSON contract never states it. A
   consumer cannot tell from the payload whether a number is the stored cache or
   an oxdoc-computed result.

Users and situations: data-engineering pipelines ingesting workbooks into
warehouses (need to know whether a value is real data or an uncomputed formula),
finance/audit reviewers producing lineage reports (need the expression text),
spreadsheet QA and migration tooling (need to find shared-formula slaves whose
cache is stale or absent), and LLM/RAG ingestion of spreadsheets (needs to avoid
presenting a formula as its result). The urgency is proportional to how much of
a workbook is formula-driven and uncalculated; a single openpyxl-written report
today loses all of its derived columns into `blank`.

The value is that the typed rows contract becomes **self-describing about
formula provenance** — expression text, cache presence, and the explicit
no-recalculation guarantee — additively, behind a versioned contract, without
touching the frozen v1 schema, plain-text/CSV output, tabular conversion, or any
DOCX/PPTX path.

## Solution shape

### 1. Core model: one nested formula record, explicitly versioned

`crates/oxdoc-core/src/models.rs`:

```rust
/// A formula occurrence captured from a worksheet cell.
///
/// `expression` is the stored `<f>` text exactly as written in the workbook
/// (no recalculation and no reference rewriting). Shared-formula slave cells
/// carry the master's expression text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct XlsxFormula {
    pub expression: String,
    /// Whether the cell carried a cached `<v>` value alongside the formula.
    pub cached: bool,
}

pub struct XlsxCell {
    pub column_index: usize,
    pub value: XlsxCellValue,
    pub has_formula: bool,
    pub formula: Option<XlsxFormula>,
}
```

Decisions and justification:

- **Nested `XlsxFormula` instead of two flat fields.** A flat
  `formula: Option<String>` plus `formula_cache: bool` encodes one invariant
  ("the cache flag is meaningful only when a formula exists") that the compiler
  cannot enforce, and it adds two fields to a public struct that is not
  `#[non_exhaustive]`. Folding the flag into the formula record makes the
  invariant structural and future metadata (for example the shared `si`/`t`
  discriminator, if ever needed) an additive field on a struct that is marked
  `#[non_exhaustive]` from birth. Non-serde: `XlsxCell`/`XlsxFormula` are plain
  library types (no `Serialize` derive today); serde shape is a CLI concern.
- **`has_formula` stays, unchanged.** It remains "the cell contains an `<f>`
  element" — an XML-presence flag. It is deliberately redundant with
  `formula.is_some()` in the common case, because the two differ exactly where
  the expression is unresolved (see §3). Keeping it preserves rows-jsonl v1
  semantics, `oxdoc-tabular`, and every existing assertion.
- **`XlsxCell` is NOT marked `#[non_exhaustive]`.** Marking it would be a second
  and permanent source break: `XlsxCell` has no `Default` impl, so
  `..Default::default()` is unavailable and *every* external struct literal would
  become impossible in one step, with no constructor to migrate to. The field
  addition is already source-breaking for struct literals; one documented
  migration (`formula: None`) is a smaller, clearer cost than a permanent
  construction ban. The source break is versioned explicitly in `CHANGELOG.md`
  with a migration note (see Backwards compatibility). If the maintainers want
  `#[non_exhaustive]`, it should be a separate decision with a `Default` impl or
  a constructor — that is a question-round item, not an assumption here.
- **Typed semantics unchanged.** `XlsxCellValue` variants keep their existing
  meaning; the formula never influences the value. `XlsxCellValue::Blank` stays
  `Blank` even for an uncached formula.

### 2. Parser: capture `<f>` text and cache presence

`crates/oxdoc-core/src/parsers/xlsx.rs`:

- `CellState` gains `formula: String`, `in_formula: bool`,
  `formula_type: Option<String>` (the `<f t=...>` attribute), `formula_si:
  Option<String>`, and `had_value: bool`.
- `<f>` handling (both the `Event::Start` and `Event::Empty` arms):
  - `Event::Start` `<f>` → `has_formula = true`, `in_formula = true`, and read
    the unprefixed `t` and `si` attributes with the existing
    `crate::parsers::attr_value` helper (it already matches by local name, so
    the worksheet-default namespace prefix is irrelevant).
  - `Event::Empty` `<f .../>` → `has_formula = true`, then resolve immediately
    (shared slave or empty element; see §3). `in_formula` stays `false`.
  - `Event::End` `</f>` → `in_formula = false` (text already accumulated).
- `Event::Text`, `Event::CData`, and `Event::GeneralRef` inside `<f>` append to
  `cell.formula` using the same decoders as cell values
  (`append_decoded_xml_text` / `append_decoded_xml_reference`), so `&amp;`,
  `&lt;`, `&quot;`, numeric character references, and CDATA are decoded exactly
  as they are for `<v>`/`<t>`. Formula text must never append to `cell.value`,
  and value text must never append to the expression: the three buffers
  (`value`, `inline`, `formula`) are disjoint and driven by their own flags.
- `<v>` handling sets `had_value = true` on `Event::Start` **and**
  `Event::Empty` (`<v/>` is a present-but-empty cache).
- `push_typed_cell` populates the new field:

```rust
let formula = cell.has_formula.then(|| XlsxFormula {
    expression: resolved_expression,          // own text, or slave master text
    cached: cell.had_value,
});
```

- `formula.cached` is the **presence** of a `<v>` element in the cell, not the
  success of value decoding: `<c t="s"><f>..</f><v>999</v></c>` whose shared
  string index is out of bounds still has `cached: true` (the workbook did store
  a cache; the existing `W003` warning still explains the resolution failure).
  This keeps the flag a pure provenance fact and avoids coupling it to the
  type-resolution fallback chain.
- `has_formula` remains `true` whenever an `<f>` element was seen, including
  cells whose expression could not be resolved (§3). Documented invariant:
  `has_formula == false ⟹ formula == None`; the converse does not hold.
- Attributes `aca`, `dt2D`, `dtr`, `cm`, and `ref` shape/region expansion stay
  ignored — only `t` and `si` acquire meaning.

### 3. Shared and array formulas: bounded single-pass resolution

**Master registration.** When `<f t="shared" si="N">EXPR</f>` is parsed:

1. the cell's own `formula.expression` is `EXPR` (its own text, even if it is
   also registered);
2. `N → EXPR` is registered in a per-worksheet `BTreeMap<String, String>` keyed
   by the **raw `si` attribute text** (no numeric parsing, so an unparsable `si`
   simply never resolves);
3. registration is skipped when the map is at its memory bound (§ bound below);
4. the **first** registration for an `si` wins; a later duplicate does not
   overwrite it (deterministic, and consistent with the repo's "first reference
   wins" duplicate rule);
5. a master whose text is empty registers nothing (nothing to resolve).

**Slave resolution.** When `<f t="shared" si="N"/>` (empty element) is parsed:

- if `N` is in the map, the slave's `formula.expression` is the master's text.
  The text is **copied verbatim**: oxdoc does not translate relative references
  to the slave's coordinates and does not evaluate anything. The payload says
  "this is the stored expression for this shared group", which is a provenance
  statement, not a computed one.
- otherwise the expression is omitted (`formula: None`, `has_formula: true`) and
  one warning is emitted at the worksheet path with the exact wording
  `unresolved shared formula index '{si}': formula expression omitted`
  (one warning per affected cell — the same granularity the existing
  shared-string-index warnings use).
- Because the parser is single-pass, a slave that appears **before** its master
  is treated as unresolved (no buffering, no second pass). Real producers write
  the master first; the warning documents the malformed ordering instead of
  silently inventing text.

**Array formulas.** `<f t="array" ref="A1:A2">EXPR</f>` appears only on the
master cell: the master captures `EXPR` (and its own cache flag) exactly like a
normal formula, and the other cells of the `ref` region carry only `<v>`, so they
remain `has_formula: false`, `formula: None` — untouched. oxdoc does not
synthesize formulas for cells that contain no `<f>` element, because claiming a
formula for a cell that does not carry one would overstate provenance.
`t="dataTable"` and any other `<f>` with text capture their text as-is under the
same rule (text present ⇒ captured; `t="shared"` + `si` is the only special
case).

**Bound (consistent with existing resource limits).** A new crate-internal
constant follows the shared-string precedent:

```rust
pub(crate) const DEFAULT_SHARED_FORMULA_MEMORY_LIMIT: usize = 1024 * 1024; // 1 MiB
fn estimated_memory_cost(expression: &str) -> usize {
    expression.len().saturating_add(ESTIMATED_INDEX_MEMORY_COST) // 16
}
```

1 MiB (an eighth of `DEFAULT_SHARED_STRING_MEMORY_LIMIT`) is justified by shape,
not by guesswork: each shared group stores **one** expression string regardless
of how many slave cells reference it, so the map grows with the number of
distinct formula groups, not with cell count. The limit is checked with
saturating arithmetic before inserting; when insertion would exceed it, that
entry is not recorded and — latched once per worksheet, so a pathological sheet
cannot flood stderr — one warning is emitted at the worksheet path with the
exact wording
`shared formula table limit reached: expressions beyond it are omitted`.
Cells whose expression is then unresolvable still produce the per-cell
`unresolved shared formula index '{si}'` warning, so affected data remains
identifiable. The bound is injectable through a crate-internal
`parse_sheet_rows_with_shared_formula_limit` seam (mirroring
`SharedStringStore::parse_with_memory_limit`) so the overflow path is unit
testable with a tiny limit instead of a megabyte fixture. `OoxmlLimits` is not
changed and no public option is added.

**No spill to disk.** Shared strings spill to temp files because a missing
shared string cannot be recovered; a missing shared **formula expression** can be
degraded with a warning while the cell, its `kind`, and its cached value are
still emitted. Adding a temp-file index for expression text would add I/O and
failure modes for no provenance gain.

**Warning classification.** Both new wordings classify as `custom`/`W999`
through the existing message-sniffing `OutputWarning::code()` (no matching
prefix exists). No `WarningCode` variant is added: `WarningCode` is a public,
**exhaustive** enum, so a new variant would be another source break for
consumers that match it, and the PPTX change (#180) set the precedent of adding
new skip wordings without new codes. Warnings continue to go to stderr for
rows-jsonl, so stdout stays a valid JSONL stream.

### 4. Versioned contract: rows-jsonl **v2**, not a new contract file

- New `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`, draft 2020-12, stable
  `$id` `.../schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`,
  `additionalProperties: false`, `const: 2` on `schema_version`, plus an exact
  mirror at `docs/schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`
  (`make docs-schemas-check` diffs `schemas/` against `docs/schemas/`).
- **Why v2 of the existing rows contract and not a new contract file:** the
  payload is the *same record* — same command, same envelope (`file`,
  `sheet_name`/`sheet_index`, `row_index`, `cells`), same cell discriminator
  (`kind`), same indices and sparse-cell rules. Only two optional cell fields
  are added. A new contract file (for example `oxdoc-xlsx-rows-formulas`) would
  duplicate the entire cell union, force every consumer to choose between two
  near-identical schemas, and create two places to change for any future cell
  kind. The `structured-text` v1→v2 bump is the exact precedent: same contract,
  new version, previous version frozen. The #180 "new contract gets its own
  schema file" precedent applies to genuinely new contracts (a slides payload is
  not a structured-text payload); a version bump of one existing contract is not
  that case.
- Additive cell fields on **all five** variants (`blank`, `string`, `boolean`,
  `number`, `error`), shared through `$defs.cellBaseProperties`:

```json
"formula": {
  "type": "string",
  "description": "Stored formula expression text. For shared formulas the master text is repeated on slave cells. Never recalculated or rewritten."
},
"formula_cached": {
  "type": "boolean",
  "description": "Present only with `formula`: true when the workbook stored a cached <v> value for the cell, false when no cache exists."
}
```

- Presence coupling is declared on the shared `cell` shape (not repeated per
  variant) so JSON cannot drift from the Rust invariant:

```json
"cell": {
  "allOf": [
    { "if": { "required": ["formula"] }, "then": { "required": ["formula_cached"] } },
    { "if": { "not": { "required": ["formula"] } }, "then": { "not": { "required": ["formula_cached"] } } }
  ],
  "oneOf": [ { "$ref": "#/$defs/blankCell" }, "…" ]
}
```

- `kind` semantics, required fields, sparse-cell rules, `not:
  {required: [sheet_name, sheet_index]}`, and every v1 field name/type are
  unchanged. v2 payloads differ from v1 payloads only by `schema_version: 2`
  and the two new optional cell fields. `v1` stays frozen and continues to
  validate previously captured v1 payloads.
- `crates/oxdoc-core/tests/schema.rs`: add `oxdoc-xlsx-rows-jsonl.schema.json`
  to the `v2` entry of `SCHEMA_VERSIONS`, add a representative v2 record test
  (one cached-formula number cell, one uncached formula `blank` cell, one error
  formula cell, one shared slave) asserting the declared fields and the
  presence-coupling invariant, and add a negative test asserting a v1-style
  payload (`schema_version: 1`, no `formula`) still fails v2 validation — the
  documented, intentional break that strict v1 consumers migrate for. If the
  hand-rolled validator cannot evaluate `allOf`/`if`/`then`, the coupling is
  asserted directly in Rust (presence iff) and the schema still declares it; the
  apply phase must extend the harness or state the limitation explicitly rather
  than dropping the constraint.
- New snapshot `tests/fixtures/snapshots/cli_xlsx_rows_v2_jsonl.jsonl`, built
  from the new `corpus/xlsx/formulas` tree with a fixed package name and
  byte-compared by a CLI test (the `pptx slides` JSONL snapshot is the
  precedent). Existing rows tests that assert `schema_version == 1` move to `2`.

### 5. CLI: two optional cell fields, version bump

`crates/oxdoc-cli/src/main.rs`:

```rust
struct RowsJsonlCell<'a> {
    column_index: usize,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")] raw: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")] value: Option<RowsJsonlValue<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")] formatted: Option<&'a str>,
    has_formula: bool,
    #[serde(skip_serializing_if = "Option::is_none")] formula: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")] formula_cached: Option<bool>,
}
```

- `TryFrom<&XlsxCell>` maps `cell.formula` into both fields (both present or both
  omitted — the flat DTO mirrors the nested model).
- `RowsJsonlRecord::schema_version` becomes `2`. There is **no**
  `--schema-version` flag and no dual emission: like structured-text v2, the
  payload version moves and the previous contract is frozen, not selectable. The
  `_ => Err(InvalidArgument)` catch-all for unknown `kind` values is unchanged.
- Field order keeps `has_formula` where it is and appends the new fields, so
  documented examples gain two trailing keys instead of being reflowed.
- The Python wrapper needs no code change (`extract_rows` passes CLI dicts
  through verbatim); one new `python/tests/test_oxdoc.py` case pins that a v2
  record carrying `formula`/`formula_cached` passes through unchanged.

### 6. Fixtures, provenance, and corpus chores

Hand-authored runtime-zipped corpus trees (no checked-in binary, matching the
`pptx` precedent and keeping `tests/fixtures/files/**` and
`tests/fixtures/compatibility-matrix.json` untouched — its entries require a
`sha256`, a snapshot, and a provenance note per checked-in binary):

- `tests/fixtures/corpus/xlsx/formulas/` — one sheet covering:
  - cached numeric formula: `<c r="B2"><f>SUM(B1:B1)</f><v>42</v></c>` →
    `formula: "SUM(B1:B1)"`, `formula_cached: true`, `kind: "number"`;
  - **uncached formula**: `<c r="C2"><f>SUM(C1:C1)</f></c>` →
    `kind: "blank"`, `has_formula: true`, `formula: "SUM(C1:C1)"`,
    `formula_cached: false`;
  - **empty cached value**: `<c r="D2"><f>IF(1=1,"","x")</f><v></v></c>` →
    `kind: "blank"`, `formula_cached: true` — the pair that removes the v1
    ambiguity;
  - **error formula**: `<c r="E2" t="e"><f>1/0</f><v>#DIV/0!</v></c>` →
    `kind: "error"`, `raw: "#DIV/0!"`, `formula: "1/0"`, `formula_cached: true`;
  - string-result formula (`t="str"`), a shared-string formula cell, and a
    formula carrying entity references (`&quot;`, `&amp;`, `&lt;`) to pin
    decoding;
  - an error cell with **no** `<f>` (control: `has_formula: false`,
    `formula` omitted).
- `tests/fixtures/corpus/xlsx/shared-formulas/` — one sheet covering:
  - master `<f t="shared" si="0" ref="A2:A4">SUM(B2:B4)</f>` with a cached
    value, plus a cached slave `<f t="shared" si="0"/>` and an **uncached**
    slave (both must carry the master text);
  - dangling slave `<f t="shared" si="9"/>` with no master → per-cell
    `unresolved shared formula index '9'` warning, cell still emitted;
  - slave-before-master ordering in a second sheet/region → same warning
    (documents single-pass semantics);
  - array master `<f t="array" ref="F2:F3">SUM(G2:G3)</f>` plus its region
    cell with only `<v>` → region cell stays non-formula;
  - mastered `si` values written with a namespace-prefixed sheet element to pin
    local-name attribute matching.
- Overflow is **not** a corpus fixture (it would need a megabyte of XML): it is
  covered by a `parsers::xlsx` unit test using the injectable
  `parse_sheet_rows_with_shared_formula_limit` seam with a tiny bound
  (mirroring the existing `SharedStringStore::parse_with_memory_limit(…, 6)`
  test), asserting the latched once-per-worksheet wording and that affected
  slaves still warn and still emit their cached values.
- Provenance notes `tests/fixtures/provenance/xlsx-formulas.md` and
  `tests/fixtures/provenance/xlsx-shared-formulas.md` following the required
  label set (`Source:`, `Producer:`, `Redistribution:`, `Purpose:`,
  `Sanitization:`) — the script `scripts/check-compatibility-corpus.py` enforces
  those labels for any manifest-listed fixture, and the repo convention is to
  state them for every new tree. Both names are added to the
  `fixture_provenance_notes_are_present` lists in
  `crates/oxdoc-core/tests/api.rs` and `crates/oxdoc-cli/tests/cli.rs`.
- `tests/fixtures/README.md` gains the two new corpus entries.
- `make compatibility-corpus-check` must stay green with **no** manifest entry,
  digest, snapshot-of-binary, or `tests/fixtures/files/**` change; the new trees
  are runtime-zipped sources only.
- The compatibility playground (`docs/compatibility-playground.md`,
  `scripts/compatibility-playground.py --check`) is an explicit allowlist;
  registering a new tree there would require another versioned CSV snapshot.
  This proposal does **not** register the new trees there (the CSV path does not
  change at all), and records that as a question-round item rather than an
  assumption.

### 7. Documentation and examples

- `docs/json-output.md` — schema table row for rows-jsonl pointing at
  `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`, the v1 row marked as frozen
  for previously captured payloads, update the sample record to v2 with the new
  fields, and state the no-recalculation guarantee plus the shared-formula
  provenance rule (master text repeated on slaves; no coordinate rewriting) and
  the uncached-formula case.
- `docs/formats/xlsx.md` — extend the current "Emits error cells (`t="e"`) and
  cached formula values…" line with the explicit guarantee: formulas are never
  recalculated, cached values are emitted as stored, and an uncached formula
  yields an empty CSV field (unchanged CSV behavior) while typed rows carry the
  expression and the missing-cache flag.
- `docs/cli.md` — `extract rows` section: `schema_version: 2`, the new optional
  cell fields, and the warning wordings for unresolved shared formulas.
- `docs/library-api.md` — typed-rows section: describe `XlsxFormula`, the
  `has_formula`/`formula` relationship, and the migration note for struct
  literals (`formula: None`).
- `README.md` — rows example gains the new fields.
- `CHANGELOG.md` — **Added** the two cell fields, expression capture, bounded
  shared-formula resolution, and the explicit no-recalculation statement;
  **Changed** rows-jsonl to schema v2 with the strict-v1-validator migration
  note (mirroring the structured-text v2 wording) and the `XlsxCell`
  source-breaking field with `formula: None` migration guidance.
- `python/tests/test_oxdoc.py` — one v2 pass-through case (no wrapper API
  change).

## Backwards compatibility

- **v1 schema frozen.** `schemas/v1/oxdoc-xlsx-rows-jsonl.schema.json` and its
  docs mirror are not modified; previously captured `schema_version: 1` payloads
  keep validating against them. New output is `schema_version: 2`.
- **Strict v1 consumers must migrate.** Because v1 sets
  `additionalProperties: false` and pins `schema_version` to `1`, every new
  payload fails v1 validation through its undeclared-field and `const` rules —
  the identical, documented break the structured-text v2 change made. Migration
  is to point validators at `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`. A v2
  payload for a formula-free workbook differs from its v1 payload only by
  `schema_version` (and no other key), so field-level consumers see no change.
- **Library API is source-breaking in exactly one place, documented.** Adding
  `XlsxCell.formula` breaks external struct literals. Migration: add
  `formula: None`, or read the field when constructing parser output. The change
  is recorded under **Changed** in `CHANGELOG.md`; the version bump itself is
  handled at release time. In-tree call sites
  (`crates/oxdoc-tabular/src/{xlsx_schema,parquet}.rs` test constructors,
  `crates/oxdoc-cli`, `crates/oxdoc-core` tests, doc examples) are updated in
  the same work unit. `XlsxCell` is intentionally not made `#[non_exhaustive]`
  (justified in §1).
- **Everything else is byte-identical.** Plain-text/CSV extraction (the
  `csv_value` path keeps mapping formula cells to the cached value or `""`),
  `slices`/DOCX/PPTX paths, `oxdoc-tabular` conversions and schema inference
  (value-only, formula-agnostic), `oxdoc info`, `oxdoc audit`, `infer schema`,
  warnings-on-stderr policy, `OoxmlLimits`, and the frozen `schemas/v1/**` and
  `schemas/v2/oxdoc-structured-text.schema.json` contracts. No existing
  snapshot may change: `xlsx_basic_csv.txt`, `xlsx_cell_types_csv.txt`,
  `xlsx_formatted_locale_csv.txt`, `xlsx_openpyxl_csv.txt`,
  `all_sheets_manifest.json`, and every DOCX/PPTX snapshot stay untouched.
- **No behavioral change for non-formula cells.** Cells without `<f>` serialize
  exactly as before apart from the envelope version.

## Acceptance-criteria mapping (issue #182)

| Acceptance criterion (as quoted) | How this proposal satisfies it |
| --- | --- |
| Versioned, backwards-compatible output/API model for formula provenance | rows-jsonl moves to `schema_version: 2` (`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` + docs mirror) with two additive optional cell fields; v1 stays frozen and valid for captured payloads; the `XlsxCell` field addition is explicitly versioned in `CHANGELOG.md` with migration guidance |
| Distinguish formula expression, cached value, and missing cache without recalculating | `formula` carries the stored `<f>` text (shared slave cells carry the master text verbatim), `formula_cached` reports whether a `<v>` cache existed, `value`/`kind` keep the stored cached value unchanged, and no evaluation library or recalculation is introduced |
| Document that formulas are never recalculated | Explicit guarantee in `docs/json-output.md`, `docs/formats/xlsx.md`, `docs/library-api.md`, and the v2 schema field descriptions; pinned by tests that a cached number equals the stored `<v>` and an uncached formula never becomes a number |
| Fixtures for cached / uncached / error / shared formulas | `corpus/xlsx/formulas` (cached, uncached, empty-cached, error, string-result, entity-decoding, non-formula error control) and `corpus/xlsx/shared-formulas` (master/slave cached and uncached, dangling `si`, slave-before-master, array master + region, prefixed elements), plus provenance notes and the provenance-presence test lists |
| Update JSON schemas, tests, examples | New v2 schema + docs mirror, `schema.rs` registration and representative/negative validation, new CLI JSONL snapshot, updated CLI rows tests, Python pass-through test, and `docs/json-output.md` / `docs/cli.md` / `docs/library-api.md` / `docs/formats/xlsx.md` / `README.md` / `CHANGELOG.md` examples |

## Scope / affected areas

Production:

- `crates/oxdoc-core/src/models.rs` — `XlsxFormula`, `XlsxCell.formula`.
- `crates/oxdoc-core/src/parsers/xlsx.rs` — `CellState` fields, `<f>`/`<v>`
  capture in Start/Empty/Text/CData/GeneralRef/End arms, shared-formula map +
  bound + injectable limit seam, two warning wordings, `push_typed_cell`
  mapping.
- `crates/oxdoc-cli/src/main.rs` — `RowsJsonlCell` fields, `schema_version: 2`.

Tests / fixtures:

- `crates/oxdoc-core/src/parsers/xlsx.rs` (unit) — capture, entity decoding,
  shared resolution, ordering, overflow bound, warning wording.
- `crates/oxdoc-core/tests/api.rs` — typed-rows API assertions per case, plus
  provenance-presence list.
- `crates/oxdoc-core/tests/schema.rs` — v2 registration, representative record,
  presence coupling, v1-payload fails v2.
- `crates/oxdoc-cli/tests/cli.rs` — rows JSONL v2 fields, snapshot byte-compare,
  stderr warnings, `schema_version: 2`, provenance-presence list.
- `crates/oxdoc-tabular/src/{xlsx_schema,parquet}.rs` — constructor updates
  only; behavior unchanged.
- `tests/fixtures/corpus/xlsx/{formulas,shared-formulas}/**`,
  `tests/fixtures/provenance/xlsx-{formulas,shared-formulas}.md`,
  `tests/fixtures/snapshots/cli_xlsx_rows_v2_jsonl.jsonl`,
  `tests/fixtures/README.md`.
- `python/tests/test_oxdoc.py`.

Docs/schemas:

- `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` + `docs/schemas/v2/…` mirror.
- `docs/json-output.md`, `docs/formats/xlsx.md`, `docs/cli.md`,
  `docs/library-api.md`, `README.md`, `CHANGELOG.md`.

## Non-goals (explicit)

1. **No recalculation and no formula evaluation.** No expression parsing, no
   dependency graph, no reference translation for shared slaves, no formula
   library dependency. `formula` is stored text, never a computed result.
2. **No changes to `schemas/v1/**` or `schemas/v2/oxdoc-structured-text.schema.json`.**
   No v3 structured-text, no new `TextBlock` field.
3. **No change to plain-text or CSV output** (`extract text`, `extract csv`,
   `--all-sheets` manifest). Formula cells keep rendering their cached value or
   an empty field; every CSV snapshot stays byte-identical.
4. **No change to `oxdoc-tabular` behavior** — `classify_cell`, Parquet
   conversion, and `infer schema` stay value-only and formula-agnostic (only
   test constructors gain `formula: None`).
5. **No DOCX or PPTX path changes**, no `XlsxCellValue` variant/semantics
   change, no `OoxmlLimits` or public-limit option change, no new dependency.
6. **No new CLI command, format, or `--schema-version` flag**; no
   `size:exception` and no chain strategy assumed by this proposal.
7. **No checked-in binary fixture and no compatibility-matrix change** in this
   change (an openpyxl-generated formula workbook is deferred; see Alternatives
   12 and the question round).
8. **No playground registration** for the new corpus trees.

## Alternatives considered (rejected)

1. **A new contract file (`schemas/v1/oxdoc-xlsx-rows-formulas.schema.json`)
   instead of v2 of the existing rows contract.** Rejected — the record, the
   command, the envelope, the `kind` discriminator, and the sparse-cell rules are
   identical; only two optional cell fields are added. A separate file would
   duplicate the five-variant cell union, force consumers to pick between two
   near-identical schemas, and create two maintenance sites for every future cell
   change. The `structured-text` v1→v2 bump is the exact precedent for extending
   one contract; #180's "own schema file" rule is for genuinely new payloads.
2. **Ship only `formula` and keep relying on `kind: "blank"` + `has_formula` for
   the cache.** Rejected — it does not satisfy "distinguish cached from absent",
   and it leaves the empty-cached-value case indistinguishable from an uncached
   formula.
3. **Two flat fields `formula: Option<String>` + `formula_cache: bool`.**
   Rejected — encodes an unenforceable "flag only valid with a formula"
   invariant and widens a non-`non_exhaustive` public struct by two fields
   instead of one.
4. **`formula` as the only field, with `Option<Option<String>>` or a sentinel
   string for "no cache".** Rejected — nested options and sentinels are
   unreadable in three languages and push ambiguity into every consumer; the
   typed pair is self-documenting and directly serializable.
5. **Mark `XlsxCell` `#[non_exhaustive]`.** Rejected for this change — it is a
   second, permanent break (no `Default` impl and no constructor exists), so
   external literals become impossible rather than merely needing one new field.
   Kept as an explicit question-round item instead of a silent decision.
6. **Add new `WarningCode` variants (for example `W007`) for the two wordings.**
   Rejected — `WarningCode` is a public exhaustive enum; adding variants breaks
   consumers that match it, and the #180 precedent classifies new skip wordings
   as `custom`/`W999` without touching the enum.
7. **Spill the shared-formula table to temporary files instead of bounding it.**
   Rejected — a missing expression degrades to a warned, still-emitted cell with
   its cached value intact, unlike a missing shared string; temp-file plumbing,
   I/O, and cleanup failure modes are not justified by provenance text.
8. **Unbounded shared-formula map.** Rejected — the parser's memory conventions
   exist precisely to keep peak memory a function of limits rather than of input
   size, and unbounded per-workbook state would break the documented memory
   notes in `docs/formats/xlsx.md`.
9. **Warn once per worksheet for unresolved slaves (dedupe) instead of per
   cell.** Rejected as the default — per-cell is the existing
   shared-string-warning granularity and names each affected cell; the flood risk
   is handled by latching the *overflow* wording once per worksheet.
10. **Buffer rows until a master appears so out-of-order slaves resolve.**
    Rejected — it converts a streaming parser into a buffering one and breaks the
    documented streaming/memory contract for a malformed-ordering case.
11. **Translate the shared master expression into each slave's coordinates.**
    Rejected — that is formula rewriting (recalculation-adjacent), it would make
    oxdoc's output differ from the stored workbook, and it can be wrong for
    structured references and array semantics.
12. **Synthesize `formula` (and `formula_cached`) for array-formula region cells
    from the master's `ref` range.** Rejected — those cells contain no `<f>`
    element, so claiming a formula would overstate provenance and change
    `has_formula` for cells whose XML never carried one.
13. **Add an openpyxl-generated `files/xlsx/openpyxl-formulas.xlsx` to the
    compatibility matrix now.** Deferred, not rejected — openpyxl is the natural
    real-producer witness for the uncached case, but it cannot produce shared
    formulas or `t="e"` cached results, and it adds a generator change, a checked-in
    binary, a provenance note, a manifest entry with `sha256`, a CSV snapshot, and
    integration tests. That is a separate work unit; see the question round.
14. **Dual emission or a `--schema-version 1` compatibility flag.** Rejected —
    structured-text v2 established a single moving version with a frozen
    predecessor and no selector; two live shapes for one command would double the
    test and docs surface forever.
15. **Emit `formula_cached: null` when there is no formula.** Rejected — the
    versioned-contract rules say optional fields are omitted, never `null`
    (`docs/json-output.md`).

## Risks

- **Review budget (highest).** Honest sizing puts this change at roughly five
  times the 400-line budget (see estimate): two schema copies, two corpus trees,
  a snapshot, a public API change that ripples through three crates' test
  constructors, and five documentation surfaces. Mitigation: delivery is
  `ask-on-risk`, so apply must pause for a delivery decision before producing an
  oversized diff; the proposal recommends chained/stacked PRs along the work-unit
  boundaries below and assumes no `size:exception`.
- **Strict v1 consumer breakage is a real, if precedented, cost.** Any pipeline
  validating rows-jsonl against the v1 schema stops validating on upgrade.
  Mitigation: the identical structured-text v2 wording in `CHANGELOG.md`, an
  explicit migration note, a v2-only payload delta of `schema_version` for
  formula-free workbooks, and a `schema.rs` test that documents the break
  instead of discovering it in the field.
- **Silent-looking provenance gaps.** A slave whose expression could not be
  resolved still emits `has_formula: true` with no `formula`, so a consumer that
  ignores stderr sees a formula cell it cannot explain. Mitigation: the two
  warning wordings are spec-locked, the invariant is documented in the schema
  descriptions and `docs/json-output.md`, and the shared-formulas fixture pins
  both the warning and the still-emitted cached value.
- **`XlsxCell` source break reaching downstream struct literals.** Mitigation:
  one documented migration (`formula: None`), the nested struct keeps future
  metadata additive, and in-tree call sites are updated in the same work unit so
  the workspace never fails to compile mid-chain.
- **Warning noise on formula-heavy malformed files.** Mitigation: overflow warns
  once per worksheet (latched); the per-cell wording exists only for cells whose
  expression is genuinely unavailable, matching existing shared-string behavior.
- **Schema/harness drift.** The hand-rolled validator in `tests/schema.rs` may
  not evaluate `allOf`/`if`/`then`, so the presence coupling could be declared
  but unenforced locally. Mitigation: assert the coupling directly in Rust and
  state the harness limitation explicitly (or extend the harness) in the same
  work unit.
- **Fixture honesty.** Hand-authored XML must actually exercise the intended
  branches (the #180 `malformed-xml` fixture initially could not fire its
  warning). Mitigation: before writing production code, prove each new tree
  loads through `fixtures::build_package` and that the RED tests fail for the
  right reason; keep the fixtures declarative and small.
- **Coverage gate (95%).** New branches (slave resolution, dangling `si`,
  overflow latch, empty cache, entity decoding, error-formula) all need tests.
  Mitigation: the two corpora plus the injectable-limit unit test cover those
  branches; the estimate already budgets their tests inside each work unit.

## Rollback

Revert `XlsxFormula` and `XlsxCell.formula`, the `CellState`/parser capture and
shared-formula map, the two CLI fields and the `schema_version` bump, delete
`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` plus its `docs/schemas/v2` mirror,
the two corpus trees, the two provenance notes, the new snapshot, the `schema.rs`
registration, and the doc/CHANGELOG edits. Removing the new field restores the
previous struct exactly, so struct-literal consumers that migrated by adding
`formula: None` must remove it — a compiled-visible, mechanical revert with no
persisted state, no data migration, and no captured payload invalidated (nothing
validated against the v2 schema before it is published, and the frozen v1 schema
was never modified). CSV/plain-text behavior was never touched, so snapshots and
the compatibility corpus are unaffected throughout.

## Size estimate (vs 400-line review budget)

Raw line counts, then an honest 1.5–2× multiplier on test/fixture weight (the
post-#177/#180 lesson: hand-authored fixtures, schema copies, and
schema-validation tests reliably land far above their first estimate).

| Work unit | Content | Raw | Realistic |
| --- | --- | --- | --- |
| WU1 — fixtures + provenance | `corpus/xlsx/formulas` + `corpus/xlsx/shared-formulas` trees (~20 XML files), 2 provenance notes, `tests/fixtures/README.md`, 2 provenance-presence lists | ~215 | **~320–430** |
| WU2 — core model + `<f>` capture | `XlsxFormula`, `XlsxCell.formula`, `CellState` fields, Start/Empty/Text/CData/GeneralRef/End capture, `push_typed_cell` mapping, tabular + CLI + core/cli test constructor churn and case tests | ~335 | **~500–670** |
| WU3 — shared/array resolution + bound | map + first-wins + injectable limit seam + 2 wordings + latch, unit tests, API tests for slave/dangling/ordering/array/overflow | ~260 | **~390–520** |
| WU4 — v2 schema + validation | `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`, docs mirror, `schema.rs` registration + representative/negative tests + coupling assertion, new JSONL snapshot | ~370 | **~410–490** |
| WU5 — CLI + Python | `RowsJsonlCell` fields, `schema_version: 2`, CLI tests (fields, snapshot, stderr), Python pass-through test | ~175 | **~260–350** |
| WU6 — docs | `docs/json-output.md`, `docs/formats/xlsx.md`, `docs/cli.md`, `docs/library-api.md`, `README.md`, `CHANGELOG.md` | ~150 | **~190–260** |
| **Total** | | **~1,505** | **~2,070–2,720** |

The estimate is **~5–5.5× the 400-line budget**, so this change cannot ship as a
single PR without an explicit exception. Because delivery is `ask-on-risk`, the
apply phase must pause and ask for a delivery decision. The proposal recommends
**chaining**, with these review slices in dependency order (each independently
reviewable, and splittable further if a slice still exceeds 400 after the
multiplier):

1. **WU1 fixtures + provenance** — no production code; unblocks every later test.
   May split into one PR per corpus tree if it measures over budget.
2. **WU2 core model + capture** — the public API change (`XlsxCell.formula`) plus
   `<f>`/`<v>` capture and in-tree call-site migration; `formula` is populated
   but shared resolution is not yet attempted.
3. **WU3 shared/array resolution + bound + warnings** — resolution map, limit
   seam, the two spec-locked wordings.
4. **WU4 v2 schema + mirror + validation + snapshot** — validated against WU2/WU3
   real output.
5. **WU5 CLI mapping + Python pass-through** — `schema_version: 2` and the two
   optional fields.
6. **WU6 docs + CHANGELOG** — including the versioned-break migration note.

Honesty notes: WU2, WU3, and WU4 each sit at or above 400 realistic lines on
their own, so the delivery decision may need to split them (for example model vs
capture; resolution vs warnings; schema vs validation/snapshot) or to accept a
`size:exception` — either way that is a user/parent decision, not an assumption
of this proposal. The 95% coverage gate and `make compatibility-corpus-check`
apply to every unit; every new production branch lands with its test in the same
unit.

## Success criteria

1. `oxdoc extract rows <FILE> --format jsonl` emits `schema_version: 2`, and a
   formula cell additionally carries `formula` (the stored `<f>` text) and
   `formula_cached` (present only with `formula`), while every other field,
   ordering, and sparse-cell rule is unchanged.
2. A cached formula cell reports the stored `<v>` value unchanged with
   `formula_cached: true`; an uncached formula cell reports
   `kind: "blank"`, `has_formula: true`, `formula: "<text>"`,
   `formula_cached: false`; an empty-but-present cache reports
   `formula_cached: true` — the three cases are mutually distinguishable.
3. No value is ever recalculated: for the corpus fixtures, every emitted `raw`
   equals the workbook's stored `<v>` (or is absent for uncached formulas), and
   `SUM`/`1/0`-style expressions never produce a computed number.
4. Shared formulas resolve: slave cells (cached and uncached) carry the master's
   expression text verbatim with their own `formula_cached` value; a slave with
   an unknown `si` emits exactly
   `unresolved shared formula index '{si}': formula expression omitted` on
   stderr, keeps `has_formula: true`, omits `formula`, and still emits its cached
   value; the shared-formula table bound emits exactly
   `shared formula table limit reached: expressions beyond it are omitted` once
   per worksheet when it first refuses an entry (unit-tested via the injectable
   limit seam).
5. Array-formula masters capture their expression; region cells that contain no
   `<f>` stay `has_formula: false` with `formula` omitted.
6. `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` and its `docs/schemas/v2`
   mirror exist and are identical (`make docs-schemas-check`); the v2 schema is
   registered in `crates/oxdoc-core/tests/schema.rs`, validates a representative
   v2 record, rejects a `formula_cached` without `formula`, and rejects a
   v1-shaped payload; no `schemas/v1/**` file is modified.
7. The new JSONL snapshot (`tests/fixtures/snapshots/cli_xlsx_rows_v2_jsonl.jsonl`)
   is byte-compared by a CLI test, and rows warnings remain on stderr so stdout
   stays a valid JSONL stream.
8. Every pre-existing snapshot is byte-identical
   (`xlsx_basic_csv.txt`, `xlsx_cell_types_csv.txt`, `xlsx_formatted_locale_csv.txt`,
   `xlsx_openpyxl_csv.txt`, `all_sheets_manifest.json`, DOCX/PPTX snapshots), and
   `oxdoc-tabular` behavior is unchanged apart from constructor updates.
9. `python` `extract_rows` passes a v2 record with `formula`/`formula_cached`
   through unchanged (pinned by test); `make compatibility-corpus-check` passes
   with no manifest, digest, or `tests/fixtures/files/**` change, and both new
   provenance notes are present (pinned by the provenance-presence tests).
10. `docs/json-output.md`, `docs/formats/xlsx.md`, `docs/cli.md`,
    `docs/library-api.md`, `README.md`, and `CHANGELOG.md` document the v2
    contract, the two new fields, the shared-formula rule, the warning wordings,
    the v1→v2 migration, and the `XlsxCell` migration note.
11. `cargo test --workspace`, `cargo fmt --all -- --check`,
    `cargo clippy --workspace --all-targets -- -D warnings`, the 95%
    line-coverage gate, and `make compatibility-corpus-check` pass.
12. Realized changed lines stay within 400 per PR, **or** the apply phase paused
    for an explicit `ask-on-risk` delivery decision (chain split or
    `size:exception`) before producing an oversized or multi-area diff.

## Parent decision traceability

| Parent decision | Where honored |
| --- | --- |
| 1. rows-jsonl v2 additive optional fields: `formula: string`, `formula_cached: boolean` present only with `formula`; shared slaves carry the master text resolved via `si`; array formulas like their master expression; `kind` semantics unchanged; never recalculate; formulas never affect values | Solution shape §1–§4; Alternatives 2, 3, 4, 11, 12, 15; Success criteria 1–3, 5 |
| 2. Core API `XlsxCell` gains formula state; explicit versioning in `CHANGELOG` with migration for struct-literal consumers; pick the cleanest shape and justify; do not mark `non_exhaustive` unless trivially safe and justify | Solution shape §1 (nested `XlsxFormula`, `XlsxCell` stays exhaustive); Backwards compatibility; Alternatives 3, 5; Success criterion 10; question-round item 2 |
| 3. Bounded `si → text` map in the single-pass streaming parser; consistent with existing limits; overflow or dangling `si` yields skip-with-warning with exact stable wording | Solution shape §3 (1 MiB bound, injectable seam, first-wins, no spill, latched overflow warning + per-cell dangling warning, exact wordings); Alternatives 6–11; Success criterion 4; question-round item 1 |
| 4. Fixtures: hand-authored corpus additions for cached, uncached, error (`t="e"`), and shared formula cases, plus provenance and compatibility-corpus chores per repo tooling | Solution shape §6; Success criteria 3, 6, 8, 9; Alternatives 12, 13, 14; question-round item 3 |
| 5. Non-goals: recalculation, formula evaluation libraries, changing v1 schema files, changing plain-text output, DOCX/PPTX paths | Non-goals 1–8; Backwards compatibility; Success criteria 3, 8 |

## Proposal question round

The five product decisions above are parent-resolved and treated as
authoritative; this section does not re-open them. These are the remaining
product-facing refinements and assumptions that the spec/apply phases should not
silently bake in. Answer, correct, skip, or ask for a second round:

1. **Warning granularity and support burden.** Unresolved shared-formula slaves
   warn once per affected cell (matching the existing shared-string warning
   granularity), while a table overflow warns once per worksheet. On a workbook
   saved without cached values, that can mean one stderr line per formula cell
   for a large sheet. Is per-cell warning detail the right support/UX tradeoff,
   or should unresolved slaves warn once per worksheet with the affected-cell
   count in the message?
2. **`XlsxCell` shape and forward compatibility.** This proposal keeps
   `XlsxCell` exhaustive (so external literals need only `formula: None`) and puts
   the metadata on a new `#[non_exhaustive]` `XlsxFormula`. If maintainers prefer
   `XlsxCell` itself to be `#[non_exhaustive]`, that needs a `Default` impl or a
   constructor and changes the migration story — should it be reconsidered now or
   left as a future decision?
3. **Real-producer coverage.** Hand-authored corpus trees cover the branch matrix
   but no real producer writes the uncached case like openpyxl does, and no
   hand-authored tree proves Excel's shared-formula output shape. Should this
   change also add an openpyxl-generated workbook to the compatibility matrix
   (generator change, checked-in binary, provenance, `sha256`, CSV snapshot), or
   is that a follow-up?
4. **Uncached-formula reading in v2.** An uncached formula still reports
   `kind: "blank"` (v1 semantics preserved) and is distinguished only by
   `has_formula: true` + `formula_cached: false`. Should v2 instead introduce a
   distinguishable value state (for example a formula-without-value marker) for
   ingestion pipelines that must not treat it as empty data — accepting a
   semantic change bigger than an additive field?
5. **Empty-but-cached values.** `<c><f>X</f><v></v></c>` keeps `kind: "blank"`
   with `formula_cached: true`. Is "blank but cached" acceptable to consumers, or
   would an explicit `raw: ""` presence distinction be worth a larger contract
   change?
6. **Overflow behavior visibility.** The 1 MiB shared-formula bound degrades to
   warnings instead of failing. Confirm that silent-under-`--quiet` stderr
   degradation is acceptable for a large workbook, or whether an unresolved
   shared-formula table should be a hard error above the bound.
7. **Compatibility playground.** The new corpus trees are not registered in
   `docs/compatibility-playground.md` (which needs a versioned CSV snapshot per
   entry). Confirm deferral, or should the cached/uncached formula CSV behavior be
   showcased there now?
