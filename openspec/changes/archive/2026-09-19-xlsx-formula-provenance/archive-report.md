# Archive Report — 2026-09-19-xlsx-formula-provenance

- Archive status: **PASS**
- Change id: `2026-09-19-xlsx-formula-provenance`
- Issue: #182 (CLOSED)
- Artifact store: `openspec` (repo-local planning home, mode `repo-local`)
- Archive date: 2026-09-19 (session date used for the archive folder label)

## Status and actionContext findings

- Native `gentle-ai.sdd-status` v2 consumed as the authoritative read-only
  projection: change `2026-09-19-xlsx-formula-provenance`, state `ready`,
  `nextRecommended: "archive"`, `blockedReasons: []`, `applyState: "all_done"`.
- `actionContext.mode: "repo-local"`, `workspaceRoot` and `allowedEditRoots`
  both resolve to `C:\Users\Usuario\Documents\projectsTubby\oxdoc` — every
  composition write and the archive move stayed inside the allowed edit root.
- Native task progress: 55/55 complete, `allComplete: true` — matches the
  persisted tasks artifact byte-truth (final gate re-read below).
- Verify-report locator `<unresolved>`: no verify-report artifact exists.
  Verification is optional in this project; per the native archive
  instructions neither a report nor task completion is an admission
  requirement, so the missing report is not a blocker. Available verification
  findings are recorded from apply-progress (below).

## Artifacts read

- `openspec/changes/2026-09-19-xlsx-formula-provenance/proposal.md`
- `openspec/changes/2026-09-19-xlsx-formula-provenance/specs/xlsx-formula-provenance/spec.md`
- `openspec/changes/2026-09-19-xlsx-formula-provenance/design.md`
- `openspec/changes/2026-09-19-xlsx-formula-provenance/tasks.md` (final gate re-read)
- `openspec/changes/2026-09-19-xlsx-formula-provenance/apply-progress.md`
- `openspec/config.yaml` (rules.spec / rules.sync / rules.archive honored)
- Native status payload (parent-injected, `gentle-ai.sdd-status` v2)
- No verify-report exists; no sync-report exists (no legacy sync ran —
  composition was owned by this archive phase).

## Final Task Completion Gate

- Re-read `tasks.md` immediately before composition and the move.
- Checked for unchecked implementation task markers `^\s*- \[ \]`: **zero
  matches** — no `- [ ]` implementation task boxes remain (55/55 checked).
- No stale-checkbox reconciliation was needed or performed; the persisted task
  bytes were preserved unmodified through the archive move.

## Verification findings (recorded state, no verify-report artifact)

Per apply-progress final section and the parent-provided final-state facts
(outranking intermediate snapshots), the change closed in this verified state:

- `cargo test --workspace`: 393 tests, 0 failures.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets
  -- -D warnings`: clean.
- Coverage gate: 96.52% line coverage ≥ 95 (`cargo llvm-cov ... --fail-under-lines 95`).
- Python: `python -m unittest discover -s python/tests` → 9 OK (pytest
  unavailable in the environment; documented environmental exception, the
  unittest recipe is the `make python-test` runner).
- `make compatibility-corpus-check` equivalent: passes, no manifest/digest/
  `tests/fixtures/files/**` change.
- Frozen-guard checks: `schemas/v1/**` and `docs/schemas/v1/**` untouched
  across the whole change; whole-change snapshot diff limited to
  `cli_xlsx_rows_v2_jsonl.jsonl`; all pre-existing snapshots byte-identical.
- Markdown links pass (`markdown-link-check` over the six edited doc surfaces).
- Delivery: 10 merged PRs (#227–#236) across 8 review slices, each slice
  measured under the 400-line budget at its own boundary; whole change
  measured ~2299 lines (base `cf0d584`).
- Two intentional, documented breaks in `CHANGELOG.md`: strict v1 rows
  validators (v1 frozen, migrate to `schemas/v2/…`) and the `XlsxCell`
  struct-literal field addition (`formula: None` migration).

## Spec composition (archive-time)

- No canonical spec existed for this domain; per composition rules the change
  delta was treated as a full domain spec and copied verbatim:
  - `openspec/changes/2026-09-19-xlsx-formula-provenance/specs/xlsx-formula-provenance/spec.md`
    → `openspec/specs/xlsx-formula-provenance/spec.md`
  - Verified byte-identical by `diff` after the copy (`COMPOSE-VERIFY-OK`).
- Operations classification: **11 of 11 ADDED operations applied** (new
  domain, no prior canonical content, no history to reconcile — all pending,
  none unresolved). No ADDED/MODIFIED/REMOVED replay against existing
  canonical content was required or performed.
- Requirements recorded as composed (all ADDED into the new canonical domain):
  1. Formula expression capture
  2. Cache-presence provenance (`formula_cached`)
  3. Core API — XlsxFormula and XlsxCell.formula
  4. Shared formula resolution — master-first, bounded
  5. Bounded shared-formula table with injectable limit
  6. Array formulas — master text only, region cells untouched
  7. Warning classification, channel, and no-recalculation guarantee
  8. Rows-jsonl schema version 2
  9. CLI emission of formula fields
  10. Documentation and CHANGELOG versioning
  11. Fixture corpus with provenance

## Destructive merge guard

- No REMOVED requirements and no MODIFIED requirements existed (new domain,
  full-copy composition). No destructive canonical write occurred; no approval
  needed or requested.

## Active same-domain change warnings

- None. `openspec/changes/` currently holds three other active changes
  (`2026-09-17-docx-section-order-related-parts`,
  `2026-09-17-structured-text-source-provenance`,
  `2026-09-18-pptx-slide-scoped-json-jsonl`) — none touches the
  `xlsx-formula-provenance` domain. Native `sameDomainActiveChanges` and
  `conflictsWith` were empty.

## Archive move

- Destination: `openspec/changes/archive/2026-09-19-xlsx-formula-provenance/`
  (created via move; `archive/` already existed).
- Collision check: destination did not exist before the move — nothing
  overwritten.
- Per the Windows `mv` lock precedent, a copy-verify-remove fallback is
  authorized: if the move fails on a file lock, copy the change tree into the
  archive, verify with `diff -r`, and only then remove the source.
- Historical report and task bytes are preserved unmodified in the archived
  tree; no archived artifact is deleted or edited.

## Deferred items (recorded, not lost)

- openpyxl-generated compatibility-matrix formula workbook (documented in
  CHANGELOG; proposal question-round item 3).
- Compatibility playground registration for the two new corpus trees
  (documented in S1/S2 apply progress).

## Ruled-out blockers

- Missing verify-report: optional artifact, not an admission gate (native
  status `verify: ready` is advisory only; native `archive: ready`).
- No unresolved FAIL/BLOCKED/CRITICAL verification issues recorded anywhere.
- No legacy flat `spec.md` (delta lives at `specs/{domain}/spec.md`).
- No destructive merge, no unchecked tasks, no missing required artifacts.
