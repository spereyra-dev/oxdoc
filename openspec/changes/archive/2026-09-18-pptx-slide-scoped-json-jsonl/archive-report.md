# Archive Report — pptx-slide-scoped-json-jsonl (#180)

- Store: openspec
- Archive date: 2026-09-18
- Archive destination: `openspec/changes/archive/2026-09-18-pptx-slide-scoped-json-jsonl/`
- Branch: `sdd-archive/pptx-slide-scoped-json-jsonl`
- Verification: optional, not run as a standalone SDD phase (verify-report
  locator `<unresolved>`); the final verified state is recorded from the
  apply-progress task 58–60 gate runs, per the native archive instruction
  ("Archive records the actual task state and any available verification
  findings; neither a report nor task completion is an admission requirement").

## Status

**PASS** — archived with no blockers.

## Consumed structured status

Native `gentle-ai.sdd-status` v2, change `pptx-slide-scoped-json-jsonl`:
`state: ready`, `applyState: all_done`, `taskProgress 60/60 allComplete`,
`dependencies.archive: ready`, `nextRecommended: archive`, `blockedReasons: []`,
`artifactStore: openspec`, `actionContext.mode: repo-local`,
`allowedEditRoots: [repo root]`, `sameDomainActiveChanges: []`,
`dependsOn/supersedes/amends/conflictsWith: []`. All composition, move, and
write paths stayed inside the allowed edit root
`C:\Users\Usuario\Desktop\...\oxdoc` (repo root).

## Preconditions checked

| Gate | Result |
| --- | --- |
| proposal.md present | yes |
| design.md present | yes |
| specs/pptx-slide-extraction/spec.md present | yes (11 requirement headings, 28 scenarios) |
| tasks.md present | yes |
| apply-progress.md present | yes |
| verify-report.md | absent — optional artifact, not a blocker |
| sync-report.md | absent — not required; archive owns composition |
| config.yaml rules | applied (see below) |
| Unchecked `- [ ]` implementation tasks | **0** (grep `^\s*- \[ \]` over tasks.md at archive time, immediately before composition and move) |
| Stale-checkbox reconciliation | not needed — no unchecked tasks |
| Verification blockers (FAIL/BLOCKED/CRITICAL) | none (no verify report exists) |
| Destructive merge | none — new canonical domain, nothing removed or overwritten |

## Final verified state at change close (from apply-progress task 58–60)

- `cargo test --workspace` all green: cli bin 25, cli 106, core lib 112,
  api 99, schema 15, tabular 9+2; fmt/clippy `-D warnings` clean; doctest 2 passed.
- Coverage: `cargo llvm-cov --fail-under-lines 95` → lines **96.39%**, exit 0.
- `make ci` component equivalents all green except one documented,
  unrelated environmental exception: `sh tests/install.sh` fails on this
  Windows host because native curl cannot open `file:///tmp/...` POSIX paths
  (curl error 37). `tests/install.sh` is untouched by this change (pre-existing
  host limitation, Markdown-only final diff) — not a change defect.
- compatibility-corpus-check passed (3 fixtures); no digest or
  compatibility-matrix change; docs-schemas-check silent; docs-links 0 broken;
  docs-check, docs-playground-check, build-release, python-test green.
- Delivered as 8 merged PRs against the 400-line review budget
  (ask-on-risk pauses resolved by maintainer-approved splits):
  #217 (fixtures), #218 (output-neutral `@id` parse + planning artifacts),
  #219 (model/API), #220 (skip warnings + fixture fix), #221 (security/notes
  characterization tests), #222 (slides schema v1 + mirror), #223 (schema
  validation + snapshots + negative tests), #224 (extract slides CLI),
  #225 (docs). GitHub issue #180 CLOSED.
- No `size:exception` used; per-unit overages were resolved by explicit
  ask-on-risk splits, never by shrinking code/tests/docs.

## Spec composition (archive-time)

Domain `pptx-slide-extraction` — the change's only delta spec.

- Canonical target `openspec/specs/pptx-slide-extraction/spec.md` did not
  exist → treated the change spec as the full domain spec and copied it
  verbatim (byte-identical, `diff -u` clean).
- Operations: **15 ADDED requirements** (all new; no MODIFIED, no REMOVED —
  the domain is new, so the delta is a full spec copy):

  1. Slide identity from p:sldId
  2. 1-based slide_ordinal in p:sldIdLst order
  3. slide_path provenance
  4. One record per slide with separate body text and notes
  5. Per-slide skip-with-warning for missing targets
  6. Malformed slide XML keeps recoverable partial text
  7. Security and hard-error boundaries are preserved
  8. Existing PPTX extraction stays byte-identical
  9. JSON document contract
  10. JSONL stream contract
  11. Versioned slides schema contract
  12. extract slides CLI subcommand
  13. Fixtures and provenance
  14. Warning texts are stable and documented
  15. Documentation and changelog

- Resume-prior-composition check: no prior sync-report, archive report, or
  history touching `openspec/specs/pptx-slide-extraction/` exists; the target
  was absent, so there were no already-applied or unresolved operations to
  reconcile. Canonical specs `docx-extraction` and `structured-text-schema`
  were untouched, as expected.

## Same-domain collisions

None. `openspec/changes/` contains no other active change touching the
`pptx-slide-extraction` domain (native status `sameDomainActiveChanges: []`,
confirmed by filesystem inspection).

## Task truth

- tasks.md at archive time: **60/60 `[x]`**, 0 unchecked implementation task
  markers (re-verified mechanically immediately before composition and move).
- Task truth preserved byte-for-byte into the archive; no checkbox edited at
  archive time; no stale-checkbox reconciliation performed or needed.

## Destructive merge guard

Not triggered: no REMOVED requirements, no MODIFIED blocks, no canonical file
overwritten, no scenarios dropped. No destructive approval was required.

## Config rules applied (openspec/config.yaml)

- `rules.sync`: delta composed into canonical source specs; task truth
  preserved.
- `rules.archive`: this report records the final verified state, including
  the documented `tests/install.sh` environmental exception that postdates the
  per-unit gate snapshots.

## Archival move

- Source: `openspec/changes/pptx-slide-scoped-json-jsonl/`
- Destination: `openspec/changes/archive/2026-09-18-pptx-slide-scoped-json-jsonl/`
- Destination did not exist beforehand (no overwrite); Windows `mv` lock
  precedent honored via copy → `diff -r` verify → source removal.
- Audit trail preserved: exploration.md, proposal.md, design.md, tasks.md,
  apply-progress.md, and the delta spec move unchanged into the archive.
- Commit: on `sdd-archive/pptx-slide-scoped-json-jsonl`, Conventional Commit,
  no push, no PRs.
