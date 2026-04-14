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

Planned release artifacts:

- Linux GNU x86_64.
- Linux musl x86_64.
- macOS x86_64 and arm64.
- Windows x86_64.

## Changelog

Release notes should include:

- Added features.
- Fixed parser behavior.
- Breaking changes.
- Performance notes.
- Security fixes.
- Known limitations.
