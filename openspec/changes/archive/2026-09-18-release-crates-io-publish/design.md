# Design — release-crates-io-publish

Change: `release-crates-io-publish` · Artifact store: openspec · Design phase 2026-09-xx.
Authoritative inputs: `proposal.md`, `specs/crates-io-release/spec.md` (8 requirements,
17 scenarios), `exploration.md`, parent resolutions 1–5. The spec is the behavior
contract; this design answers *where, how, and in what order* the work happens.

## 0. Parent-resolution ledger (supersede the proposal body where they conflict)

| # | Parent resolution | Effect on proposal text |
| --- | --- | --- |
| 1 | Checklist page is **`docs/releasing.md`** (new page; sidebar + README lists) | Supersedes proposal §3 (extend `docs/release-process.md`) and its Alternative 2 rejection. The spec already fixed `docs/releasing.md`; the proposal body is the outlier. |
| 2 | CHANGELOG keeps **all documented breaks** under the dated 2.0.0 heading (rows-JSONL v2 + structured-JSON v2 + `XlsxCell` source break) | Closes proposal question 6; spec requirement "CHANGELOG dates the release" already requires all three. |
| 3 | README: only the existing crates.io badge lifecycle (pre-publish removal, post-publish restoration); no new badges | Confirms proposal question 1 option A. The README `Status` line refresh (1.x → 2.x wording) is a text truthfulness edit, not a badge, and stays in slice 1. |
| 4 | Tag+push before local cargo publish; consequence (GitHub Release live before `cargo install` resolves) documented in releasing.md | Confirms proposal question 4; spec requirement "Human-gated local publish" already fixes this order. |
| 5 | Real publish is local (maintainer credentials), human-gated, core→tabular→cli, no CI token | Confirms spec; no CI secret/workflow/automation of any kind. |

All proposal question-round items are now closed by resolutions 1–5 plus the spec;
none re-open here.

## 1. Version-reference inventory (file-by-file, verified on `main`)

Version triple: `oxdoc-core` 1.2.0 → **2.0.0**, `oxdoc-tabular` 0.1.0 → **0.2.0**,
`oxdoc-cli` 1.2.0 → **2.0.0**. All MUST-EDIT rows land in the single version-bump
commit (slice 1, step 1 of the releasing flow).

### 1.1 MUST edit in the version-bump commit

| File | Edit | Why / evidence |
| --- | --- | --- |
| `crates/oxdoc-core/Cargo.toml` | line 3: `version = "1.2.0"` → `"2.0.0"` | spec req 1 |
| `crates/oxdoc-tabular/Cargo.toml` | line 3: `version = "0.1.0"` → `"0.2.0"`; line 22: `oxdoc-core = { path = "../oxdoc-core", version = "1.2.0" }` → `version = "2.0.0"` | cross-dep; 0.x consumers isolate via `^0.2` |
| `crates/oxdoc-cli/Cargo.toml` | line 3: `version = "1.2.0"` → `"2.0.0"`; line 23: `oxdoc-core … version = "1.2.0"` → `"2.0.0"`; line 24: `oxdoc-tabular … version = "0.1.0"` → `"0.2.0"` (exact minor — 0.x requirement MUST name `"0.2.0"`, not `"0.2"`) | spec req 1 |
| `Cargo.lock` | three `[[package]]` version lines: `oxdoc-core` (line ~131) → 2.0.0, `oxdoc-tabular` (line ~842) → 0.2.0, `oxdoc-cli` (line ~813) → 2.0.0. **Regenerate with `cargo check --workspace --all-features` and COMMIT the result.** | `.github/workflows/release.yml` builds with `--locked` on the tagged commit (lines 92, 140, 142); an uncommitted lock breaks the tag build. Committing is the repo's existing convention. |
| `fuzz/Cargo.lock` | `oxdoc-core` path-dep entry (name line + `version = "1.2.0"`) → 2.0.0. Regenerate via `cargo metadata --manifest-path fuzz/Cargo.toml` (or any cargo command under `fuzz/`) and COMMIT. | `fuzz/Cargo.toml` declares the dep by `path` only (no `version` key), so there is no manifest edit — only the lock. |
| `CHANGELOG.md` | `## Unreleased` (line 7) → `## 2.0.0 - <release date>` + one-line crate-version note naming 2.0.0 / 0.2.0 / 2.0.0; **all three breaks stay**: rows-JSONL v2 (lines 45–56), `XlsxCell.formula` source break (58–64, with migration note), structured-JSON v2 (66–84) | parent resolution 2; spec req "CHANGELOG dates the release" |
| `tests/fixtures/snapshots/cli_info_json.json` | line 2 `"oxdoc_version": "1.2.0"` → `"2.0.0"` — **REQUIRED** | `crates/oxdoc-cli/tests/cli.rs:1525` compares the full JSON value against this snapshot (`assert_eq!(actual, expected)`); the bump breaks the suite without it |
| `crates/oxdoc-core/tests/schema.rs` | line ~122 literal `"oxdoc_version": "1.2.0"` → `"2.0.0"` | representative audit-jsonl payload; schema-valid either way, kept current |
| `docs/library-api.md` | line 10 snippet `oxdoc-tabular = { version = "0.1.0", … }` → `"0.2.0"` | spec req 1 |
| `crates/oxdoc-tabular/README.md` | snippet `version = "0.1.0"` → `"0.2.0"`; closing link `../../docs/spikes/xlsx-arrow-parquet.md` → absolute `https://github.com/spereyra-dev/oxdoc/blob/main/docs/spikes/xlsx-arrow-parquet.md` (see §2) | spec req 2 |
| `docs/cli.md` (line ~524), `docs/json-output.md` (line ~204), `docs/audit.md` (line ~37), `docs/recipes.md` (line ~342) | example payload `"oxdoc_version": "1.2.0"` → `"2.0.0"` | four one-line stale examples |
| `README.md` | line 16 Status: "stable 1.x CLI and Rust API contract" → 2.x wording; line 5: crates.io badge line **removed** (restored in slice 2) | parent resolution 3; spec req "Documentation truthfulness" |

### 1.2 Verified non-changes (do NOT touch; recorded so the apply phase doesn't "fix" them)

| Candidate | Verdict | Reason |
| --- | --- | --- |
| `tests/fixtures/snapshots/cli_audit_json.json`, `all_sheets_manifest.json` (both carry `"oxdoc_version": "1.2.0"`) | optional consistency bump (recommended) | Loaded by `crates/oxdoc-core/tests/schema.rs` (lines 109, 158) and validated **against the schema only** — `oxdoc_version` is `{"type":"string"}` with no const/pattern, so no test breaks. Bumping keeps the representative payloads truthful for 2 lines; skipping is also spec-safe. |
| `tests/fixtures/compatibility-matrix.json` (`"producer_version": "1.2.0"`) | **never touch** | That is the *external OOXML producer's* version (e.g. Word), asserted by `scripts/check-compatibility-corpus.py` provenance rules — not the oxdoc version. |
| `package.json` / `package-lock.json` (1.2.0 hits) | never touch | `package.json` is `oxdoc-docs` with **no version field**; the lock's 1.2.0 entries are unrelated npm deps. |
| `python/pyproject.toml`, `.github/workflows/publish-python.yml` | never touch (spec req 8) | Python wrapper unaffected. |
| `docs/cli.md` line 73 | no edit | Mentions the *field name* `oxdoc_version`, not a value. |
| `docs/installation.md`, `README.md` `cargo install oxdoc-cli` lines, `crates/oxdoc-cli/README.md`, `python/README.md` | no edit (slice 1 or 2) | `cargo install oxdoc-cli` is version-less; truth becomes real at publish. Verified-only in slice 2. |
| `python/README.md` crates.io mention | no edit | Same as above. |

## 2. Metadata fixes (exact diffs)

`crates/oxdoc-tabular/Cargo.toml` — four additions plus version lines (aligned with
`oxdoc-core`/`oxdoc-cli` patterns):

```diff
 [package]
 name = "oxdoc-tabular"
-version = "0.1.0"
+version = "0.2.0"
 authors.workspace = true
 description = "XLSX schema inference and optional Arrow/Parquet conversion for oxdoc"
 edition.workspace = true
 homepage.workspace = true
 keywords.workspace = true
 categories.workspace = true
 license.workspace = true
+readme = "README.md"
 repository.workspace = true
 rust-version.workspace = true
 publish = true
+include = ["Cargo.toml", "README.md", "src/**", "examples/**"]
+
+[package.metadata.docs.rs]
+features = ["parquet"]
@@
-oxdoc-core = { path = "../oxdoc-core", version = "1.2.0" }
+oxdoc-core = { path = "../oxdoc-core", version = "2.0.0" }
```

- `readme` — the package page currently renders with no README even though the file
  exists.
- `include` — the sibling pattern; today the directory is clean (Cargo.toml, README,
  src, two feature-gated examples), so this closes future drift, and the two
  `required-features = ["parquet"]` examples must ship for the crate to build.
- `[package.metadata.docs.rs] features = ["parquet"]` — explicit list, NOT
  `all-features = true` (would silently widen docs.rs to every future optional
  feature). If docs.rs times out building parquet+Arrow 60, the documented fallback
  is dropping the metadata, never widening it.
- `publish = true` stays explicit and truthful.

`crates/oxdoc-tabular/README.md` — the escaping relative link:

```diff
-See [`docs/spikes/xlsx-arrow-parquet.md`](../../docs/spikes/xlsx-arrow-parquet.md)
+See [`docs/spikes/xlsx-arrow-parquet.md`](https://github.com/spereyra-dev/oxdoc/blob/main/docs/spikes/xlsx-arrow-parquet.md)
 for architecture, measurements, and completed production gates.
```

This is the only relative link in any crate README that crosses the package
boundary; the apply phase re-audits all three crate READMEs (`cargo package --list`
must include every linked file's README-visible equivalent, and every link must
resolve as it would on a crates.io page).

Per-crate metadata verification table (apply phase, from spec req 2):
description (crate-specific, non-empty) · license `MIT` (workspace SPDX) ·
repository/homepage `https://github.com/spereyra-dev/oxdoc` · `readme = "README.md"`
wired and present (all three) · include list present (all three) · keywords
`docx, xlsx, ooxml, parser, cli` (exactly 5, lowercase) · categories
`command-line-utilities, parser-implementations` · `rust-version 1.88` ·
`publish` not false · docs.rs metadata (tabular only) · no escaping README links.

## 3. `docs/releasing.md` skeleton (new page; parent resolution 1)

New Docsify page. Registered in **two lists** (parent resolution 1):

1. `docs/_sidebar.md` → **Project** section, added next to `[Release Process](release-process.md)`
   as `- [Releasing](releasing.md)`.
2. `README.md` → "Key documentation pages" list, as `- [Releasing](docs/releasing.md)`.

Single source of truth: `docs/releasing.md` owns the ordered flow. `docs/release-process.md`
§Crates.io Publishing keeps its invariant content (dependency order rationale, keep-list,
the existing `--no-verify` paragraph) and gains one pointer line to
`[Releasing](releasing.md)` for the step-by-step — no duplicated step list. Optional
one-line consistency edit: `docs/contributing.md` line 93 may name both pages for
release/distribution changes.

### Skeleton (ordered, each step = command → expected evidence → stop-on-failure gate)

```markdown
# Releasing (crates.io)

> **Irreversibility warning.** A published crates.io version can never be
> replaced or deleted — only yanked. Every step below exists to protect that
> operation; never publish without passing every prior gate.

## Preconditions (step 0) — HUMAN GATE
- Clean `main`, CI green, `git status --porcelain` empty (cargo publish refuses a
  dirty tree), crates.io token loaded on this machine (`cargo login` state is not
  queryable — verify with a `cargo publish -p oxdoc-core --dry-run` against the
  real index), maintainer authorizes publishing the approved triple.

## 1. Version-bump commit
The file-by-file inventory (§1 above) in one commit; `cargo pkgid` and the test
suite prove it.

## 2. Approved-version gate — STOP ON MISMATCH
    cargo pkgid -p oxdoc-core     # expect …#oxdoc-core@2.0.0
    cargo pkgid -p oxdoc-tabular  # expect …#oxdoc-tabular@0.2.0
    cargo pkgid -p oxdoc-cli      # expect …#oxdoc-cli@2.0.0
    grep -n '^## 2.0.0' CHANGELOG.md

## 3. Metadata gate — STOP ON GAP
tabular `readme`/`include`/docs.rs metadata present; per-crate table (§2) checked.

## 4. Full local gate — STOP ON RED
`make ci` (fmt, check, clippy, test, doctest, coverage ≥ 95, scripts-test,
docs-check, docs-links, docs-schemas-check, docs-playground-check).

## 5. Review + merge — HUMAN GATE
Release PR reviewed; merge to `main`.

## 6. Package + dry-run (pre-publish) — STOP ON FAILURE
    cargo package -p oxdoc-core                    # full verify
    cargo package -p oxdoc-tabular --no-verify     # resolution caveat, below
    cargo package -p oxdoc-cli --no-verify
    cargo package -p oxdoc-core --list && cargo package -p oxdoc-tabular --list \
      && cargo package -p oxdoc-cli --list         # inspect: no tests/fixtures/workspace-only files
    cargo publish -p oxdoc-core --dry-run          # full

### Resolution caveat and retry policy
`oxdoc-tabular` and `oxdoc-cli` cannot fully verify until their upstream crate
exists on the registry (the registry requirement is not satisfied by the path
dependency during publish resolution). Pre-publish validation for those two is
therefore `cargo package … --no-verify` + `--list` inspection; the full
`cargo publish --dry-run` for each MUST be repeated immediately after its
upstream publishes. After a publish, the sparse index may lag: if a dry-run fails
with a resolution error for a version you just published, **wait and re-run —
never skip a dry-run**.

## 7. Tag + push
    git tag -a v2.0.0 -m "oxdoc 2.0.0"   # on the merged release commit
    git push origin v2.0.0               # triggers release.yml

**Ordering consequence (documented, parent-fixed):** the tag goes out *before*
the crates, so the GitHub Release — which advertises `cargo install oxdoc-cli`
— can be public before crates.io resolves the crate. The publish sequence
follows immediately in the same session.

## 8. Confirm + publish oxdoc-core — HUMAN GATE
Re-confirm 2.0.0 / 0.2.0 / 2.0.0 with the maintainer out loud; clean tree;
    cargo publish -p oxdoc-core
    cargo search oxdoc-core --limit 1    # expect oxdoc-core = "2.0.0"

## 9. Publish oxdoc-tabular then oxdoc-cli — STOP BETWEEN CRATES
    cargo publish -p oxdoc-tabular --dry-run   # now resolves core 2.0.0; retry on index lag
    cargo publish -p oxdoc-tabular
    cargo search oxdoc-tabular --limit 1       # expect "0.2.0"
    cargo publish -p oxdoc-cli --dry-run
    cargo publish -p oxdoc-cli
    cargo search oxdoc-cli --limit 1           # expect "2.0.0"
    # end-to-end receipt, clean target dir:
    CARGO_TARGET_DIR=$(mktemp -d) cargo install oxdoc-cli --version 2.0.0

## 10. Post-publish reconciliation
Docs flip per the traffic light (§4); `make docs-links`; receipts into
`openspec/changes/release-crates-io-publish/apply-progress.md`.

## Recovery (yank-only)
Published versions are immutable. A wrong release is fixed forward with a patch
version (2.0.1 / 0.2.1 — which re-cascades the internal requirement bumps and
repeats this checklist). `cargo yank --version <v> -p <crate>` is the documented
last resort: it hides the version from new resolution without deleting it
(existing lockfiles keep working; the tarball stays public). Do not re-publish
the same version — `crate version already exists` is permanent.

## CI publishing (future prerequisite — explicitly deferred)
crates.io has no OIDC trusted-publishing flow for cargo (unlike PyPI's
`id-token: write`). A future CI job would need a `CARGO_REGISTRY_TOKEN`
repository secret, a `workflow_dispatch` approved-version input asserted against
all three manifests (modeled on `publish-python.yml`), dependency-ordered jobs
with index-lag waits, and a documented token rotation/revocation policy. This
change adds none of that.
```

## 4. Documentation traffic light (per file, per slice)

| File | Slice 1 (pre-publish commit) | Slice 2 (post-publish commit) |
| --- | --- | --- |
| `docs/discoverability.md` | Move the "crates.io CLI" row out of **Published Channels** into a new "Publishing with 2.0.0" entry under **Channels Not Yet Published** (activation steps: approved version, ordered dry-runs, receipts; note the README badge returns with the release). Adjust the badge-paragraph sentence that says the crates.io badge reports its upstream version. | Return crates.io CLI to **Published Channels** with published versions and the install command; drop the Not-Yet-Published entry. |
| `README.md` | Remove the crates.io badge line (badge policy: only add after destination is public). Refresh the `Status` line to 2.x wording. | Restore the badge (same line), verified against the live crate page. Drop "unpublished" from the `oxdoc-tabular` description (line ~356). No other README edits; no new badges. |
| `docs/launch-publicity.md` | Rewrite the crates.io outreach item to name all three crates in dependency order with the 2.0.0 versions; point to `docs/releasing.md` for the checklist. (The page stays 1.0-launch-themed; only the crates.io item is reconciled.) | No edit (already accurate). |
| `docs/roadmap.md` + `ROADMAP.md` | Unchanged — "publish to crates.io" as pending work is true. | Phase 5 bullet flips to implemented (both files, same wording as ROADMAP.md lines 61–62). |
| `.markdown-link-check.json` | Keep the `^https://crates.io/crates/oxdoc-cli/?$` ignore (destination still 404s). | Remove the ignore so `make docs-links` validates the live URL. Gated on the live page — MUST NOT be removed while it would 404. |
| `docs/installation.md` | No edit ("installed from … crates.io" is forward-looking here). | Verify only: sentence now true; no edit. |
| `docs/release-process.md` | Add pointer line to `docs/releasing.md` in §Crates.io Publishing; keep invariants and `--no-verify` paragraph. | No edit. |
| `docs/_sidebar.md`, `README.md` docs list | Add `docs/releasing.md` entries (both). | No edit. |
| `docs/contributing.md` | Optional: extend the release-changes line to name `releasing.md`. | No edit. |

Gates: `make docs-links` checks `README.md` + `docs/**/*.md` (Makefile line 143–144)
with `.markdown-link-check.json`; `docs-check`, `docs-schemas-check`,
`docs-playground-check` must stay green at every commit.

## 5. Slice plan and receipt format

### Slice 1 — version + metadata + snapshot + checklist + pre-publish docs (one PR, ships)

Single version-bump commit carrying every MUST-EDIT row of §1 (including
releasing.md, sidebar/README list entries, launch-publicity, discoverability
pre-state, release-process pointer), because `cargo pkgid`, `cargo test
--workspace`, and the docs gates must all be green in that one commit. Then the
pre-publish packaging receipt commit (§1 step 6 outputs — receipts only, into
`openspec/changes/release-crates-io-publish/apply-progress.md`).

Verifiable **before** any publish: everything. The change stops here — a truthful,
complete pre-publish state — if the human publish gate is not authorized.

### Slice 2 — applied AFTER the human-gated publish (receipts + docs flip)

Executed only after the maintainer has run steps 8–9. Content: the slice-2 column
of §4 (discoverability restore, README badge + "unpublished" drop, both roadmaps,
link-check ignore removal) plus the publish receipts. `docs-links` re-run with
the ignore removed; badge target verified with an HTTP 200 on
`https://crates.io/crates/oxdoc-cli`.

### Receipt format (in `openspec/changes/release-crates-io-publish/apply-progress.md`, section "Publish receipts")

```markdown
### Receipt: <step-id> — <crate> <command-kind>
- date/commit: <ISO date> @ <git sha or "local publish session">
- command: `cargo publish -p oxdoc-core --dry-run`
- result: pass | fail(then retried) | skipped(N/A: <reason>)
- evidence: 1–3 trimmed output lines (e.g. `oxdoc-core v2.0.0 found`,
  `.crate file packed: …/oxdoc-core-2.0.0.crate (… kB)`, search hit)
```

Required receipts for a complete change: per-crate `cargo package --list`;
`cargo package -p oxdoc-core` (full) and `cargo publish -p oxdoc-core --dry-run`
(full); `cargo package -p oxdoc-tabular/cli --no-verify` (pre-publish); full
`--dry-run` for tabular and cli post-upstream-publish; three real publishes with
registry checks; the clean-target `cargo install oxdoc-cli --version 2.0.0`; the
badge HTTP verification. Any dry-run failure that was resolved by the wait-and-retry
policy is recorded as `fail(then retried)` with both outputs.

## 6. Verification per slice

### Slice 1 gates (all local, all must pass before review/merge)

1. `cargo pkgid -p oxdoc-core|oxdoc-tabular|oxdoc-cli` reports 2.0.0 / 0.2.0 / 2.0.0; `grep -n '^## 2.0.0' CHANGELOG.md` matches.
2. `cargo check --workspace --all-features` regenerates both lockfiles; commit them (`release.yml --locked` depends on it).
3. `make ci` green — fmt, check, clippy (`-D warnings`), test (snapshot bumped, `prints_info_as_json_and_text` passes), doctest, coverage ≥ 95 with **no coverage delta** (test/literal edits only), scripts-test, docs-check, docs-links (crates.io ignore still present), docs-schemas-check, docs-playground-check.
4. Per-crate packaging receipts: `cargo package -p oxdoc-core` (full verify), `cargo package -p oxdoc-tabular --no-verify`, `cargo package -p oxdoc-cli --no-verify`, plus `--list` × 3 showing no tests/fixtures/workspace-only files and README + examples included for tabular.
5. `cargo publish -p oxdoc-core --dry-run` (full) recorded as a receipt.
6. Protected-path diff check: no `crates/*/src/**`, `schemas/**`, `docs/schemas/**`, `python/**`, `.github/workflows/**`, `action.yml`, `install.sh` changes; `python/pyproject.toml` byte-identical.

### Slice 2 gates (post-publish)

1. Full `cargo publish --dry-run` for tabular and cli (now resolving), each retried per the wait-and-retry policy — never skipped.
2. Three real publishes in order core → tabular → cli, each with a registry verification (`cargo search … --limit 1` or crates.io API) showing 2.0.0 / 0.2.0 / 2.0.0, stop between crates.
3. End-to-end: `cargo install oxdoc-cli --version 2.0.0` in a clean target dir (`CARGO_TARGET_DIR=$(mktemp -d)`).
4. Docs flip: `make docs-links` green with the ignore removed; badge URL HTTP 200; roadmap bullets flipped in both files.
5. `v2.0.0` tag exists on the merged release commit; release workflow produced its GitHub Release/artifacts independently of the crates.

## 7. Rollback anchors (unchanged from proposal, restated for the apply phase)

- Before any real publish: revert the version-bump commit — nothing else was touched at the registry, repo returns exactly.
- After tag, before crates live: delete local+remote tag and the draft GitHub Release; crates still unpublished.
- After a publish: immutable — fix forward with a patch version re-running the checklist; `cargo yank` as last resort; docs-flip slice 2 is independently revertible (docs + one JSON config).

## 8. Size estimate vs the 400-line review budget

| Content | Realized estimate |
| --- | --- |
| Slice 1: version/reference bump (3 manifests, 2 locks, CHANGELOG, snapshot + literal, 2 snippets, 4 examples, README status/badge) | 50–70 |
| Slice 1: tabular metadata + README link fix | 12–16 |
| Slice 1: `docs/releasing.md` new page + sidebar/README/release-process pointers | 120–160 |
| Slice 1: pre-publish docs truth (discoverability, launch-publicity) | 40–55 |
| Slice 1 total (one PR) | **~220–300** |
| Slice 2 (post-publish flip + receipts) | **30–45** |
| Shipped total | **~250–345** — fits 400 as a single PR; if the count crosses, the parent's `ask-on-risk` gate decides (natural boundary: slice 2). |

## 9. Residual risks (design-level deltas only; proposal §Risks carries the rest)

- **`--locked` release builds** (found during design): the tag triggers
  `cargo build --locked`, so the version-bump commit MUST include both lockfiles —
  already mandated in §1; called out here because forgetting it surfaces only after
  the tag, i.e. after the point of no return for the registry.
- **`cli_audit_json.json` / `all_sheets_manifest.json` staleness** (found during
  design): schema-validated, so not breaking, but the apply phase should bump them
  in the same commit (2 lines) unless the review budget is tight.
- **Tag-before-publish gap**: consequence documented in releasing.md step 7;
  publish sequence follows immediately per parent resolution 4.
- **Human-gate stall**: slice 1 is a complete, truthful deliverable on its own;
  slice 2 is blocked-not-skipped if the gate is not authorized.
