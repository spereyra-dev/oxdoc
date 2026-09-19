# Archive Report — release-crates-io-publish

Change: `release-crates-io-publish` · Archive phase 2026-09-18 ·
Artifact store: openspec · Branch `sdd-archive/crates-io-release`.

## Status

**PASS** — archived cleanly. All archive preconditions met; no blockers; no
destructive canonical writes required.

## Structured status and actionContext findings

- Consumed native `gentle-ai.sdd-status` v2 for `release-crates-io-publish`:
  `state: ready`, `nextRecommended: archive`, `dependencies.archive: ready`,
  `taskProgress: 37/37 allComplete`, `applyState: all_done`, `blockedReasons: []`,
  `relationships.sameDomainActiveChanges: []`.
- `actionContext.mode: repo-local`, `workspaceRoot` and `allowedEditRoots` =
  `C:\Users\Usuario\Documents\projectsTubby\oxdoc` — all archive writes
  (canonical spec, archive report, move, commit) stayed inside the workspace root.
- `verifyReport` locator: `<unresolved>` (no verification report was ever
  persisted for this change). Per the archive instructions, verification is
  optional and its absence does not block archive; the verified final state is
  recorded below from the apply-progress receipts.

## Final Task Completion Gate

Re-read `tasks.md` immediately before composition and the move: **37/37 tasks
checked, zero `- [ ]` implementation boxes remain**. No stale-checkbox
reconciliation was needed. Task 10's slice-1 deferral (tabular/cli
`cargo package --no-verify` blocked on the unpublished upstream) is recorded in
apply-progress as resolved in slice 2 with first-hand pass receipts.

## Artifacts read

- `openspec/config.yaml` (rules.archive / rules.sync applied)
- `openspec/changes/release-crates-io-publish/proposal.md`
- `openspec/changes/release-crates-io-publish/specs/crates-io-release/spec.md`
- `openspec/changes/release-crates-io-publish/design.md`
- `openspec/changes/release-crates-io-publish/tasks.md` (twice: preflight + gate)
- `openspec/changes/release-crates-io-publish/apply-progress.md` (with receipts)
- No `verify-report.md` and no `sync-report.md` exist for this change.

## Domains synced / spec composition

| Domain | Action | Evidence |
| --- | --- | --- |
| `crates-io-release` | **New canonical spec** — full copy of the change delta to `openspec/specs/crates-io-release/spec.md` | No canonical file existed; no other active change touches the domain (`sameDomainActiveChanges: []`, filesystem check). Copy verified byte-identical via `diff`. |

- Requirement names carried into the new canonical spec (the delta uses full
  `### Requirement:` blocks, not ADDED/MODIFIED/REMOVED operation sections —
  a new-domain full-copy composition):
  1. Approved-version consistency
  2. Publishable crate metadata
  3. Packaging validation before publish
  4. Documented release checklist
  5. Documentation truthfulness (pre/post-publish traffic light)
  6. Human-gated local publish
  7. CHANGELOG dates the release
  8. No behavior or CI change
- ADDED/MODIFIED/REMOVED requirement names: none (no operation sections present;
  8 requirements composed by full copy).
- Already-applied / pending / unresolved operations: none to reconcile — no
  prior composition exists for this domain (new canonical file).
- Destructive merge approvals/blockers: none — no REMOVED requirements, no
  MODIFIED blocks, no existing canonical content touched. The five existing
  canonical specs (`docx-extraction`, `structured-text-schema`,
  `pptx-slide-extraction`, `xlsx-formula-provenance`, `integration-recipes`)
  were not touched.

## Same-domain change warnings

None. `release-crates-io-publish` is the only active change and the only one
authoring `specs/crates-io-release/`.

## Verification findings (final state; no verify-report artifact exists)

From `apply-progress.md` receipts and slice-2 gate evidence (final-state truth,
outranking intermediate snapshots):

- Real publish completed under maintainer authorization on 2026-09-18: tag
  `v2.0.0` pushed at `79bef11`; `oxdoc-core` 2.0.0, `oxdoc-tabular` 0.2.0,
  `oxdoc-cli` 2.0.0 published to crates.io in dependency order
  (core → tabular → cli, stop between crates), each with dry-run and registry
  verification. First-hand independent checks: crates.io API `max_version`
  matches all three; deferred dependent-crate `cargo package --no-verify` and
  full `cargo publish --dry-run` re-runs passed post-publish; clean-target
  `cargo install oxdoc-cli --version 2.0.0` succeeded; `v2.0.0` GitHub Release
  live (verified via web page; GitHub REST API was IP-rate-limited).
- Shipped as 2 merged PRs: #241 (slice 1: version bump + metadata +
  `docs/releasing.md` + pre-publish docs truth + packaging receipts, merged at
  `79bef11`) and #242 (slice 2: post-publish docs flip + receipts, merged at
  `b509bf9`). GitHub issue #173 CLOSED.
- Final gates: `cargo test --workspace` green; fmt/clippy clean; coverage
  96.53% ≥ 95; docs gates green, with `docs-links` green including the live
  crates.io URL.
- Known recorded deviation (honest, non-blocking): `.markdown-link-check.json`
  gained a scoped `httpHeaders` block for `https://crates.io/` (browser-like
  UA/Accept headers) instead of a bare ignore removal, because crates.io's bot
  filter 403/404s header-less automated requests. This deviates from the design
  letter ("remove the ignore") while preserving its intent ("docs-links
  validates the live URL"); recorded in apply-progress slice-2 deviations.
- Known environment limitations (pre-existing, not caused by this change):
  `make` not installed locally (recipes run as individual component commands);
  `tests/install.sh` fails only in the local Windows/MSYS environment (verified
  pre-existing on clean `main`; green in Linux CI).

## Archive move

- `openspec/changes/release-crates-io-publish/` →
  `openspec/changes/archive/2026-09-18-release-crates-io-publish/`
- Destination collision check: no existing
  `openspec/changes/archive/2026-09-18-release-crates-io-publish/` before the
  move; date 2026-09-18 matches the change's publish/session date.
- Move method: copy → verify → remove (Windows mv lock precedent accepted by
  the parent). Archived contents preserved as-is (audit trail); nothing inside
  the change folder was modified except the addition of this `archive-report.md`
  before the move.

## Unchecked task lines

None — `tasks.md` has no `- [ ]` lines (37/37 `[x]`).

## Partial-archive / reconciliation notes

No partial archive; no stale-checkbox reconciliation performed. The task-10
deferral story (fail-then-retried receipts, post-upstream resolution) is
already honestly recorded in `apply-progress.md` and is preserved verbatim in
the archived copy.
