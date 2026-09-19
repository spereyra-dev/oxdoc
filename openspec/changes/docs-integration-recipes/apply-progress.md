# Apply Progress — docs-integration-recipes

## Slice 1 (PR 1) — page + Recipes A/B/C + navigation links — DONE

### Completed tasks (persisted checkboxes updated in tasks.md)

- [x] Task 1 — `docs/recipes.md` page title, ~20-line intro ("how to read these
  recipes": workflows not flag references, `cli.md` flag authority, labeled
  example-output blocks, deterministic/text-safe/no-rendering premise), and the
  five-part recipe template (Goal / Command / Example output / Constraints /
  See also).
- [x] Task 2 — Recipe A - Shell pipelines: 6 command blocks (text JSONL batch,
  stdin `-`, csv with `--sheet`/`--delimiter`, rows with `--value-mode
  formatted`, slides JSONL, exit-code `if/else` with `--warnings json`), two
  example-output blocks (JSONL records transcribed from `docs/cli.md` /
  `docs/json-output.md`; stderr warning line verbatim), constraints blurb,
  see-also links.
- [x] Task 3 — Recipe B - GitHub Actions: self-use framing naming
  `.github/workflows/action.yml` in prose (no URLs), one job-skeleton YAML
  block (checkout, setup action with `version: v0.1.0` and `<commit-sha>`
  placeholder per design D5, extract + audit with stdout redirects,
  `jq`-derived job summary, `actions/upload-artifact@v7`), illustrative
  job-summary line, constraints blurb, links `github-action.md` + `cli.md`
  without copying its examples.
- [x] Task 4 — Recipe C - Python pandas orchestration: separate install lines
  (`oxdoc-python` + `cargo install oxdoc-cli`), `Oxdoc(binary=...)` note,
  `extract_rows` → `pandas.DataFrame` block with numeric-as-string conversion,
  `extract_csv` → `io.StringIO`, illustrative minimal `df` repr,
  `extract_text_records` per-file error tolerance + `OxdocResult.warnings`
  block, constraints blurb (formulas never recalculated; exception types), 
  see-also links.
- [x] Task 5 — navigation insertions only: `docs/_sidebar.md` one line after
  Python Integration (before Compatibility Playground, per design D2/spec);
  `README.md` one line after Python Integration in "Key documentation pages".
- [x] Task 6 — Slice 1 gates run and recorded below.
- [x] Task 7 — committed as one work unit; diff surface confirmed; no
  `openspec/specs/**` file touched.

### Files changed (Slice 1)

- `docs/recipes.md` (new, 236 lines)
- `docs/_sidebar.md` (+1 line)
- `README.md` (+1 line)
- `openspec/changes/docs-integration-recipes/**` (proposal, design, tasks,
  spec, this progress file — openspec bookkeeping, excluded from budget)

### Verification evidence (Slice 1)

Run on Windows (no `make` available); each gate run as its exact Makefile
recipe equivalent:

| Gate | Equivalent command | Result |
|------|--------------------|--------|
| docs-check | `npx --yes docsify-cli@4 serve docs --port 3457` + curl + `grep "oxdoc documentation"` | PASS (title served; `recipes.md` served with `# Integration Recipes`) |
| docs-links | `find README.md docs -name '*.md' -print0 \| xargs -0 npx --yes markdown-link-check@3 --config .markdown-link-check.json` over 36 files | PASS, 0 dead links; `docs/recipes.md` alone: 6/6 links ✓ |
| docs-schemas-check | `diff -ru schemas/v1 docs/schemas/v1 && diff -ru schemas/v2 docs/schemas/v2` | PASS |
| docs-playground-check | `python scripts/compatibility-playground.py --check` (`python3` is a Store stub on this host) | PASS |
| cargo fmt | `cargo fmt --all -- --check` | PASS (clean) |
| cargo clippy | `cargo clippy --workspace --all-targets -- -D warnings` | PASS (clean) |
| cargo test | `cargo test --workspace` | PASS — 7 suites, 393 tests, 0 failed |

Note: port 3000 on this host is occupied by an unrelated local server, so the
docs-check equivalent used port 3457 (the Makefile default is `DOCS_PORT=3000`
and CI runs it as-is; the gate content is identical).

### Anchor + link discipline checks (task 6)

- Headings: `#integration-recipes`, `#recipe-a-shell-pipelines`,
  `#recipe-b-github-actions`, `#recipe-c-python-pandas-orchestration` — all
  ASCII letters/digits/spaces/hyphens; deep links resolve in the served site.
- `grep -nE "https?://"` over `docs/recipes.md`: **zero** absolute URLs (design
  D3 plan: zero; spec allows at most one), no blob URLs.
- All links relative (`cli.md`, `json-output.md`, `errors-and-warnings.md`,
  `github-action.md`, `audit.md`, `python-integration.md`); no schema links in
  Slice 1 recipes, so the "published mirror" rule is satisfied vacuously here
  (Recipe E in Slice 2 will add `schemas/v1/oxdoc-audit-jsonl.schema.json`).
- Slice 1 spans spec "Recipe coverage" categories 1–3; categories 4–5 remain
  for Slice 2, as planned.

### Measured changed lines (design D8 convention)

`git diff --stat <base>...HEAD -- . ':(exclude)openspec' ':(exclude).gitignore'`
after the Slice 1 commit: **238 changed lines** (new `docs/recipes.md` 236 +
`README.md` 1 + `docs/_sidebar.md` 1). Under the 400-line review budget; per
the tasks forecast and preflight, the two-slice chain continues regardless
(Slice 2 = PR 2), with a single-PR fallback acceptable if the whole change
stays under 400.

### Deviations from design

- None in content. Environmental only: `make` unavailable on this host, so the
  four gates were run as their exact Makefile recipe equivalents (recorded
  above); `python3` resolves to the Microsoft Store stub, so `python` was used
  for the playground check.


### Workload / PR boundary

Slice 1 = PR 1 on the `stacked-to-main` chain (`auto-chain` per session
preflight). Commit `aa9ed60` (`docs(recipes): ...`) is the single work unit,
containing exactly `docs/recipes.md`, `docs/_sidebar.md`, `README.md`, and the
six change-artifact files; no push, no PR (parent instruction).

### Structured status consumed

Native `gentle-ai.sdd-status` v2 consumed at phase start: change
`docs-integration-recipes`, `applyState: ready`, `nextRecommended: apply`,
`actionContext.mode: repo-local`, `allowedEditRoots` = workspace root. All
edits stayed inside the authoritative workspace. No `actionContext` warnings.
---

## Slice 2 (PR 2) — Recipes D/E + closing reference list + final gates — DONE

### Completed tasks (persisted checkboxes updated in tasks.md)

- [x] Task 8 — Recipe D - Backend ingestion: goal sentence; limited-request
  command block with all four global limit flags at their documented defaults
  (`--max-input-size 67108864`, `--max-package-uncompressed-size 268435456`,
  `--max-part-size 67108864`, `--max-compression-ratio 200`) plus
  `--warnings json` and stderr redirect; stdin intake block with a 10 MiB
  `--max-input-size`; batch triage (`audit --format jsonl`) +
  `oxdoc diagnostics --format json` block; two labeled example-output blocks
  (JSON warning record verbatim from `docs/cli.md`; JSONL error record
  shape-consistent with `docs/schemas/v1/oxdoc-audit-jsonl.schema.json`,
  `document_type: "unknown"`); typed-limit prose (`E014` input size, `E011`
  uncompressed size, `E005` oversized part, `E006` suspicious entry, stable
  `E001`–`E010` classes linked to `errors-and-warnings.md`); constraints
  blurb (stdin buffered only up to `--max-input-size` for ZIP
  central-directory seekability, no temp files / no stdin spill, XLSX
  bounded library-internal spill unchanged, warnings never contaminate
  stdout, secure defaults, never renders); see-also `cli.md`,
  `json-output.md`, `security.md`, `performance.md`,
  `errors-and-warnings.md`.
- [x] Task 9 — Recipe E - Audit workflow: goal; `audit --format json` /
  `--format jsonl` / `--format text` command block (text shown as the no-`jq`
  human path per design D7); triage step 1 `jq` severity filter
  (`.signals[] | select(.severity == "high")`) and triage step 2 E-code
  routing (`select(.error) | [.file, .error.code] | @tsv`); one
  example-output block transcribed verbatim from the JSON Shape in
  `docs/audit.md` (`workbook.xlsx` / `hidden_sheet` / `warning` signal);
  constraints blurb (factual signals only — no risk scoring, no rendering,
  no mutation; severity buckets `info`/`warning`/`high`; `audit`/`error`
  mutually exclusive via schema `oneOf`, linked to
  `schemas/v1/oxdoc-audit-jsonl.schema.json` mirror; failed files
  `document_type: "unknown"`; stable `E001`–`E010` classes); see-also
  `audit.md`, `errors-and-warnings.md`, `json-output.md`, and the JSONL
  schema mirror.
- [x] Task 10 — closing `## Reference pages` list: relative links to
  `cli.md`, `json-output.md`, `audit.md`, `errors-and-warnings.md`,
  `github-action.md`, `python-integration.md`, plus the schema mirrors used
  (`schemas/v1/oxdoc-audit.schema.json`,
  `schemas/v1/oxdoc-audit-jsonl.schema.json`). Confirmed no recipe adds a
  flag table, a full output-field list, or any `github-action.md` example.
- [x] Task 11 — final gates on the complete page (results below) + command
  read-through evidence (below).
- [x] Task 12 — whole-change guards (results below).
- [x] Task 13 — handoff to verify: this section + the completed page are the
  handoff artifacts; verify must re-run the four docs gates and statically
  confirm the six spec requirements.

### Files changed (Slice 2)

- `docs/recipes.md` (+151 lines: Recipe D, Recipe E, `## Reference pages`)
- `openspec/changes/docs-integration-recipes/**` (tasks.md checkboxes, this
  progress file — openspec bookkeeping, excluded from budget)

### Verification evidence (Slice 2, final gates on the complete page)

Run on Windows (no `make` available); each gate run as its exact Makefile
recipe equivalent:

| Gate | Equivalent command | Result |
|------|--------------------|--------|
| docs-links | `find README.md docs -name '*.md' -print0 \| xargs -0 npx --yes markdown-link-check@3 --config .markdown-link-check.json` over all 36 files | PASS — zero dead links across the whole tree |
| docs-links (page focus) | `npx markdown-link-check@3 docs/recipes.md` | PASS — 10/10 links ✓ (`cli.md`, `json-output.md`, `errors-and-warnings.md`, `github-action.md`, `audit.md`, `python-integration.md`, `security.md`, `performance.md`, `schemas/v1/oxdoc-audit-jsonl.schema.json`, `schemas/v1/oxdoc-audit.schema.json`) |
| docs-check | `npx --yes docsify-cli@4 serve docs --port 3458` + curl | PASS — title served; `recipes.md` served with `## Recipe D - Backend ingestion`, `## Recipe E - Audit workflow`, `## Reference pages` |
| docs-schemas-check | `diff -rq schemas/v1 docs/schemas/v1 && diff -rq schemas/v2 docs/schemas/v2` | PASS |
| docs-playground-check | `python scripts/compatibility-playground.py --check` | PASS |
| cargo fmt | `cargo fmt --all -- --check` | PASS (clean) |
| cargo clippy | `cargo clippy --workspace --all-targets -- -D warnings` | PASS (clean) |
| cargo test | `cargo test --workspace` | PASS — 7 suites, 393 tests, 0 failed (25+108+128+101+20+9+2) |

### Command read-through evidence (task 11)

Every recipe command/YAML/Python block was read against `docs/cli.md`,
`docs/audit.md`, `docs/json-output.md`, `docs/errors-and-warnings.md`, and
the live binary (`target/debug/oxdoc --help`, `audit --help`, `diagnostics
--format json` executed):

- All four global limit flags exist with the exact defaults transcribed
  (67108864 / 268435456 / 67108864 / 200); `--help` also confirms
  `--max-input-size` "also bounds stdin buffering" — matching Recipe D's
  stdin constraint.
- `oxdoc audit --help` confirms `--format <FORMAT>` possible values are
  exactly `text, json, jsonl` with default `json` — matching Recipe E.
- `oxdoc diagnostics --format json` executed successfully; emits
  `schema_version`, `oxdoc_version`, `platform` — matching `docs/cli.md`.
- Recipe E example output is the `workbook.xlsx` audit record transcribed
  verbatim from the JSON Shape in `docs/audit.md`; the JSON warning record
  in Recipe D is verbatim from `docs/cli.md`; the JSONL error record matches
  the `oxdoc-audit-jsonl.schema.json` requirements (`schema_version`,
  `file`, `document_type`, `error.code`, `error.message`;
  `additionalProperties: false` respected).
- Triage field-path validation: `jq` is not installed on this host, so the
  two jq filters' field paths were validated by running the equivalent
  filter logic (node) over the transcribed audit JSON and the JSONL error
  record — `.signals[].severity/kind/path` and `.error`/`.error.code`/`.file`
  all resolve as documented; the recipes themselves still present `jq` as
  the documented primary triage tool with `--format text` as the no-`jq`
  path.

### Anchor + link discipline checks (tasks 10–12)

- Headings on the complete page: `#integration-recipes`,
  `#recipe-a-shell-pipelines`, `#recipe-b-github-actions`,
  `#recipe-c-python-pandas-orchestration`, `#recipe-d-backend-ingestion`,
  `#recipe-e-audit-workflow`, `#reference-pages` — all ASCII
  letters/digits/spaces/hyphens; A–C slugs unchanged from Slice 1; all six
  served sections confirmed via curl.
- `grep -nE "https?://"` over `docs/recipes.md`: **zero** absolute URLs
  (whole page), no blob URLs; all links relative.
- Schema links point at the published mirrors via relative paths
  (`schemas/v1/oxdoc-audit-jsonl.schema.json`,
  `schemas/v1/oxdoc-audit.schema.json` resolve under `docs/schemas/`).

### Whole-change guards (task 12)

- Budgetable diff (design D8 convention), measured against the pre-Slice-1
  base `403d3b6` with
  `git diff --stat 403d3b6 -- . ':(exclude)openspec' ':(exclude).gitignore'`:
  **389 changed lines** (README.md +1, docs/_sidebar.md +1,
  docs/recipes.md +387) — under the 400-line review budget.
- Slice 2 alone: **151 lines**, only `docs/recipes.md`.
- Full change surface (`git diff --name-only 403d3b6`, excluding openspec
  bookkeeping and `.gitignore`): exactly `README.md`, `docs/_sidebar.md`,
  `docs/recipes.md`. Zero files under `crates/`, `python/`, `schemas/`,
  `docs/schemas/`, `.github/workflows/`, or `tests/` differ from base.
- Every recipe heading slug verified unchanged from
  `#recipe-a-shell-pipelines` through `#recipe-e-audit-workflow`.
- Because the whole-change budgetable total is under 400, a single PR is
  acceptable per the tasks/design fallback; the two commits (Slice 1
  `aa9ed60`, Slice 2 below) remain reviewable work units and PR 1 (#238) is
  already merged, so PR 2 can be created from this branch either way.

### Deviations from design

- None in content. Environmental only (same as Slice 1): `make` unavailable
  on this host, so gates ran as exact Makefile recipe equivalents;
  `python3` resolves to the Microsoft Store stub, so `python` was used; jq
  absent, so triage field paths were validated with an equivalent filter
  (recipes still document jq as primary, per design D7).

### Remaining tasks

- None. All 13 tasks are checked in `tasks.md`. Next phase: verify
  (re-run four docs gates + static confirmation of the six spec
  requirements), then archive.

### Workload / PR boundary

Slice 2 = PR 2 on the `stacked-to-main` chain (`auto-chain` per session
preflight). One work-unit commit containing `docs/recipes.md` + the updated
openspec change artifacts. No push, no PR creation (parent instruction).

### Structured status consumed

Native `gentle-ai.sdd-status` v2 consumed at phase start: change
`docs-integration-recipes`, `applyState: ready`, `nextRecommended: apply`,
`actionContext.mode: repo-local`, `allowedEditRoots` = workspace root. All
edits stayed inside the authoritative workspace. No `actionContext`
warnings. After completion, native status should move `applyState` to
`all_done`; the fresh native recommendation (classically `archive`) should
be re-read before the next phase.

