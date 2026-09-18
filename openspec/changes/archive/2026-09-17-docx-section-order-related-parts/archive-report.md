# Archive Report — docx-section-order-related-parts

## Archive status: PASS

Archived 2026-09-17 by the SDD archive phase. Change closed and moved to
`openspec/changes/archive/2026-09-17-docx-section-order-related-parts/`.

## Structured status / actionContext consumed

- Native `gentle-ai.sdd-status` v2: `changeName:
  docx-section-order-related-parts`, `artifactStore: openspec`,
  `nextRecommended: archive`, `dependencies.archive: ready`, `applyState:
  all_done`, `taskProgress 14/14 allComplete`, no `blockedReasons`.
- `actionContext.mode: repo-local`, workspaceRoot and `allowedEditRoots` =
  `C:\Users\Usuario\Documents\projectsTubby\oxdoc` — all writes (canonical
  spec, archive report, move target) stayed inside the authoritative root.
- Verify-report locator: `<unresolved>` — verification was optional and not
  run in-store; final-state verification facts are recorded below per
  `rules.archive`. Not a blocker per the archive preconditions.

## Preconditions checked

- Required artifacts present: `proposal.md`, `specs/docx-extraction/spec.md`,
  `design.md`, `tasks.md`, `apply-progress.md`. `config.yaml` rules
  (`rules.archive`, `rules.sync`) applied.
- No `verify-report.md` in the store (optional artifact, absent — recorded,
  not blocking).
- Final Task Completion Gate (re-read immediately before composition):
  zero `- [ ]` implementation task markers in `tasks.md`; 14/14 `[x]`.
  No stale-checkbox reconciliation was needed or performed.
- No same-domain active changes (the only change under `openspec/changes/`
  was this one).
- Archive destination `openspec/changes/archive/2026-09-17-…` did not exist
  before the move; nothing overwritten.

## Artifacts read

`proposal.md`, `specs/docx-extraction/spec.md`, `design.md`, `tasks.md`,
`apply-progress.md` (both sessions), `exploration.md` (present in change
root), `openspec/config.yaml`, native status projection.

## Final verified state (rules.archive: report actual, incl. post-apply fixes)

- Issue #177 CLOSED on GitHub; WU1 merged as PR #210 (squash `4c6a74d`), WU2
  merged as PR #211 (squash on top of `4c6a74d`).
- `cargo test --workspace`: 321 tests, 0 failures.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets
  -- -D warnings`: pass.
- Coverage gate passes locally: 96.16% total lines (docx.rs 97.72% lines,
  100% functions) against the 95% gate.
- python-docx generated snapshot byte-identical; SHA-256 recorded in
  `tests/fixtures/provenance/docx-python-docx-basic.md` matches (fixture not
  regenerated).
- Measured sizes: WU1 ≈ 846–868 changed lines (accepted `size:exception` by
  maintainer decision); WU2 reviewable slice ≈ 329–370 lines, within the
  400-line budget.
- No CLI snapshot changes; no public output schema changes.

## Domains synced

| Domain | Source delta | Canonical target | Result |
| --- | --- | --- | --- |
| docx-extraction | `openspec/changes/docx-section-order-related-parts/specs/docx-extraction/spec.md` | `openspec/specs/docx-extraction/spec.md` | Created (first archive — no canonical specs existed). Full domain spec copied verbatim; byte-identical verified. |

## Requirement operations applied (canonical composition)

No prior canonical spec existed, so the delta was composed as a complete new
canonical domain spec (new-spec path, not ADDED/MODIFIED/REMOVED replay).
Requirements now canonical in `openspec/specs/docx-extraction/spec.md`:

1. Section-aware ordering of header and footer parts
2. Deduplication by resolved part path at first reference
3. Unreferenced header and footer parts are appended, not dropped
4. Footnotes, endnotes, and comments keep relationship order
5. Missing and dangling reference ids produce stable warnings
6. Existing warning and error behavior is preserved exactly
7. titlePg is a no-op for output
8. evenAndOddHeaders flag is not consulted (deferred)
9. One shared ordering helper across all extraction paths
10. Deterministic output for single-section documents with ordered rels
11. No public output schema changes

- ADDED (via new canonical spec): all 11 requirements above.
- MODIFIED: none. REMOVED: none.
- Resume-prior-composition reconciliation: no prior sync report, prior
  archive report, or prior canonical content existed; nothing to classify as
  already applied or unresolved. First composition, no partial history.
- Destructive merge guard: not triggered — no REMOVED operations, no
  replacement of existing canonical content; zero canonical lines destroyed.

## Same-domain active change warnings

None — no other active change touches `docx-extraction` or any other domain.

## Task truth

- `tasks.md`: 14/14 complete (`- [x]`), no unchecked implementation tasks
  remain. Apply-progress sessions 1 (WU1, tasks 1–7) and 2 (WU2, tasks 8–14)
  both recorded completion with persisted checkbox updates.

## Blockers / approvals

- No verification blockers (no report; final state independently verified
  green as recorded above).
- No destructive merges; no `size:exception` decisions were needed by
  archive itself — the recorded WU1 exception was a delivery-phase maintainer
  decision, preserved here as history.
- No explicit partial-archive approval needed; archive is complete, not
  partial.

## Archived path

- Before: `openspec/changes/docx-section-order-related-parts/`
- After: `openspec/changes/archive/2026-09-17-docx-section-order-related-parts/`

## Recovery metadata

- Branch at archive time: `sdd-archive/docx-section-order-related-parts`
  (checked out; committed locally, not pushed, no PR created).
- To restore an archived change for rework: copy the archived folder back to
  `openspec/changes/{change}/` and reset `openspec/specs/docx-extraction/`
  if a full revert of the canonical composition is desired. The archive
  folder is an immutable audit trail; do not edit in place.
- Canonical spec of record: `openspec/specs/docx-extraction/spec.md`
  (composed from the delta at archive; provenance = this report + archived
  change folder).
