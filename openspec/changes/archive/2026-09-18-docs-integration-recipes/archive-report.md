# Archive Report — docs-integration-recipes (issue #176)

- Archive date: 2026-09-18
- Archive destination: `openspec/changes/archive/2026-09-18-docs-integration-recipes/`
- Branch at archive time: `sdd-archive/integration-recipes`
- Artifact store: `openspec` (repo-local, per session preflight)
- Archive status: **PASS**

## Structured status and actionContext findings

Native `gentle-ai.sdd-status` v2 consumed at phase start:

- `changeName: docs-integration-recipes`, `nextRecommended: archive`, `archive: ready`, `applyState: all_done`.
- `taskProgress: 13/13 complete, allComplete: true`.
- `actionContext.mode: repo-local`, `allowedEditRoots` = workspace root `C:\Users\Usuario\Documents\projectsTubby\oxdoc` — all archive writes (canonical spec, report, move) stayed inside the authoritative root.
- No same-domain active changes; no `dependsOn`/`supersedes`/`amends`/`conflictsWith` relationships.
- `verifyReport: missing` — verification was optional (native phase instruction: "a missing, stale, malformed, or failed report does not block archive"). No verify report exists and none is required.

## Artifacts read

- `openspec/changes/docs-integration-recipes/proposal.md`
- `openspec/changes/docs-integration-recipes/specs/integration-recipes/spec.md`
- `openspec/changes/docs-integration-recipes/design.md`
- `openspec/changes/docs-integration-recipes/tasks.md` (re-read at the Final Task Completion Gate, immediately before composition and move)
- `openspec/changes/docs-integration-recipes/apply-progress.md` (Slices 1 and 2, complete)
- `openspec/config.yaml` (`rules.archive`: archive reports final verified state; `rules.sync`: compose applicable delta specs, preserve task truth)
- Archive destination collision check: no `2026-09-18-docs-integration-recipes` existed; `openspec/changes/archive/` already present.

## Task completion gate

- Re-read the persisted `tasks.md` immediately before composition and the move: **no `- [ ]` implementation task boxes remain** (all 13 tasks `[x]`).
- No stale-checkbox reconciliation was needed and none was performed; task bytes are preserved as applied.

## Verification findings

No verify-report artifact exists (optional, not run). The archived final state is recorded from the apply-progress evidence trail and the parent-provided final-state facts, which outrank intermediate snapshots:

- Shipped as 2 merged PRs: #238 (Slice 1: recipes A/B/C + nav + planning artifacts) and #239 (Slice 2: recipes D/E + reference list). Issue #176 CLOSED.
- Whole-change budgetable diff 389 lines (design D8 convention: excludes `openspec/**` bookkeeping and `.gitignore`), measured against base `403d3b6` — under the 400-line review budget; no `size:exception` claimed.
- Docs-only; zero changes under `crates/`, `python/`, `schemas/`, `docs/schemas/`, `.github/workflows/`, `tests/`.
- Final verified state at close: docs-links whole tree 0 dead links; docsify serve + docs-schemas-check + playground checks PASS; `cargo test --workspace` 393 green; fmt/clippy clean.
- Environmental notes documented honestly in apply-progress: `make` unavailable on the Windows host — recipes invoked directly; `jq` unavailable locally — triage filters validated with equivalent logic; port contention on docsify serve handled by parsing the bind port.

## Domains synced and spec composition

| Domain | Operation | Result |
|--------|-----------|--------|
| `integration-recipes` | New canonical domain (no canonical existed) | Full copy of the change delta spec to `openspec/specs/integration-recipes/spec.md`; byte-identical (`diff` verified) |

- The delta spec carries no `ADDED`/`MODIFIED`/`REMOVED` requirement sections — it is a complete domain spec (Purpose + six requirements with scenarios), so the full-copy rule applied.
- Requirement names composed: Recipe coverage; Example-output labeling; Safety and non-rendering constraints; Navigation links; Docsify-stable heading anchors; Link-validation gate; No duplication of reference content. (All ADDED-by-copy; no MODIFIED/REMOVED operations.)
- Existing canonical domains (`docx-extraction`, `structured-text-schema`, `pptx-slide-extraction`, `xlsx-formula-provenance`) untouched, as the proposal required.
- No legacy flat `spec.md`; no `sync-report.md` (archive-time composition owned the composition; none was pending — the delta had a single new-domain operation, which was applied).
- No destructive merge: zero REMOVED requirements, zero MODIFIED blocks; no destructive approval required or requested.
- Same-domain active-change collision check: no other change under `openspec/changes/` touches the `integration-recipes` domain.
- No `## RENAMED Requirements` sections encountered.

## Destructive merge approvals / blockers

- None. No destructive canonical spec operation was performed; no parent approval was required or requested for composition.

## Non-critical partial archive / reconciliation

- None required: full archive, all artifacts present, all tasks complete.

## Archived path

- `openspec/changes/docs-integration-recipes/` → `openspec/changes/archive/2026-09-18-docs-integration-recipes/` (move performed after composition and report write; destination did not exist; audit trail preserved in full).

## Commit

- Conventional Commit on `sdd-archive/integration-recipes`: `docs(specs): archive docs-integration-recipes and compose canonical integration-recipes spec (#176)` — no push, no PR (parent instruction).
