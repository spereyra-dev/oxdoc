# Tasks — docs-integration-recipes (issue #176)

Docs-only change. No file under `crates/`, `python/`, `schemas/`,
`docs/schemas/`, `.github/workflows/`, or `tests/` is modified; no CLI,
library, schema, or output behavior changes. `strict_tdd` is configured in
`openspec/config.yaml`, but no Rust/Python source is touched, so RED→GREEN
cycles would be vacuous (the 95% coverage gate is satisfied vacuously, as the
proposal states). The applicable practical checks are the four docs gates:
`make docs-check`, `make docs-links`, `make docs-schemas-check`,
`make docs-playground-check`, plus a manual command read-through against
`docs/cli.md` and `oxdoc --help`.

Inputs read for this task list: `proposal.md`, `specs/integration-recipes/spec.md`,
`openspec/config.yaml`, `Makefile` (docs targets), `.markdown-link-check.json`,
`README.md` (Key documentation pages), `docs/_sidebar.md`, `docs/cli.md`,
`docs/audit.md`, `docs/python-integration.md`, `.github/workflows/action.yml`.
`design.md` is absent; the proposal declares it optional (`design?`) for this
docs-only change, so tasks derive from proposal + spec.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~455–660 total (`docs/recipes.md` ~345–500 with the 1.5× example/output multiplier; two nav insertions ~10; change artifacts `spec.md` + `tasks.md` ~110–160) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 → PR 2 (Slice 1: page + Recipes A/B/C + nav links + spec artifact; Slice 2: Recipes D/E + closing reference list + final gates) |
| Delivery strategy | auto-chain |
| Chain strategy | stacked-to-main |

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
```

Notes on the forecast: the docs-page estimate uses the proposal's honest 1.5×
multiplier on command/example blocks (post-#177/#180 lesson); the docs page
alone straddles the 400-line budget and the change's own SDD artifacts push the
change total clearly above it, so the two-slice boundary below is planned, not
improvised. Per-slice estimates: Slice 1 ~200–290 realized lines, Slice 2
~125–185 realized lines. Apply MUST measure the realized diff (`git diff --stat`)
and record it; because delivery is `auto-chain` with chain strategy
`stacked-to-main`, the chain executes without a pause unless the realized diff
lands under 400 (then a single PR is acceptable). No `size:exception` is
assumed or claimed.

## Slice 1 — page + Recipes A/B/C + navigation links (PR 1)

- [x] 1. Create `docs/recipes.md` with the page title, the intro / "how to read these recipes" note (~15–20 lines: recipes are workflows not flag references, `cli.md` is the flag authority, example-output blocks are labeled and transcribed or explicitly illustrative, commands verified against the current CLI surface), and the four-part recipe template (Goal / Command / Example output / Constraints / See also). No external links other than the allowed documented ones. Spec: "Recipe coverage" (intro + template) and "Example-output labeling" (labeling rule stated up front).

- [x] 2. Write **Recipe A — Shell pipelines** in `docs/recipes.md`: one-sentence goal, 5–6 complete copy/paste command blocks (text/csv/rows/slides batch extraction, `--format jsonl` streaming, stdin `-`, exit codes `0`/`1`), one fenced example-output block labeled as example output and consistent with `docs/cli.md` / `docs/json-output.md` shapes, and a constraints blurb covering warnings-on-stderr (`warning[<category>/<code>]: <path>: <message>`, stdout stream-clean, `--warnings json`). No elided `...` inside executable lines. See-also links: `cli.md`, `json-output.md`, `errors-and-warnings.md`. Spec: "Recipe coverage" / "Each recipe is copy/paste-ready"; "Example-output labeling" / "Output blocks are labeled"; "Safety and non-rendering constraints" / "Each recipe carries its applicable constraints"; "No duplication of reference content" / "Flags resolve through links".

- [x] 3. Write **Recipe B — GitHub Actions** in `docs/recipes.md`: goal, a job-skeleton YAML block generalizing the repository's self-use pattern (`.github/workflows/action.yml`, `uses: ./`, `version`/`path` outputs), adding pipeline value beyond `docs/github-action.md` (stdout redirect, `actions/upload-artifact@v7`, job summary derived from audit JSON), one labeled (illustrative) log/summary output block, and a constraints blurb. Link `docs/github-action.md` for its existing examples and do NOT copy those three examples into the page; name `.github/workflows/action.yml` in prose/code and emit no absolute blob URL. Spec: "Recipe coverage" / "Actions recipe adds pipeline value"; "Example-output labeling" / "Output blocks are labeled"; "No duplication of reference content" / "Actions examples are not duplicated"; "Link-validation gate" (no blob URLs).

- [x] 4. Write **Recipe C — Python / pandas orchestration** in `docs/recipes.md`: goal, install block showing the `oxdoc-python` wrapper and the `oxdoc` binary are installed separately, `Oxdoc(binary="./target/release/oxdoc")`, `extract_rows(...)` → `pandas.DataFrame` with the numeric-as-string caveat and the schema-v2 `formula` / `formula_cached` keys, `extract_csv(...)` → `io.StringIO`, `extract_text_records([...])` per-file error tolerance, and `OxdocResult.warnings`. One minimal block explicitly labeled illustrative (the `DataFrame` repr), one constraints blurb (formulas are never recalculated), and see-also links `python-integration.md`, `json-output.md`. Spec: "Recipe coverage" / "Each recipe is copy/paste-ready"; "Example-output labeling" / "Output blocks are labeled" and "No fabricated content"; "Safety and non-rendering constraints".

- [x] 5. Add the two navigation insertions only: in `docs/_sidebar.md`, append `- [Integration Recipes](recipes.md)` as the last entry of the **Usage** section (after Python Integration); in `README.md`, add `- [Integration Recipes](docs/recipes.md)` to the "Key documentation pages" list after Python Integration (line 55). Touch no other existing page. Spec: "Navigation links" / "Sidebar entry under Usage", "README documentation list entry", "Minimal diff outside the new page".

- [x] 6. Verify Slice 1 gates locally and record the output in `openspec/changes/docs-integration-recipes/apply-progress.md`: `make docs-check`, `make docs-links`, `make docs-schemas-check`, `make docs-playground-check` all pass; confirm every link in `docs/recipes.md` is relative (`cli.md`, `json-output.md`, ...), at most one absolute `github.com/spereyra-dev/oxdoc` URL, no blob URLs, and any schema link points under `docs/schemas/v1/...`. Spec: "Link-validation gate" / "Docs links gate green", "Schema links use published mirrors", "Other docs gates stay green"; "Docsify-stable heading anchors" / "Deep links resolve" (check `docs/recipes.md#recipe-a-shell-pipelines`, `#recipe-b-github-actions`, `#recipe-c-python-pandas-orchestration`).

- [x] 7. Commit Slice 1 as one reviewable work unit on the `stacked-to-main` chain: include the change's spec artifact `openspec/changes/docs-integration-recipes/specs/integration-recipes/spec.md` alongside `docs/recipes.md`, `docs/_sidebar.md`, and `README.md`. Confirm `git diff --stat` for the slice touches only `docs/recipes.md`, `docs/_sidebar.md`, `README.md`, and files under `openspec/changes/docs-integration-recipes/`, and that no `openspec/specs/**` file is created or edited. Spec: "Navigation links" / "Minimal diff outside the new page"; "Recipe coverage" (Slice 1 covers categories 1–3, Slice 2 completes 4–5).

## Slice 2 — Recipes D/E + closing reference list + final gates (PR 2)

- [ ] 8. Append **Recipe D — Backend ingestion under resource limits** to `docs/recipes.md`: goal; a batch/limited request command block using `--max-input-size`, `--max-package-uncompressed-size`, `--max-part-size`, `--max-compression-ratio`; typed limit failures `E014` (input size), `E011` (uncompressed size), `E005`/`E006` (part size, suspicious entry); `--warnings json` machine-readable stderr; a JSONL batch error record with `document_type: "unknown"`; `oxdoc diagnostics --format json`; and the stdin no-spill constraint (stdin buffered only up to `--max-input-size` because ZIP central-directory access needs a seekable source; no temp file). Example-output blocks labeled and consistent with `docs/cli.md` / `docs/json-output.md`. See-also links: `cli.md`, `json-output.md`, `security.md`, `performance.md`. Spec: "Recipe coverage" / "All five recipes present" and "Each recipe is copy/paste-ready"; "Example-output labeling" / "No fabricated content"; "Safety and non-rendering constraints" / "Each recipe carries its applicable constraints".

- [ ] 9. Append **Recipe E — Audit workflow** to `docs/recipes.md`: goal; `oxdoc audit --format json` / `--format jsonl` / `--format text` command blocks; a triage step filtering signals by `kind`/`severity` (for example `jq` over audit JSON) plus a basic E-code triage step routing failed files by the stable `error.code` values carried in audit error records; `document_type: "unknown"` on failed files; a constraints blurb stating audit output is factual signals only (no risk scoring, no mutation) and that `audit` / `error` are mutually exclusive in the JSONL contract (link `docs/schemas/v1/oxdoc-audit-jsonl.schema.json`). Example-output block labeled and transcribed from `docs/audit.md` shapes. See-also links: `audit.md`, `errors-and-warnings.md`, `json-output.md`. Spec: "Recipe coverage" / "All five recipes present"; "Example-output labeling"; "Safety and non-rendering constraints" / "No new behavior claims"; "Link-validation gate" / "Schema links use published mirrors".

- [ ] 10. Close `docs/recipes.md` with the reference list required by spec: links to `cli.md`, `json-output.md`, `audit.md`, `errors-and-warnings.md`, `github-action.md`, `python-integration.md`, plus the `docs/schemas/...` mirrors where used. Confirm no recipe restates a flag table, a full output-field list, or the `docs/github-action.md` examples. Spec: "No duplication of reference content" / "Closing reference list", "Flags resolve through links", "Actions examples are not duplicated".

- [ ] 11. Run the final gates on the complete page and record results in `openspec/changes/docs-integration-recipes/apply-progress.md`: `make docs-links`, `make docs-check` (all five recipe anchors resolve), `make docs-schemas-check`, `make docs-playground-check`; then read every recipe command/YAML/Python block against `docs/cli.md` and, where available, `oxdoc --help`, recording the read-through as the evidence trail. Spec: "Link-validation gate" / "Docs links gate green", "Other docs gates stay green"; "Docsify-stable heading anchors" / "Deep links resolve" and "Anchors survive heading edits" (headings use only letters, digits, spaces, and hyphens).

- [ ] 12. Whole-change guards before PR 2 review: `git diff --stat` across the chain touches only `docs/recipes.md`, `docs/_sidebar.md`, `README.md`, and `openspec/changes/docs-integration-recipes/**`; no file under `crates/`, `python/`, `schemas/`, `docs/schemas/`, `.github/workflows/`, or `tests/` differs; every recipe heading slug is unchanged from `#recipe-a-shell-pipelines` through `#recipe-e-audit-workflow`; and the realized changed-line count is measured and recorded — if the measured total exceeds 400, land the two slices as separate stacked PRs (no `size:exception`). Spec: "Navigation links" / "Minimal diff outside the new page"; "Docsify-stable heading anchors" / "Anchors survive heading edits"; proposal Rollback and Success criteria 6–7, 10.

## Post-apply handoff

- [ ] 13. Hand `docs/recipes.md` + `apply-progress.md` evidence (gate outputs, command read-through notes, measured line count) to the verify phase; the verify report must re-run the four docs gates and confirm the six spec requirements statically against the page. Spec: all six requirements (verification is read-only over the page and the gates; no test harness is added — Non-goal 5).
