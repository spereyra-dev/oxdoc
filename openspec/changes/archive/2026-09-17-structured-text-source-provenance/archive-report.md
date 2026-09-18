# Archive Report — structured-text-source-provenance

## Status

PASS — archive completed successfully.

## Executive Summary

Change `structured-text-source-provenance` (issue #179) archived after all 35
tasks completed, three PRs merged (WU1 #213 variant plumbing; WU2 #214 schema
v2 + CLI + snapshots; WU3 #215 docs), and final verification recorded in the
parent handoff: `cargo test --workspace` 329 tests / 0 failures, fmt/clippy
clean, coverage 96.22% (gate 95), docs-schemas-check identical for v1 and v2,
markdown links validated. Intentional v1-strict breakage is documented in
`docs/json-output.md` and `CHANGELOG.md` with migration guidance.

## Structured Status and Action Context

- Native `gentle-ai.sdd-status` v2 consumed: change state `ready`,
  `nextRecommended: archive`, `blockedReasons: []`, apply state `all_done`
  (35/35), verify dependency `ready` (verification optional; no report exists).
- `actionContext.mode: repo-local`, workspace root
  `C:\Users\Usuario\Documents\projectsTubby\oxdoc`; all reads, compositions,
  report write, and archive move stayed inside `allowedEditRoots`.

## Artifacts Read

- `openspec/changes/structured-text-source-provenance/proposal.md` (via status engine artifact paths)
- `openspec/changes/structured-text-source-provenance/specs/docx-extraction/spec.md`
- `openspec/changes/structured-text-source-provenance/specs/structured-text-schema/spec.md`
- `openspec/changes/structured-text-source-provenance/design.md` (status engine: done)
- `openspec/changes/structured-text-source-provenance/tasks.md` (re-read at final gate)
- `openspec/changes/structured-text-source-provenance/apply-progress.md`
- `openspec/config.yaml` (`rules.archive`, `rules.sync`)
- Prior archive `openspec/changes/archive/2026-09-17-docx-section-order-related-parts/` (composition history; Windows mv lock precedent)
- No verify-report exists (optional; missing report is not a blocker).
- No sync-report exists; archive owns composition per current contract.

## Final Task Completion Gate

Re-read `tasks.md` immediately before composition: 35/35 `- [x]`, zero
`- [ ]` implementation task markers. No stale-checkbox reconciliation needed.

## Domains Synced (Archive-Time Spec Composition)

### docx-extraction — MODIFIED (pending → applied)

- Requirement: `No public output schema changes` — full canonical requirement
  block replaced in `openspec/specs/docx-extraction/spec.md` (delta was the
  `#177`-era variant prohibition; new block permits exactly one additive
  optional `variant` field carried by schema v2). 2 scenarios replaced.
- Prior state check: canonical block matched the pre-delta `#177`-era variant
  (deferring variant labeling), confirming this MODIFIED operation was pending,
  not already applied. No prior archive report claimed this operation for the
  canonical file, and the delta's scenario set (2 scenarios) did not exist in
  the canonical file before this write.
- All 7 unrelated canonical requirements preserved untouched.

### structured-text-schema — full copy (new canonical)

- `openspec/specs/structured-text-schema/spec.md` did not exist; delta copied
  verbatim (diff verified identical). 6 requirements added:
  `Structured-text schema version 2`, `Schema versioning policy`,
  `Payload carries schema_version 2`,
  `Plain-text and tables output never carry provenance metadata`,
  `PPTX structured blocks are labeled slide and notes`,
  `Global ordinal semantics are documented and stable`.

## ADDED/MODIFIED/REMOVED Summary

- MODIFIED: `No public output schema changes` (docx-extraction)
- ADDED (via new canonical spec, structured-text-schema): the 6 requirements listed above
- REMOVED: none

## Destructive Merge Guard

The MODIFIED replacement removed a prior requirement block (~9 lines) and its
2 scenarios in favor of the full corrected block — an ordinary MODIFIED
operation with the complete requirement text present in the delta (not partial;
no scenarios silently dropped: 2 in, 2 out). Parent-provided inputs explicitly
authorize this composition. No REMOVED operations. No destructive approval
beyond this delta was needed or granted.

## Same-Domain Collisions

None. `openspec/changes/` contained no other active change (only `archive` and
this change). Native `relationships.sameDomainActiveChanges` was empty.

## Unresolved Operations

None. Both operations classified pending before write and applied once; no
replay, no overwrites of later work.

## Verification Findings

No `verify-report.md` was produced (verification was optional per native
status). The final verified state comes from the parent handoff's
final-state facts: 329 tests / 0 failures, fmt/clippy clean, coverage 96.22%
≥ 95, docs-schemas-check pass for v1+v2, markdown links validated. Archive
records this actual state per `rules.archive`.

## Archive Move

- Source: `openspec/changes/structured-text-source-provenance/`
- Destination: `openspec/changes/archive/2026-09-17-structured-text-source-provenance/`
- Method: copy-verify-remove (Windows mv lock fallback, per archived #177
  precedent). Destination verified empty before copy; full recursive diff
  verified after copy before removing the source.

## Audit Trail

All change artifacts (proposal, specs, design, tasks 35/35, apply-progress,
this report) are preserved byte-for-byte under the dated archive directory.
