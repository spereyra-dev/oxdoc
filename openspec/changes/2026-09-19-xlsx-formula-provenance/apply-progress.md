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
