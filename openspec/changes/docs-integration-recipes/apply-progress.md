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

### Remaining tasks (Slice 2, unchecked in tasks.md)

- [ ] 8. Recipe D — Backend ingestion under resource limits.
- [ ] 9. Recipe E — Audit workflow (jq primary triage + `--format text` alternative).
- [ ] 10. Closing `## Reference pages` list.
- [ ] 11. Final gates on the complete page + command read-through evidence.
- [ ] 12. Whole-change guards (file-surface, slugs, measured diff; stack PRs if > 400).
- [ ] 13. Post-apply handoff to verify.

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
