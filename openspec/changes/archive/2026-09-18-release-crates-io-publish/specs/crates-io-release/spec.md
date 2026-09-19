# Crates.io Release Specification

## Purpose

Define what must be true when `oxdoc-core`, `oxdoc-tabular`, and `oxdoc-cli` are
published to crates.io for the first time (change `release-crates-io-publish`,
issue #173): the approved version triple is consistent across every in-repo
reference, each crate's metadata is complete and verifiable, packaging is
validated before the irreversible registry operation, the release flow is
documented as a repeatable checklist, every public claim about crates.io state
is true at the commit that carries it, the real publish is a human-gated local
operation, and the CHANGELOG dates the release correctly. No runtime, CLI,
output, schema, or workflow behavior changes.

Approved version triple (fixed by the maintainer, 2026-09-18):
`oxdoc-core` 2.0.0 · `oxdoc-tabular` 0.2.0 · `oxdoc-cli` 2.0.0.

## Requirements

### Requirement: Approved-version consistency

The system's release artifacts MUST agree on the approved version triple
(`oxdoc-core` 2.0.0, `oxdoc-tabular` 0.2.0, `oxdoc-cli` 2.0.0): the three crate
manifests, the cross-crate dependency requirements
(`oxdoc-tabular → oxdoc-core = "2.0.0"`, `oxdoc-cli → oxdoc-core = "2.0.0"`,
`oxdoc-cli → oxdoc-tabular = "0.2.0"` — a 0.x requirement MUST name the exact
minor), `Cargo.lock` (three version lines), `fuzz/Cargo.lock` (`oxdoc-core`
path-dep entry), the `cli_info_json.json` snapshot fixture
(`"oxdoc_version": "2.0.0"`), the representative `oxdoc-core/tests/schema.rs`
version literal, the `docs/library-api.md` and `crates/oxdoc-tabular/README.md`
dependency snippets (`version = "0.2.0"`), the four doc example payloads
(`docs/cli.md`, `docs/json-output.md`, `docs/audit.md`, `docs/recipes.md`), and
the CHANGELOG release section. All of these references MUST move together in a
single version-bump commit. `cargo pkgid -p <crate>` MUST report the approved
version for each of the three crates. `python/pyproject.toml` MUST remain at
0.1.0 and MUST NOT be touched.

#### Scenario: cargo pkgid confirms the triple after the version-bump commit

- GIVEN the version-bump commit is applied to a clean checkout
- WHEN `cargo pkgid -p oxdoc-core`, `cargo pkgid -p oxdoc-tabular`, and
  `cargo pkgid -p oxdoc-cli` are run
- THEN the reported versions are exactly 2.0.0, 0.2.0, and 2.0.0, and
  `grep -n '^## 2.0.0' CHANGELOG.md` matches.

#### Scenario: dependency requirements name the exact tabular minor

- GIVEN the manifests after the bump
- WHEN the `oxdoc-tabular` requirement in `crates/oxdoc-cli/Cargo.toml` is read
- THEN it is `"0.2.0"` (not `"0.1.0"`), and both tabular and cli require
  `oxdoc-core = "2.0.0"`.

#### Scenario: the test suite survives the version bump

- GIVEN the snapshot fixture and doc examples were updated in the same commit
- WHEN `cargo test --workspace --all-features --all-targets` runs
- THEN `prints_info_as_json_and_text` passes against the updated
  `cli_info_json.json` snapshot and no version-literal test fails.

### Requirement: Publishable crate metadata

Each of the three crates MUST ship complete crates.io metadata: a
crate-specific non-empty `description`, an SPDX `MIT` license, workspace
`repository` and `homepage` (`https://github.com/spereyra-dev/oxdoc`), a
crate-local `readme` wired to an existing `README.md`, a non-leaking `include`
list (no tests, fixtures, or workspace-only files in `cargo package --list`
output), at most 5 lowercase keywords
(`docx, xlsx, ooxml, parser, cli`), valid crates.io category slugs
(`command-line-utilities`, `parser-implementations`), `rust-version 1.88`, and
a `publish` flag that is not `false`. `crates/oxdoc-tabular/Cargo.toml` MUST
additionally declare `readme = "README.md"`, an `include` list covering
`Cargo.toml`, `README.md`, `src/**`, and `examples/**` (both feature-gated
examples must ship so the crate builds), and
`[package.metadata.docs.rs] features = ["parquet"]` so the optional parquet
module renders on docs.rs (`all-features` MUST NOT be used). The tabular README
MUST NOT contain any relative link that escapes the package: the
`docs/spikes/xlsx-arrow-parquet.md` link becomes an absolute repository link,
and no link in any crate README escapes the package.

#### Scenario: tabular package page renders its README and docs

- GIVEN the tabular metadata additions are applied
- WHEN `cargo package -p oxdoc-tabular --list` is run and the crate page is
  inspected after publish
- THEN the package includes the README and both `examples/**` files, contains no
  tests/fixtures/workspace-only files, and docs.rs builds the `parquet` feature.

#### Scenario: README link audit finds no escaping relative links

- GIVEN the three crate READMEs
- WHEN every link is resolved as it would be on a crates.io package page
- THEN every link resolves; the tabular spike-doc link is an absolute repository
  URL, and no relative link crosses the package boundary.

#### Scenario: per-crate metadata verification table passes

- GIVEN the metadata table (description, license, repository, homepage, readme,
  keywords, categories, rust-version, include, publish, docs.rs metadata) is
  checked per crate
- WHEN each field is read from the manifests and workspace inheritance
- THEN every cell matches the expected value; `oxdoc-tabular` sets
  `publish = true` (or omits the flag, defaulting to publishable), and docs.rs
  metadata exists for `oxdoc-tabular` only.

### Requirement: Packaging validation before publish

Before any real publish, each crate MUST be validated in dependency order
(`oxdoc-core`, then `oxdoc-tabular`, then `oxdoc-cli`) with `cargo package -p
<crate>`, a `cargo package -p <crate> --list` file-list inspection, and `cargo
publish -p <crate> --dry-run` where registry resolution allows. Because
`oxdoc-tabular` and `oxdoc-cli` cannot fully verify until their upstream crate
exists on the registry, pre-publish validation for those two crates MUST use
`cargo package … --no-verify` plus the file-list inspection, and the full `cargo
publish --dry-run` MUST be repeated for each crate immediately after its
upstream is live. Sparse-index propagation lag MUST be handled by waiting and
re-running — a dry-run MUST never be skipped. Each validation run MUST be
recorded as a receipt in
`openspec/changes/release-crates-io-publish/apply-progress.md`.

#### Scenario: core packages and dry-runs fully before anything publishes

- GIVEN a clean checkout of the version-bump commit
- WHEN `cargo package -p oxdoc-core` and `cargo publish -p oxdoc-core --dry-run`
  run
- THEN both succeed with full verification (no `--no-verify`) and their output is
  recorded as a receipt.

#### Scenario: tabular and cli pre-publish validation uses the documented workaround

- GIVEN `oxdoc-core` 2.0.0 is not yet on the registry
- WHEN `oxdoc-tabular` and `oxdoc-cli` are validated pre-publish
- THEN `cargo package -p oxdoc-tabular --no-verify` and
  `cargo package -p oxdoc-cli --no-verify` plus `--list` inspections succeed and
  are recorded, and the checklist documents that the full dry-run for each is
  repeated after its upstream publishes.

#### Scenario: dry-run is never skipped despite index lag

- GIVEN `cargo publish -p oxdoc-tabular --dry-run` fails immediately after the
  core publish due to sparse-index propagation lag
- WHEN the documented retry instruction is followed
- THEN the dry-run is re-run after a wait and MUST pass before the real tabular
  publish; skipping it is not an accepted outcome.

### Requirement: Documented release checklist

A repeatable, ordered crates.io release checklist MUST exist in
`docs/releasing.md` (the parent-resolved location, superseding the proposal's
preference for extending `docs/release-process.md`). The checklist MUST document
the exact ordered flow: preconditions (clean `main`, CI green, local cargo
credentials, maintainer authorization) → approved-version gate (`cargo pkgid`
per crate plus the CHANGELOG heading check, stopping on mismatch) → metadata
gate → full local gate (`make ci` including docs gates and coverage ≥ 95%) →
review + merge → package + dry-run in dependency order (with the tabular/CLI
resolution caveat and the `--no-verify` workaround) → tag + push (`v2.0.0`)
→ real publish in dependency order → receipts → post-publish docs
reconciliation. It MUST state the maintainer credential prerequisite (a logged-in
crates.io token on the publishing machine) and the irreversibility warning:
published versions can never be replaced or deleted, only yanked. It MUST
document the recovery policy (fix forward with a patch version that re-cascades
requirement bumps; `cargo yank --version <v> -p <crate>` as the documented last
resort) and a "CI publishing (future prerequisite)" subsection (no OIDC flow for
cargo; a future CI job would need `CARGO_REGISTRY_TOKEN`, an approved-version
input asserted against all three manifests, dependency-ordered jobs, and token
rotation), explicitly deferred by this change.

#### Scenario: a maintainer can execute a release from the checklist alone

- GIVEN only `docs/releasing.md` and a machine with cargo credentials
- WHEN the maintainer follows the checklist from a clean `main`
- THEN each step names its command, its expected evidence, and its stop-on-failure
  gate, and the steps appear in the exact order above with no step omitted.

#### Scenario: the irreversibility warning and recovery are present

- GIVEN the checklist document
- WHEN its publishing and recovery sections are read
- THEN they state that published versions are immutable (yank-only), document
  fix-forward via a patch version, name `cargo yank` as the last resort, and
  describe the deferred CI-token prerequisite.

### Requirement: Documentation truthfulness (pre/post-publish traffic light)

Every public claim about crates.io state MUST be true at the commit that carries
it. Pre-publish (slice 1): `docs/discoverability.md` moves crates.io out of
Published Channels into a "not yet published" entry with the 2.0.0 activation
plan; `README.md` carries no crates.io badge (the README badge policy: a badge
is added only after its destination is public); `docs/launch-publicity.md`
describes the real dependency order including `oxdoc-tabular` with 2.0.0
versions; `docs/roadmap.md` and `ROADMAP.md` keep crates.io publication listed
as pending; the README `Status` line and `oxdoc-tabular` description are
refreshed for truthfulness (the "unpublished" wording drops only after publish).
Post-publish (slice 2, after publish receipts): crates.io returns to Published
Channels with the published versions and install command; the README crates.io
badge is added/restored and verified against the live crate page; the roadmap
Phase 5 bullet flips to implemented; the
`^https://crates.io/crates/oxdoc-cli/?$` ignore in `.markdown-link-check.json`
is removed once the crate page exists so `make docs-links` validates the live
URL (and MUST NOT be removed while the page would 404). `make docs-links`,
`docs-check`, and the other docs gates MUST stay green at every commit.

#### Scenario: pre-publish docs do not claim a live channel

- GIVEN the slice-1 commits (before any real publish)
- WHEN `docs/discoverability.md`, `README.md`, and the roadmap files are read
- THEN crates.io is described as upcoming (with the 2.0.0 plan), the README has
  no crates.io badge, and `make docs-links` is green with the crates.io ignore
  still present.

#### Scenario: post-publish flip restores the badge and removes the ignore

- GIVEN the publish receipts exist for all three crates
- WHEN the slice-2 docs flip is applied
- THEN discoverability lists crates.io as published with the versions, the
  README badge points at the live crate page, the roadmap Phase 5 bullet reads
  as implemented, the link-check ignore is removed, and `make docs-links` stays
  green.

#### Scenario: the ignore removal is gated on the live page

- GIVEN the crates are not yet published
- WHEN the docs traffic-light is evaluated
- THEN the `.markdown-link-check.json` crates.io ignore remains in place, and it
  is removed only after the crate page exists.

### Requirement: Human-gated local publish

The real `cargo publish` MUST be executed locally from the maintainer's machine
— never from CI — and MUST NOT be preceded by any CI credential change: no
`CARGO_REGISTRY_TOKEN` or other publish secret is added to the repository, no
publish workflow is added, and no new publish automation (Makefile target,
script) is created. Before the first real publish, the approved version triple
MUST be re-confirmed with the maintainer, the working tree MUST be clean
(`git status --porcelain` empty), and all dry-run validations MUST have passed.
The crates MUST be published strictly in dependency order: `oxdoc-core` 2.0.0,
then `oxdoc-tabular` 0.2.0 (whose full dry-run re-runs first), then
`oxdoc-cli` 2.0.0, with a registry verification after each (`cargo search
oxdoc-core --limit 1`, or the crates.io API) and a stop between crates. The tag
`v2.0.0` is cut and pushed before the local publish (parent-fixed order; the
checklist documents that the GitHub Release can therefore be public before
crates.io resolves). Each publish and its registry verification, plus one
end-to-end receipt (`cargo install oxdoc-cli --version 2.0.0` in a clean target
directory), MUST be recorded in
`openspec/changes/release-crates-io-publish/apply-progress.md`. Recovery from a
bad release is yank-only/fix-forward as documented in the checklist.

#### Scenario: no publish happens without the human gate

- GIVEN the version-bump, checklist, and pre-publish docs work is complete but
  the maintainer has not re-confirmed the version triple
- WHEN the publish step is reached
- THEN no `cargo publish` (non-dry-run) is executed, and the change stops at
  slice 1 with accurate pre-publish docs rather than publishing without
  approval.

#### Scenario: publish order and verification receipts

- GIVEN the maintainer has re-confirmed 2.0.0 / 0.2.0 / 2.0.0 and the tree is
  clean
- WHEN the publishes execute
- THEN core publishes first with a registry check showing 2.0.0, tabular second
  (0.2.0), cli third (2.0.0), the end-to-end `cargo install oxdoc-cli --version
  2.0.0` succeeds, and every receipt is recorded in `apply-progress.md`.

#### Scenario: no CI credential or workflow change

- GIVEN the full change diff
- WHEN `.github/workflows/**`, repository secrets, and the Makefile are inspected
- THEN no `CARGO_REGISTRY_TOKEN`, no publish workflow, and no new publish
  automation exists; `release.yml` and the other workflows are byte-identical.

### Requirement: CHANGELOG dates the release

`CHANGELOG.md` MUST date the release: the former `## Unreleased` section becomes
`## 2.0.0 - <release date>` and MUST preserve every documented breaking change
it contained — the two versioned output-contract breaks (rows-JSONL v2 and
structured-JSON v2) and the `oxdoc-core::XlsxCell` `formula` source break with
its migration note. The 2.0.0 section MUST include a one-line note naming the
three crate versions (2.0.0 / 0.2.0 / 2.0.0) so the tabular 0.2.0 difference is
discoverable. No breaking-change entry MAY be silently dropped or moved out of
the release section.

#### Scenario: the release section carries all documented breaks

- GIVEN the CHANGELOG before and after the edit
- WHEN the `## 2.0.0` section is compared with the former `## Unreleased` content
- THEN the rows-JSONL v2 break, the structured-JSON v2 break, and the
  `XlsxCell.formula` source break (with migration note) all remain under the
  2.0.0 heading, dated with the release date, and the crate-version note is
  present.

#### Scenario: no Unreleased section remains for this release

- GIVEN the dated CHANGELOG
- WHEN the document is read
- THEN there is no leftover `## Unreleased` heading for this release's content
  and the previous released sections (1.2.0, 1.1.0, 1.0.0) are unchanged.

### Requirement: No behavior or CI change

The change MUST NOT alter runtime behavior, the CLI surface, output contracts,
schemas (`v1`/`v2`), warnings, or errors; MUST NOT modify any file under
`crates/*/src/`, `schemas/`, `docs/schemas/`, `python/`, or
`.github/workflows/`; MUST NOT change `python/pyproject.toml`; and MUST NOT add
docs.rs, download, or other new README badges beyond the existing crates.io
badge's pre-publish removal / post-publish restoration. `make ci` MUST be green
after the change, including `scripts-test`, `docs-check`, `docs-links`,
`docs-schemas-check`, `docs-playground-check`, and coverage ≥ 95%, with no
coverage delta from the version/metadata edits.

#### Scenario: behavior-sensitive paths are untouched

- GIVEN the full change diff
- WHEN the changed-file list is checked against the protected paths
  (`crates/*/src/**`, `schemas/**`, `docs/schemas/**`, `python/**`,
  `.github/workflows/**`, `action.yml`, `install.sh`)
- THEN no protected path appears in the diff.

#### Scenario: local gates stay green after the bump

- GIVEN the version-bump and metadata edits
- WHEN `make ci` runs
- THEN fmt, check, clippy, tests, doctests, coverage ≥ 95, and all docs gates
  pass with no coverage regression.
