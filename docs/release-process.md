# Release Process

`oxdoc` has not published a versioned release yet. This page documents the intended release process.

## Pre-Release Checklist

- All CI checks pass on `main`.
- `CHANGELOG.md` has a release entry.
- CLI help and docs are updated.
- Version numbers are updated consistently.
- Release artifacts are built from a clean checkout.

## Versioning

Before 1.0, breaking changes may happen in minor releases. The project should still document them clearly.

After 1.0, use semantic versioning:

- Patch: bug fixes with compatible behavior.
- Minor: compatible new functionality.
- Major: breaking CLI or API changes.

## Crates.io Publishing

The intended crates.io plan is to publish both crates once the pre-1.0 API and CLI contracts are ready for external consumers:

- Publish `oxdoc-core` first because it is the library crate.
- Publish `oxdoc-cli` second because it depends on the matching `oxdoc-core` version.
- Keep the CLI binary name as `oxdoc`, even though the crate package is `oxdoc-cli`.
- Keep crate READMEs focused on package-specific installation and API/CLI usage.

Before publishing, run dry-runs from a clean checkout:

```bash
cargo publish -p oxdoc-core --dry-run
cargo publish -p oxdoc-cli --dry-run
```

If `oxdoc-core` has not been published yet, the `oxdoc-cli` dry-run may fail registry resolution during verification. In that case, publish `oxdoc-core` first, then rerun the `oxdoc-cli` dry-run before publishing the CLI crate.

Both crates must keep:

- MIT licensing through package metadata.
- A crate-local `README.md`.
- Package include lists that avoid publishing workspace-only integration fixtures.
- Version numbers aligned when the CLI depends on a new core API.
- Release notes that call out breaking API or CLI behavior before 1.0.

## Binary Artifacts

GitHub Releases are published by the manual `release` workflow. Maintainers should create and push the intended tag first, then run the workflow with that tag name. The workflow defaults to draft prereleases so artifacts can be inspected before they are made public.

Release artifacts:

- `oxdoc-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `oxdoc-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-pc-windows-msvc.zip`

Each archive contains the `oxdoc` binary, `README.md`, and `LICENSE`. The Windows archive contains `oxdoc.exe`.

## Checksums And Signing

The release workflow generates a `SHA256SUMS` file in the publish job and attaches it to the GitHub Release next to the archives. Verify downloads with:

```bash
sha256sum -c SHA256SUMS
```

On macOS, use:

```bash
shasum -a 256 -c SHA256SUMS
```

Checksums provide integrity checks for downloaded artifacts. They are not a cryptographic signature of maintainer identity. The project does not currently have a dedicated signing key or Sigstore policy, so release signing is intentionally deferred until maintainers can document key ownership, rotation, and verification steps. When signing is added, keep `SHA256SUMS` for compatibility and attach detached signatures or provenance files alongside it.

## Changelog

Release notes should include:

- Added features.
- Fixed parser behavior.
- Breaking changes.
- Performance notes.
- Security fixes.
- Known limitations.
