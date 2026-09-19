# Exploration — release: publish Rust crates to crates.io (issue #173)

Change: `release-crates-io-publish` · Artifact store: openspec · Reviewed 2026-09-xx on `main`.

## Preflight

SDD session preflight block was present and well-formed (mode auto, openspec store,
ask-on-risk on review-budget risk, 400-line review budget). Publishing is a
human-controlled gate per preflight: no real `cargo publish` occurs in this change's
automation; real publish stays maintainer-owned.

## 1. Current crate metadata audit

Workspace `[workspace.package]` (`Cargo.toml`): authors `Santiago Pereyra Marchetti`,
edition 2024, homepage `https://github.com/spereyra-dev/oxdoc`, keywords
`docx, xlsx, ooxml, parser, cli` (exactly 5 — crates.io's max, OK), categories
`command-line-utilities, parser-implementations` (valid crates.io slugs), license
`MIT`, readme `README.md`, repository `https://github.com/spereyra-dev/oxdoc`,
rust-version `1.88`.

| Field | oxdoc-core 1.2.0 | oxdoc-tabular 0.1.0 | oxdoc-cli 1.2.0 |
| --- | --- | --- | --- |
| own description | yes ("Core OOXML parsing library for oxdoc") | yes | yes |
| readme | explicit `README.md` | **missing — no `readme` key at all** | explicit `README.md` |
| include list | yes (Cargo.toml, README, src, examples, benches) | **missing — packages whole crate dir** | yes (Cargo.toml, README, src) |
| publish flag | default true | `publish = true` explicit | default true |
| LICENSE file in package | no (SPDX `license` field is used; workspace `LICENSE` not in include) | same | same |

Crate-local READMEs exist for all three (`crates/*/README.md`); tabular's README
already documents `version = "0.1.0"` install snippets and the `parquet` feature.

Notable findings:

- **oxdoc-tabular lacks `readme`** — crates.io would render the package page with
  no README even though `crates/oxdoc-tabular/README.md` exists. This violates
  `docs/release-process.md`'s "All crates must keep … A crate-local `README.md`"
  contract in spirit (file exists, metadata doesn't wire it).
- **oxdoc-tabular lacks an include list** while the other two crates have one;
  release-process.md requires "Package include lists that avoid publishing
  workspace-only integration fixtures." Today the directory is clean (Cargo.toml,
  README, src, examples only), so the risk is future drift, not current leakage.
- No `[package.metadata.docs.rs]` in any crate. oxdoc-tabular's `parquet` module
  (optional deps arrow-array/arrow-schema/parquet 60.0.0) will not render on
  docs.rs under default features unless docs.rs metadata enables it.
- `.markdown-link-check.json` ignore pattern for `https://crates.io/crates/oxdoc-cli/?$`
  confirms the crates.io page currently 404s — **none of the crates are published yet**.

## 2. Dependency graph and publish order

- `oxdoc-core`: no internal deps.
- `oxdoc-tabular`: depends on `oxdoc-core { path, version = "1.2.0" }`.
- `oxdoc-cli`: depends on `oxdoc-core { path, version = "1.2.0" }` and
  `oxdoc-tabular { path, version = "0.1.0", default-features = false }`.

Required publish order (matches release-process.md): **core → tabular → cli**.
`oxdoc-cli`'s dry-run cannot resolve `oxdoc-tabular`/`oxdoc-core` from the registry
before they exist; release-process.md documents the `cargo package -p … --no-verify`
workaround plus full dry-run repetition after each upstream publish. Version
requirements use semver-compatible ranges (`"1.2.0"` = `^1.2`), so path deps need no
version bump to publish, but the maintainer-approved release version does.

## 3. Release infrastructure today

- `.github/workflows/release.yml`: tag/workflow_dispatch driven; builds binaries for
  5 targets, benchmark evidence bundle, SHA256SUMS, GitHub Release via
  softprops/action-gh-release. **No crates.io publish job; no CARGO_REGISTRY_TOKEN
  secret referenced anywhere.**
- `.github/workflows/publish-python.yml`: the house pattern for an approved-version
  publish — `workflow_dispatch` with a maintainer-approved `version` input that is
  asserted to match `python/pyproject.toml` before building. Useful template, but
  crates.io has no OIDC token flow, so a Rust equivalent would need a
  `CARGO_REGISTRY_TOKEN` secret or a local maintainer-driven publish.
- Makefile: no `package`/`publish` targets; `scripts-test`, `docs-links`,
  `docs-check` exist. No crates.io dry-run automation.

## 4. Version, tag, and CHANGELOG state

- Annotated tags (`.git/packed-refs`): `v0.1.0`, `v1.0.0`, `v1.1.0`, `v1.2.0`.
  Convention: `vMAJOR.MINOR.PATCH` matching crate versions at release time.
- CHANGELOG.md: released sections `1.2.0 - 2026-08-04`, `1.1.0`, `1.0.0`; a large
  **Unreleased** section is pending, including a documented source break:
  `oxdoc-core::XlsxCell` gains a `formula: Option<XlsxFormula>` field (migration
  note included). release-process.md says after 1.0 use strict semver — a struct
  field addition that breaks external struct literals is a major bump
  (2.0.0) unless the maintainer explicitly approves treating it otherwise. The
  approved version is a maintainer gate and currently undecided; if core bumps,
  tabular/cli manifests and CHANGELOGs must follow.
- README.md already carries a crates.io badge for `oxdoc-cli`; `docs/discoverability.md`
  lists crates.io as a *Published Channel*, but the link-check ignore pattern and
  `docs/roadmap.md` ("Publish … to crates.io in dependency order" as pending work)
  contradict that. Discoverability doc needs correction before/with the real publish.

## 5. Credentials and publishing prerequisites (verified without publishing)

- Local crates.io token state is **not queryable** — `cargo login` status has no
  status command; a dry-run genuinely does not need a token. The maintainer context
  states credentials are loaded on this machine and publishing is authorized, but
  the exact version requires explicit maintainer approval before any real publish.
- Real publish prerequisites: (a) maintainer-approved release version for all three
  crates, (b) registry token on the publishing machine or a repo secret, (c) crates
  published strictly in dependency order, (d) full `cargo publish --dry-run`
  (or package) for every crate from a clean checkout.

## 6. Gaps vs acceptance criteria

1. **Metadata verification** — fix tabular `readme`; add tabular include list (or
   consciously document its omission); decide docs.rs metadata for tabular's
   parquet docs; confirm README links resolve on crates.io pages.
2. **Repeatable release checklist/dry-run flow** — release-process.md has the order
   and dry-run commands, but `docs/launch-publicity.md` is stale (core → cli,
   omits tabular) and there is no automated/manifest-checked dry-run flow. Add a
   documented checklist (and optionally a script or workflow) that runs
   `cargo package` + `cargo publish --dry-run` per crate and gates on approved version.
3. **Validate `cargo package` / `cargo publish --dry-run` for every crate** — must be
   run in implementation/verify phases (this exploration had no shell access);
   note the ordering constraint: cli/tabular dry-runs only fully verify after core
   is on the registry (or via the documented `--no-verify` package check).
4. **Publishing gate** — mirror the publish-python.yml approved-version pattern;
   document the exact blocking prerequisite (approved version + token) in
   release-process.md; defer real `cargo publish` to the maintainer.
5. **Docs consistency** — reconcile discoverability.md (crates.io listed as
   published while it 404s), launch-publicity.md (missing tabular), and the README
   badge policy ("Only add a README badge after its destination is public").

## 7. Risks

- **Version-semantics risk (blocking)**: the Unreleased `XlsxCell.formula` break
  forces a major/minor decision for `oxdoc-core` and cascades into tabular/cli
  versions; must be resolved by the maintainer before packaging.
- **Ordering risk**: publishing out of order leaves `oxdoc-cli` unpublished or
  dry-run failures that look like defects.
- **Docs drift risk**: three docs surfaces disagree on crates.io state; publishing
  without fixing discoverability.md breaks its own release-time validation rules.
- **Registry immutability**: crates.io versions can never be replaced — a bad
  `cargo publish` is permanent; dry-runs and `cargo package -l` inspection are the
  only cheap safety net.
- **Not executed here**: `git tag -l`, `cargo package`, `cargo publish --dry-run`
  and credential checks require shell/cargo; they are scoped to the next phase.

## 8. Next recommended

1. Proposal scope: metadata fixes (tabular readme + include/docs.rs), a documented
   repeatable dry-run/release checklist, docs consistency fixes, and an explicit
   maintainer approval gate for the real publish.
2. Ask the maintainer: approved release versions for the three crates (resolving
   the `XlsxCell.formula` semver question) before any packaging verification.
3. In the verify phase, run `cargo package` and `cargo publish --dry-run` per crate
   in dependency order from a clean checkout and record results as receipts.
