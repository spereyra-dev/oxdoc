# Apply Progress — release-crates-io-publish

Change: `release-crates-io-publish` · Slice 1 (tasks 1–22) · Branch
`release-crates-io-s1` · Strict TDD active (`openspec/config.yaml`).
Slice 2 (tasks 23–37) appended below · Branch `release-crates-io-s2`.

## Executive summary

Slice 1 ships the complete pre-publish state: approved version triple
(`oxdoc-core` 2.0.0 · `oxdoc-tabular` 0.2.0 · `oxdoc-cli` 2.0.0) across every
MUST-EDIT reference, both lockfiles committed for the `--locked` release build,
tabular publishable metadata, the `docs/releasing.md` ordered checklist with
the irreversibility warning and recovery policy, list registrations in both
`docs/_sidebar.md` and `README.md`, the pre-publish documentation traffic light
(badge removed, crates.io moved to "not yet published", launch-publicity
dependency order), and the pre-publish packaging receipts. Tasks 23–30 (human publish gate) and
Slice 2 (31–37) were completed after the maintainer authorization executed the
publish session on 2026-09-18; see the "Slice 2" section below.

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
- [x] 10. Package in dependency order — **partially blocked pre-publish**:
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
  **RESOLVED in slice 2**: once `oxdoc-core` 2.0.0 was live, both
  `cargo package … --no-verify` runs succeeded (receipts below; also the
  full `--dry-run` for both crates) — the task is now checked.
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

## Deviations from design (slice 1)

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
  cargo flag to skip that check. The task's checkbox stayed **unchecked**
  after slice 1; the `--list` file-list inspections (all three, which skip the
  upload-resolution step) did run and pass, and the deferred full validation
  for tabular/cli was documented in `docs/releasing.md` step 6 and re-entered
  the flow via the post-upstream dry-runs in tasks 27–28. **Resolved in
  slice 2** (receipts below); the checkbox is now checked.

## Deviations from design (slice 2)

- `.markdown-link-check.json`: the crates.io ignore was removed as planned,
  but `markdown-link-check`'s default request headers get 403/404 from
  crates.io's bot filter even for live pages (bare `curl` → 403; a UA-only
  request → 404; the same filter 404s famous crates like `serde`). With full
  browser headers the live page returns HTTP 200. Instead of re-adding an
  ignore for a live URL, the config gained an `httpHeaders` block scoped to
  `https://crates.io/` with browser-like `User-Agent`/`Accept`/
  `Accept-Language` headers, so `make docs-links` now genuinely validates the
  live crate page — the design's intent ("docs-links validates the live URL")
  preserved, green at 37 files / zero dead links.
- Pre-existing local working-tree state unrelated to this slice, left
  untouched and excluded from the diff measurement: `.gitignore` (+`.atl/`
  local-runtime entry), untracked `.gga` and `.pi/` local directories.
- `make` is not installed (same as slice 1); the docs-gate recipes were run as
  their individual Makefile component commands.
- The GitHub REST API was rate-limited from this IP (HTTP 403) at slice-2
  time, so the `v2.0.0` GitHub Release was verified via the web page
  (`releases/tag/v2.0.0` → HTTP 200, `expanded_assets` → HTTP 200 with
  assets) and the pushed tag (`git ls-remote --tags origin v2.0.0` →
  `79bef11`).

## Slice 2 — human publish gate + post-publish flip (tasks 23–37)

Branch `release-crates-io-s2` off `main` @ `79bef11` (the merged release
commit, PR #241). The maintainer authorization executed the publish session on
2026-09-18; slice 2 independently verified every registry claim and re-ran the
documented post-upstream validations first-hand. Strict TDD: no runtime logic
exists in this change, so no RED/GREEN cycle applies (same rationale as
slice 1 — docs/config/truth edits only).

### Receipt: human-gate — release PR merge + publish authorization (tasks 23–24)
- date/commit: 2026-09-18 @ 79bef11
- command: maintainer review of the release PR; merge to `main` (#241);
  out-loud re-confirmation of 2.0.0 / 0.2.0 / 2.0.0; clean-tree check; local
  `cargo publish` authorized
- result: pass
- evidence: `main` HEAD = 79bef11 `feat(release): prepare 2.0.0 crates.io
  release with pre-publish gates (#241)`; slice-2 re-run of `cargo pkgid` × 3
  → 2.0.0 / 0.2.0 / 2.0.0 and `grep -n '^## 2.0.0' CHANGELOG.md` → line 7.

### Receipt: tag — v2.0.0 create + push (task 25)
- date/commit: 2026-09-18 @ 79bef11
- command: `git tag -a v2.0.0 -m "oxdoc 2.0.0"` then `git push origin v2.0.0`
- result: pass
- evidence: slice-2 first-hand `git ls-remote --tags origin v2.0.0` →
  `79bef119a2db956f45b16a3b5bad49a5754c4fc9 refs/tags/v2.0.0` (tag on the
  merged release commit).

### Receipt: publish — oxdoc-core cargo publish (task 26)
- date/commit: 2026-09-18 @ 79bef11 (maintainer local publish session)
- command: `cargo publish -p oxdoc-core`
- result: pass
- evidence: `Uploaded oxdoc-core v2.0.0 to registry crates-io` / `Published
  oxdoc-core v2.0.0`; slice-2 registry check: crates.io API `max_version:
  2.0.0` and `cargo search oxdoc --limit 5` hit.

### Receipt: publish — oxdoc-tabular dry-run re-run + real publish (task 27)
- date/commit: 2026-09-18 @ 79bef11 (maintainer local publish session)
- command: `cargo publish -p oxdoc-tabular --dry-run` (after core went live)
  then `cargo publish -p oxdoc-tabular`
- result: pass (the ~30s sparse-index wait covered the documented retry
  policy; never skipped)
- evidence: `Uploaded oxdoc-tabular v0.2.0` / `Published oxdoc-tabular
  v0.2.0`; slice-2 first-hand dry-run re-run: `Packaged 10 files, 106.9KiB
  (23.9KiB compressed)`, `Uploading oxdoc-tabular v0.2.0`, `warning:
  aborting upload due to dry run`, exit 0; registry check: API `max_version:
  0.2.0`.

### Receipt: publish — oxdoc-cli dry-run + real publish (task 28)
- date/commit: 2026-09-18 @ 79bef11 (maintainer local publish session)
- command: `cargo publish -p oxdoc-cli --dry-run` then `cargo publish -p
  oxdoc-cli` (strict core → tabular → cli order, stop between crates)
- result: pass
- evidence: `Uploaded oxdoc-cli v2.0.0` / `Published oxdoc-cli v2.0.0`;
  slice-2 first-hand dry-run re-run: `Packaged 7 files, 109.2KiB (23.8KiB
  compressed)`, `aborting upload due to dry run`, exit 0; registry check: API
  `max_version: 2.0.0`.

### Receipt: end-to-end — oxdoc-cli clean-target install (task 29)
- date/commit: 2026-09-19 @ release-crates-io-s2 (first-hand, slice 2)
- command: `CARGO_TARGET_DIR=$(mktemp -d) cargo install oxdoc-cli --version
  2.0.0`
- result: pass
- evidence: `Updating crates.io index`, `Compiling oxdoc-cli v2.0.0`,
  `Finished release profile ... in 32.76s`, `Installed package \`oxdoc-cli
  v2.0.0\` (executable \`oxdoc.exe\`)`; target dir deleted after the run.

### Receipt: registry — independent crates.io verification (task 31)
- date/commit: 2026-09-19 @ release-crates-io-s2
- command: `curl https://crates.io/api/v1/crates/<crate>` × 3 + `cargo search
  oxdoc --limit 5`
- result: pass
- evidence: API `max_version` — oxdoc-core 2.0.0, oxdoc-tabular 0.2.0,
  oxdoc-cli 2.0.0; `cargo search`: `oxdoc-cli = "2.0.0"`, `oxdoc-core =
  "2.0.0"`, `oxdoc-tabular = "0.2.0"`.

### Receipt: deferred-validation — tabular/cli cargo package --no-verify (task 10)
- date/commit: 2026-09-19 @ release-crates-io-s2
- command: `cargo package -p oxdoc-tabular --no-verify` · `cargo package -p
  oxdoc-cli --no-verify`
- result: pass (both; the slice-1 `fail(then retried)` deferral is now closed
  — resolution succeeds against the live registry deps)
- evidence: tabular `Packaged 10 files, 106.9KiB (23.9KiB compressed)`; cli
  `Packaged 7 files, 109.2KiB (23.8KiB compressed)` (with the expected
  `ignoring test 'cli'` note — tests are excluded by the include list).

### Receipt: docs — badge/crate-page HTTP verification (tasks 33/35)
- date/commit: 2026-09-19 @ release-crates-io-s2
- command: `curl -A <browser UA> -H "Accept: text/html,…" -H
  "Accept-Language: en-US,…" https://crates.io/crates/oxdoc-cli` ·
  `curl https://img.shields.io/crates/v/oxdoc-cli.svg`
- result: pass
- evidence: crate page HTTP 200 (with browser headers; crates.io's bot filter
  returns 403/404 to header-less/UA-only automated requests — verified against
  the `serde` control page too); badge SVG HTTP 200 with the restored badge
  backed by crates.io API `max_version 2.0.0`.

### Receipt: docs — docs-links with the crates.io ignore removed (task 35)
- date/commit: 2026-09-19 @ release-crates-io-s2
- command: `find README.md docs -name '*.md' -print0 | xargs -0 npx --yes
  markdown-link-check@3 --config .markdown-link-check.json` (the Makefile
  `docs-links` recipe; make is not installed)
- result: pass
- evidence: 37 files checked, zero dead links, exit 0; the
  `https://crates.io/crates/oxdoc-cli` URL is now actually checked (not
  ignored) via the new `httpHeaders` block (see deviations).

### Receipt: docs — GitHub Release/artifacts independent of crates (task 37)
- date/commit: 2026-09-19 @ release-crates-io-s2
- command: `git ls-remote --tags origin v2.0.0` · `curl -L
  https://github.com/spereyra-dev/oxdoc/releases/tag/v2.0.0` · `curl -L
  https://github.com/spereyra-dev/oxdoc/releases/expanded_assets/v2.0.0`
- result: pass
- evidence: tag at `79bef11`; release page HTTP 200; expanded-assets page
  HTTP 200 (assets present). GitHub REST API was rate-limited (HTTP 403) from
  this IP; web pages used instead.

### Slice 2 files changed

`README.md` (crates.io badge restored at the pre-slice-1 position +
"unpublished" dropped from the tabular description), `docs/discoverability.md`
(crates.io CLI row back in Published Channels with the published versions and
install command, badge paragraph restored, "### crates.io" entry removed from
Channels Not Yet Published), `docs/roadmap.md` + `ROADMAP.md` (Phase 5 crates.io
bullet → "Implemented in the 2.0.0 crates.io release", identical wording in
both files), `.markdown-link-check.json` (crates.io ignore removed; `httpHeaders`
block added).

`docs/installation.md` verified-only: its crates.io sentence ("installed from
… crates.io") is now true; no edit needed, byte-identical to `main`.

### Slice 2 gate evidence

- `cargo fmt --all -- --check` — clean; `cargo clippy --workspace
  --all-features --all-targets -- -D warnings` — clean; `cargo test --workspace
  --all-features --all-targets` — all suites ok (0 failures).
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines
  95 --summary-only` — exit 0, TOTAL lines 96.53% (10547 lines, 366 missed) —
  ≥ 95 gate met, no coverage delta (no logic touched in either slice).
- `docs-check` — docsify serves on :3300, served HTML contains
  `oxdoc documentation`.
- `docs-schemas-check` — `diff -ru schemas/v1 docs/schemas/v1` and `diff -ru
  schemas/v2 docs/schemas/v2` — identical (mirror check).
- `docs-playground-check` (`scripts/compatibility-playground.py --check`) —
  pass; `scripts/check-compatibility-corpus.py` — "compatibility corpus
  validation passed (3 fixtures)".
- `docs-links` — green with the crates.io ignore removed (receipt above).
- Protected paths: slice-2 diff touches only README.md, ROADMAP.md,
  docs/roadmap.md, docs/discoverability.md, .markdown-link-check.json (+
  openspec bookkeeping) — nothing under `crates/*/src/**`, `schemas/**`,
  `docs/schemas/**`, `python/**`, `.github/workflows/**`, `action.yml`,
  `install.sh`; no `CARGO_REGISTRY_TOKEN`, no new workflow, no Makefile change.

### Slice 2 structured status

- Consumed native `gentle-ai.sdd-status` v2 for `release-crates-io-publish`:
  `applyState: ready`, `nextRecommended: apply`, `actionContext.mode:
  repo-local`, `allowedEditRoots: [workspace root]`, `blockedReasons: []`. All
  edits inside the workspace root. Commit-time forecast: tasks 23–37 all
  checked; 37/37 tasks complete (taskProgress will read 37/37).

## Remaining tasks

None. All 37 tasks are complete (task 10's deferred full packaging validation
resolved in slice 2 once `oxdoc-core` 2.0.0 was live). The human publish gate
tasks 23–30 were executed by the maintainer authorization on 2026-09-18 and
recorded below.

## Workload / PR boundary

- Slice 1 authored lines (measured `git diff main --stat`, excluding
  `.gitignore` and openspec bookkeeping): 104 changed lines across 22 tracked
  files + 167 lines in the new `docs/releasing.md` = **271 lines**, within the
  400-line budget (task forecast ~220–300).
- Slice 2 authored lines (measured `git diff main --stat`, excluding
  `.gitignore` and openspec bookkeeping): **19 insertions / 27 deletions = 46
  changed lines** across 5 files (`README.md`, `ROADMAP.md`, `docs/roadmap.md`,
  `docs/discoverability.md`, `.markdown-link-check.json`), within the 400-line
  budget (task forecast ~30–45 + link-check config).
- One PR to `main` per the resolved chain strategy (`stacked-to-main`, Slice 1
  pre-publish). Slice 2 is branch-off-`main`-after-publish, gated on tasks
  23–30, not stacked ahead.

## Structured status

- Consumed native `gentle-ai.sdd-status` v2 for `release-crates-io-publish`:
  `applyState: ready`, `nextRecommended: apply`, `actionContext.mode:
  repo-local`, `allowedEditRoots: [workspace root]`, `blockedReasons: []`. No
  warnings. Apply executed inside the workspace root only.
