# Release Process

This page documents the release process for GitHub Release binaries, the shell
installer, Homebrew tap updates, and future crates.io publishing.

## Pre-Release Checklist

- All CI checks pass on `main`.
- `CHANGELOG.md` has a release entry.
- CLI help and docs are updated.
- Version numbers are updated consistently.
- Release artifacts are built from a clean checkout.
- `install.sh` and Homebrew formula rendering tests pass.

## Versioning

Before 1.0, breaking changes may happen in minor releases. The project should still document them clearly.

After 1.0, use semantic versioning:

- Patch: bug fixes with compatible behavior.
- Minor: compatible new functionality.
- Major: breaking CLI or API changes.

## Shell Installer

`install.sh` is a small Unix installer for GitHub Release assets. It:

- detects supported macOS/Linux targets,
- downloads the matching `tar.gz` archive,
- downloads `SHA256SUMS`,
- verifies the archive checksum,
- installs `oxdoc` into `$HOME/.local/bin` unless `OXDOC_INSTALL_DIR` is set.

Run its offline test harness before release changes:

```bash
make scripts-test
```

The installer supports these maintainer/test overrides:

- `OXDOC_VERSION`, for example `v1.0.0` or `1.0.0`.
- `OXDOC_TARGET`, for explicit target asset selection.
- `OXDOC_INSTALL_DIR`, for destination directory.
- `OXDOC_REPO`, for forks.
- `OXDOC_DOWNLOAD_BASE`, for tests or mirrors that already contain release assets.

## Homebrew Tap

The project keeps Homebrew tap generation in-repo but does not commit tap
formula output here. After a GitHub Release is public:

1. Download the source tarball checksum from GitHub or compute it locally.
2. Render the formula:

   ```bash
   scripts/render-homebrew-formula.sh v1.0.0 <source-tarball-sha256> > Formula/oxdoc.rb
   ```

3. Copy or commit `Formula/oxdoc.rb` into `spereyra-dev/homebrew-tap`.
4. Run `brew test oxdoc` in the tap before publishing the tap update.

The formula builds from tagged source with Cargo. This keeps the tap aligned
with Homebrew conventions and avoids shipping platform-specific bottles before
the project has enough release volume to maintain them.

## Crates.io Publishing

The intended crates.io plan is to publish both crates once the 1.0 API and CLI contracts are ready for external consumers:

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
- Release notes that call out breaking API or CLI behavior.

## Binary Artifacts

GitHub Releases are published by the `release` workflow. Pushing a `v*` tag
publishes a non-draft release. Maintainers can also run the workflow manually
with an existing tag and choose draft/prerelease flags for inspection.

Release artifacts:

- `oxdoc-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `oxdoc-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-pc-windows-msvc.zip`

Each archive contains the `oxdoc` binary, `README.md`, `LICENSE`, and
`install.sh`. The Windows archive contains `oxdoc.exe`.

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
