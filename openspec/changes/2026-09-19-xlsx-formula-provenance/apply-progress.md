# Apply Progress — 2026-09-19-xlsx-formula-provenance

Artifact store: openspec · strict TDD (cargo test) · review budget 400 changed lines ·
chain strategy: stacked-to-main (PR 1 of 8), maintainer-confirmed via session preflight.

## S1 — Fixtures + provenance: `formulas` corpus (tasks 1–4) — COMPLETE

### TDD Cycle Evidence

| Task | Cycle | Test | RED evidence | GREEN evidence |
| --- | --- | --- | --- | --- |
| 1 | RED→GREEN | `fixture_provenance_notes_are_present` (oxdoc-core api, oxdoc-cli cli) | `cargo test -p oxdoc-core --test api fixture_provenance_notes_are_present` panicked: `NotFound` (`xlsx-formulas.md` missing); same panic in oxdoc-cli | Both pass after note authored (core: 1 passed; cli: 1 passed) |
| 4 | RED→GREEN | `xlsx_formulas_corpus_loads_and_carries_the_documented_cells` (oxdoc-core api) | `cargo test -p oxdoc-core --test api xlsx_formulas_corpus_loads` FAILED — fixture tree absent (`build_package` panic) before authoring | Passes after tree authored (1 passed; 0 failed) |

Fixture-honesty proof (the #180 `malformed-xml` lesson): the tree loads through
`fixtures::build_package("xlsx/formulas", "formula-provenance.xlsx")` and the typed-rows
baseline assertions exercise every intended cell branch (B2 cached number, C2 uncached
blank, D2 empty-but-cached blank, E2 error formula, F2 string result with decoded `&amp;`,
G2 shared-string index resolution, H2 CDATA, I2 numeric references, J2 non-formula error
control), before any S4 production code exists.

### Files changed

- `tests/fixtures/corpus/xlsx/formulas/[Content_Types].xml` (new)
- `tests/fixtures/corpus/xlsx/formulas/_rels/.rels` (new)
- `tests/fixtures/corpus/xlsx/formulas/xl/workbook.xml` (new, sheet `Data` r:id `rId1`)
- `tests/fixtures/corpus/xlsx/formulas/xl/_rels/workbook.xml.rels` (new)
- `tests/fixtures/corpus/xlsx/formulas/xl/sharedStrings.xml` (new, 0=`alpha`, 1=`text`)
- `tests/fixtures/corpus/xlsx/formulas/xl/worksheets/sheet1.xml` (new, design §5.1 matrix)
- `tests/fixtures/provenance/xlsx-formulas.md` (new, full label set incl. `Sanitization:`,
  runtime-zipped / no-binary statement)
- `tests/fixtures/README.md` (+1 line, `corpus/xlsx/formulas` entry)
- `crates/oxdoc-core/tests/api.rs` (provenance lists ×2, load-proof test)
- `crates/oxdoc-cli/tests/cli.rs` (provenance list)
- openspec change artifacts included in the commit

### Verification (Gate S1)

- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → all 8 suites ok (369 tests, 0 failures)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → exit 0 (gate passes)
- `python3 scripts/check-compatibility-corpus.py` (`make compatibility-corpus-check`) → "compatibility corpus validation passed (3 fixtures)"; note: `make` unavailable in this shell, invoked the recipe command directly
- `git status --porcelain schemas/v1 docs/schemas/v1 tests/fixtures/files tests/fixtures/compatibility-matrix.json` → empty (frozen guards hold; no manifest/digest/binary churn, no snapshot changes)
- Compatibility playground intentionally not registered (recorded deferral)

### Measured changed lines vs budget

`git diff main --numstat` excluding `.gitignore` (pre-existing local edit): 145 additions,
1 deletion, **146 total** — under the 400-line budget and inside the ~170–220 realistic
estimate for S1 (numstat counts the compact fixture XML lines; no code was compressed to
fit).

### Deviations from design

- None. `A2` intentionally stays an empty plain cell (design §5.1 note); warning list is
  empty because the corpus triggers no warnings at the v1 baseline.

### Remaining tasks

- S2: tasks 5–8 (`shared-formulas` corpus, provenance, load proof)
- S3: tasks 9–14; S4: tasks 15–21; S5: tasks 22–30; S6: tasks 31–36; S7: tasks 37–44;
  S8: tasks 45–50; cross-slice guards: tasks 51–55 (all unchecked)

---

## S2 — Fixtures + provenance: `shared-formulas` corpus (tasks 5–8) — COMPLETE

### TDD Cycle Evidence

| Task | Cycle | Test | RED evidence | GREEN evidence |
| --- | --- | --- | --- | --- |
| 5 | RED→GREEN | `fixture_provenance_notes_are_present` (oxdoc-core api, oxdoc-cli cli) | `cargo test -p oxdoc-core --test api fixture_provenance_notes_are_present` panicked: `NotFound` (`xlsx-shared-formulas.md` missing); same panic in oxdoc-cli | Both pass after note authored (core: 1 passed; cli: 1 passed) |
| 8 | RED→GREEN→TRIANGULATE | `shared_formulas_corpus_loads_and_carries_the_documented_cells` (oxdoc-core api) | FAILED before the tree existed — `build_package` panic (`NotFound`, path) at `tests/fixtures/mod.rs:23` | Passes after tree authored (1 passed; 0 failed) |

Fixture-honesty proof: the tree loads through
`fixtures::build_package("xlsx/shared-formulas", "shared-formulas.xlsx")` and the v1
baseline assertions exercise every intended branch before S5 production code exists:
A2 shared master (Number 6), B2 dangling `si=9` slave (Number 4, no warning yet), F2
array master (Number 3), A3 cached slave (Number 9), F3 array region control
(`has_formula: false`, Number 5), A4 uncached slave (Blank), A5/A6 first/duplicate
masters (Number 1/Number 2), A7 slave (Blank), and the prefixed sheet B1 (slave before
master, Blank), B2 (master, Number 1), B3 (slave, Blank) — all with `has_formula: true`
except F3, warnings empty at the v1 baseline.

### RED-cycle correction (fixture honesty in action)

The first GREEN attempt failed the load-proof: the initial sheet1.xml grouped A3/A4
and A5/A6/A7 onto shared `<row>` elements, so `ParsedRow::set` replaced same-column
cells (column A) and rows came out [0, 5] instead of [0, 1, 5]. A scratch debug run
showed the parser was correct — the fixture layout was wrong: `A3`/`A4`/`A5`/`A6`/`A7`
are rows 3–7 of column A (consistent with `ref="A2:A4"`). sheet1.xml was rewritten with
one row per cell address (rows r=2..7); the test was re-authored to assert the six rows
individually. No production code changed (S2 has none, by design).

### Files changed

- `tests/fixtures/corpus/xlsx/shared-formulas/[Content_Types].xml` (new)
- `tests/fixtures/corpus/xlsx/shared-formulas/_rels/.rels` (new)
- `tests/fixtures/corpus/xlsx/shared-formulas/xl/workbook.xml` (new, sheets `Shared`
  r:id `rId1`, `Prefixed` r:id `rId2`)
- `tests/fixtures/corpus/xlsx/shared-formulas/xl/_rels/workbook.xml.rels` (new, both
  worksheets mapped)
- `tests/fixtures/corpus/xlsx/shared-formulas/xl/worksheets/sheet1.xml` (new, design
  §5.2 `Shared` sheet; sharedStrings omitted — no `t="s"` cells, design marks it optional)
- `tests/fixtures/corpus/xlsx/shared-formulas/xl/worksheets/sheet2.xml` (new, `Prefixed`
  sheet, every element `x:`-prefixed)
- `tests/fixtures/provenance/xlsx-shared-formulas.md` (new, full label set incl.
  `Sanitization:`, runtime-zipped / no-binary statement)
- `tests/fixtures/README.md` (+1 line, `corpus/xlsx/shared-formulas` entry)
- `crates/oxdoc-core/tests/api.rs` (provenance lists ×2 + runtime-zipped list, load-proof
  test)
- `crates/oxdoc-cli/tests/cli.rs` (provenance list)
- openspec change artifacts (tasks.md marks, apply-progress.md) included in the commit

### Verification (Gate S2)

- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → all 8 suites ok, 0 failures (370 tests incl. the new one)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → exit 0 (gate passes)
- `python scripts/check-compatibility-corpus.py` (`make compatibility-corpus-check`; `make`
  unavailable in this shell, recipe invoked directly) → "compatibility corpus validation
  passed (3 fixtures)"
- `git status --porcelain schemas/v1 docs/schemas/v1 tests/fixtures/files
  tests/fixtures/compatibility-matrix.json tests/fixtures/snapshots` → empty (frozen guards
  hold; no manifest/digest/binary/snapshot churn)
- Compatibility playground intentionally not registered (recorded deferral)

### Measured changed lines vs budget

`git diff main --numstat` excluding `.gitignore` (pre-existing local edit, not committed):
157 additions, 1 deletion, **158 total** — under the 400-line budget and inside the
~160–210 realistic estimate for S2.

### Deviations from design

- `xl/sharedStrings.xml` omitted from the tree (design §5.2 marks it optional/keep
  minimal; no `t="s"` cell exists in either sheet, and the parser treats a missing
  sharedStrings part as an empty store). No other deviation.
- First-draft sheet1.xml packed column-A rows incorrectly (same fixture-honesty RED as
  documented above); corrected before GREEN. Final layout matches design §5.2 exactly.

### Remaining tasks

- S3: tasks 9–14; S4: tasks 15–21; S5: tasks 22–30; S6: tasks 31–36; S7: tasks 37–44;
  S8: tasks 45–50; cross-slice guards: tasks 51–55 (all unchecked)

---

## S3 — Core model: `XlsxFormula` + `XlsxCell.formula` + call-site migration (tasks 9–14) — COMPLETE

### TDD Cycle Evidence

| Task | Cycle | Test | RED evidence | GREEN evidence |
| --- | --- | --- | --- | --- |
| 9 | RED | `xlsx_formula_model_preserves_the_has_formula_invariant` + `new_formula_warnings_classify_as_custom_w999` (models unit) + corpus B2 api assertion | `cargo test -p oxdoc-core` compile failure as specified: `error[E0432]: unresolved import super::XlsxFormula` (no `XlsxFormula` in `models`), `error[E0560]: struct XlsxCell has no field named formula` (×3), `error[E0599]: no function ... unresolved_shared_formula_index / shared_formula_table_limit_reached`, `error[E0609]: no field formula on type XlsxCell` (×4) — lib test + `api` both failed to compile | See cycle note below |
| 10–12 | GREEN | same tests | — | Unit: 10 passed (2 new); `cargo test --workspace`: all 8 suites ok, 0 failures |
| 13 | TRIANGULATE | invariant loops added to both corpus tests (`has_formula == false` ⟹ `formula == None`) | — | Pass over real parser output for `xlsx/formulas` and `xlsx/shared-formulas` (both sheets incl. `Prefixed`) |
| 14 | REFACTOR/Gate | fmt, clippy, workspace, coverage | — | All clean (see Verification) |

Cycle note (task 9 api assertion — S4 boundary): the RED api assertion asserted
`cells[0].formula.expression == "SUM(B1:B1)"`, which cannot pass inside S3 by design —
S3 is the behavior-neutral slice (`push_typed_cell` uses `formula: None`; capture lands in
S4 tasks 17–19, and S4 task 16 re-adds the per-cell `formula.expression`/`formula.cached`
assertions as its own RED). Within S3 the assertion was re-scoped to the truthful baseline
`cells[0].formula.is_none()` with a comment pointing at S4 task 16. No S4 behavior was
implemented early; the corpus B2 expression assertion is unchanged work for S4.

### Files changed

- `crates/oxdoc-core/src/models.rs` (+109/−3): `#[non_exhaustive] XlsxFormula { expression, cached }`
  with doc comments (stored `<f>` text, never recalculated/rewritten, master text verbatim on
  slaves); `XlsxCell.formula: Option<XlsxFormula>` as the last field (`XlsxCell` stays
  exhaustive; `has_formula`/`XlsxCellValue` untouched); two warning constructors
  `unresolved_shared_formula_index` and `shared_formula_table_limit_reached` with byte-exact
  spec wordings (no new `WarningCode` variant); two unit tests.
- `crates/oxdoc-core/src/parsers/xlsx.rs` (+2): `push_typed_cell` maps `formula: None` (single
  construction path preserved for S4).
- `crates/oxdoc-core/tests/api.rs` (+31/−1): B2 baseline re-scoped + invariant loops over both
  corpus trees (formulas rows incl. headers; shared-formulas both sheets).
- `crates/oxdoc-tabular/src/parquet.rs` (+2): test literal + `cell` helper gain `formula: None`
  (`number_cell` delegates to `cell`).
- `crates/oxdoc-tabular/src/xlsx_schema.rs` (+5): inline literal + `blank`/`string`/`boolean`/
  `number` helpers gain `formula: None`.
- `classify_cell`, Parquet conversion, and schema inference were not changed (never read
  `formula`).

### Verification (Gate S3)

- `cargo fmt --all -- --check` → clean (one fmt normalization pass applied first)
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → all 8 suites ok, 0 failures (372 tests incl. 2 new unit tests)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only`
  → exit 0 (gate passes)
- `git status --porcelain schemas/v1 docs/schemas/v1 tests/fixtures/files
  tests/fixtures/compatibility-matrix.json tests/fixtures/snapshots` → empty (frozen guards
  hold; no snapshot, manifest, digest, or binary churn)
- Envelope honesty: every payload stays at `schema_version: 1`; no JSON formula field is
  emitted before S7 (CLI emission untouched in this slice).

### Measured changed lines vs budget

`git diff main --numstat` excluding `.gitignore` (pre-existing local edit, not committed):
149 additions, 4 deletions, **153 total** — under the 400-line budget and below the
~230–300 realistic estimate for S3 (the migration is genuinely mechanical: one line per
struct literal).

### Deviations from design

- Task 9's api B2 assertion re-scoped to the S3-guaranteed invariant (`formula.is_none()`)
  as documented in the cycle note above; the expression-level assertion is S4 task 16's RED.
  No other deviation.

### Remaining tasks

- S4: tasks 15–21; S5: tasks 22–30; S6: tasks 31–36; S7: tasks 37–44; S8: tasks 45–50;
  cross-slice guards: tasks 51–55 (all unchecked)

---

## S4 — Parser capture: `<f>` text and `<v>` presence (tasks 15–21) — COMPLETE

### TDD Cycle Evidence

| Task | Cycle | Test | RED evidence | GREEN evidence |
| --- | --- | --- | --- | --- |
| 15 | RED | 7 unit tests in `parsers::xlsx::tests` (via `parse_formula_rows` helper over `parse_sheet_rows`) | `cargo test -p oxdoc-core --lib parsers::xlsx`: 6 failed (`formula` is `None` — capture unimplemented: `captures_cached_formula_expression_and_cache_presence`, `decodes_formula_entities_cdata_and_numeric_references_like_cell_values`, `keeps_formula_and_value_buffers_disjoint`, `marks_cache_presence_for_empty_v_element`, `keeps_cached_true_when_shared_string_index_is_out_of_bounds`, `captures_empty_f_element_as_empty_expression`); `leaves_non_formula_cells_untouched` passed as expected (asserts current behavior) | All pass after tasks 17–19 |
| 16 | RED | `xlsx_formulas_corpus_loads_and_carries_the_documented_cells` extended with per-cell `formula.expression`/`formula.cached` (design §5.1 shapes); S3 baseline B2 comment/assertion replaced per the documented handoff | `cargo test -p oxdoc-core --test api xlsx_formulas_corpus`: `left: None, right: Some("SUM(B1:B1)")` | Passes with all 9 cells asserted (B2–I2 expressions + cache flags, J2 `formula == None`) |
| 17–19 | GREEN | same suites | — | `cargo test --workspace`: all 8 suites ok, 0 failures (379 tests incl. 7 new unit tests) |
| 20 | TRIANGULATE | no-recalculation proof over the corpus | — | api test asserts every emitted `raw` equals the stored `<v>` (B2 `"2"`, E2 `"#DIV/0!"`, F2 `"alpha & beta"`, G2 index-resolved, H2 `"1"`, I2 `"5"`), C2/D2 stay `kind: blank`, `SUM`/`1/0` never yield a computed number, J2 stays `has_formula: false`; `git status` on `tests/fixtures/snapshots` empty (xlsx CSV snapshots + manifest byte-identical) |
| 21 | REFACTOR/Gate | fmt, clippy, workspace, coverage, corpus | — | All clean (see Verification) |

Implementation notes (task 18 detail worth keeping): the `Empty` `<f …/>` arm reads
`t`/`si` via `attr_value` directly from the self-closing element (the `Start` arm never
fires for it); a shared slave (`t="shared"` with `si`) leaves `formula: None` (S5
resolves it), every other empty `<f/>` stores `Some("")` with `in_formula` staying
false. `Start` `<f>` reads `t`/`si` by local name (prefixed sheets keep working);
`End` `</f>` routes `formula = Some(mem::take(formula_buffer))` — Start-opened shared
masters capture their own text in S4 (registration is S5). `had_value` is set on `<v>`
`Start` **and** `Empty`; `push_typed_cell` maps `formula.cached` from `had_value` only
(never type-resolution success — the out-of-bounds `t="s"` unit test pins `cached: true`
with the unchanged W003 wording). Text/CData/GeneralRef route to `formula_buffer` via
`append_decoded_xml_text`/`append_decoded_xml_reference` when `in_formula`, else to the
value buffers — the three buffers stay disjoint with one routing flag each.

### Files changed

- `crates/oxdoc-core/src/parsers/xlsx.rs` (+308/−17): `CellState` gains `in_formula`,
  `formula_buffer`, `formula: Option<String>`, `formula_type`, `formula_si`, `had_value`;
  event arms (`Start`/`Empty` `<f>`, `<v>` Start+Empty `had_value`,
  Text/CData/GeneralRef routing, `End` `</f>`); `push_typed_cell` maps own expression +
  cache presence into `XlsxFormula` (single construction path); 7 unit tests +
  `parse_formula_rows` helper.
- `crates/oxdoc-core/tests/api.rs` (+42/−4): per-cell formula assertions over the
  `xlsx/formulas` corpus per design §5.1 (S3 baseline replaced per handoff).
- openspec change artifacts (tasks.md marks, apply-progress.md) included in the commit.

### Verification (Gate S4)

- `cargo fmt --all -- --check` → clean (one fmt normalization pass applied first)
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → all 8 suites ok, 0 failures (379 tests)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95
  --summary-only` → exit 0 (line coverage 96.43% ≥ 95)
- `python scripts/check-compatibility-corpus.py` (`make compatibility-corpus-check`;
  `make` unavailable in this shell) → "compatibility corpus validation passed (3 fixtures)"
- `git status --porcelain schemas/v1 docs/schemas/v1 tests/fixtures/files
  tests/fixtures/compatibility-matrix.json tests/fixtures/snapshots` → empty (frozen
  guards hold; no snapshot, manifest, digest, or binary churn)
- Envelope honesty: `schema_version` stays `1`; no JSON formula field is emitted (CLI
  untouched in this slice); no `OoxmlLimits`/public-option change; no new dependency.

### Measured changed lines vs budget

`git diff main --numstat` excluding `.gitignore` (pre-existing local edit, not
committed): 350 additions, 21 deletions, **371 total** — under the 400-line budget and
inside the ~300–400 realistic estimate for S4. No comments, tests, or blank lines were
compressed to fit.

### Deviations from design

- None. Shared slaves (corpus `A3`/`A4`/`A7`/`B1`/`B3`, dangling `B2` `si=9`) keep
  `has_formula: true` with `formula: None` — S4-stable behavior, resolved in S5. The
  `shared-formulas` corpus api baseline assertions are unchanged (they assert only
  `has_formula` and values, which did not move).

### Remaining tasks

- S5: tasks 22–30; S6: tasks 31–36; S7: tasks 37–44; S8: tasks 45–50; cross-slice
  guards: tasks 51–55 (all unchecked)

---

## S5 — Shared/array resolution + bounded table + warnings (tasks 22–30) — SPLIT INTO S5a/S5b (pre-agreed fallback)

Realized diff for the whole S5 slice measured **512 lines** (485 additions,
27 deletions) > the 400-line budget, so the pre-agreed fallback applied:
**S5a** (table + default constant + injectable seam + master registration +
resolution, unit/api tests) is committed; **S5b** (warning emission + latch +
api/CLI warning tests) is fully implemented and verified in the working tree
but intentionally **left uncommitted** ("STOP at the S5a gate instead of
committing further"). S5b measures 217 lines on top of S5a and passes the
full gate, ready to be committed as the next work unit.

### S5a — table + seam + master registration + resolution (commit 051dbc8)

### TDD Cycle Evidence (S5a)

| Task | Cycle | Test | RED evidence | GREEN evidence |
| --- | --- | --- | --- | --- |
| 22 (partial) | RED→GREEN | 5 resolution unit tests in `parsers::xlsx::tests`: `resolves_cached_and_uncached_shared_slaves_verbatim`, `treats_slave_before_master_as_unresolved_without_buffering`, `first_registration_wins`, `registers_nothing_for_empty_master_text`, `default_shared_formula_limit_is_one_mib_and_ooxml_limits_unchanged` | `cargo test -p oxdoc-core parsers::xlsx` compile failure: `unresolved imports super::DEFAULT_SHARED_FORMULA_MEMORY_LIMIT, super::parse_sheet_rows_with_shared_formula_limit` (seam + constant undeclared) — same RED pattern as S3 | All pass after tasks 24–27 (lib: 128 → later 128 with S5b tests too) |
| 23 (partial) | RED→GREEN | `shared_formulas_corpus_loads_and_carries_the_documented_cells` rewritten with resolution assertions (master/slave expressions verbatim, dangling B2 `formula: None` with `raw: "4"` retained, first-wins `A7`→`FIRST()`, array F2/F3, prefixed sheet B1/B2/B3) | `cargo test -p oxdoc-core --test api shared_formulas_corpus`: `assertion left == right failed, left: 0, right: 1` (warning count — pre-S5b no warning existed; first failing point of the rewritten assertions) | Passes with all resolution assertions |
| 24 | GREEN (RED above) | `SharedFormulaTable { expressions: BTreeMap<String,String>, memory_bytes, memory_limit }` keyed by **raw `si` text**, `register` check order (empty → nothing; `si` present → first wins; saturating overflow → record nothing; else insert+cost), `resolve`. Note: `overflow_warned` lands in S5b per the pre-agreed split | — | Committed in 051dbc8 |
| 25 (partial) | GREEN (RED above) | `DEFAULT_SHARED_FORMULA_MEMORY_LIMIT = 1024*1024` (1 MiB) + `estimated_formula_memory_cost` (len saturating-add 16, the same per-entry constant as `xlsx_shared_strings.rs`); latch warning lands in S5b | — | Constant + cost committed in 051dbc8 |
| 26 | GREEN (RED above) | `parse_sheet_rows_with_shared_formula_limit(source, path, shared_strings, format_context, sink, shared_formula_memory_limit)` mirroring `SharedStringStore::parse_with_memory_limit`; `parse_sheet_rows` delegates with the default; `visit_rows_with_read_options`, `write_sheet_csv`, `fuzz_parse_sheet` keep the default entry point — `OoxmlLimits`/public API unchanged | — | Committed in 051dbc8; `- [x]` in tasks.md |
| 27 (partial) | GREEN (RED above) | `End </f>` registers when `t="shared"` + `si` present + non-empty text (master keeps own text even when refused); `Empty <f t="shared" si="N"/>` resolves immediately — hit ⇒ master text verbatim, miss ⇒ `formula = None` (per-cell warning lands in S5b). No buffering/second pass | — | Resolution committed; warning emission uncommitted (S5b) |

Cycle notes: the first GREEN attempt had one wrong test expectation — the
overflow test initially asserted refused masters lose their text; the design
is explicit that a Start-opened shared master keeps its own text even when
registration is refused, so the test was corrected to assert
`Some("AAAAA()")`/`Some("BBBBB()")` (code unchanged, code was right).

### S5b — warning emission + latch + api/CLI warning tests (uncommitted, ready)

Implemented and verified in the working tree on top of 051dbc8:

- `overflow_warned: bool` latch added to `SharedFormulaTable`; `register`
  pushes `shared_formula_table_limit_reached(path)` exactly once per
  worksheet (latched) when an insertion would exceed the bound.
- `Empty <f/>` miss branch now pushes
  `unresolved_shared_formula_index(path, si)` per affected cell.
- Unit tests (re)added: `warns_per_cell_for_dangling_si` (exact wording ×2,
  cells still emit), `latches_overflow_warning_once_per_worksheet` (tiny
  20-byte limit via the seam helper `parse_formula_rows_with_shared_formula_limit`,
  mirroring the `SharedStringStore::parse_with_memory_limit(…, 6)` test),
  warning assertions restored in slave-before-master and empty-master tests.
- api.rs: exactly-one-warning assertions for dangling `si=9` (sheet `Shared`)
  and slave-before-master `si='0'` (sheet `Prefixed`) with byte-exact wordings.
- cli.rs: `keeps_rows_jsonl_stdout_clean_when_warnings_are_emitted` extended
  with a dangling-`si` package — stdout stays valid JSONL, stderr carries the
  exact unresolved wording (S7 will extend further with the v2 snapshot).
- Task 29 confirmation: rows are still emitted one `ParsedRow` at a time; the
  only new cross-row state is the bounded table; no `OoxmlLimits` field or
  public option changed (vfs.rs untouched, `git status` clean).

### S5b gate (run on the uncommitted working tree)

- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → 386 passed, 0 failed (384 in S5a; +2 in S5b)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → exit 0
- `python scripts/check-compatibility-corpus.py` → "compatibility corpus validation passed (3 fixtures)"
- `git status --porcelain schemas/v1 docs/schemas/v1 tests/fixtures/files
  tests/fixtures/compatibility-matrix.json tests/fixtures/snapshots` → empty
  (frozen guards hold; no snapshot, manifest, digest, or binary churn)
- Envelope honesty: `schema_version` stays `1`; no JSON formula field emitted
  (CLI fields are S7); no new dependency; no temp-file spill.

### Measured changed lines vs budget

- Whole S5 slice (S5a commit + S5b working tree, excluding `.gitignore` and
  openspec bookkeeping): 485 additions + 27 deletions = **512 total** → over
  budget, fallback applied as pre-agreed.
- S5a commit 051dbc8: 316 insertions + 25 deletions = **341 total** (diff
  measure 343 including the pre-existing `.gitignore` edit) — under 400.
- S5b working tree on top of S5a: 195 additions + 22 deletions = **217
  total** — under 400 on its own.

### Deviations from design

- The pre-agreed S5a/S5b split was applied (S5 measured 512 > 400). Within
  S5a, `register` deliberately omits the `overflow_warned` latch (silent
  refusal) so no dead field/warning path sits in the committed slice; S5b
  adds it exactly per task 24/25 wording.
- The S5a commit does not include the openspec task/progress marks (they
  were produced after the gate); they sit uncommitted next to S5b and should
  land with the S5b work-unit commit (S1–S4 precedent includes them per
  slice).

### Remaining tasks

- S5b commit (uncommitted, gate-green): completes tasks 22, 23, 24, 25, 27,
  28 and the S5 gate run (task 30) — then check those boxes.
- S6: tasks 31–36; S7: tasks 37–44; S8: tasks 45–50; cross-slice guards:
  tasks 51–55 (all unchecked)

---

## S6 — Rows-jsonl schema v2 + harness (tasks 31–36) — SPLIT VIA PRE-AGREED SECOND FALLBACK (S6a committed, S6b ready uncommitted)

Realized diff for the whole S6 slice measured **564 lines** (263 additions + 1 deletion in
`crates/oxdoc-core/tests/schema.rs`, plus two new 150-line schema files = 300) > the
400-line budget, so the pre-agreed second fallback applied ("move S6's snapshot-free
harness tests into their own slice"): **S6a** (schema v2 + byte-identical mirror +
registration + mirror-identity test + representative v2 record test) is committed
(af9d6d0, 392 lines); **S6b** (the `const` harness extension + coupling, negative, and
additive-delta tests, 172 lines) is fully implemented and verified in the working tree,
gate-green, ready to be committed as the next work unit. Per `ask-on-risk`, the executor
stopped at the S6a gate instead of committing further — the S5a/S5b precedent.

### TDD Cycle Evidence (S6)

| Task | Cycle | Test | RED evidence | GREEN evidence |
| --- | --- | --- | --- | --- |
| 31 | RED | 5 new tests in `crates/oxdoc-core/tests/schema.rs` + v2 `SCHEMA_VERSIONS` registration | `cargo test -p oxdoc-core --test schema`: 6 failed (5 new tests + `schemas_have_stable_public_metadata`), all on the missing v2 file (`Os NotFound` at `read_json_schema`) | See tasks 32–35 |
| 33–34 | GREEN | `xlsx_rows_v2_schema_and_mirror_are_identical`, registration | — | Schema authored (v1 + design §4.1 deltas), mirror byte-identical (`cmp` identical), committed in af9d6d0; 17 tests pass in the S6a state |
| 32 | GREEN (RED above) | `v1_rows_payload_fails_frozen_v2_validation` | Intermediate proof: with the v2 file present but **before** the `const` extension the negative test failed with `a v1 payload must fail the v2 schema (schema_version const 2)` (payload validated — all v1 fields stay declared in v2) | After the ~7-line `const` equality check in `validate_against`, passes; the negative now fails validation on the `schema_version` const, as designed |
| 31/35 | GREEN | `representative_xlsx_rows_v2_jsonl_record_matches_schema_shape` (cached number, uncached blank, cached error, shared slave; five-variant walk; per-cell coupling), `xlsx_rows_v2_formula_free_record_differs_from_v1_only_by_schema_version` (additive delta + five-variant declaration + v1 frozen-no-formula), `xlsx_rows_v2_enforces_formula_presence_coupling` | — | All pass; full suite 20/20 in schema.rs |

Harness limitation stated inline (never silent): `coupling_accepts` reads the two
declared `allOf`/`if`/`then` branches from the shared `cell` shape and applies them
directly in Rust, panicking on any unsupported shape; the test comment states the
hand-rolled harness does not evaluate `allOf`/`if`/`then` and that the constraint is
enforced there instead.

### Files changed

- `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` (new, 150 lines): `$id` v2, `const: 2`
  on `schema_version`, `formula`/`formula_cached` in `$defs.cellBaseProperties`, both
  `$ref`s added to all five variants' properties (optional, not required), coupling
  declared once on the shared `cell` shape via `allOf`/`if`/`then`; every other v1
  element byte-preserved.
- `docs/schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json` (new, byte-identical mirror).
- `crates/oxdoc-core/tests/schema.rs`: S6a (+91/−1 committed) + S6b working tree
  (+172): registration, identity test, representative v2 test, `const` equality check in
  `validate_against` (strengthens every existing schema test), coupling test +
  `coupling_accepts` helper, negative test, additive-delta test.
- openspec artifacts (tasks.md marks, apply-progress.md) included with the slice.

### Verification (Gate S6, run on the S6b working tree)

- `cargo fmt --all -- --check` → clean (one fmt normalization pass applied first)
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → 8 suites ok, 0 failures (391 tests incl. 6 new schema tests)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → exit 0
- `diff -ru schemas/v1 docs/schemas/v1 && diff -ru schemas/v2 docs/schemas/v2` (`make docs-schemas-check`; `make` unavailable in this shell) → identical
- `python scripts/check-compatibility-corpus.py` → "compatibility corpus validation passed (3 fixtures)"
- `git status --porcelain schemas/v1 docs/schemas/v1 tests/fixtures/files tests/fixtures/compatibility-matrix.json tests/fixtures/snapshots` → empty (v1 frozen; no snapshot/manifest/digest/binary churn)
- Envelope honesty: `schema_version` stays `1` in CLI payloads (the flip is S7); no JSON formula field is emitted; `schemas/v2/oxdoc-structured-text.schema.json` untouched.

### Measured changed lines vs budget

- Whole S6 slice (S6a commit + S6b working tree, excluding `.gitignore` and openspec
  bookkeeping): 263 additions + 1 deletion + 300 new schema/mirror lines = **564 total**
  → over budget, pre-agreed second fallback applied.
- S6a commit af9d6d0: 391 insertions + 1 deletion = **392 total** — under 400.
- S6b working tree on top of S6a: **172 additions** in `schema.rs` — under 400 on its own.

### Deviations from design

- The pre-agreed second fallback split was applied (S6 measured 564 > 400). S6b is
  implemented, verified, and gate-green in the working tree but uncommitted pending the
  maintainer's delivery decision (second stacked PR for the S6b work unit, or an explicit
  `size:exception` for one combined S6 PR). No other deviation: the v2 schema is exactly
  the design §4.1 delta set, and the harness changes are exactly design §4.2 items 1–6.

### Remaining tasks

- S6b commit (uncommitted, gate-green): completes tasks 31, 32, 35 and the S6 gate run
  (task 36) — then check those boxes.
- S7: tasks 37–44; S8: tasks 45–50; cross-slice guards: tasks 51–55 (all unchecked)

---

## S7 — CLI emission + Python pass-through (tasks 37–44) — COMPLETE

Status note: the S5b and S6b work units that S6's progress section left uncommitted
landed as commits a3ce683 (#232), 1a0a7e2 (#233), and 1b5a217 (#234); tasks 22–36 are
checked in tasks.md. This executor resumed from the clean stacked state at 1b5a217.

### TDD Cycle Evidence

| Task | Cycle | Test | RED evidence | GREEN evidence |
| --- | --- | --- | --- | --- |
| 37 | RED | `extracts_sparse_typed_xlsx_rows_as_jsonl` (version 1→2 + TODAY() formula fields), `extracts_formula_provenance_as_rows_jsonl_v2` (per-cell field sets), `extracts_xlsx_rows_as_v2_jsonl_snapshot` (byte-compare), corpus-warning case in `keeps_rows_jsonl_stdout_clean_when_warnings_are_emitted`; python pin `test_extract_rows_passes_v2_formula_fields_through` | `cargo test -p oxdoc-cli --test cli`: 3 failed — sparse: `left: Null, right: "TODAY()"`; corpus: `left: Number(1), right: 2`; snapshot: `NotFound` (snapshot absent). Warning-channel case passed (S5b already landed the stderr wording; it is a channel pin). Python pin passed immediately (the wrapper already passes dicts through verbatim — no wrapper change by design, so a RED cannot occur without artificially breaking it) | See tasks 38–42 |
| 38–39 | GREEN | `RowsJsonlCell` gains `formula: Option<&'a str>` + `formula_cached: Option<bool>` after `has_formula` (declaration order = byte order); `TryFrom<&XlsxCell>` derives both from the single `cell.formula` match (both-or-neither) | — | Snapshot + corpus tests pass |
| 40 | GREEN | single `RowsJsonlRecord` construction site in `extract_rows_command` → `schema_version: 2` | — | `grep 'schema_version: 1' main.rs` → 6 remaining literals (text/tables/audit/slides) untouched |
| 41 | GREEN | snapshot generated once from real CLI output over the runtime-zipped `formulas` corpus (package name `formula-provenance.xlsx`, zip built with Stored compression, sorted entries — mirrors `build_package`); content reviewed against design §5.1 before commit (all 9 cells match the table; header row carries no formula fields; field order `column_index, kind, raw?, value?, formatted?, has_formula, formula?, formula_cached?`) | — | `cargo test -p oxdoc-cli --test cli` → 108 passed, 0 failed |
| 42 | GREEN | python audit: the two `fake_oxdoc` tests hardcode `schema_version: 1` payloads but do **not** mirror live CLI output (fake binary fixtures per design §3.3) → stay as-is; new pass-through pin passes; `python/src/oxdoc/client.py` unchanged | — | `PYTHONPATH=python/src python -m unittest discover -s python/tests` → 9 tests OK |
| 43 | TRIANGULATE | snapshot test run twice → identical bytes (sha256 `3002440a…aef4c6` both runs); formula-free `xlsx/basic` CLI run → `schema_version: 2`, 0 formula-bearing cells, key order `column_index, kind, raw, value, has_formula` (record differs from v1 only by the envelope, matching the S6 record-shape delta); `git status` on snapshots → only `cli_xlsx_rows_v2_jsonl.jsonl` added | — | Confirmed |
| 44 | REFACTOR/Gate | full gate | — | See Verification |

### Files changed

- `crates/oxdoc-cli/src/main.rs` (+12/−1): `RowsJsonlCell.formula`/`formula_cached`
  after `has_formula`; `TryFrom<&XlsxCell>` derives both from the single
  `cell.formula` match; the single `extract_rows_command` site flips to
  `schema_version: 2` (no `--schema-version` flag, no dual emission, catch-all
  unchanged).
- `crates/oxdoc-cli/tests/cli.rs` (+115/−1): sparse rows test version bump + TODAY()
  formula assertions; new `extracts_formula_provenance_as_rows_jsonl_v2`; new
  `extracts_xlsx_rows_as_v2_jsonl_snapshot`; corpus warning case in
  `keeps_rows_jsonl_stdout_clean_when_warnings_are_emitted`.
- `python/tests/test_oxdoc.py` (+18): `test_extract_rows_passes_v2_formula_fields_through`
  pin (wrapper unchanged).
- `tests/fixtures/snapshots/cli_xlsx_rows_v2_jsonl.jsonl` (new, 2 lines, generated once
  from real CLI output; divergence would be stop-and-investigate, never regeneration).
- openspec change artifacts (tasks.md marks, apply-progress.md) included in the commit.

### Verification (Gate S7)

- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → 8 suites ok, 0 failures (cli suite: 108 passed)
- `PYTHONPATH=python/src python -m unittest discover -s python/tests` → 9 tests OK
  (note: `pytest` is not installed in this environment; the repo's `make python-test`
  recipe is the unittest discover runner and was invoked directly)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95
  --summary-only` → exit 0 (line coverage 96.52% ≥ 95)
- `python scripts/check-compatibility-corpus.py` (`make compatibility-corpus-check`;
  `make` unavailable in this shell) → "compatibility corpus validation passed (3 fixtures)"
- `git status --porcelain tests/fixtures/snapshots tests/fixtures/files
  tests/fixtures/compatibility-matrix.json schemas/v1 docs/schemas/v1` → only the new
  snapshot file (frozen v1, manifests, digests, binaries, and every existing snapshot
  untouched; stop-and-investigate guards hold)
- Envelope honesty: the rows-jsonl payload is the only format at `schema_version: 2`;
  text/tables/audit/slides payloads stay at `1`; no other snapshot changed.

### Measured changed lines vs budget

`git diff main --numstat` excluding `.gitignore` (pre-existing local edit) and
openspec bookkeeping: 145 additions, 2 deletions across
main.rs/cli.rs/test_oxdoc.py = **145 code lines**, plus the new 2-line snapshot file
(149 total) — under the 400-line budget and below the ~210–290 realistic estimate for
S7 (the stderr-warning extension was already landed in S5b, and the python wrapper
needed no change).

### Deviations from design

- None. The task-37 stderr-warning case uses the `xlsx/shared-formulas` corpus as
  specified; the corpus case in `keeps_rows_jsonl_stdout_clean_when_warnings_are_emitted`
  passed on first run because S5b already established the exact wording and channel —
  recorded honestly as a channel pin rather than a RED.

### Remaining tasks

- S8: tasks 45–50; cross-slice guards: tasks 51–55 (all unchecked)
