# Releasing (crates.io)

This page is the single source of truth for the step-by-step crates.io release
flow. [Release Process](release-process.md) keeps the rationale for the
dependency order and the packaging keep-list; this page is the ordered
checklist a maintainer executes.

> **Irreversibility warning.** A published crates.io version can never be
> replaced or deleted — only yanked. `cargo publish` of a version that already
> exists fails permanently. Every step below exists to protect that operation;
> never publish without passing every prior gate.

## Preconditions (step 0) — HUMAN GATE

- The release branch is merged to a clean `main` and CI is green.
- `git status --porcelain` is empty (`cargo publish` refuses a dirty tree).
- A crates.io token is loaded on this machine (`cargo login` state is not
  queryable; verify it by running the step 6 dry-run against the real index).
- The maintainer authorizes publishing the approved version triple
  (`oxdoc-core` 2.0.0 · `oxdoc-tabular` 0.2.0 · `oxdoc-cli` 2.0.0).
  **Do not start without this.**

## 1. Version-bump commit

One commit moves every version reference together: the three crate manifests,
the cross-crate dependency requirements (`oxdoc-tabular → oxdoc-core = "2.0.0"`,
`oxdoc-cli → oxdoc-core = "2.0.0"`, `oxdoc-cli → oxdoc-tabular = "0.2.0"`),
**both** `Cargo.lock` and `fuzz/Cargo.lock`, the `cli_info_json.json` snapshot,
the schema-test version literal, the doc snippets and example payloads, and the
CHANGELOG heading. Evidence: `cargo pkgid` (step 2) and a green test suite.

## 2. Approved-version gate — STOP ON MISMATCH

```bash
cargo pkgid -p oxdoc-core     # expect …#oxdoc-core@2.0.0
cargo pkgid -p oxdoc-tabular  # expect …#oxdoc-tabular@0.2.0
cargo pkgid -p oxdoc-cli      # expect …#oxdoc-cli@2.0.0
grep -n '^## 2.0.0' CHANGELOG.md
```

Stop and fix if any version or the CHANGELOG heading does not match the
approved triple.

## 3. Metadata gate — STOP ON GAP

Each crate's `description`, `license`, `repository`/`homepage`, `readme`,
`include`, `keywords`, `categories`, `rust-version`, and `publish` flag are
complete, and `oxdoc-tabular` carries `readme = "README.md"`, an `include` list
covering `src/**` and `examples/**`, and `[package.metadata.docs.rs] features
= ["parquet"]`. Stop on any gap.

## 4. Full local gate — STOP ON RED

```bash
make ci
```

This runs `cargo fmt --all -- --check`, `cargo check --workspace
--all-features --all-targets`, `cargo clippy … -D warnings`, the full test and
doctest suites, coverage ≥ 95%, `scripts-test`, `docs-check`, `docs-links`,
`docs-schemas-check`, and `docs-playground-check`. Stop on any red check.

## 5. Review + merge — HUMAN GATE

The release PR is reviewed and merged to `main`. The packaging receipts from
step 6 (recorded in
`openspec/changes/release-crates-io-publish/apply-progress.md`) must show only
passes.

## 6. Package + dry-run (pre-publish) — STOP ON FAILURE

From the clean merged checkout, in dependency order:

```bash
cargo package -p oxdoc-core                    # full verify
cargo package -p oxdoc-core --list             # inspect: no tests, fixtures,
                                               # or workspace-only files
cargo publish -p oxdoc-core --dry-run          # full verification
```

### Resolution caveat and retry policy

`oxdoc-tabular` and `oxdoc-cli` cannot be packaged or dry-run before their
upstream crate exists on the registry. Packaging for upload replaces the
intra-workspace path dependency with its registry requirement, so resolution
fails with `failed to select a version for the requirement oxdoc-core =
"^2.0.0"` (observed even with `--no-verify` and `--offline`; only the `--list`
file-list inspection, which skips the upload-resolution step, works
pre-publish).

Pre-publish validation for those two crates is therefore:

- the `cargo package -p <crate> --list` file-list inspection (works without
  registry resolution), and
- the **deferred full validation** immediately after the upstream publish:
  re-run `cargo package -p oxdoc-tabular --no-verify` (and `--list`), then the
  full `cargo publish --dry-run` for each crate immediately after its upstream
  is live.

After a publish, the sparse index may lag: if a dry-run fails with a resolution
error for a version that was just published, **wait and re-run — never skip a
dry-run**. A skipped dry-run is a failed release.

## 7. Tag + push

```bash
git tag -a v2.0.0 -m "oxdoc 2.0.0"   # on the merged release commit
git push origin v2.0.0               # triggers release.yml
```

**Ordering consequence:** the tag goes out *before* the crates are published,
so the GitHub Release — which advertises `cargo install oxdoc-cli` — can be
public before crates.io resolves the crates. The publish sequence follows
immediately in the same session.

## 8. Confirm + publish `oxdoc-core` — HUMAN GATE

Re-confirm the approved triple (2.0.0 / 0.2.0 / 2.0.0) with the maintainer out
loud and verify the tree is still clean, then:

```bash
cargo publish -p oxdoc-core
cargo search oxdoc-core --limit 1    # expect oxdoc-core = "2.0.0"
```

## 9. Publish `oxdoc-tabular` then `oxdoc-cli` — STOP BETWEEN CRATES

```bash
cargo publish -p oxdoc-tabular --dry-run   # now resolves core 2.0.0;
                                           # on index lag, wait and re-run
cargo publish -p oxdoc-tabular
cargo search oxdoc-tabular --limit 1       # expect oxdoc-tabular = "0.2.0"

cargo publish -p oxdoc-cli --dry-run       # now resolves tabular 0.2.0;
                                           # on index lag, wait and re-run
cargo publish -p oxdoc-cli
cargo search oxdoc-cli --limit 1           # expect oxdoc-cli = "2.0.0"

# End-to-end receipt from a clean target directory:
CARGO_TARGET_DIR=$(mktemp -d) cargo install oxdoc-cli --version 2.0.0
```

Stop between crates: never publish the next crate before the previous
publish's registry check passes.

## 10. Post-publish reconciliation

Record every receipt in the "Publish receipts" section of
`openspec/changes/release-crates-io-publish/apply-progress.md`, then flip the
documentation traffic light: return crates.io to Published Channels in
[Discoverability](discoverability.md), restore the README crates.io badge,
flip the Phase 5 roadmap bullet in both roadmap files, and remove the
`https://crates.io/crates/oxdoc-cli` ignore in `.markdown-link-check.json`
only after the crate page resolves (`make docs-links` must stay green).

## Recovery (yank-only)

Published versions are immutable. A wrong release is fixed forward with a
patch version (2.0.1 / 0.2.1 — which re-cascades the internal requirement
bumps and repeats this checklist). `cargo yank --version <v> -p <crate>` is
the documented last resort: it hides the version from new resolution without
deleting it (existing lockfiles keep working; the tarball stays public). Do
not re-publish the same version — the `crate version already exists` failure
is permanent.

## CI publishing (future prerequisite — explicitly deferred)

crates.io has no OIDC trusted-publishing flow for cargo (unlike PyPI's
`id-token: write`). A future CI job would need a `CARGO_REGISTRY_TOKEN`
repository secret, a `workflow_dispatch` approved-version input asserted
against all three crate manifests (modeled on
`.github/workflows/publish-python.yml`), dependency-ordered jobs with
index-lag waits, and a documented token rotation/revocation policy. This is
explicitly deferred; the real publish stays a local, human-gated operation.
