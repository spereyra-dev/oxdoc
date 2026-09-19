# Exploration — xlsx-formula-provenance (#182)

Explored against: `main`; OpenSpec store; canonical specs `docx-extraction`,
`structured-text-schema`, `pptx-slide-extraction` (none govern XLSX typed rows
today — there is no XLSX rows domain in `openspec/specs` yet). Upstream
precedents: #179/#180 established "new contracts get their own schema file"
and the versioning policy; #180 (pptx-slide-extraction) is archived.

## 1. Goal restated

Distinguish, in the typed XLSX row output, (a) the formula expression,
(b) the cached value stored in the workbook, and (c) the case where a formula
exists but no cached value is present — **without recalculating anything** —
behind a versioned, backwards-compatible output/API contract, with fixtures
for cached, uncached, error, and shared formula cases.

## 2. Current formula handling (crates/oxdoc-core/src/parsers/xlsx.rs)

- `<f>` elements are detected in `parse_sheet_rows` in both Start and Empty
  event arms; they only set `CellState.has_formula = true`. The **formula text
  is never captured**: `Event::Text` is appended to `cell.value` only when
  `in_value` (`<v>`) or `in_inline_text` (`<t>` of inline strings) is set.
- No `<f>` attribute is read at all. In particular:
  - shared formulas `<f t="shared" si="0" ref="A1:A3">SUM(...)</f>` (master)
    and `<f t="shared" si="0"/>` (slaves, empty element, expression inherited
    from master) both just set `has_formula`;
  - array formulas `<f t="array" ref="...">EXPR</f>` likewise (only the master
    cell carries the element; region cells carry only cached `<v>` values);
  - `aca`, `dt2D`, `dtr`, `cm` attributes are ignored.
- Cached `<v>` resolution in `push_typed_cell` keys off the `<c t=...>` type
  only: `t="s"` → shared string, `t="b"` → boolean, `t="str" | "inlineStr"` →
  `XlsxCellValue::String`, `t="e"` → `XlsxCellValue::Error { raw }`, empty
  value → `Blank`, otherwise `Number { raw, formatted? }`.
- **Uncached formula collapse**: a cell with `<f>` and no `<v>` produces
  `value: Blank` with `has_formula: true`. In rows-jsonl this is emitted as
  `kind: "blank", has_formula: true` — distinguishable from an empty cell only
  through `has_formula`. Note the residual ambiguity: a formula whose cached
  string value is legitimately empty also lands on `Blank`.
- Error cells (`t="e"`) keep the cached error string (`#DIV/0!`, `#N/A`, …);
  they are almost always formula results but can exist without a formula.
- CSV output (`csv_value`/`write_csv_row`) renders the cached value or `""`;
  it has no formula awareness. **Must not change.**

## 3. Typed model and public API surface

- `crates/oxdoc-core/src/models.rs`: `XlsxRow { row_index, cells }`,
  `XlsxCell { column_index, value, has_formula }`, `XlsxCellValue`
  (`Blank | String { raw, value } | Boolean { raw, value } | Number { raw,
  formatted } | Error { raw }`).
  - `XlsxCellValue` is `#[non_exhaustive]`; **`XlsxCell` is not** — adding a
    field to `XlsxCell` is a source-breaking change for any external struct
    literal (in-tree call sites: tabular tests, cli mapping, core tests, docs
    examples). The AGENTS/spec rule requires the proposal to explicitly
    version any breaking change. Alternatives: add `formula: Option<String>`
    (breaking, needs versioning note) vs. a new wrapper/accessor API.
  - Doc comments already state the cached-value/no-recalculation semantics.
- Public extraction entry points (`lib.rs`): `visit_xlsx_rows*` (path/reader/
  limits/read-options, 6 fns) feed `XlsxRow` to visitors; CSV variants are
  independent. Any new expression capture lives in the parser so all
  `visit_*` callers see it.
- CLI (`crates/oxdoc-cli/src/main.rs`): `RowsJsonlRecord { schema_version,
  file, sheet_name?, sheet_index?, row_index, cells }` with `RowsJsonlCell {
  column_index, kind, raw?, value?, formatted?, has_formula }` via
  `TryFrom<&XlsxCell>`; `kind` ∈ `blank|string|boolean|number|error`, with a
  catch-all `_ => Err(InvalidArgument)` for future enum variants.
  Warnings go to **stderr** for rows-jsonl (stdout stays a valid stream).
- `crates/oxdoc-tabular`: `classify_cell` ignores `has_formula`; formula
  cells convert from cached values (locked by
  `parquet.rs::converts_formula_cells_from_cached_values`). `xlsx_schema.rs`
  inference is value-based only. **No behavioral change needed or wanted**
  beyond accommodating any new `XlsxCell` field in test constructors.
- Python wrapper: `src/oxdoc/client.py::extract_rows` passes CLI JSONL dicts
  through verbatim; `tests/test_oxdoc.py` fixtures hardcode
  `schema_version: 1` payloads. New optional fields flow through unchanged;
  docs/README examples may deserve updating (AC "update examples").

## 4. Schemas and versioning policy

- `schemas/v1/oxdoc-xlsx-rows-jsonl.schema.json` (draft 2020-12, stable `$id`,
  `additionalProperties: false`) is frozen; every cell variant requires
  `column_index`, `kind`, `has_formula`.
- Policy (docs/json-output.md §Versioned Schemas): new output fields ⇒ new
  schema version; the previous schema stays frozen and v1 payloads remain
  validatable. So adding `formula` (and any cache-presence marker) to the
  rows-jsonl payload ⇒ **`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`**,
  emitted with `schema_version: 2`. This is the same contract at a new
  version — the #180 "own schema file" precedent applies to new contracts,
  not to version bumps of an existing one (the v1/v2 structured-text bump is
  the exact precedent to follow).
- Mandatory chores for a v2 schema (from #180's checklist):
  - mirrored copy under `docs/schemas/v2/…` (Makefile `docs-schemas-check`
    diffs `schemas/` vs `docs/schemas/`);
  - `crates/oxdoc-core/tests/schema.rs` — add v2 filename to the versions
    table plus a representative payload test; the harness checks declared
    fields/types but does not enforce `const` discriminators locally
    (rows tests assert them in Rust instead);
  - `docs/json-output.md` — new table row + "XLSX Rows JSONL" section update
    (including the "never recalculated" statement, AC 2);
  - `docs/formats/xlsx.md` (currently: "Emits error cells (`t="e"`) and cached
    formula values as their stored workbook values" — extend with the
    explicit no-recalculation guarantee);
  - `docs/cli.md`, `CHANGELOG.md`, README/python examples if touched.
- Library types stay unversioned (documented convention); only the CLI
  payload carries `schema_version`.

## 5. Fixtures & corpus coverage

- `tests/fixtures/corpus/xlsx/` has `basic`, `formatted-locale`,
  `app-metadata` (source XML dirs built into zips by `fixtures::build_package`
  in tests). **None contains a `<f>` element** — formula coverage today is
  synthetic unit tests only:
  - `xlsx.rs::emits_typed_sparse_rows_to_sink`: `<c r="E3" s="0"><f>TODAY()</f>
    <v>44927</v></c>` → cached Number + `has_formula: true` (covers "cached");
  - `cli.rs` typed-rows test asserts `cells[4]["has_formula"] == true` and
    `schema_version == 1`;
  - `api.rs:1608` asserts `has_formula`.
- Missing fixtures vs the AC:
  - **uncached formula**: `<c r="A1"><f>SUM(B1:B2)</f></c>` with no `<v>`
    (openpyxl writes formulas without cached values by default — a natural
    producer; hand-crafted corpus XML is also fine and is the existing
    pattern);
  - **error formula**: `<c r="A1" t="e"><f>1/0</f><v>#DIV/0!</v></c>`;
  - **shared formula**: master with `t="shared" si="0" ref` + expression and
    slave cells with `<f t="shared" si="0"/>` (slaves currently emit
    `has_formula: true, kind: "blank"` when uncached);
  - **array formula**: `<f t="array" ref="...">EXPR</f>` master + region
    cells with only cached values.
- Compatibility corpus gate: `scripts/check-compatibility-corpus.py` +
  `tests/fixtures/compatibility-matrix.json` currently list one xlsx binary
  (`files/xlsx/openpyxl-basic.xlsx`) with required `provenance`, `sha256`,
  `capabilities`, `snapshot`. New binary fixtures need a provenance note
  under `tests/fixtures/provenance/xlsx-*.md` + manifest entry + digest; new
  `corpus/` source dirs need provenance updates only if registered in the
  playground/manifest (`docs/compatibility-playground.md` references
  `corpus/xlsx/basic` + snapshot `xlsx_basic_csv.txt`).

## 6. Gaps vs acceptance criteria

| AC | Status |
| --- | --- |
| Versioned, backwards-compatible output/API model | `has_formula` exists everywhere but there is no expression capture and no cache-presence distinction; rows-jsonl needs v2; `XlsxCell` field addition is source-breaking and must be versioned explicitly |
| Document that formulas are never recalculated | Partially implied in models.rs doc comments and docs/formats/xlsx.md line 19; not stated as a guarantee in docs/json-output.md |
| Fixtures: cached / uncached / error / shared formulas | Only "cached" exists, as a synthetic unit test; no corpus fixture contains any `<f>` element |
| Update JSON schemas, tests, examples | No v2 schema, no v2 representative tests, examples show v1 shape (docs/json-output.md sample, README, python tests) |

## 7. What must NOT change

- Plain-text/CSV extraction: `csv_value` maps formula cells to the cached
  value (or empty when uncached); snapshots lock this.
- Existing rows-jsonl output for non-formula cells and all v1 payloads
  (frozen v1 schema keeps validating; v2 only adds optional fields).
- `XlsxCellValue` variant semantics and the tabular/parquet conversion from
  cached values (`xlsx_schema` inference and the parquet test stay value-only).
- Shared-string parsing (`xlsx_shared_strings.rs`) and workbook/sheet
  selection behavior.
- Frozen v1 schemas and the structured-text v2 contract.

## 8. Risks / decisions to surface in the proposal

1. **`XlsxCell` API shape**: adding `formula: Option<String>` breaks external
   struct literals (struct is not `#[non_exhaustive]`). Options: accept and
   explicitly version it, make the struct `#[non_exhaustive]` (itself
   breaking for literals), or expose a new accessor/type. Must be decided at
   proposal time.
2. **v2 payload representation**: e.g. optional `formula` string on any cell
   kind + a cache-presence marker. Choices: reuse `kind` semantics and let
   uncached formulas stay `kind: "blank"` with `has_formula: true` (fully
   back-compatible shapes, only additive), vs. a new kind or `value_present`
   flag to remove the empty-string-vs-absent-cache ambiguity. Recommend
   additive-only.
3. **Shared formula resolution**: expression text lives only on the master
   cell; surfacing it per-cell for slaves requires buffering `si → text`
   across rows. The parser is single-pass streaming with bounded memory
   conventions (see shared-strings memory limit) — decide whether slaves get
   the resolved expression, a marker, or the master-only text. Array formulas
   have the same master/region split.
4. **Warning policy**: if a `<f t="shared" si>` references a missing master,
   decide warning vs. silent pass-through (rows warnings go to stderr; there
   is no per-line error contract in rows-jsonl).
5. **Review budget**: v2 schema + mirror (~120), parser + shared-formula map
   (~80–150), CLI mapping (~40), tests/fixtures/provenance/docs (~150) —
   lands at or near the 400-line `ask-on-risk` threshold; expect a size
   conversation at proposal time.
6. Compatibility-corpus chores (manifest entry, provenance note, digest) for
   any new binary fixture.

## 9. Recommended next step

Proceed to proposal with explicit decisions on §8 items 1–4; draft the delta
spec as a new XLSX rows domain (or extend a new `xlsx-rows` domain), pinning:
v2 additive schema, no-recalculation guarantee, fixture matrix (cached /
uncached / error / shared / array), and the versioned `XlsxCell` API change.
