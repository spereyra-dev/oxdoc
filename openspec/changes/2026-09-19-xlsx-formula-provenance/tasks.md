# Tasks — XLSX formula provenance in typed rows (issue #182)

- Change id: `2026-09-19-xlsx-formula-provenance`
- Status: tasks (SDD tasks phase, artifact store: openspec)
- Inputs: `openspec/changes/2026-09-19-xlsx-formula-provenance/proposal.md`,
  `openspec/changes/2026-09-19-xlsx-formula-provenance/specs/xlsx-formula-provenance/spec.md`,
  `openspec/changes/2026-09-19-xlsx-formula-provenance/design.md` (authoritative; honest
  re-slice into S1–S8, each reviewable and each intended under 400 realistic lines),
  `openspec/config.yaml`
- Execution: auto · artifact store openspec · delivery ask-on-risk · review budget 400
  changed lines · strict TDD (`cargo test`) · 95% line-coverage gate
- Slice order: **S1 → S2 → S3 → S4 → S5 → S6 → S7 → S8**, stacked to main; commit
  boundaries equal slice boundaries; `schema_version` stays `1` until S7 so no
  intermediate state lies to consumers.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1,225 raw → **~1,860–2,280 realized** (1.5–2× test/fixture multiplier on every slice) |
| 400-line budget risk | High (whole change) — S4 sits at the edge, S5 exceeds it in the pessimistic case |
| Chained PRs recommended | Yes — 8 stacked-to-main PRs |
| Suggested split | PR 1 (S1) → PR 2 (S2) → PR 3 (S3) → PR 4 (S4) → PR 5 (S5, pre-agreed S5a/S5b fallback) → PR 6 (S6) → PR 7 (S7) → PR 8 (S8) |
| Delivery strategy | ask-on-risk (pause and ask before S1; `exception-ok` is never inferred) |
| Chain strategy | stacked-to-main (recommended by design; binding confirmation at the apply-phase ask, otherwise `pending`) |

Per-slice estimates (raw × 1.5–2× multiplier on test/fixture weight — the post-#177/#180 lesson):

| Slice | Content | Raw | Realistic (1.5–2×) | 400-line budget risk |
| --- | --- | --- | --- | --- |
| **S1** | `tests/fixtures/corpus/xlsx/formulas/**` tree, provenance note, `tests/fixtures/README.md`, provenance lists | 110 | ~170–220 | Low |
| **S2** | `tests/fixtures/corpus/xlsx/shared-formulas/**` tree, provenance note, README, provenance lists | 105 | ~160–210 | Low |
| **S3** | core model: `XlsxFormula`, `XlsxCell.formula`, warning constructors, in-tree call-site migration, model unit tests | 140 | ~230–300 | Medium |
| **S4** | parser capture: `CellState` fields, `<f>`/`<v>` Start/Empty/Text/CData/GeneralRef/End arms, `had_value`, `push_typed_cell` mapping (own text only), unit + api tests | 190 | ~300–400 | Medium–High (at edge) |
| **S5** | shared resolution + bound: `SharedFormulaTable`, injectable limit seam, first-wins, dangling/ordering/overflow wordings + latch, unit + api tests | 200 | ~320–420 | **High (pre-agreed split)** |
| **S6** | schema v2: `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` + `docs/schemas/v2` mirror, `schema.rs` registration + `const` extension + representative/coupling/negative/identity tests | 200 | ~280–380 | Medium–High |
| **S7** | CLI + python: `RowsJsonlCell` fields, single `schema_version: 2` site, snapshot + byte-compare, stderr-warning test, rows test bump, python pass-through | 130 | ~210–290 | Medium |
| **S8** | docs + CHANGELOG: six documentation surfaces incl. both versioned breaks | 150 | ~190–260 | Low |
| **Total** | | **~1,225** | **~1,860–2,280** | **High** |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
```

Delivery notes (ask-on-risk, prose — not tasks):

- The apply phase must pause before S1 and confirm the eight-slice chained plan (or an
  explicit `size:exception`) with the maintainer. Nothing in this plan assumes an
  exception, and `exception-ok` is never inferred.
- Pre-agreed S5 fallback: if realized `additions + deletions` exceeds 400, split into
  **S5a** (`SharedFormulaTable` + default constant + injectable seam + unit tests) and
  **S5b** (warning emission + latch + api/CLI warning tests). The seam is clean because
  the two wordings are constructors added in S3 and the emission points are isolated in
  `register` and the `Empty` `<f/>` arm.
- Second fallback if S4 or S6 measured over budget: move S4's corpus api assertions into
  S5, or S6's snapshot-free harness tests into their own slice. Never relax the coverage
  gate or merge S8 docs into a code slice.

## Cross-slice guards (apply to every task below)

- **Frozen v1 contract.** No file under `schemas/v1/**` or `docs/schemas/v1/**` is
  modified or deleted; `schemas/v2/oxdoc-structured-text.schema.json` and its mirror are
  untouched. Verify with `git status --porcelain schemas/v1 docs/schemas/v1`.
- **Byte-identity regressions.** `tests/fixtures/snapshots/xlsx_basic_csv.txt`,
  `xlsx_cell_types_csv.txt`, `xlsx_formatted_locale_csv.txt`, `xlsx_openpyxl_csv.txt`,
  `all_sheets_manifest.json`, every DOCX/PPTX snapshot, and every `oxdoc-tabular`
  behavior test stay byte-identical. The only permitted existing-test edits are the
  rows `schema_version` assertions moving `1` → `2` in S7 (rows-jsonl only).
- **Stop-and-investigate.** Any unexpected snapshot churn, any diff under
  `tests/fixtures/files/**` or in `tests/fixtures/compatibility-matrix.json`, any new
  `sha256`/manifest requirement, or any newly failing existing test means STOP and
  report — never update the snapshot, regenerate a digest, or regenerate output.
- **Envelope honesty.** `schema_version` moves to `2` only in S7. S3–S6 must leave every
  payload at `1`, and no slice may emit a formula field in JSON before S7.
- **No blast radius.** No `XlsxCellValue` variant or semantics change, no `OoxmlLimits`
  or public-limit option change, no new dependency, no recalculation/evaluation code, no
  reference translation, no `--schema-version` flag, no new CLI command, no new
  `WarningCode` variant, no temp-file spill for formula expressions.
- **No playground registration.** The two new corpus trees are not added to
  `docs/compatibility-playground.md` or its CSV snapshot (deferral recorded).

---

## S1 — Fixtures + provenance: `formulas` corpus (~170–220 lines)

Spec coverage: Fixture corpus with provenance.

### RED

- [x] 1. Append `xlsx-formulas.md` to the provenance-presence list in
  `crates/oxdoc-core/tests/api.rs` (`fixture_provenance_notes_are_present`, ~line 2845)
  and to the list in `crates/oxdoc-cli/tests/cli.rs` (~line 2008). Run
  `cargo test -p oxdoc-core --test api fixture_provenance_notes_are_present` and
  `cargo test -p oxdoc-cli --test cli fixture_provenance_notes_are_present`; record the
  missing-note panic as RED. Implements: `xlsx-formula-provenance` → Fixture corpus with
  provenance → "Provenance and corpus gates stay green".
- [x] 2. Hand-author the runtime-zipped tree `tests/fixtures/corpus/xlsx/formulas/`
  (`[Content_Types].xml`, `_rels/.rels`, `xl/workbook.xml` with sheet `Data` and
  `r:id="rId1"`, `xl/_rels/workbook.xml.rels`, `xl/sharedStrings.xml` with `0 = "alpha"`
  and `1 = "text"`, `xl/worksheets/sheet1.xml`) using exactly the design §5.1 cell
  contract: `A1`/`B1`/`C1` shared-string header controls; `B2`
  `<c r="B2"><f>SUM(B1:B1)</f><v>2</v></c>`; `C2` `<c r="C2"><f>SUM(C1:C1)</f></c>`;
  `D2` `<c r="D2"><f>IF(1=1,&quot;&quot;,&quot;x&quot;)</f><v></v></c>`; `E2`
  `<c r="E2" t="e"><f>1/0</f><v>#DIV/0!</v></c>`; `F2` `t="str"` with
  `CONCATENATE(A2,&quot; &amp; &quot;,&lt;B2&gt;)`; `G2` `t="s"` with `LEN(A2)` and
  `<v>0</v>`; `H2` CDATA `IF(A2<>"","y","n")` with `<v>1</v>`; `I2` numeric reference
  `LEN&#40;A2&#41;` with `<v>5</v>`; `J2` `<c r="J2" t="e"><v>#N/A</v></c>` with no
  `<f>`. Implements: `xlsx-formula-provenance` → Fixture corpus with provenance →
  "Corpus covers the formula case matrix".
- [x] 3. Add `tests/fixtures/provenance/xlsx-formulas.md` with the required label set
  (`Source:`, `Producer:`, `Redistribution:`, `Purpose:`, `Sanitization:`) stating
  hand-authored material and the runtime-zipped convention (`no .xlsx binary is checked
  in`), and add the `xlsx/formulas` entry to `tests/fixtures/README.md`. Touch neither
  `tests/fixtures/compatibility-matrix.json` nor `tests/fixtures/files/**`. Implements:
  `xlsx-formula-provenance` → Fixture corpus with provenance → "Provenance and corpus
  gates stay green".

### GREEN / TRIANGULATE

- [x] 4. Add `xlsx_formulas_corpus_loads_and_carries_the_documented_cells` to
  `crates/oxdoc-core/tests/api.rs`: prove
  `fixtures::build_package("xlsx/formulas", "formula-provenance.xlsx")` loads, then assert
  the v1 baseline that exists today per cell (`has_formula: true` for `B2`–`I2`,
  `has_formula: false` for `J2`, `kind: "blank"` for `C2`/`D2`, `kind: "error"` for
  `E2`/`J2`, cached `raw` values unchanged). This loader + baseline is the fixture-honesty
  proof (the #180 `malformed-xml` lesson) that the tree really fires the intended branches
  before S4 writes production code. Implements: `xlsx-formula-provenance` → Fixture corpus
  with provenance → "Corpus covers the formula case matrix".

### Gate S1

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `make compatibility-corpus-check`, and the coverage gate
`cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only`.
Capture `git diff --stat -- tests/fixtures crates/oxdoc-core/tests/api.rs crates/oxdoc-cli/tests/cli.rs`
and compare against ~170–220 lines.

## S2 — Fixtures + provenance: `shared-formulas` corpus (~160–210 lines)

Spec coverage: Fixture corpus with provenance; Shared formula resolution; Array formulas.

Depends on S1 merged (provenance-list pattern and README convention established).

- [x] 5. RED: append `xlsx-shared-formulas.md` to both provenance-presence lists as in
  task 1 and record the missing-note panic from
  `cargo test -p oxdoc-core --test api fixture_provenance_notes_are_present` and
  `cargo test -p oxdoc-cli --test cli fixture_provenance_notes_are_present`. Implements:
  `xlsx-formula-provenance` → Fixture corpus with provenance → "Provenance and corpus
  gates stay green".
- [x] 6. Hand-author `tests/fixtures/corpus/xlsx/shared-formulas/` with a two-sheet
  workbook (`Shared` = `rId1`, `Prefixed` = `rId2`, both mapped in
  `xl/_rels/workbook.xml.rels`). Sheet 1 per design §5.2: `A2`
  `<f t="shared" si="0" ref="A2:A4">SUM(B2:B4)</f><v>6</v>` master; `A3` cached slave
  `<f t="shared" si="0"/><v>9</v>`; `A4` uncached slave `<f t="shared" si="0"/>`; `B2`
  dangling `<f t="shared" si="9"/><v>4</v>`; `F2` array master
  `<f t="array" ref="F2:F3">SUM(G2:G3)</f><v>3</v>`; `F3` region cell `<v>5</v>` only;
  `A5` first master `<f t="shared" si="1">FIRST()</f><v>1</v>`; `A6` duplicate master
  `<f t="shared" si="1">SECOND()</f><v>2</v>`; `A7` slave `<f t="shared" si="1"/>`.
  Sheet 2 uses namespace-prefixed elements throughout (`<x:worksheet xmlns:x="…">`,
  `<x:row>`, `<x:c>`, `<x:f …>`, `<x:v>`): `B1` slave-before-master
  `<x:f t="shared" si="0"/>`, `B2` later master `<x:f t="shared" si="0">PrefixedSum()</x:f>`,
  `B3` slave after master. Implements: `xlsx-formula-provenance` → Fixture corpus with
  provenance → "Corpus covers the formula case matrix", and → Shared formula resolution →
  "Slave resolves to master text verbatim", "Dangling si warns per cell and still emits
  the cell", "Slave before master is unresolved", "First registration wins".
- [x] 7. Add `tests/fixtures/provenance/xlsx-shared-formulas.md` with the required label
  set and the runtime-zipped/no-binary statement, and add the `xlsx/shared-formulas` entry
  to `tests/fixtures/README.md`. Implements: `xlsx-formula-provenance` → Fixture corpus
  with provenance → "Provenance and corpus gates stay green".
- [x] 8. TRIANGULATE: add `shared_formulas_corpus_loads_and_carries_the_documented_cells`
  to `crates/oxdoc-core/tests/api.rs` proving
  `fixtures::build_package("xlsx/shared-formulas", "shared-formulas.xlsx")` loads with both
  sheets and asserting today's v1 baseline (`has_formula: true` for `A2`/`A3`/`A4`/`B2`/
  `F2`/`A5`/`A6`/`A7`/`B1`/`B2`/`B3` on their sheets, `has_formula: false` for `F3`). S5
  replaces the baseline with resolution assertions. Implements: `xlsx-formula-provenance` →
  Fixture corpus with provenance → "Corpus covers the formula case matrix"; → Array
  formulas → "Array master captures expression, region cell stays non-formula".

### Gate S2

Same commands as Gate S1; compare `git diff --stat` against ~160–210 lines.

## S3 — Core model: `XlsxFormula` + `XlsxCell.formula` + call-site migration (~230–300 lines)

Spec coverage: Core API; Warning classification, channel, and no-recalculation guarantee.
Depends on S1/S2 (corpus available for invariant assertions). Behavior-neutral slice: no
capture, no warnings emitted yet, `schema_version` stays `1`.

### RED

- [x] 9. Add failing unit tests in the `#[cfg(test)]` module of
  `crates/oxdoc-core/src/models.rs`: `xlsx_formula_model_preserves_the_has_formula_invariant`
  (`has_formula == false` implies `formula == None`; an unresolved slave case keeps
  `has_formula: true` with `formula: None`; an uncached formula keeps `XlsxCellValue::Blank`)
  and `new_formula_warnings_classify_as_custom_w999` (both exact wordings →
  `code() == W999` / `category() == Custom`). Add an api assertion in
  `crates/oxdoc-core/tests/api.rs` that the corpus `B2` cell carries
  `formula.expression == "SUM(B1:B1)"` and `formula.cached`. Run
  `cargo test -p oxdoc-core` and record the compile failure (`XlsxFormula` undeclared) as
  RED. Implements: `xlsx-formula-provenance` → Core API → "Invariant holds in both
  directions where guaranteed"; → Warning classification → "Warning classification,
  channel, and no-recalculation guarantee".

### GREEN

- [x] 10. In `crates/oxdoc-core/src/models.rs`, add
  `#[derive(Debug, Clone, PartialEq, Eq)] #[non_exhaustive] pub struct XlsxFormula { pub expression: String, pub cached: bool }`
  with doc comments stating that `expression` is the stored `<f>` text (never recalculated,
  never rewritten, shared slaves carry the master text verbatim), and add
  `pub formula: Option<XlsxFormula>` as the **last** field of `XlsxCell` (after
  `has_formula`). Do **not** mark `XlsxCell` `#[non_exhaustive]` and do not change
  `has_formula` or any `XlsxCellValue` variant. Implements: `xlsx-formula-provenance` →
  Core API → "Migration is mechanical for struct literals", "Invariant holds in both
  directions where guaranteed".
- [x] 11. Add the two warning constructors beside the existing family in
  `crates/oxdoc-core/src/models.rs`, with byte-exact messages:
  `unresolved_shared_formula_index(path, si)` →
  `unresolved shared formula index '{si}': formula expression omitted`, and
  `shared_formula_table_limit_reached(path)` →
  `shared formula table limit reached: expressions beyond it are omitted`. Add no
  `WarningCode` variant. Implements: `xlsx-formula-provenance` → Warning classification →
  "Formulas are never recalculated" (classification half).
- [x] 12. Migrate every in-tree `XlsxCell` struct literal in this same commit so the
  workspace compiles and behavior is unchanged: `crates/oxdoc-core/src/parsers/xlsx.rs`
  `push_typed_cell` (~line 863) uses `formula: None` for now;
  `crates/oxdoc-tabular/src/xlsx_schema.rs` line ~557 and the `blank`/`string`/`boolean`/
  `number` helpers (~690–722); `crates/oxdoc-tabular/src/parquet.rs` line ~1538 and the
  `cell`/`number_cell` helpers (~1657–1665). `classify_cell`, Parquet conversion, and
  `infer schema` must not read `formula`. Run `cargo test --workspace` → GREEN. Implements:
  `xlsx-formula-provenance` → Core API → "Migration is mechanical for struct literals".

### TRIANGULATE

- [x] 13. Assert the invariant over real parser output for both corpus trees
  (`crates/oxdoc-core/tests/api.rs`): every cell with `has_formula: false` has
  `formula == None`, and no value/`kind` changed for any previously asserted cell.
  Confirm the `oxdoc-tabular` behavior tests pass unmodified. Implements:
  `xlsx-formula-provenance` → Core API → "Invariant holds in both directions where
  guaranteed".

### REFACTOR / Gate S3

- [x] 14. Keep exactly one construction path for `XlsxFormula` (parser mapping, S4); run
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, and the coverage gate; compare `git diff --stat` against
  ~230–300 lines and confirm no payload or snapshot changed.

## S4 — Parser capture: `<f>` text and `<v>` presence (~300–400 lines)

Spec coverage: Formula expression capture; Cache-presence provenance.
Depends on S3. This slice captures **own text only** — shared slaves remain
`formula: None` until S5.

### RED

- [x] 15. Add failing unit tests to the `#[cfg(test)]` module of
  `crates/oxdoc-core/src/parsers/xlsx.rs` (via the existing `parse_sheet_rows*` helpers):
  `captures_cached_formula_expression_and_cache_presence`,
  `decodes_formula_entities_cdata_and_numeric_references_like_cell_values` (same
  `&quot;`/`&amp;`/`&lt;`/`&#40;`/CDATA sequences decoded inside `<f>` and inside `<v>`),
  `keeps_formula_and_value_buffers_disjoint`, `marks_cache_presence_for_empty_v_element`,
  `keeps_cached_true_when_shared_string_index_is_out_of_bounds` (existing `W003` unchanged),
  `captures_empty_f_element_as_empty_expression`, and
  `leaves_non_formula_cells_untouched`. Run `cargo test -p oxdoc-core parsers::xlsx` and
  record RED. Implements: `xlsx-formula-provenance` → Formula expression capture →
  "Cached formula captures expression and value", "Entity and CDATA decoding in formulas",
  "Non-formula cells are untouched"; → Cache-presence provenance → all three scenarios.
- [x] 16. Extend `crates/oxdoc-core/tests/api.rs` with per-cell assertions over the
  `xlsx/formulas` corpus (`B2`, `C2`, `D2`, `E2`, `F2`, `G2`, `H2`, `I2`, `J2`) expecting
  the design §5.1 record shapes: `kind`/`raw`/`value` unchanged, plus `formula.expression`
  and `formula.cached` per cell. Run `cargo test -p oxdoc-core --test api` and record RED.
  Implements: `xlsx-formula-provenance` → Formula expression capture → "Cached formula
  captures expression and value"; → Cache-presence provenance → "Uncached formula is
  distinguishable from a blank cell", "Empty-but-cached value keeps cached semantics".

### GREEN

- [x] 17. Extend `CellState` in `crates/oxdoc-core/src/parsers/xlsx.rs` (~line 28) with
  `in_formula: bool`, `formula_buffer: String`, `formula: Option<String>`,
  `formula_type: Option<String>`, `formula_si: Option<String>`, `had_value: bool`.
  `formula: Option<String>` (not `String` + flag) is the design's refinement: `None` =
  no `<f>` or unresolved slave, `Some(text)` = stored expression possibly empty. Implements:
  `xlsx-formula-provenance` → Formula expression capture; → Shared formula resolution →
  "Dangling si warns per cell and still emits the cell".
- [x] 18. Implement the event arms inside the existing loop: `Start` `<f>` sets
  `has_formula = true`, `in_formula = true`, and reads `t`/`si` through
  `attr_value(&element, "t")` / `attr_value(&element, "si")` (local-name matching keeps
  prefixed sheets working); `Empty` `<f …/>` sets `has_formula = true` and, when not
  `t="shared"` with `si`, sets `formula = Some(String::new())` while leaving
  `in_formula` false; `Start` `<v>` **and** `Empty` `<v/>` set `had_value = true`;
  `Text`/`CData` route to `formula_buffer` when `in_formula` via
  `append_decoded_xml_text` and `GeneralRef` via `append_decoded_xml_reference`;
  `End` `</f>` sets `in_formula = false` and `formula = Some(formula_buffer)`.
  Keep `aca`, `dt2D`, `dtr`, `cm`, and `ref` unread. Implements:
  `xlsx-formula-provenance` → Formula expression capture → all three scenarios; →
  Cache-presence provenance → "Cache flag survives failed type resolution".
- [x] 19. In `push_typed_cell` (~line 772) map the resolved expression:
  `let formula = cell.formula.map(|expression| XlsxFormula { expression, cached: cell.had_value });`
  so `formula.cached` is pure XML `<v>` presence (`Event::Start` or `Event::Empty`), never
  the success of type resolution, and the value logic above it is untouched. Run
  `cargo test --workspace` → GREEN. Implements: `xlsx-formula-provenance` →
  Cache-presence provenance → "Cache flag survives failed type resolution"; → Warning
  classification → "Formulas are never recalculated".

### TRIANGULATE

- [x] 20. Prove the no-recalculation guarantee on the corpus: every emitted `raw` equals
  the stored `<v>` (and is absent for uncached formulas), `SUM`/`1/0` never yield a
  computed number, and `C2`/`D2` remain `kind: "blank"` while `J2` stays
  `has_formula: false`. Confirm `tests/fixtures/snapshots/xlsx_*_csv.txt` and
  `all_sheets_manifest.json` are byte-identical. Implements: `xlsx-formula-provenance` →
  Warning classification → "Formulas are never recalculated"; → Formula expression capture
  → "Non-formula cells are untouched".

### REFACTOR / Gate S4

- [x] 21. Keep the three buffers (`value`, `inline`, `formula`) disjoint with one routing
  flag each and no duplicated decode logic; run
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, the coverage gate, and `make compatibility-corpus-check`;
  compare `git diff --stat` against ~300–400 lines. If realized > 400, move the corpus api
  assertions to S5 (declared fallback) rather than shipping an oversized diff.

## S5 — Shared/array resolution + bounded table + warnings (~320–420 lines)

Spec coverage: Shared formula resolution; Bounded shared-formula table; Array formulas.
Depends on S4. Pre-agreed split if realized > 400: **S5a** (table + default constant +
injectable seam + unit tests) / **S5b** (warning emission + latch + api/CLI warning tests).

### RED

- [ ] 22. Add failing unit tests to `crates/oxdoc-core/src/parsers/xlsx.rs`:
  `resolves_cached_and_uncached_shared_slaves_verbatim`,
  `warns_per_cell_for_dangling_si`,
  `treats_slave_before_master_as_unresolved_without_buffering`,
  `first_registration_wins`, `registers_nothing_for_empty_master_text`,
  `latches_overflow_warning_once_per_worksheet` (drive
  `parse_sheet_rows_with_shared_formula_limit` with a tiny limit, mirroring the existing
  `SharedStringStore::parse_with_memory_limit(…, 6)` test), and
  `default_shared_formula_limit_is_one_mib_and_ooxml_limits_unchanged`. Run
  `cargo test -p oxdoc-core parsers::xlsx` and record RED. Implements:
  `xlsx-formula-provenance` → Shared formula resolution → all four scenarios; → Bounded
  shared-formula table → "Overflow latches once per worksheet", "Default bound follows
  existing memory conventions".
- [ ] 23. Add failing api assertions in `crates/oxdoc-core/tests/api.rs` over
  `xlsx/shared-formulas`: sheet `Shared` (`A2` master text + cache true, `A3` cached slave
  with master text verbatim and its own cache true and cached value emitted, `A4` uncached
  slave with master text and cache false, `B2` dangling `si=9` with `has_formula: true`/
  `formula == None`/`raw: "4"` retained plus exactly one
  `unresolved shared formula index '9': formula expression omitted` warning, `A5`/`A6`
  masters keep their own text, `A7` resolves to `FIRST()`, `F2` array master captures
  `SUM(G2:G3)`, `F3` stays `has_formula: false` with no formula state) and sheet
  `Prefixed` (`B1` unresolved, `B2` registers `PrefixedSum()`, `B3` resolves to it —
  local-name attribute matching). Run `cargo test -p oxdoc-core --test api` and record RED.
  Implements: `xlsx-formula-provenance` → Shared formula resolution → "Slave resolves to
  master text verbatim", "Dangling si warns per cell and still emits the cell", "Slave
  before master is unresolved", "First registration wins"; → Array formulas → "Array
  master captures expression, region cell stays non-formula".

### GREEN

- [ ] 24. Add `SharedFormulaTable { expressions: BTreeMap<String, String>, memory_bytes: usize, memory_limit: usize, overflow_warned: bool }`
  in `crates/oxdoc-core/src/parsers/xlsx.rs` keyed by **raw `si` attribute text** (never
  numerically parsed) with `register(&mut self, si, expression, path, warnings)` and
  `resolve(&self, si) -> Option<&str>`. `BTreeMap` keeps fuzz/replay order deterministic.
  Implements: `xlsx-formula-provenance` → Shared formula resolution → "First registration
  wins".
- [ ] 25. Implement `register` check order: (a) empty expression → register nothing;
  (b) `si` already present → return without overwriting (first wins); (c) saturating
  `memory_bytes + estimated_formula_memory_cost(expression) > memory_limit` → record
  nothing and, if `!overflow_warned`, push `shared_formula_table_limit_reached(path)` and
  set the latch; (d) otherwise insert and add the cost. Add
  `pub(crate) const DEFAULT_SHARED_FORMULA_MEMORY_LIMIT: usize = 1024 * 1024;` and
  `estimated_formula_memory_cost` (expression length saturating-add the same 16-byte
  per-entry constant used by `crates/oxdoc-core/src/parsers/xlsx_shared_strings.rs`). No
  temp-file spill. Implements: `xlsx-formula-provenance` → Bounded shared-formula table →
  "Overflow latches once per worksheet", "Default bound follows existing memory
  conventions".
- [ ] 26. Add the crate-internal seam
  `parse_sheet_rows_with_shared_formula_limit(source, path, shared_strings, format_context, sink, shared_formula_memory_limit)`
  mirroring `SharedStringStore::parse_with_memory_limit`; `parse_sheet_rows` delegates with
  `DEFAULT_SHARED_FORMULA_MEMORY_LIMIT`, and `visit_rows_with_read_options`,
  `write_sheet_csv`, and `fuzz_parse_sheet` keep using the default entry point so
  `OoxmlLimits` and the public API never change. Implements: `xlsx-formula-provenance` →
  Bounded shared-formula table → "Default bound follows existing memory conventions".
- [ ] 27. Wire emission points: `End` `</f>` registers when `formula_type == Some("shared")`
  and `formula_si` is present and the captured text is non-empty (the Start-opened master
  still keeps its own text); `Empty` `<f t="shared" si="N"/>` resolves immediately — hit ⇒
  `formula = Some(master_text.to_owned())` copied verbatim, miss ⇒ `formula = None` plus
  one `unresolved_shared_formula_index(path, si)` warning per affected cell; no buffering
  or second pass for slave-before-master. Run `cargo test --workspace` → GREEN. Implements:
  `xlsx-formula-provenance` → Shared formula resolution → "Slave resolves to master text
  verbatim", "Dangling si warns per cell and still emits the cell", "Slave before master is
  unresolved".

### TRIANGULATE

- [ ] 28. Verify the overflow aftermath and the warning channel:
  `shared_formula_table_limit_reached` appears exactly once per worksheet while affected
  slaves still warn per cell and still emit cached values; both new wordings classify
  `W999`/`custom`; and stderr warnings leave stdout a valid JSONL stream (extend
  `keeps_rows_jsonl_stdout_clean_when_warnings_are_emitted` in
  `crates/oxdoc-cli/tests/cli.rs` only if S7 has not landed the extension yet — otherwise
  defer to S7). Implements: `xlsx-formula-provenance` → Bounded shared-formula table →
  "Overflow latches once per worksheet"; → Warning classification → "Warnings stay on
  stderr".
- [ ] 29. Confirm streaming and memory claims: rows are still emitted one `ParsedRow` at a
  time, the only new cross-row state is the bounded table, and no `OoxmlLimits` field or
  public option changed. Implements: `xlsx-formula-provenance` → Bounded shared-formula
  table → "Default bound follows existing memory conventions".

### REFACTOR / Gate S5

- [ ] 30. Run `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, the
  coverage gate, and `make compatibility-corpus-check`; compare `git diff --stat` against
  ~320–420 lines and apply the S5a/S5b fallback if over budget before opening the PR.

## S6 — Rows-jsonl schema v2 + harness (~280–380 lines)

Spec coverage: Rows-jsonl schema version 2.
Depends on S4/S5 (validated against real output shapes; uses inline JSON records, so it
does not depend on S7's snapshot). `schema_version` stays `1` in CLI payloads here.

### RED

- [ ] 31. Add failing tests to `crates/oxdoc-core/tests/schema.rs`: register
  `"oxdoc-xlsx-rows-jsonl.schema.json"` under the `v2` entry of `SCHEMA_VERSIONS`
  (~line 253, v1 entry unchanged),
  `representative_xlsx_rows_v2_jsonl_record_matches_schema_shape` (cached-formula number
  cell, uncached formula `blank` cell, error formula cell keeping its cached error as
  `kind: "error"` with the stored error string as `raw` and the formula text captured, and
  a shared slave — asserting declared fields, the schema `const`, and presence coupling via
  the record), `xlsx_rows_v2_enforces_formula_presence_coupling` (direct Rust assertion
  that `formula` is present ⟺ `formula_cached` is present, with an inline comment stating
  that the hand-rolled harness does not evaluate `allOf`/`if`/`then` and that the coupling
  is enforced here instead — never silently dropped),
  `v1_rows_payload_fails_frozen_v2_validation` (`catch_unwind` pattern with
  `schema_version: 1` and no formula fields), and
  `xlsx_rows_v2_schema_and_mirror_are_identical`. Run
  `cargo test -p oxdoc-core --test schema` and record RED. Implements:
  `xlsx-formula-provenance` → Rows-jsonl schema version 2 → "Representative v2 record
  validates", "Presence coupling is enforced", "v1 payload fails v2 validation", "v1 stays
  frozen and mirror stays identical".

### GREEN

- [ ] 32. Extend `validate_against` (~line 446) and the cell path in `validate_cell`
  (~line 544) with the ~10-line `const` equality check: for each output field whose
  subschema declares `const`, assert equality. This is what makes the v1-payload-fails-v2
  scenario reachable (all v1 fields remain *declared* in v2) and strengthens every existing
  schema test. Implements: `xlsx-formula-provenance` → Rows-jsonl schema version 2 →
  "v1 payload fails v2 validation".
- [ ] 33. Create `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`: JSON Schema draft 2020-12,
  stable `$id` ending `/schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`,
  `additionalProperties: false`, `properties.schema_version` = `const: 2`;
  `$defs.cellBaseProperties` gains `formula` (string, description states the stored
  expression, shared master text repeated on slaves, never recalculated or rewritten) and
  `formula_cached` (boolean, description states "present only with `formula`"); all five
  variants (`blankCell`, `stringCell`, `booleanCell`, `numberCell`, `errorCell`) add both as
  optional `$ref`s; the shared `cell` shape declares presence coupling once via
  `allOf`/`if`/`then`. Every other v1 element (required fields, `kind` consts, `oneOf`,
  `not: {required: [sheet_name, sheet_index]}`, sparse-cell description) is byte-preserved.
  Implements: `xlsx-formula-provenance` → Rows-jsonl schema version 2 → "Representative v2
  record validates", "Presence coupling is enforced".
- [ ] 34. Copy it byte-identically to `docs/schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`
  and do **not** modify any file under `schemas/v1/**` or `docs/schemas/v1/**` (verify with
  `git status --porcelain schemas/v1 docs/schemas/v1`). Run `cargo test --workspace` and
  `make docs-schemas-check` → GREEN. Implements: `xlsx-formula-provenance` → Rows-jsonl
  schema version 2 → "v1 stays frozen and mirror stays identical".

### TRIANGULATE

- [ ] 35. Prove the additive delta: a v2 payload for a formula-free workbook differs from
  its v1 payload only by `schema_version` (assert on the `xlsx/basic` corpus record shape
  built inline), and the v2 schema declares `formula`/`formula_cached` on all five
  variants. Re-confirm `docs/schemas/v1` and `schemas/v1` are byte-frozen. Implements:
  `xlsx-formula-provenance` → Rows-jsonl schema version 2 → "v1 stays frozen and mirror
  stays identical".

### REFACTOR / Gate S6

- [ ] 36. Run `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `make docs-schemas-check`, and the coverage gate; compare `git diff --stat` against
  ~280–380 lines.

## S7 — CLI emission + Python pass-through (~210–290 lines)

Spec coverage: CLI emission of formula fields; Rows-jsonl schema version 2 → "Snapshot
byte-compare locks v2 output".
Depends on S6. This is the **only** slice that moves the payload envelope to `2`.

### RED

- [ ] 37. Add failing CLI assertions in `crates/oxdoc-cli/tests/cli.rs`:
  `extracts_sparse_typed_xlsx_rows_as_jsonl` (~line 616) moves its `schema_version`
  assertion to `2` and additionally asserts `formula`/`formula_cached` for its `TODAY()`
  cell; add `extracts_formula_provenance_as_rows_jsonl_v2` over
  `fixtures::build_package("xlsx/formulas", "formula-provenance.xlsx")` asserting per-cell
  field sets (cached cell carries both fields, uncached cell carries `formula_cached: false`
  with `kind: "blank"` and no `raw`); extend
  `keeps_rows_jsonl_stdout_clean_when_warnings_are_emitted` (~line 728) with the
  `xlsx/shared-formulas` package asserting the byte-exact dangling-`si` wording on stderr
  and stdout still parsing as JSONL; add
  `extracts_xlsx_rows_as_v2_jsonl_snapshot` byte-comparing stdout against
  `fixtures::read_snapshot("cli_xlsx_rows_v2_jsonl.jsonl")`. Add
  `test_extract_rows_passes_v2_formula_fields_through` to `python/tests/test_oxdoc.py`
  pinning verbatim pass-through of a v2 record with `formula`/`formula_cached`. Run
  `cargo test -p oxdoc-cli --test cli`, `python -m pytest python/tests`, and record RED.
  Implements: `xlsx-formula-provenance` → CLI emission of formula fields → "Cached formula
  cell carries both fields", "Uncached formula cell carries expression without cache",
  "Python pass-through is unchanged"; → Warning classification → "Warnings stay on stderr".

### GREEN

- [ ] 38. Add to `RowsJsonlCell` in `crates/oxdoc-cli/src/main.rs` (~line 1618), after
  `has_formula`: `#[serde(skip_serializing_if = "Option::is_none")] formula: Option<&'a str>`
  and `#[serde(skip_serializing_if = "Option::is_none")] formula_cached: Option<bool>`.
  Declaration order is the emitted byte order. Implements: `xlsx-formula-provenance` → CLI
  emission of formula fields → "Cached formula cell carries both fields".
- [ ] 39. Update `TryFrom<&XlsxCell>` (~line 1638) to derive both fields from the single
  nested value:
  `match &cell.formula { Some(formula) => (Some(formula.expression.as_str()), Some(formula.cached)), None => (None, None) }`
  so the two fields are structurally always-both-or-neither, and leave the `_ => Err(InvalidArgument)`
  catch-all for unknown `kind` values unchanged. Implements: `xlsx-formula-provenance` → CLI
  emission of formula fields → "Uncached formula cell carries expression without cache".
- [ ] 40. Change the **single** `RowsJsonlRecord { schema_version: 1, … }` construction site
  in `extract_rows_command` (`crates/oxdoc-cli/src/main.rs`, ~line 979) to
  `schema_version: 2`; leave the other `schema_version: 1` literals (text/tables/audit/
  slides payloads at ~lines 649/690/707/1762/1814/1829) untouched. No `--schema-version`
  flag and no dual emission. Implements: `xlsx-formula-provenance` → CLI emission of
  formula fields → "Cached formula cell carries both fields".
- [ ] 41. Generate and freeze
  `tests/fixtures/snapshots/cli_xlsx_rows_v2_jsonl.jsonl` once from the implementation
  output over `tests/fixtures/corpus/xlsx/formulas/` with the fixed package name
  `formula-provenance.xlsx` (so the `file` key is deterministic); review the content
  against the design §5.1 table before commit, then run
  `cargo test -p oxdoc-cli --test cli` → GREEN. Implements: `xlsx-formula-provenance` →
  Rows-jsonl schema version 2 → "Snapshot byte-compare locks v2 output".
- [ ] 42. Add the Python pass-through case body (no wrapper change in
  `python/src/oxdoc/client.py`), audit `python/tests/test_oxdoc.py` for hardcoded
  `schema_version: 1` rows payloads and move only those mirroring live CLI output to `2`,
  and run `python -m pytest python/tests` → GREEN. Implements: `xlsx-formula-provenance` →
  CLI emission of formula fields → "Python pass-through is unchanged".

### TRIANGULATE

- [ ] 43. Prove field order and determinism: the snapshot comparison locks
  `column_index, kind, raw?, value?, formatted?, has_formula, formula?, formula_cached?`;
  run the snapshot test twice and confirm identical bytes; confirm a formula-free workbook
  record differs from v1 only by `schema_version`; and confirm no other snapshot changed.
  Implements: `xlsx-formula-provenance` → Rows-jsonl schema version 2 → "Snapshot
  byte-compare locks v2 output".

### REFACTOR / Gate S7

- [ ] 44. Run `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `python -m pytest python/tests`, the coverage gate, and
  `make compatibility-corpus-check`; compare `git diff --stat` against ~210–290 lines.

## S8 — Documentation + CHANGELOG (~190–260 lines)

Spec coverage: Documentation and CHANGELOG versioning; Warning classification → "Formulas
are never recalculated" (documentation half).
Depends on S7 merged (docs describe the shipped v2 payload). Docs only — no Rust, no
schema file change.

- [ ] 45. Update `docs/json-output.md`: point the `oxdoc extract rows --format jsonl` row
  at `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`, keep the v1 link marked as frozen for
  previously captured payloads, update the sample record to `schema_version: 2` with the two
  trailing keys, and state the no-recalculation guarantee, the shared-formula provenance
  rule (master text repeated on slaves, no coordinate rewriting), and the uncached-formula
  case. Implements: `xlsx-formula-provenance` → Documentation and CHANGELOG versioning →
  "Docs state the no-recalculation guarantee".
- [ ] 46. Update `docs/formats/xlsx.md`: extend the current formula-cells line with the
  explicit guarantee that formulas are never recalculated and cached values are emitted as
  stored, that an uncached formula yields an empty CSV field (CSV behavior unchanged), that
  typed rows carry the expression and the missing-cache flag, and that CSV extraction now
  surfaces the new unresolved-shared-formula warnings on stderr while CSV bytes stay
  unchanged. Implements: `xlsx-formula-provenance` → Documentation and CHANGELOG
  versioning → "Docs state the no-recalculation guarantee".
- [ ] 47. Update `docs/cli.md` (`extract rows` section): `schema_version: 2`, the two
  optional cell fields, and both exact warning wordings. Implements:
  `xlsx-formula-provenance` → Documentation and CHANGELOG versioning → "Docs state the
  no-recalculation guarantee".
- [ ] 48. Update `docs/library-api.md` typed-rows section: describe `XlsxFormula`, the
  `has_formula`/`formula` relationship (including `has_formula: true` with
  `formula: None` for unresolved slaves), the no-recalculation guarantee, and the
  `formula: None` migration note for struct-literal consumers. Implements:
  `xlsx-formula-provenance` → Core API → "Migration is mechanical for struct literals".
- [ ] 49. Update `README.md` rows example so the new fields appear as two trailing keys,
  and add the `CHANGELOG.md` entry: **Added** the `formula`/`formula_cached` cell fields,
  expression capture, bounded shared-formula resolution, and the explicit
  no-recalculation statement; **Changed** the rows-jsonl move to schema v2 with the
  strict-v1-validator migration note (mirroring the structured-text v2 wording) and the
  `XlsxCell` source break with `formula: None` migration guidance; record the
  openpyxl-generated compatibility-matrix workbook as deferred. Implements:
  `xlsx-formula-provenance` → Documentation and CHANGELOG versioning → "CHANGELOG versions
  both breaks".

### Gate S8

- [ ] 50. Run `make docs-check docs-links docs-schemas-check`,
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, and the coverage gate; compare `git diff --stat` against
  ~190–260 lines.

## Cross-slice final guards

- [ ] 51. Confirm the frozen-v1 invariant on the whole change diff:
  `git diff --name-only` (and `--diff-filter=D`) lists no path under `schemas/v1/**` or
  `docs/schemas/v1/**`, and `schemas/v2/oxdoc-structured-text.schema.json` plus its mirror
  are untouched. Implements: `xlsx-formula-provenance` → Rows-jsonl schema version 2 →
  "v1 stays frozen and mirror stays identical".
- [ ] 52. Confirm the snapshot inventory: the only changed or added snapshot is
  `tests/fixtures/snapshots/cli_xlsx_rows_v2_jsonl.jsonl`; `xlsx_basic_csv.txt`,
  `xlsx_cell_types_csv.txt`, `xlsx_formatted_locale_csv.txt`, `xlsx_openpyxl_csv.txt`,
  `all_sheets_manifest.json`, and every DOCX/PPTX snapshot are byte-identical. Any other
  snapshot diff is a stop-and-investigate signal, not a mechanical update.
- [ ] 53. Confirm no manifest, digest, or binary churn: `git status --porcelain
  tests/fixtures/files tests/fixtures/compatibility-matrix.json` is empty and
  `make compatibility-corpus-check` passes, with both new provenance notes present in the
  `fixture_provenance_notes_are_present` lists of `crates/oxdoc-core/tests/api.rs` and
  `crates/oxdoc-cli/tests/cli.rs`. Implements: `xlsx-formula-provenance` → Fixture corpus
  with provenance → "Provenance and corpus gates stay green".
- [ ] 54. Run the full release gate once on the final stacked state:
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo llvm-cov --workspace --all-features --all-targets
  --fail-under-lines 95 --summary-only`, `python -m pytest python/tests`,
  `make compatibility-corpus-check`, and `make docs-schemas-check`. Implements:
  `xlsx-formula-provenance` → all requirements' verification clauses.
- [ ] 55. Preserve work-unit commit boundaries: each slice keeps its tests with its code,
  S3's `formula: None` migration lands in the same commit as the model field, S5 ships its
  wordings with their emission points, S7 is the only envelope bump, and S8 lands after
  S7 so no documentation describes an unshipped payload.
