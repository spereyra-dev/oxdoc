# Design — XLSX formula provenance in typed rows (2026-09-19-xlsx-formula-provenance)

Status: design (SDD design phase, artifact store: openspec)
Authoritative spec: `specs/xlsx-formula-provenance/spec.md` (9 requirements, 30 scenarios)
Proposal: `../proposal.md` (parent decisions 1–5 treated as fixed)

This design answers the eight questions posed by the parent: parser capture
integration, the shared-formula map, model/CLI/schema shapes, harness strategy,
fixtures, the strict-TDD test plan, work-unit slicing, and rollback/risks. All
changes stay inside `packages/coding-agent` scope equivalent here: `oxdoc-core`
(parser + models), `oxdoc-cli` (rows JSONL), `oxdoc-tabular` (constructors
only), fixtures, schemas, docs, and the Python wrapper's tests.

---

## 1. Parser capture: `<f>` in the existing cell state machine

### 1.1 Where it plugs in

`parse_sheet_rows` (`crates/oxdoc-core/src/parsers/xlsx.rs`) is a single
`read_event_into` loop with one `current_cell: Option<CellState>` and two
routing flags (`in_value`, `in_inline_text`). Formula capture adds a third
routing flag and two buffers to `CellState`:

```rust
#[derive(Debug, Default)]
struct CellState {
    value_type: Option<String>,
    column_index: Option<usize>,
    style_index: Option<usize>,
    value: String,            // existing: <v> / inline <t> text
    has_formula: bool,        // existing: an <f> element was seen
    in_value: bool,
    in_inline_text: bool,
    // new:
    in_formula: bool,                 // between <f> and </f>
    formula_buffer: String,           // raw accumulated <f> text (pre-resolution)
    formula: Option<String>,          // resolved expression: own text, master text, or None
    formula_type: Option<String>,     // raw t attribute (local name via attr_value)
    formula_si: Option<String>,       // raw si attribute (no numeric parsing)
    had_value: bool,                  // a <v> element was present (Start or Empty)
}
```

**Design decision — `formula: Option<String>` on `CellState`, not `String` +
separate flag.** The spec requires an unresolved shared slave to report
`has_formula: true` with `formula: None`. A plain `String` cannot represent
"unresolved" without a second boolean, and `Some("")` would collide with the
legitimate empty-expression case (`<f></f>`). `Option<String>` is the minimal
representation: `None` = no `<f>` or unresolved slave; `Some(text)` = the stored
expression, possibly empty. This refines the proposal's `formula: String`
sketch without changing any spec behavior.

### 1.2 Event-arm changes (all inside the existing match)

| Event | Current behavior | New behavior |
| --- | --- | --- |
| `Start` `<f>` | `cell.has_formula = true` | `has_formula = true`, `in_formula = true`, read `t` and `si` via `attr_value(&element, "t")` / `attr_value(&element, "si")` into `formula_type` / `formula_si` |
| `Start` `<v>` | `cell.in_value = true` | unchanged **plus** `cell.had_value = true` |
| `Empty` `<v/>` | (unhandled) | `cell.had_value = true` (present-but-empty cache) |
| `Empty` `<f …/>` | `cell.has_formula = true` | `has_formula = true`; then resolve immediately: if `formula_type == Some("shared")` and `formula_si` present → consult the shared-formula table (§2): hit ⇒ `formula = Some(master_text)`, miss ⇒ `formula = None` + per-cell warning. Non-shared empty `<f/>` ⇒ `formula = Some(String::new())`. `in_formula` stays `false` |
| `Text` / `CData` / `GeneralRef` | appended to `cell.value` iff `in_value \|\| in_inline_text` | same routing for value buffers; **new branch**: if `cell.in_formula`, append to `formula_buffer` using `append_decoded_xml_text` / `append_decoded_xml_reference` (identical decoders as `<v>`/`<t>`) |
| `End` `</f>` | (unhandled) | `in_formula = false`; if this was a Start-opened `<f>`: take `formula_buffer` as the cell's own expression `formula = Some(text)`; if `formula_type == Some("shared")` and `formula_si` present and the text is non-empty → register `si → text` in the table (first wins, bounded, §2). A Start-opened shared master keeps its own text even when it also registers |
| `End` `</v>` / `</t>` | flag reset | unchanged |

Attribute rules: `attr_value` matches by local name, so worksheet-namespace
prefixes (`<x:f t="shared" x:si=…>`-style sheets) resolve identically — pinned
by the prefixed sheet in the shared-formulas corpus. `aca`, `dt2D`, `dtr`,
`cm`, `ref` remain unread. `si` is never parsed as a number: an unparsable `si`
simply never matches a registration (raw-text keys, per spec).

### 1.3 Disjointness invariant

`in_value`, `in_inline_text`, and `in_formula` are mutually exclusive by XML
structure (`<v>`, `<t>`, `<f>` do not nest inside each other), and each text
event routes to exactly one buffer. Formula text can never append to
`cell.value` and vice versa. This is pinned by the entity/CDATA unit tests
comparing formula decoding against the same sequences decoded inside `<v>`/`<t>`.

### 1.4 `had_value` and cache semantics in `push_typed_cell`

`push_typed_cell` gains no parameters. Its mapping tail becomes:

```rust
let formula = cell.formula.map(|expression| XlsxFormula {
    expression,
    cached: cell.had_value,
});
row.set(XlsxCell {
    column_index: target_column,
    value,                       // unchanged logic, formula never influences it
    has_formula: cell.has_formula,
    formula,
});
```

`formula.cached` is the **XML presence** of `<v>` (`Event::Start` or
`Event::Empty`), never the success of type resolution: a
`<c t="s"><f>…</f><v>999</v></c>` with an out-of-bounds shared-string index
still reports `cached: true` while the existing `W003` warning explains the
failure (unit test). Documented invariant: `has_formula == false ⟹ formula ==
None`; the converse does not hold (only unresolved shared slaves).

### 1.5 Memory accounting: 1 MiB bound, degrade-with-warning, and the seam

- **Why not spill like shared strings (8 MiB)?** A missing shared *string*
  cannot be recovered — the emitted value would be silently wrong — so the
  store spills to temp files to stay correct. A missing shared *formula
  expression* degrades safely: the cell, its `kind`, and its cached value still
  emit, and the gap is named by a warning. Temp-file plumbing (two more `TempFile`
  handles, cleanup failure modes, `Drop` ordering) buys no provenance. This is
  proposal Alternative 7, confirmed here with the code-level reason: the
  spill machinery (`SharedStringStoreBuilder::spill_to_disk`) exists to protect
  *value correctness*; expression text has a built-in degrade path.
- **Why 1 MiB (an eighth of the 8 MiB shared-string budget)?** Shape, not
  guesswork: the map holds **one** entry per distinct shared-formula group
  (`si`), independent of how many slave cells reference it. Real sheets have
  far fewer distinct `si` groups than shared strings. The bound is checked with
  saturating arithmetic before insertion using the same cost model as shared
  strings:

```rust
pub(crate) const DEFAULT_SHARED_FORMULA_MEMORY_LIMIT: usize = 1024 * 1024;
const ESTIMATED_INDEX_MEMORY_COST: usize = 16; // same per-entry constant as shared strings

fn estimated_formula_memory_cost(expression: &str) -> usize {
    expression.len().saturating_add(ESTIMATED_INDEX_MEMORY_COST)
}
```

- **Injectable seam.** A crate-internal
  `parse_sheet_rows_with_shared_formula_limit(source, path, shared_strings,
  format_context, sink, shared_formula_memory_limit)`, mirroring
  `SharedStringStore::parse_with_memory_limit` exactly (same call shape, same
  testability goal). `parse_sheet_rows` delegates with
  `DEFAULT_SHARED_FORMULA_MEMORY_LIMIT`. `visit_rows_with_read_options`,
  `write_sheet_csv`, and `fuzz_parse_sheet` keep calling the default entry
  point — `OoxmlLimits` and the public API are untouched (non-goal 5). The
  overflow path is unit-tested with a tiny limit, never a megabyte fixture.

---

## 2. Shared formula map: bounded, first-wins, master-first

### 2.1 Data structure

A per-worksheet table owned by `parse_sheet_rows*` (one instance per sheet
parse — per-worksheet scope falls out naturally because `parse_sheet_rows`
processes exactly one sheet):

```rust
struct SharedFormulaTable {
    expressions: BTreeMap<String, String>, // raw si text → expression
    memory_bytes: usize,
    memory_limit: usize,
    overflow_warned: bool,                 // latch: once per worksheet
}

impl SharedFormulaTable {
    /// Registers si → expression. First registration wins; empty expressions
    /// register nothing; overflow records nothing and warns once (latched).
    fn register(&mut self, si: &str, expression: &str, path: &str,
                warnings: &mut Vec<OutputWarning>) { … }

    /// Master text for si, if registered.
    fn resolve(&self, si: &str) -> Option<&str> { … }
}
```

`BTreeMap` (not `HashMap`) keeps fuzz/replay behavior deterministic without
relying on hasher details; lookups are the only read path so the ordering never
affects output.

### 2.2 Exact emission points

1. **Master registration** — `End` `</f>` of a Start-opened `<f t="shared"
   si="N">EXPR</f>`: `register("N", "EXPR", path, &mut warnings)`. Order of
   checks inside `register`: (a) `expression.is_empty()` → return, nothing
   registered; (b) `si` already present → return (**first wins**, later
   duplicates never overwrite — consistent with the repo's first-reference-wins
   duplicate rule and pinned by a unit test); (c)
   `memory_bytes + estimated_formula_memory_cost(expression) > memory_limit`
   (saturating) → entry not recorded, and if `!overflow_warned` push
   `shared_formula_table_limit_reached(path)` and set `overflow_warned = true`
   (one warning per worksheet, so a pathological sheet cannot flood stderr);
   (d) otherwise insert and add the cost.
2. **Slave resolution** — `Empty` `<f t="shared" si="N"/>`:
   `resolve("N")` hit ⇒ `cell.formula = Some(master_text.to_owned())` — copied
   **verbatim**, no reference translation, no evaluation (provenance statement,
   not a computed one). Miss ⇒ `cell.formula = None` + one
   `unresolved_shared_formula_index(path, si)` warning **per affected cell**
   (same granularity as the existing shared-string-index warnings), pushed from
   the `Empty` arm where `path` and `warnings` are already in scope. The cell
   still flows through `push_typed_cell`: `has_formula: true`, `formula: None`,
   `kind` and cached value unaffected.
3. **Slave before master** — no special handling: the single-pass stream has
   not registered the master yet, so the slave takes path 2's miss branch and
   the later master still registers and captures its own text. No buffering, no
   second pass (proposal Alternative 10 rejected; the streaming/memory contract
   in `docs/formats/xlsx.md` is preserved).
4. **Overflow aftermath** — cells whose `si` was refused by the bound still
   miss in `resolve` and therefore still emit the per-cell unresolved warning;
   their cached values still emit. Both spec wordings appear exactly as locked.

### 2.3 Streaming with bounded memory

`visit_xlsx_rows*` stays streaming: rows are still handed to the sink one
`ParsedRow` at a time and never buffered; the only new cross-row state is the
table, whose size is bounded by `memory_limit` (1 MiB default, saturating
checks) and whose warning output is latched. Peak memory remains a function of
limits, not input size — the property `docs/formats/xlsx.md` documents for
shared strings now also holds for the formula table.

---

## 3. Model changes and serialization shapes

### 3.1 `crates/oxdoc-core/src/models.rs`

```rust
/// A formula occurrence captured from a worksheet cell.
///
/// `expression` is the stored `<f>` text exactly as written in the workbook
/// (never recalculated, never rewritten). Shared-formula slave cells carry
/// the master's expression text verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct XlsxFormula {
    pub expression: String,
    /// Whether the cell carried a cached `<v>` value alongside the formula.
    pub cached: bool,
}
```

`XlsxCell` gains `pub formula: Option<XlsxFormula>` as the last field.
`XlsxCell` is **not** marked `#[non_exhaustive]` (proposal §1: no `Default`
impl exists, so `non_exhaustive` would ban struct literals outright; one
documented `formula: None` migration is the smaller, reversible cost).
`has_formula`, `XlsxCellValue` variants, and all value semantics are unchanged.
Doc comments on both types state the no-recalculation guarantee and the
`has_formula`/`formula` invariant.

Warning constructors join the existing family in `models.rs` (keeping the
one-constructor-per-wording pattern):

```rust
pub fn unresolved_shared_formula_index(path: impl Into<String>, si: impl Into<String>) -> Self {
    Self::new(path, format!("unresolved shared formula index '{}': formula expression omitted", si.into()))
}
pub fn shared_formula_table_limit_reached(path: impl Into<String>) -> Self {
    Self::new(path, "shared formula table limit reached: expressions beyond it are omitted")
}
```

No `WarningCode` variant is added (public exhaustive enum — Alternative 6
rejected). Both messages fall through `code()`'s match to `Custom`/`W999` and
`category()` `Custom` naturally; a `models.rs` unit test pins both wordings'
classification so a future prefix cannot accidentally capture them. Warnings
keep flowing through the existing `emit_warnings` stderr path for rows-jsonl,
so stdout stays a valid JSONL stream.

### 3.2 CLI: flat fields at the record level (nested in core, flat in JSON)

`crates/oxdoc-cli/src/main.rs` keeps `XlsxFormula` nested in the core model but
serializes two flat optional fields on `RowsJsonlCell` — the DTO mirrors the
invariant "both present or both omitted" structurally, because both are derived
from the single `Option<&XlsxFormula>`:

```rust
#[derive(Debug, serde::Serialize)]
struct RowsJsonlCell<'a> {
    column_index: usize,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<RowsJsonlValue<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formatted: Option<&'a str>,
    has_formula: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula_cached: Option<bool>,
}

// TryFrom<&XlsxCell>:
let (formula, formula_cached) = match &cell.formula {
    Some(formula) => (Some(formula.expression.as_str()), Some(formula.cached)),
    None => (None, None),
};
```

**Byte-compare constraints.** Field order is serde struct declaration order;
`has_formula` keeps its position and the two new keys are appended, so
documented examples gain two trailing keys instead of being reflowed. No
`json!` macro is introduced and no `preserve_order` serde_json feature is
needed — the existing pptx-slides snapshot tests already lock this mechanism
("Field order is locked byte-for-byte by the snapshot comparison"). The new
`cli_xlsx_rows_v2_jsonl.jsonl` snapshot byte-compares the whole stream.

**`schema_version: 2` plumbing.** Exactly one construction site exists:
`extract_rows_command` (`crates/oxdoc-cli/src/main.rs`, the `RowsJsonlRecord {
schema_version: 1, … }` literal around line 979). It becomes `schema_version:
2`. No flag, no dual emission (Alternative 14 rejected). The `InvalidArgument`
catch-all for unknown `kind` values is unchanged. The other `schema_version:
1` literals in `main.rs` (text/tables/audit/slides payloads) are untouched.

**In-tree call-site migration** (same work unit as the model change):
`crates/oxdoc-tabular/src/xlsx_schema.rs` (four test constructors: `blank`,
`string`, `boolean`, `number`) and `crates/oxdoc-tabular/src/parquet.rs` (three
literals: line 1538 and the `cell`/`number_cell` helpers) each gain `formula:
None`; behavior is unchanged — `classify_cell`, Parquet conversion, and schema
inference stay value-only and never read `formula`. Core/CLI test constructors
and doc examples are updated in the same unit.

### 3.3 Python pass-through

`python/src/oxdoc/client.py::extract_rows` passes CLI JSONL dicts through
verbatim (`self._json(line, command)`); **no wrapper change**. One new
`python/tests/test_oxdoc.py` case pins that a v2 record carrying
`formula`/`formula_cached` passes through unchanged. Existing Python tests that
hardcode `schema_version: 1` rows payloads are audited: those mirroring live
CLI output move to `2`; pure hardcoded-fixture tests stay valid as-is.

---

## 4. Schema v2: structure and harness strategy

### 4.1 Full v2 structure

New file `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` + byte-identical
mirror `docs/schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` (`make
docs-schemas-check` diffs the trees). It is the v1 file with exactly these
deltas:

- `$id` → `…/schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`; `description`
  mentions schema version 2; `properties.schema_version` → `const: 2`.
- `$defs.cellBaseProperties` gains:

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

- Each of the five variants (`blankCell`, `stringCell`, `booleanCell`,
  `numberCell`, `errorCell`) adds to `properties` (not `required`):

```json
"formula": { "$ref": "#/$defs/cellBaseProperties/formula" },
"formula_cached": { "$ref": "#/$defs/cellBaseProperties/formula_cached" }
```

- Presence coupling is declared **once**, on the shared `cell` shape, so JSON
  cannot drift from the Rust invariant:

```json
"cell": {
  "oneOf": [
    { "$ref": "#/$defs/blankCell" },
    { "$ref": "#/$defs/stringCell" },
    { "$ref": "#/$defs/booleanCell" },
    { "$ref": "#/$defs/numberCell" },
    { "$ref": "#/$defs/errorCell" }
  ],
  "allOf": [
    { "if": { "required": ["formula"] },
      "then": { "required": ["formula_cached"] } },
    { "if": { "not": { "required": ["formula"] } },
      "then": { "not": { "required": ["formula_cached"] } } }
  ]
}
```

Everything else — required fields, `kind` consts, `not: {required:
[sheet_name, sheet_index]}`, `additionalProperties: false` at every level,
sparse-cell description — is byte-preserved from v1. `schemas/v1/**` and
`docs/schemas/v1/**` are frozen; `schemas/v2/oxdoc-structured-text.schema.json`
is untouched.

### 4.2 Harness strategy: minimal `const` extension + direct Rust coupling assertions

Decision: **direct Rust assertions for the `allOf`/`if`/`then` coupling, plus a
minimal `const`-checking extension to the hand-rolled validator.** Justification:

- The harness in `crates/oxdoc-core/tests/schema.rs` (`validate_against` /
  `validate_cell`) only checks required fields, declared properties, JSON
  types, and `$ref` resolution into `$defs.cellBaseProperties`. It does **not**
  evaluate `const`, `oneOf` exclusion, `not`, or conditionals.
- Building a conditional-subschema evaluator (`if`/`then`/`not`) for one
  coupling constraint is a disproportionate investment in throwaway test
  machinery; the direct assertion is clearer and precedented (rows tests
  already assert `const` values and key presence in Rust).
- **However**, the "v1 payload fails v2" scenario cannot be asserted without a
  `const` check: a v1 payload's fields are all still *declared* in v2 (they
  only differ in `schema_version: 1` vs `const: 2`), so the structured-text
  precedent (which failed on an *undeclared* field) does not transfer. A ~10-line
  extension to `validate_against` — for each output field whose schema declares
  a `const`, assert equality — makes the negative test work through the same
  `catch_unwind` pattern used by `v2_payload_fails_frozen_v1_validation`, and
  additionally strengthens every existing schema test (they all currently
  ignore consts). This extension is honest, small, and reusable.

Concrete `schema.rs` changes:

1. Register `"oxdoc-xlsx-rows-jsonl.schema.json"` under the `"v2"` entry of
   `SCHEMA_VERSIONS` in `schemas_have_stable_public_metadata` (v1 entry stays —
   both files now exist).
2. Extend `validate_against` (and the cell path in `validate_cell`) with the
   `const` equality check described above.
3. New test `representative_xlsx_rows_v2_jsonl_record_matches_schema_shape`:
   a v2 record with a cached-formula number cell, an uncached formula `blank`
   cell, an error formula cell (`t="e"` keeping its cached error value as
   `kind: "error"` with `raw` the stored error string and the formula text
   captured), and a shared slave; asserts `schema_version` matches the schema
   const, walks the five variant definitions, and directly asserts the coupling
   invariant on the record (`formula` present ⟺ `formula_cached` present).
4. New test `xlsx_rows_v2_enforces_formula_presence_coupling` (direct Rust,
   with an explicit comment stating the harness does not evaluate
   `allOf`/`if`/`then` and that the constraint is enforced here instead — never
   silently dropped): a cell with `formula_cached` but no `formula` must be
   rejected. Since this cannot fail through `validate_object`, the test asserts
   the coupling rule itself (presence iff) against the schema's declared
   properties and documents the harness limitation inline.
5. New test `v1_rows_payload_fails_frozen_v2_validation` (catch_unwind pattern):
   a v1-shaped payload (`schema_version: 1`, no `formula` fields) fails v2
   validation through the new `const` check — the documented, intentional break.
6. A test asserts the v2 and mirror files are byte-identical (cheap
   `include_str!`/`fs::read_to_string` comparison of both paths), guarding
   `make docs-schemas-check` locally.

---

## 5. Fixture plan

Both trees are hand-authored runtime-zipped sources under
`tests/fixtures/corpus/xlsx/`, loaded through `fixtures::build_package`
(`tests/fixtures/mod.rs` collects files recursively, sorts them, and zips —
deterministic). No checked-in binary, no `tests/fixtures/files/**` change, no
manifest/digest/snapshot-of-binary change; `make compatibility-corpus-check`
stays green with zero manifest edits. Overflow is **not** a fixture (a
megabyte of XML would be absurd); it is covered through the §1.5 limit seam.

### 5.1 `tests/fixtures/corpus/xlsx/formulas/` (single sheet `Data`)

Standard OOXML tree: `[Content_Types].xml`, `_rels/.rels`,
`xl/workbook.xml` (one sheet, `r:id="rId1"`),
`xl/_rels/workbook.xml.rels`, `xl/sharedStrings.xml` (0: `"alpha"`,
1: `"text"`), `xl/worksheets/sheet1.xml` with `sheetData`:

| Cell | XML | Expected record shape |
| --- | --- | --- |
| `A1`, `B1`, `C1` | `t="s"` shared strings 0/1/0 | header strings, no formula (control for `B2`/`C2`) |
| `B2` | `<c r="B2"><f>SUM(B1:B1)</f><v>2</v></c>` | `kind: "number"`, `raw: "2"`, `formula: "SUM(B1:B1)"`, `formula_cached: true` |
| `C2` | `<c r="C2"><f>SUM(C1:C1)</f></c>` | `kind: "blank"`, `has_formula: true`, `formula: "SUM(C1:C1)"`, `formula_cached: false` |
| `D2` | `<c r="D2"><f>IF(1=1,&quot;&quot;,&quot;x&quot;)</f><v></v></c>` | `kind: "blank"`, `formula: 'IF(1=1,"","x")'` (decoded `&quot;`), `formula_cached: true` — the empty-but-cached case |
| `E2` | `<c r="E2" t="e"><f>1/0</f><v>#DIV/0!</v></c>` | `kind: "error"`, `raw: "#DIV/0!"`, `formula: "1/0"`, `formula_cached: true` |
| `F2` | `<c r="F2" t="str"><f>CONCATENATE(A2,&quot; &amp; &quot;,&lt;B2&gt;)</f><v>alpha &amp; beta</v></c>` | `kind: "string"`, `formula` with decoded `&quot;`/`&amp;`/`&lt;`/`&gt;` |
| `G2` | `<c r="G2" t="s"><f>LEN(A2)</f><v>0</v></c>` | `kind: "string"`, `value: "alpha"` (shared-string index resolves), `formula: "LEN(A2)"`, `formula_cached: true` |
| `H2` | `<c r="H2"><f><![CDATA[IF(A2<>"","y","n")]]></f><v>1</v></c>` | `formula: 'IF(A2<>"","y","n")'` — CDATA decoding; `kind: "number"`, `raw: "1"` |
| `I2` | `<c r="I2"><f>LEN&#40;A2&#41;</f><v>5</v></c>` | `formula: "LEN(A2)"` — numeric character references |
| `J2` | `<c r="J2" t="e"><v>#N/A</v></c>` | control: `kind: "error"`, `has_formula: false`, no formula fields |

`<c r="A2">…</c>` with `<v>alpha-ish</v>`? No — A2 stays an empty plain cell;
`F2`'s cached string is independent. (The table above is the exact content
contract for the apply phase; every row of it maps to a named test assertion.)

### 5.2 `tests/fixtures/corpus/xlsx/shared-formulas/` (two sheets)

Tree: same skeleton, `xl/workbook.xml` declares `Shared` (`rId1`) and
`Prefixed` (`rId2`), rels map both, `xl/sharedStrings.xml` optional (keep
minimal), `xl/worksheets/sheet1.xml` (default namespace) and
`xl/worksheets/sheet2.xml` (namespace-prefixed elements).

**sheet1 — `Shared`:**

| Cell | XML | Expected |
| --- | --- | --- |
| `A2` | `<c r="A2"><f t="shared" si="0" ref="A2:A4">SUM(B2:B4)</f><v>6</v></c>` | master: `formula: "SUM(B2:B4)"`, `formula_cached: true`, `kind: "number"` |
| `A3` | `<c r="A3"><f t="shared" si="0"/><v>9</v></c>` | cached slave: `formula: "SUM(B2:B4)"` verbatim, `formula_cached: true`, cached value emitted |
| `A4` | `<c r="A4"><f t="shared" si="0"/></c>` | uncached slave: `formula: "SUM(B2:B4)"`, `formula_cached: false`, `kind: "blank"` |
| `B2` | `<c r="B2"><f t="shared" si="9"/><v>4</v></c>` | dangling slave: `has_formula: true`, `formula` omitted, `formula_cached: false`, cached `raw: "4"` still emitted, stderr exactly one `unresolved shared formula index '9': formula expression omitted` for this cell |
| `F2` | `<c r="F2"><f t="array" ref="F2:F3">SUM(G2:G3)</f><v>3</v></c>` | array master: `formula: "SUM(G2:G3)"`, `formula_cached: true` |
| `F3` | `<c r="F3"><v>5</v></c>` | array region cell: `has_formula: false`, no formula fields — no synthesis |
| `A5`, then `A6` | `<c r="A5"><f t="shared" si="7"/></c>` … later `<c r="A6"><f t="shared" si="7">X()</f></c>` on a later row? No — ordering case lives on sheet2; here instead: **first-wins**: `A5` master `<f t="shared" si="1">FIRST()</f><v>1</v>`, `A6` duplicate master `<f t="shared" si="1">SECOND()</f><v>2</v>`, `A7` slave `<f t="shared" si="1"/>` | slave resolves to `FIRST()` (first registration wins), both masters keep their own text |

**sheet2 — `Prefixed`** (every element prefixed, e.g. `<x:worksheet
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">`,
`<x:row>`, `<x:c>`, `<x:f …>`, `<x:v>`):

| Cell | XML | Expected |
| --- | --- | --- |
| `B1` | `<x:c r="B1"><x:f t="shared" si="0"/></x:c>` (slave **before** master) | unresolved: `has_formula: true`, `formula` omitted, per-cell warning `unresolved shared formula index '0': formula expression omitted` — documents single-pass semantics |
| `B2` | `<x:c r="B2"><x:f t="shared" si="0">PrefixedSum()</x:f><v>1</v></x:c>` | later master still registers and captures its own text |
| `B3` | `<x:c r="B3"><x:f t="shared" si="0"/></x:c>` | slave after master on the same (prefixed) sheet resolves to `PrefixedSum()` — pins local-name attribute matching under prefixes |

### 5.3 Provenance, README, and lists

- `tests/fixtures/provenance/xlsx-formulas.md` and
  `tests/fixtures/provenance/xlsx-shared-formulas.md` with the required label
  set (`Source:`, `Producer:`, `Redistribution:`, `Purpose:`,
  `Sanitization:`) — hand-authored, no third-party material.
- `tests/fixtures/README.md` gains both corpus entries.
- Both names are appended to the `fixture_provenance_notes_are_present` lists:
  `crates/oxdoc-core/tests/api.rs` (the `["xlsx-basic.md",
  "xlsx-app-metadata.md"]` list) and `crates/oxdoc-cli/tests/cli.rs`.
- The compatibility playground is explicitly **not** registered (proposal
  question-round item 7 — deferral recorded, no CSV snapshot churn).

### 5.4 How tests load them

`fixtures::build_package("xlsx/formulas", "formula-provenance.xlsx")` and
`fixtures::build_package("xlsx/shared-formulas", "shared-formulas.xlsx")`
produce the runtime zips; typed-rows API tests use `visit_xlsx_rows`, CLI tests
run the `oxdoc` binary against the paths. Before any production code lands
(WU1), each tree must be proven to load through `build_package` and to fail the
not-yet-written RED assertions for the right reason — the #180
`malformed-xml` lesson.

---

## 6. Test plan — strict TDD per requirement

Each requirement is developed RED→GREEN (`cargo test`), with the failing test
written first and verified to fail for the right reason. Layers: **unit**
(`parsers::xlsx` internal tests, synthetic XML via the existing
`parse_sheet_rows*` helpers), **api** (`crates/oxdoc-core/tests/api.rs`, corpus
trees), **cli** (`crates/oxdoc-cli/tests/cli.rs`, real binary), **schema**
(`crates/oxdoc-core/tests/schema.rs`), **python** (`python/tests/test_oxdoc.py`).

| Req | Unit | API | CLI | Schema | Python |
| --- | --- | --- | --- | --- | --- |
| R1 expression capture | cached formula captures text; `Event::Empty` `<f/>`; entity (`&quot;`/`&amp;`/`&lt;`), numeric-ref, and CDATA decoding equal to `<v>` decoding; non-formula cells untouched (incl. error-no-`<f>`); value/formula buffers disjoint | corpus `formulas` tree: per-cell assertions for B2/C2/D2/F2/G2/H2/I2/J2 | rows JSONL `formula` field per case; snapshot byte-compare | representative record includes captured expressions | pass-through includes `formula` |
| R2 `formula_cached` | `<v>` Start / `<v/>` Empty / absent; out-of-bounds `t="s"` index keeps `cached: true` + W003 unchanged; three cases mutually distinguishable | corpus D2/C2/B2 | `formula_cached` values; uncached cell has no `raw` | coupling assertions | pass-through includes `formula_cached` |
| R3 core API | invariant `has_formula == false ⟹ formula == None`; unresolved slave keeps `has_formula: true, formula: None` | typed-rows assertions | (via fields) | — | — |
| R4 shared resolution | master registers own text; slave resolves verbatim (cached + uncached); dangling `si` per-cell warning + cell still emitted; slave-before-master unresolved; first-wins | corpus `shared-formulas` sheet1/sheet2 assertions | stderr wording byte-exact; stdout valid JSONL | shared-slave cell in representative record | — |
| R5 bounded table | seam test with tiny limit: latch emits exactly one overflow wording per worksheet; affected slaves still warn per cell; default constant = 1 MiB; `OoxmlLimits` unchanged | — | — | — | — |
| R6 array formulas | master captures text; region cell with only `<v>` stays `has_formula: false` | corpus F2/F3 | — | — | — |
| R7 warnings channel/classification | `code()/category()` = W999/custom for both wordings | — | extends `keeps_rows_jsonl_stdout_clean_when_warnings_are_emitted`-style test with a dangling-`si` zip: stdout parses as JSONL, warnings on stderr | — | — |
| R8 schema v2 | — | — | — | registration, representative record, coupling (direct Rust + stated limitation), v1-payload-fails-v2 (via new `const` check), v2/mirror identity | — |
| R9 CLI emission | — | — | `schema_version: 2`; both-fields-together mapping; field order via snapshot; existing rows test at cli.rs:616 moves to `2` | — | pass-through test |
| R10 docs/CHANGELOG | — | — | — | — | — (doc review; `make docs-schemas-check` covers mirrors) |

**Byte-identity regressions (must stay green untouched, or move only their
`schema_version` assertion):** `xlsx_basic_csv.txt`,
`xlsx_cell_types_csv.txt`, `xlsx_formatted_locale_csv.txt`,
`xlsx_openpyxl_csv.txt`, `all_sheets_manifest.json`, every DOCX/PPTX snapshot,
`oxdoc-tabular` behavior tests, and the existing typed-rows CLI test (only its
`schema_version` line changes 1→2; non-formula cell field sets are asserted
unchanged). Python tests hardcoding v1 payloads are audited as in §3.3. Note:
CSV extraction of a workbook with unresolvable shared formulas will now emit
the new warnings on stderr (the parser is shared); CSV **bytes** are unchanged
and every existing CSV snapshot contains no `<f>`, so snapshots stay
byte-identical — this consequence is documented in `docs/formats/xlsx.md`.

---

## 7. Work-unit split (honest 1.5–2× multiplier applied)

The proposal's WU1–WU6 land at ~2,070–2,720 realistic lines total — ~5× the
400-line budget. Applying the multiplier per unit and re-slicing the three
units that sit at or above 400 (WU2, WU3, WU4) gives eight review slices, each
independently green (`cargo test --workspace`, fmt, clippy, coverage gate,
`make compatibility-corpus-check`) at its commit boundary:

| Slice | Content | Raw est. | Realistic est. |
| --- | --- | --- | --- |
| **S1** fixtures + provenance — `corpus/xlsx/formulas` tree, provenance note, README entry, provenance lists | 110 | ~170–220 |
| **S2** fixtures + provenance — `corpus/xlsx/shared-formulas` tree, provenance note, README entry, provenance lists | 105 | ~160–210 |
| **S3** core model — `XlsxFormula`, `XlsxCell.formula`, warning constructors, call-site migration (tabular ×7, cli, core/doc tests), models unit tests | 140 | ~230–300 |
| **S4** parser capture — `CellState` fields, Start/Empty/Text/CData/GeneralRef/End arms, `had_value`, `push_typed_cell` mapping (own text only; shared slaves still `formula: None`), unit + api tests for R1/R2/R6 own-text cases | 190 | ~300–400 |
| **S5** shared resolution + bound — `SharedFormulaTable`, seam, first-wins, dangling/ordering/overflow wordings + latch, unit tests (R4/R5) + api tests for the shared-formulas corpus | 200 | ~320–420 ⚠ |
| **S6** schema v2 — schema + mirror, `schema.rs` registration + `const` extension + representative + coupling + negative + identity tests | 200 | ~280–380 |
| **S7** CLI + python — `RowsJsonlCell` fields, `schema_version: 2`, snapshot file + byte-compare test, stderr-warning CLI test, rows test version bump, python pass-through | 130 | ~210–290 |
| **S8** docs + CHANGELOG — six doc surfaces + CHANGELOG (both versioned breaks) | 150 | ~190–260 |
| **Total** | | ~1,225 | ~1,860–2,280 |

Commit boundaries = slice boundaries; the two fixture slices (S1, S2) each stay
under 400 so no per-tree split is needed. **S5 is the one slice that may still
cross 400** in the pessimistic case (table + seam + three warning paths + their
tests); if it measures over budget at implementation time, pre-agreed fallback:
split into S5a (table + seam + unit tests) and S5b (warning emission + latch +
api/cli warning tests) — a clean seam because the wordings are constructors in
S3 and the emission points are isolated in `register`/the `Empty` arm. Delivery
remains `ask-on-risk`: apply pauses before S1 to confirm the chained
eight-slice plan (or an exception) with the maintainer; no chain strategy is
assumed by this design.

Dependency order is strict: S1–S2 unblock all tests; S3 compiles the workspace
with `formula: None` everywhere (no behavior change); S4 fills own-text
capture; S5 adds resolution + warnings; S6 validates the shape S4/S5 produce
(schema tests use inline JSON records so they do not depend on S7's snapshot);
S7 flips the CLI payload; S8 documents. `schema_version` moves to 2 only in S7 —
S3–S6 keep the CLI at v1 so no intermediate state lies to consumers.

---

## 8. Rollback and risks

### Rollback

Pure code revert, no persisted state: remove `XlsxFormula` and
`XlsxCell.formula`, the `CellState` capture fields and shared-formula table,
the two CLI fields and the `schema_version` bump, delete
`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` + mirror, the two corpus trees,
two provenance notes, the snapshot, and the doc/CHANGELOG edits. Migrated
struct literals (`formula: None`) become compile errors — mechanical, visible.
The frozen v1 schema was never modified, so no previously captured payload is
invalidated; CSV/plain-text behavior was never touched, so snapshots and the
compatibility corpus are unaffected throughout. Per-slice rollback is safe
because each slice compiles and passes the full gate independently, and
`schema_version` stays 1 until S7.

### Risks (ranked)

1. **Review budget** — even re-sliced, eight PRs is real orchestration cost.
   Mitigation: `ask-on-risk` pause before S1; maintainer-approved chaining per
   repo precedent (#180); S5 fallback split pre-agreed.
2. **Schema/harness drift** — the coupling could be declared but unenforced.
   Mitigation: §4.2 decision is explicit (direct Rust assertions + stated
   limitation + the small `const` extension), and the identity test keeps the
   mirror honest.
3. **Fixture honesty** — hand-authored XML may not fire the intended branch.
   Mitigation: §5 tables are the exact content contract; RED-before-GREEN per
   case; build-proof through `build_package` before production code.
4. **Strict v1 consumer breakage** — real but precedented (structured-text v2).
   Mitigation: CHANGELOG wording mirrors structured-text v2, the
   v1-payload-fails-v2 test documents the break, formula-free v2 payloads
   differ from v1 only by `schema_version`.
5. **`XlsxCell` source break** — one-field, documented (`formula: None`);
   in-tree sites updated in S3 so the workspace never fails mid-chain.
6. **Warning noise / stderr on CSV** — per-cell unresolved warnings on
   formula-heavy malformed files; CSV extractions now surface them on stderr.
   Mitigation: overflow latched once per worksheet; per-cell granularity matches
   the shared-string precedent and names each affected cell; documented in
   `docs/formats/xlsx.md`.
7. **Coverage gate (95%)** — every new branch (slave hit/miss, latch, empty
   cache, entity/CDATA, array, non-formula control) has a named test in §6
   inside its own slice.
8. **Snapshot determinism** — `file` in the snapshot is fixed by the test's
   `build_package` name argument; `collect_files` sorts entries; zip content is
   byte-stable. Verified once in S7 by running the test twice.
