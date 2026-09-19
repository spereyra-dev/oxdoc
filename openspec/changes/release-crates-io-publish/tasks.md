# Tasks — release-crates-io-publish (issue #173)

Change: `release-crates-io-publish` · Artifact store: openspec · Tasks phase.
Authoritative inputs: `proposal.md`, `specs/crates-io-release/spec.md` (8 requirements,
17 scenarios), `design.md` (authoritative; parent-resolution ledger §0 supersedes the
proposal body), `openspec/config.yaml` (`strict_tdd: true`, review budget 400).

Approved version triple: `oxdoc-core` 2.0.0 · `oxdoc-tabular` 0.2.0 · `oxdoc-cli` 2.0.0.

Strict TDD is active in `openspec/config.yaml`, so the version bump is sequenced
RED → GREEN → TRIANGULATE → REFACTOR with `cargo test --workspace`. This change adds no
runtime logic, so RED/GREEN operate on the existing version-asserting tests
(`crates/oxdoc-cli/tests/cli.rs::prints_info_as_json_and_text`, which compares the whole
JSON value to `tests/fixtures/snapshots/cli_info_json.json`): the fixture moves first and
must fail against the un-bumped manifests, then the manifests move to GREEN.
REFACTOR is the consistency/lint sweep, since there is no logic to restructure.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~250–345 shipped (Slice 1 ~220–300: version refs 50–70, tabular metadata 12–16, `docs/releasing.md` + pointers 120–160, pre-publish docs 40–55; Slice 2 ~30–45) |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | Slice 1 PR (pre-publish, one PR) → human publish gate → Slice 2 PR (post-publish docs flip + receipts) |
| Delivery strategy | auto-chain |
| Chain strategy | stacked-to-main |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Medium

Notes:
- Delivery follows the session preflight (`auto-chain`, artifact store openspec, 400-line
  budget). Chain strategy is resolved (`stacked-to-main`): Slice 1 lands on `main` first;
  Slice 2 is **not** chained ahead of Slice 1 because it is gated on a real, irreversible
  registry publish, so it is branch-off-`main`-after-publish, not a stacked dependency.
- Slice 2 is a maintainer-gated post-publish PR, not a speculative parallel branch. If the
  human publish gate is not authorized, the change stops at Slice 1 (accurate pre-publish
  docs + checklist) and Slice 2 is **blocked, not skipped** (spec req 6, scenario "no
  publish happens without the human gate").
- Counting only shipped code/docs lines, ~250–345 fits the 400 budget as one PR each. Adding
  the SDD artifacts (`spec.md` with 17 scenarios + this file + receipts, ~140–200 realized)
  pushes the change total near/over 400, which is exactly why the slice boundary is
  pre-declared rather than left to mid-apply discovery.
- The **human publish authorization** (tasks 23–30) is a preserved consent/irreversibility
  gate and is independent of the delivery-shape decision above. `exception-ok` is not
  inferred and no `size:exception` is requested.

## Cross-guards (apply at every task; violations are stop-the-line)

- **Never touch** `tests/fixtures/compatibility-matrix.json` `producer_version` — that value
  is the external OOXML producer's version, not oxdoc's (design §1.2).
- **Never touch** `package.json` / `package-lock.json` — `package.json` (`oxdoc-docs`) has no
  version field and the lock's 1.2.0 hits are unrelated npm deps.
- **Never touch** `python/pyproject.toml` (stays 0.1.0) or `.github/workflows/publish-python.yml`.
- **No CI token**: no `CARGO_REGISTRY_TOKEN`, no publish workflow, no new Makefile target or
  script, no `.github/workflows/**` edit (spec req 7, scenario "no CI credential or workflow change").
- **No protected-path edits**: `crates/*/src/**`, `schemas/**`, `docs/schemas/**`, `python/**`,
  `action.yml`, `install.sh`.
- **No new badges**: only the existing crates.io badge line's pre-publish removal / post-publish
  restoration (parent resolution 3).
- **Publish order is fixed**: `oxdoc-core` → `oxdoc-tabular` → `oxdoc-cli`, stop between crates.
- **Dry-runs are never skipped**: index-lag failures are handled by wait-and-re-run only; a
  skipped dry-run is a failed release (spec req 3, scenario "dry-run is never skipped despite index lag").

## Slice 1 — pre-publish (one PR to `main`)

### A. Version bump, strict TDD (spec req 1; CHANGELOG req; design §1)

- [x] 1. RED — update `tests/fixtures/snapshots/cli_info_json.json` line 2 to
  `"oxdoc_version": "2.0.0"` and the representative literal in
  `crates/oxdoc-core/tests/schema.rs` (~line 122) to `"2.0.0"`; run
  `cargo test -p oxdoc-cli --test cli prints_info_as_json_and_text` against the still-1.2.0
  manifests and record the failure output (expected RED — proves the fixture/manifest coupling).
  Spec: "Approved-version consistency" → scenario "the test suite survives the version bump".
  (Design §1.1 marks both files REQUIRED/consistency; `cli_audit_json.json` and
  `all_sheets_manifest.json` are schema-validated and may be bumped in the same edit, 2 lines,
  unless the review count is tight — design §1.2.)
- [x] 2. GREEN — apply the manifest edits: `crates/oxdoc-core/Cargo.toml` line 3 → `"2.0.0"`;
  `crates/oxdoc-tabular/Cargo.toml` line 3 → `"0.2.0"` and line 22 `oxdoc-core` requirement →
  `"2.0.0"`; `crates/oxdoc-cli/Cargo.toml` line 3 → `"2.0.0"`, line 23 `oxdoc-core` → `"2.0.0"`,
  line 24 `oxdoc-tabular` → `"0.2.0"` (exact minor; `"0.2"`/`"0.1.0"` are wrong). Then regenerate
  and commit **both lockfiles**: `Cargo.lock` via `cargo check --workspace --all-features`, and
  `fuzz/Cargo.lock` (core path-dep entry → 2.0.0) via `cargo metadata --manifest-path fuzz/Cargo.toml`
  — no `fuzz/Cargo.toml` edit, it declares the dep by path only. Run
  `cargo test --workspace --all-features --all-targets` → GREEN.
  Spec: scenarios "cargo pkgid confirms the triple after the version-bump commit" and
  "dependency requirements name the exact tabular minor".
  Guard: `release.yml` builds with `--locked` (lines 92, 140, 142), so an uncommitted lockfile
  only fails after the tag — both locks must be in this commit (design §9).
- [x] 3. TRIANGULATE — prove the triple independently of the test suite:
  `cargo pkgid -p oxdoc-core` → 2.0.0 · `cargo pkgid -p oxdoc-tabular` → 0.2.0 ·
  `cargo pkgid -p oxdoc-cli` → 2.0.0 · `grep -n '^## 2.0.0' CHANGELOG.md` matches ·
  `cargo test --workspace --all-features --all-targets` and `cargo test --doc --workspace --all-features`.
  Spec: scenario "cargo pkgid confirms the triple after the version-bump commit".
- [x] 4. REFACTOR — sweep the remaining in-repo version references so nothing is left stale:
  `docs/library-api.md` line 10 snippet → `"0.2.0"`; `crates/oxdoc-tabular/README.md` install
  snippet → `"0.2.0"`; the four example payloads `"oxdoc_version": "1.2.0"` → `"2.0.0"` in
  `docs/cli.md` (~line 524), `docs/json-output.md` (~line 204), `docs/audit.md` (~line 37),
  `docs/recipes.md` (~line 342); `README.md` line 16 `Status` "stable 1.x CLI and Rust API
  contract" → 2.x wording. Do **not** edit `docs/cli.md` line 73 (field name, not a value) and
  do not touch the version-less `cargo install oxdoc-cli` lines. Then
  `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-features --all-targets -- -D warnings`.
- [x] 5. CHANGELOG — convert `CHANGELOG.md` line 7 `## Unreleased` → `## 2.0.0 - <release date>`
  with a one-line note naming all three crate versions (2.0.0 / 0.2.0 / 2.0.0), preserving
  **all three** documented breaks under the dated heading: rows-JSONL v2 (lines ~45–56),
  `XlsxCell.formula` source break with its migration note (~58–64), structured-JSON v2 (~66–84);
  sections 1.2.0 / 1.1.0 / 1.0.0 unchanged and no leftover `## Unreleased` for this content.
  Spec: "CHANGELOG dates the release" → scenarios "the release section carries all documented
  breaks" and "no Unreleased section remains for this release" (parent resolution 2).

### B. Publishable crate metadata (spec req 2; design §2)

- [x] 6. `crates/oxdoc-tabular/Cargo.toml` metadata diff: add `readme = "README.md"`, add
  `include = ["Cargo.toml", "README.md", "src/**", "examples/**"]`, add
  `[package.metadata.docs.rs]` with `features = ["parquet"]` (never `all-features = true`), keep
  `publish = true` explicit and truthful, keep the `version = "0.2.0"` / `oxdoc-core = "2.0.0"`
  lines from task 2.
  Spec: "Publishable crate metadata" → scenario "tabular package page renders its README and docs".
- [x] 7. `crates/oxdoc-tabular/README.md` — replace the escaping relative link
  `[`docs/spikes/xlsx-arrow-parquet.md`](../../docs/spikes/xlsx-arrow-parquet.md)` with the
  absolute `https://github.com/spereyra-dev/oxdoc/blob/main/docs/spikes/xlsx-arrow-parquet.md`.
  Spec: scenario "README link audit finds no escaping relative links".
- [x] 8. README link audit across all three crate READMEs (`crates/oxdoc-core/README.md`,
  `crates/oxdoc-tabular/README.md`, `crates/oxdoc-cli/README.md`): every link must resolve as it
  would on a crates.io package page, and no relative link may cross the package boundary. Record
  the audit result as a receipt.
- [x] 9. Per-crate metadata verification table (apply phase, spec req 2 → scenario "per-crate
  metadata verification table passes"), checked **per crate**, not per field presence:
  `description` (crate-specific, non-empty) · `license` = `MIT` (`workspace = true`) ·
  `repository`/`homepage` = `https://github.com/spereyra-dev/oxdoc` · `readme = "README.md"`
  present for **all three** with the file existing · `include` present for all three · `keywords`
  = `docx, xlsx, ooxml, parser, cli` (exactly 5, lowercase) · `categories` =
  `command-line-utilities, parser-implementations` · `rust-version` = `1.88` · `publish` not
  `false` · docs.rs metadata for `oxdoc-tabular` **only**. Evidence:
  `cargo metadata --no-deps --format-version 1` (or reading each manifest + `[workspace.package]`).

### C. Packaging validation receipts, pre-publish (spec req 3; design §5)

- [ ] 10. Package in dependency order: `cargo package -p oxdoc-core` (full verify) ·
  `cargo package -p oxdoc-tabular --no-verify` · `cargo package -p oxdoc-cli --no-verify` ·
  `cargo package -p oxdoc-core --list` && `… -p oxdoc-tabular --list` && `… -p oxdoc-cli --list`.
  Inspect each list: no tests, fixtures, or workspace-only files; tabular must include its
  README and both `examples/**` files (`xlsx_to_parquet.rs`, `tabular_gate.rs`).
  Spec: scenario "tabular and cli pre-publish validation uses the documented workaround".
- [x] 11. `cargo publish -p oxdoc-core --dry-run` (full verification, no `--no-verify`) and
  record it. `oxdoc-tabular`/`oxdoc-cli` dry-runs **cannot** fully verify pre-publish (their
  registry requirement is unsatisfied while core 2.0.0 is unpublished) — that is the documented
  caveat, not a reason to skip any later dry-run.
  Spec: scenario "core packages and dry-runs fully before anything publishes".
- [x] 12. Record every packaging/dry-run receipt in
  `openspec/changes/release-crates-io-publish/apply-progress.md` under a "Publish receipts"
  section, using the design's `### Receipt: <step-id> — <crate> <command-kind>` format
  (`date/commit`, `command`, `result: pass | fail(then retried) | skipped(N/A: <reason>)`,
  1–3 trimmed evidence lines). Never record a receipt for a command that was not run.

### D. Release checklist `docs/releasing.md` (spec req 4; design §3)

- [x] 13. Author the new page `docs/releasing.md` containing the exact ordered flow with a
  command → expected evidence → stop-on-failure gate for each step: (0) preconditions — clean
  `main`, CI green, `git status --porcelain` empty, local crates.io token loaded, maintainer
  authorization (**HUMAN**) → (1) version-bump commit → (2) approved-version gate `cargo pkgid`
  per crate + `grep -n '^## 2.0.0' CHANGELOG.md` (**stop on mismatch**) → (3) metadata gate
  (**stop on gap**) → (4) `make ci` (**stop on red**) → (5) review + merge (**HUMAN**) →
  (6) package + dry-run in dependency order (**stop on failure**) → (7) tag + push `v2.0.0`
  → (8) confirm + publish core (**HUMAN**) → (9) publish tabular then CLI (**stop between
  crates**) → (10) post-publish reconciliation.
  Spec: "Documented release checklist" → scenario "a maintainer can execute a release from the
  checklist alone".
- [x] 14. In `docs/releasing.md`, include the mandatory content that makes the checklist
  actionable and honest: the **irreversibility warning** (published versions can never be
  replaced or deleted, only yanked); the **resolution caveat** (`cargo package … --no-verify`
  + `--list` for tabular/CLI pre-publish, full `cargo publish --dry-run` repeated immediately
  after each upstream goes live) with the sparse-index lag retry instruction "wait and re-run —
  never skip a dry-run"; the **clean-tree precondition** before each real publish; the
  **tag-before-publish ordering consequence** (the GitHub Release advertising
  `cargo install oxdoc-cli` can be public before crates.io resolves — parent resolution 4); the
  **recovery** subsection (fix forward with a patch version 2.0.1 / 0.2.1 that re-cascades the
  requirement bumps; `cargo yank --version <v> -p <crate>` as documented last resort; do not
  re-publish the same version); and the **"CI publishing (future prerequisite)"** subsection
  (no OIDC flow for cargo → future `CARGO_REGISTRY_TOKEN` secret + `workflow_dispatch`
  approved-version input asserted against all three manifests modeled on `publish-python.yml`
  + dependency-ordered jobs + token rotation, explicitly deferred).
  Spec: scenario "the irreversibility warning and recovery are present"; req 4.
- [x] 15. Register the page in **both** lists (parent resolution 1):
  `docs/_sidebar.md` → **Project** section as `- [Releasing](releasing.md)` next to
  `[Release Process](release-process.md)`; and `README.md` → "Key documentation pages" list
  (after `docs/github-action.md` / near the other `docs/…` entries) as
  `- [Releasing](docs/releasing.md)`.
- [x] 16. `docs/release-process.md` §Crates.io Publishing (line ~70): add one pointer line to
  `[Releasing](releasing.md)` for the step-by-step, keeping the existing invariant content
  (dependency order rationale, keep-list, `--no-verify` paragraph) — no duplicated step list,
  single source of truth. Optional: extend `docs/contributing.md` line 93 to name both pages.

### E. Pre-publish documentation truth (spec req 5; design §4)

- [x] 17. `docs/discoverability.md` — move the `crates.io CLI` row (line 12) out of **Published
  Channels** into a new "Publishing with 2.0.0" entry under **Channels Not Yet Published**
  (line 22), naming the activation steps (approved version, ordered dry-runs, receipts) and the
  note that the README badge returns with the release; adjust the badge sentence at lines 16–17
  so it no longer claims a live crates.io badge. Do **not** edit the release verification list
  lines 71–78.
  Spec: "Documentation truthfulness" → scenario "pre-publish docs do not claim a live channel".
- [x] 18. `README.md` — remove the `crates.io` badge line (line 5) only; no other badge added
  and no other README badge line touched (the README badge policy: add a badge only after its
  destination is public). The `Status` 2.x wording is task 4; the `oxdoc-tabular` "unpublished"
  wording stays until Slice 2.
- [x] 19. `docs/launch-publicity.md` — rewrite the crates.io/crate-summary section (lines ~41–63)
  to the real dependency order including `oxdoc-tabular`, with the 2.0.0 / 0.2.0 / 2.0.0 versions
  and a pointer to `docs/releasing.md` for the checklist; make the outreach checklist item
  (line ~148) name all three crates in dependency order.
- [x] 20. Confirm no slice-1 edit landed in `docs/roadmap.md` or `ROADMAP.md` ("publish to
  crates.io" as pending work is true pre-publish) and that `.markdown-link-check.json` still
  carries the `^https://crates.io/crates/oxdoc-cli/?$` ignore (the page still 404s).
  Spec: scenario "the ignore removal is gated on the live page".

### F. Slice 1 local gates

- [x] 21. Run the full gate from the repo root: `make ci` (fmt `--check`, check
  `--workspace --all-features --all-targets`, clippy `-D warnings`, test, doctest, coverage ≥ 95
  with **no coverage delta**, `scripts-test`, `docs-check`, `docs-links` with the crates.io
  ignore still present, `docs-schemas-check`, `docs-playground-check`).
  Spec: "No behavior or CI change" → scenario "local gates stay green after the bump".
- [x] 22. Protected-path diff check on the slice-1 commit:
  `git diff --name-only main...HEAD` must contain nothing under `crates/*/src/**`, `schemas/**`,
  `docs/schemas/**`, `python/**`, `.github/workflows/**`, `action.yml`, `install.sh`;
  `git diff --exit-code main...HEAD -- python/pyproject.toml` must be clean; no
  `CARGO_REGISTRY_TOKEN`, no new workflow, no new Makefile target/script; and
  `tests/fixtures/compatibility-matrix.json`, `package.json`, and `package-lock.json` unchanged.
  Spec: scenario "behavior-sensitive paths are untouched".

## Human publish gate (maintainer-executed; not automatable)

- [ ] 23. HUMAN GATE — maintainer publishes the release PR review, merges Slice 1 to `main`,
  and authorizes the registry operation. Slice 2 must not start before this.
  Spec: scenario "no publish happens without the human gate".
- [ ] 24. HUMAN GATE — re-confirm the triple out loud against the merged commit
  (2.0.0 / 0.2.0 / 2.0.0 via `cargo pkgid` × 3 and `grep -n '^## 2.0.0' CHANGELOG.md`), confirm
  a clean tree (`git status --porcelain` empty), and confirm all pre-publish dry-runs passed.
- [ ] 25. Tag on the merged release commit and push: `git tag -a v2.0.0 -m "oxdoc 2.0.0"` then
  `git push origin v2.0.0` (triggers `release.yml`; the GitHub Release may complete before
  crates.io resolves — expected, documented).
- [ ] 26. Publish `oxdoc-core`: `cargo publish -p oxdoc-core`, then verify with
  `cargo search oxdoc-core --limit 1` → 2.0.0. Record the receipt.
- [ ] 27. Publish `oxdoc-tabular`: re-run `cargo publish -p oxdoc-tabular --dry-run` now that
  core 2.0.0 resolves (on index lag: wait and re-run — never skip), then
  `cargo publish -p oxdoc-tabular`, then verify `cargo search oxdoc-tabular --limit 1` → 0.2.0.
  Record the receipt, including any `fail(then retried)` output.
- [ ] 28. Publish `oxdoc-cli`: `cargo publish -p oxdoc-cli --dry-run` then
  `cargo publish -p oxdoc-cli`, then verify `cargo search oxdoc-cli --limit 1` → 2.0.0. Record
  the receipt. **Stop between crates** — publish strictly core → tabular → cli.
  Spec: scenario "publish order and verification receipts".
- [ ] 29. End-to-end receipt: `CARGO_TARGET_DIR=$(mktemp -d) cargo install oxdoc-cli --version 2.0.0`
  succeeds from the registry. Record it.
- [ ] 30. Record all publish receipts (tasks 26–29) in the "Publish receipts" section of
  `openspec/changes/release-crates-io-publish/apply-progress.md` using the `### Receipt:` format.
  Failure handling: never re-publish the same version; resume from the failed crate; fix forward
  with a patch; `cargo yank --version <v> -p <crate>` as last resort.

## Slice 2 — post-publish flip + receipts (separate PR to `main`, after the publish)

- [ ] 31. Precondition check: the three publish receipts (tasks 26–28) exist and each registry
  check shows 2.0.0 / 0.2.0 / 2.0.0, plus the clean-target install receipt (task 29). If a receipt
  is missing, stop — do not flip docs to claim a channel that is not verified published.
- [ ] 32. `docs/discoverability.md` — return crates.io CLI to **Published Channels** with the
  published versions (2.0.0 / 0.2.0 / 2.0.0) and the `cargo install oxdoc-cli` command, restore
  the badge sentence, and drop the "Publishing with 2.0.0" entry from **Channels Not Yet Published**.
  Spec: "Documentation truthfulness" → scenario "post-publish flip restores the badge and removes
  the ignore".
- [ ] 33. `README.md` — restore the crates.io badge line (line 5, same line as before, verified
  with an HTTP 200 against `https://crates.io/crates/oxdoc-cli`) and drop "unpublished" from the
  `oxdoc-tabular` description (~line 356). No other README edits, no new badges.
- [ ] 34. `docs/roadmap.md` and `ROADMAP.md` — flip the Phase 5 bullet to implemented using the
  same wording in both files (reference: `ROADMAP.md` lines ~61–62: "Publish `oxdoc-core`,
  `oxdoc-tabular`, and `oxdoc-cli` to crates.io in dependency order").
- [ ] 35. `.markdown-link-check.json` — remove the `^https://crates.io/crates/oxdoc-cli/?$`
  ignore **only** now that the crate page exists (HTTP 200 confirmed in task 33); `make docs-links`
  must stay green with the ignore removed. If the page does not resolve, leave the ignore in place
  and report the blocker.
  Spec: scenario "the ignore removal is gated on the live page".
- [ ] 36. Verify `docs/installation.md` needs no edit (its crates.io sentence is now true) and
  run the docs gates: `make docs-links`, `make docs-check`, `docs-schemas-check`,
  `docs-playground-check`.
- [ ] 37. Record the slice-2 receipts, including the badge HTTP verification and the
  `docs-links` run with the ignore removed, in the "Publish receipts" section of
  `openspec/changes/release-crates-io-publish/apply-progress.md`; confirm the `v2.0.0` tag exists
  on the merged release commit and that the release workflow produced its GitHub Release/artifacts
  independently of the crates.
