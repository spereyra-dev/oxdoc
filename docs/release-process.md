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

## Binary Artifacts

Planned release artifacts:

- Linux GNU x86_64.
- Linux musl x86_64.
- macOS x86_64 and arm64.
- Windows x86_64.

## Crates.io

Publishing `oxdoc-core` and `oxdoc-cli` to crates.io should wait until the public API and CLI behavior are less volatile.

## Changelog

Release notes should include:

- Added features.
- Fixed parser behavior.
- Breaking changes.
- Performance notes.
- Security fixes.
- Known limitations.
