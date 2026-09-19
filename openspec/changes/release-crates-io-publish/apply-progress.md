# Apply Progress — release-crates-io-publish

Change: `release-crates-io-publish` · Slice 1 (tasks 1–22) · Branch
`release-crates-io-s1` · Strict TDD active (`openspec/config.yaml`).

## Executive summary

Slice 1 ships the complete pre-publish state: approved version triple
(`oxdoc-core` 2.0.0 · `oxdoc-tabular` 0.2.0 · `oxdoc-cli` 2.0.0) across every
MUST-EDIT reference, both lockfiles committed for the `--locked` release build,
tabular publishable metadata, the `docs/releasing.md` ordered checklist with
the irreversibility warning and recovery policy, list registrations in both
`docs/_sidebar.md` and `README.md`, the pre-publish documentation traffic light
(badge removed, crates.io moved to "not yet published", launch-publicity
dependency order), and the pre-publish packaging receipts. Tasks 23–30 (human
publish gate) and Slice 2 (31–37) remain unchecked by design.

## TDD Cycle Evidence

| Cycle | Step | Action | Evidence |
| --- | --- | --- | --- |
| 1 (version bump) | RED | Bumped `tests/fixtures/snapshots/cli_info_json.json`, `cli_audit_json.json`, `all_sheets_manifest.json` line 2 and `crates/oxdoc-core/tests/schema.rs:122` literal to `"2.0.0"` while manifests still declared 1.2.0 | `cargo test -p oxdoc-cli --test cli prints_info_as_json_and_text` → **FAILED**: `assertion 'left == right' failed` with `oxdoc_version: String("1.2.0")` (actual, from un-bumped manifest) vs `String("2.0.0")` (fixture) at `crates\oxdoc-cli\tests\cli.rs:1527` |
| 1 (version bump) | GREEN | Edited the three manifests (core `2.0.0`, tabular `0.2.0` + `oxdoc-core = "2.0.0"`, cli `2.0.0` + exact-minor `oxdoc-tabular = "0.2.0"`); regenerated `Cargo.lock` (`cargo check --workspace --all-features`) and `fuzz/Cargo.lock` (`cargo metadata --manifest-path fuzz/Cargo.toml`) | `cargo test --workspace --all-features --all-targets` → all suites ok, exit 0 (128, 101, 20, 27 passing among others; no failures) |
| 1 (version bump) | TRIANGULATE | `cargo pkgid -p oxdoc-core` → `…#oxdoc-core@2.0.0`; `-p oxdoc-tabular` → `0.2.0`; `-p oxdoc-cli` → `2.0.0`; `grep -n '^## 2.0.0' CHANGELOG.md` → line 7 match after the CHANGELOG edit | all pass |
| 1 (version bump) | REFACTOR | Version-reference sweep (no logic exists to restructure): `docs/library-api.md`, `crates/oxdoc-tabular/README.md` snippet + spike-doc absolute link, four doc example payloads (`docs/cli.md`, `docs/json-output.md`, `docs/audit.md`, `docs/recipes.md`), README Status line 1.x → 2.x | `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-features --all-targets -- -D warnings` clean |

## Completed tasks (persisted checkboxes)

- [x] 1. RED — snapshot fixtures + schema.rs literal moved first; focused test
  failure recorded above.
- [x] 2. GREEN — three manifests + cross-crate requirements + **both lockfiles**
  (committed; `release.yml` builds `--locked`); workspace suite green.
- [x] 3. TRIANGULATE — `cargo pkgid` × 3 = 2.0.0 / 0.2.0 / 2.0.0; CHANGELOG
  heading grep matches.
- [x] 4. REFACTOR — stale version references swept; fmt + clippy clean.
- [x] 5. CHANGELOG — `## 2.0.0 - 2026-09-18` with the one-line crate-version
  note (2.0.0 / 0.2.0 / 2.0.0); all three documented breaks preserved under the
  dated heading (rows-JSONL v2, `XlsxCell.formula` source break + migration
  note, structured-JSON v2); 1.2.0 / 1.1.0 / 1.0.0 sections unchanged; no
  `## Unreleased` remains.
- [x] 6. Tabular metadata: `readme = "README.md"`,
  `include = ["Cargo.toml", "README.md", "src/**", "examples/**"]`,
  `[package.metadata.docs.rs] features = ["parquet"]` (not `all-features`),
  `publish = true` kept, `version = "0.2.0"`, `oxdoc-core = "2.0.0"`.
- [x] 7. Tabular README escaping relative link → absolute
  `https://github.com/spereyra-dev/oxdoc/blob/main/docs/spikes/xlsx-arrow-parquet.md`.
- [x] 8. README link audit: `crates/oxdoc-core/README.md` and
  `crates/oxdoc-cli/README.md` contain **zero** markdown links (the CLI README
  has only a code-block `curl` URL for `install.sh`, not a package link);
  `crates/oxdoc-tabular/README.md` has exactly one link, now absolute. No
  relative link crosses any package boundary.
- [x] 9. Per-crate metadata table verified via
  `cargo metadata --no-deps --format-version 1` + manifests (this cargo's
  metadata omits `include`/`publish` keys for inherited fields, so `include`
  was read from the manifests and is proven functionally by `cargo package
  --list` in task 10): description (crate-specific, non-empty) ✓ · license
  `MIT` ✓ · repository/homepage `https://github.com/spereyra-dev/oxdoc` ✓ ·
  `readme = "README.md"` present + file exists (all three) ✓ · `include`
  present (all three; tabular's covers README + both examples) ✓ · keywords
  `docx, xlsx, ooxml, parser, cli` (exactly 5, lowercase) ✓ · categories
  `command-line-utilities, parser-implementations` ✓ · `rust-version 1.88` ✓ ·
  `publish` not `false` (tabular explicit `true`; core/cli inherit default) ✓ ·
  docs.rs metadata for tabular only ✓ · no escaping README links ✓.
- [ ] 10. Package in dependency order — **partially blocked pre-publish**:
  `cargo package -p oxdoc-core` (full verify) ran and passed, and all three
  `--list` inspections ran and passed (receipts below), but the
  `cargo package -p oxdoc-tabular --no-verify` and `… -p oxdoc-cli
  --no-verify` full packages **cannot succeed** until `oxdoc-core` 2.0.0 is
  on the registry (cargo's packaging-for-upload resolution ignores the path
  dependency for the registry requirement; no cargo flag skips the check —
  verified with an `--offline` retry). Their deferred full validation is
  documented in `docs/releasing.md` step 6 and re-enters via the
  post-upstream dry-runs in tasks 27–28. Checkbox intentionally left
  unchecked; reported to the parent as a spec/design assumption gap.
- [x] 11. `cargo publish -p oxdoc-core --dry-run` (full verification, no
  `--no-verify`) recorded — pass (receipt below). `oxdoc-tabular`/`oxdoc-cli`
  dry-runs **cannot** fully verify pre-publish (their registry requirement is
  unsatisfied while core 2.0.0 is unpublished) — that is the documented
  caveat, not a reason to skip any later dry-run.
- [x] 12. Every packaging/dry-run command actually run is recorded below in
  the design's `### Receipt:` format, including the two honest
  `fail(then retried)` receipts for the tabular/cli full package and the
  documented N/A for their pre-publish dry-runs.
- [x] 13. `docs/releasing.md` authored: steps 0–10, each with command →
  expected evidence → stop-on-failure gate, in the mandated order.
- [x] 14. Mandatory content present: irreversibility warning (header),
  credential prerequisite + clean-tree precondition (step 0/8), resolution
  caveat + `--no-verify` workaround + "wait and re-run — never skip a dry-run"
  retry policy (step 6), tag-before-publish ordering consequence (step 7),
  recovery subsection (fix-forward patch versions 2.0.1 / 0.2.1, `cargo yank`
  last resort, never re-publish), "CI publishing (future prerequisite)"
  subsection (no OIDC; `CARGO_REGISTRY_TOKEN` + approved-version input +
  dependency-ordered jobs + rotation, explicitly deferred).
- [x] 15. Registered in both lists: `docs/_sidebar.md` Project section as
  `- [Releasing](releasing.md)` next to Release Process; `README.md`
  "Key documentation pages" as `- [Releasing](docs/releasing.md)`.
- [x] 16. `docs/release-process.md` §Crates.io Publishing gains one pointer
  line to `[Releasing](releasing.md)`; invariant content kept, no duplicated
  step list. `docs/contributing.md` line updated to name both pages.
- [x] 17. `docs/discoverability.md`: crates.io CLI row removed from Published
  Channels; badge sentence no longer claims a live crates.io badge; new
  "Publishing with 2.0.0"-equivalent "### crates.io" entry under Channels Not
  Yet Published with activation steps and the badge-returns note. Release
  verification list (lines ~71–78) untouched.
- [x] 18. `README.md` crates.io badge line removed only (line 5 area); no new
  badges; Status 2.x wording is task 4; the tabular "unpublished" wording stays
  until Slice 2.
- [x] 19. `docs/launch-publicity.md`: crates.io section rewritten to the real
  dependency order including `oxdoc-tabular` with 2.0.0 / 0.2.0 / 2.0.0 and a
  pointer to `docs/releasing.md`; outreach checklist item names all three
  crates in dependency order.
- [x] 20. No slice-1 edit in `docs/roadmap.md` / `ROADMAP.md`
  (`git status --porcelain` clean for both); `.markdown-link-check.json` still
  carries the `^https://crates.io/crates/oxdoc-cli/?$` ignore.
- [x] 21. Full local gate green (details below).
- [x] 22. Protected-path diff check on the slice-1 commit (details below).

## Publish receipts

Pre-publish validation receipts (design §5 format; 2026-09-18 @ commit
`e3725d3` — slice-1 version-bump commit; the two `fail(then retried)` receipts
carry the caveat that their successful retry is the documented post-upstream
re-run, not a pre-publish skip).

### Receipt: pre-publish — oxdoc-core cargo package (full verify)
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo package -p oxdoc-core`
- result: pass
- evidence: `Packaged 20 files, 362.5KiB (64.4KiB compressed)`; `Verifying
  oxdoc-core v2.0.0` build `Finished dev profile`; note:
  `warning: ignoring test 'schema' as tests\schema.rs is not included in the
  published package` (expected — the include list intentionally excludes
  tests).

### Receipt: pre-publish — oxdoc-core cargo package --list
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo package -p oxdoc-core --list`
- result: pass
- evidence: 20 entries (README, benches, 3 examples, src/**); no tests,
  fixtures, or workspace-only files.

### Receipt: pre-publish — oxdoc-tabular cargo package --list
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo package -p oxdoc-tabular --list`
- result: pass
- evidence: 10 entries — README.md, `examples\tabular_gate.rs`,
  `examples\xlsx_to_parquet.rs`, src (lib, parquet, xlsx_schema); no tests,
  fixtures, or workspace-only files; both feature-gated examples ship.

### Receipt: pre-publish — oxdoc-cli cargo package --list
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo package -p oxdoc-cli --list`
- result: pass
- evidence: 7 entries — README.md, src (main, update); no tests, fixtures, or
  workspace-only files.

### Receipt: pre-publish — oxdoc-tabular cargo package --no-verify
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo package -p oxdoc-tabular --no-verify` (retried with
  `--offline`)
- result: fail(then retried — the successful retry is the documented
  post-upstream-publish re-run in `docs/releasing.md` step 6 / tasks 27–28,
  not a skipped validation)
- evidence: `error: failed to prepare local package for uploading` / `failed
  to select a version for the requirement oxdoc-core = "^2.0.0"` / `candidate
  versions found which didn't match: 1.2.0, 1.1.0, 1.0.0`; offline retry:
  `candidate versions found which didn't match: 1.2.0`.

### Receipt: pre-publish — oxdoc-cli cargo package --no-verify
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo package -p oxdoc-cli --no-verify`
- result: fail(then retried — same post-upstream re-run policy as tabular)
- evidence: `failed to select a version for the requirement oxdoc-core =
  "^2.0.0"` / `candidate versions found which didn't match: 1.2.0, 1.1.0,
  1.0.0` / `required by package oxdoc-cli v2.0.0`.

### Receipt: pre-publish — oxdoc-core cargo publish --dry-run (full)
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo publish -p oxdoc-core --dry-run` (no `--no-verify`)
- result: pass
- evidence: `Packaged 20 files, 362.5KiB`; `Verifying oxdoc-core v2.0.0`
  `Finished dev profile`; `Uploading oxdoc-core v2.0.0` then `warning:
  aborting upload due to dry run`.

### Receipt: pre-publish — oxdoc-tabular / oxdoc-cli cargo publish --dry-run
- date/commit: 2026-09-18 @ e3725d3
- command: `cargo publish -p oxdoc-tabular --dry-run` / `cargo publish -p
  oxdoc-cli --dry-run`
- result: skipped(N/A: the registry requirement `oxdoc-core 2.0.0` cannot
  resolve while core 2.0.0 is unpublished — the documented pre-publish caveat;
  the full dry-run for each is repeated immediately after its upstream
  publishes, per `docs/releasing.md` step 6 and tasks 27–28; a dry-run is
  never skipped once its upstream is live)
- evidence: same unsatisfied-requirement output as the two package receipts
  above.

## Files changed (slice 1)

Version bump + metadata: `crates/oxdoc-core/Cargo.toml`,
`crates/oxdoc-tabular/Cargo.toml`, `crates/oxdoc-cli/Cargo.toml`, `Cargo.lock`,
`fuzz/Cargo.lock`, `CHANGELOG.md`,
`tests/fixtures/snapshots/cli_info_json.json`,
`tests/fixtures/snapshots/cli_audit_json.json`,
`tests/fixtures/snapshots/all_sheets_manifest.json`,
`crates/oxdoc-core/tests/schema.rs`, `docs/library-api.md`,
`crates/oxdoc-tabular/README.md`, `docs/cli.md`, `docs/json-output.md`,
`docs/audit.md`, `docs/recipes.md`, `README.md`.

Checklist + pre-publish docs: `docs/releasing.md` (new),
`docs/_sidebar.md`, `docs/release-process.md`, `docs/contributing.md`,
`docs/discoverability.md`, `docs/launch-publicity.md`.

NEVER-TOUCH verification: `tests/fixtures/compatibility-matrix.json`,
`package.json`, `package-lock.json`, `python/pyproject.toml`,
`.github/workflows/**`, `crates/*/src/**`, `schemas/**`, `docs/schemas/**`,
`action.yml`, `install.sh` — all byte-identical to `main` (task 22 diff check).

## Test / gate evidence

- `cargo test --workspace --all-features --all-targets` — pass (exit 0).
- `cargo test --doc --workspace --all-features` — pass (2 docs).
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-features --all-targets -- -D warnings` — clean.
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines
  95 --summary-only` — exit 0, TOTAL lines 96.52% (10547 lines, 367 missed);
  no coverage delta (no logic touched).
- `scripts-test` components: `sh -n` on the four shell scripts — pass; python
  `py_compile` on scripts + python sources — pass;
  `release-benchmark-bundle.py --self-test` — pass; `tests/homebrew_formula.sh`
  — pass; **`tests/install.sh` fails with exit 1 in this Windows/MSYS
  environment (`curl` cannot open the test's `/tmp/...` `file://` tarball path)
  — verified pre-existing on a clean `main` worktree, untouched by this
  change** (CI on Linux runs the same test green).
- `scripts/check-compatibility-corpus.py` — "compatibility corpus validation
  passed (3 fixtures)".
- `scripts/compatibility-playground.py --check` — pass.
- `diff -ru schemas/v1 docs/schemas/v1` and `diff -ru schemas/v2
  docs/schemas/v2` — identical.
- `docs-check` — docsify serves; served HTML contains
  `oxdoc documentation` (verified on a free port; port 3000 is occupied by an
  unrelated local dev server on this machine).
- `docs-links` (all 37 files, `markdown-link-check@3` with
  `.markdown-link-check.json`, crates.io ignore still present) — zero dead
  links, exit 0.
- `build-release` — `cargo build --workspace --all-features --release` pass.

## Deviations from design

- `make` is not installed on this machine, so the `make ci` recipe was executed
  as its individual component commands (identical invocations from the
  Makefile); `docs-check` was verified on port 3300 because an unrelated local
  server occupies port 3000.
- `cargo metadata --no-deps` in this toolchain does not expose
  `include`/`publish` for these packages; the metadata table evidence combines
  `cargo metadata` output with direct manifest reads, and `cargo package
  --list` (task 10 receipts) proves the include lists functionally.
- **Task 10 as written is not achievable pre-publish** (spec scenario
  "tabular and cli pre-publish validation uses the documented workaround"):
  `cargo package -p oxdoc-tabular --no-verify` and `… -p oxdoc-cli
  --no-verify` fail at registry resolution (`failed to select a version for
  the requirement oxdoc-core = "^2.0.0"` — candidate versions 1.2.0, 1.1.0,
  1.0.0), even with `--no-verify` and `--offline`. Packaging for upload
  replaces the path dependency with its registry requirement and there is no
  cargo flag to skip that check. Task 10's checkbox stays **unchecked**; the
  `--list` file-list inspections (all three, which skip the upload-resolution
  step) did run and pass, and the deferred full validation for tabular/cli is
  documented in `docs/releasing.md` step 6 and re-enters the flow via the
  post-upstream dry-runs in tasks 27–28. This spec assumption gap is reported
  to the parent for the verify/archive phases.

## Remaining tasks

Blocked pre-publish (re-enters the flow after `oxdoc-core` 2.0.0 is live):

- [ ] 10. Deferred full `cargo package` verification for `oxdoc-tabular` and
  `oxdoc-cli` (`--no-verify`) — succeeds only once `oxdoc-core` 2.0.0 exists
  on the registry; the `--list` inspections for all three crates already
  passed (receipts above).

Human publish gate (unchecked, maintainer-executed):

- [ ] 23. HUMAN GATE — release PR review/merge + registry authorization.
- [ ] 24. HUMAN GATE — re-confirm triple out loud + clean tree + dry-runs passed.
- [ ] 25. Tag `v2.0.0` and push.
- [ ] 26. Publish `oxdoc-core` + registry check.
- [ ] 27. Publish `oxdoc-tabular` (dry-run re-run first) + registry check.
- [ ] 28. Publish `oxdoc-cli` + registry check. Stop between crates.
- [ ] 29. Clean-target `cargo install oxdoc-cli --version 2.0.0` receipt.
- [ ] 30. Record publish receipts.

Slice 2 (unchecked, post-publish PR):

- [ ] 31. Precondition: publish receipts exist and registry checks match.
- [ ] 32. `docs/discoverability.md` post-publish flip.
- [ ] 33. `README.md` badge restore + tabular "unpublished" drop.
- [ ] 34. Roadmap Phase 5 flip in `docs/roadmap.md` and `ROADMAP.md`.
- [ ] 35. Remove the crates.io link-check ignore once the crate page is live.
- [ ] 36. Verify `docs/installation.md` needs no edit; docs gates re-run.
- [ ] 37. Record slice-2 receipts.

## Workload / PR boundary

- Slice 1 authored lines (measured `git diff main --stat`, excluding
  `.gitignore` and openspec bookkeeping): 104 changed lines across 22 tracked
  files + 167 lines in the new `docs/releasing.md` = **271 lines**, within the
  400-line budget (task forecast ~220–300).
- One PR to `main` per the resolved chain strategy (`stacked-to-main`, Slice 1
  pre-publish). Slice 2 is branch-off-`main`-after-publish, gated on tasks
  23–30, not stacked ahead.

## Structured status

- Consumed native `gentle-ai.sdd-status` v2 for `release-crates-io-publish`:
  `applyState: ready`, `nextRecommended: apply`, `actionContext.mode:
  repo-local`, `allowedEditRoots: [workspace root]`, `blockedReasons: []`. No
  warnings. Apply executed inside the workspace root only.
