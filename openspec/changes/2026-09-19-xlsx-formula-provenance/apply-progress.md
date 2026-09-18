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
