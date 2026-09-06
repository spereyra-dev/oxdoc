# Distribution Discoverability

This page keeps the project’s public installation and discovery surfaces
accurate. Only add a README badge after its destination is public and has been
checked.

## Published Channels

| Channel | User link | Install or use |
| --- | --- | --- |
| GitHub Releases | [Latest release](https://github.com/spereyra-dev/oxdoc/releases/latest) | `curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh \| sh` on macOS/Linux, or download an archive for any supported platform. |
| crates.io CLI | [`oxdoc-cli`](https://crates.io/crates/oxdoc-cli) | `cargo install oxdoc-cli` |
| Documentation | [GitHub Pages](https://spereyra-dev.github.io/oxdoc/) | Browse the guides and reference. |
| Continuous integration | [CI workflow](https://github.com/spereyra-dev/oxdoc/actions/workflows/ci.yml) | Review the checks run for `main` and pull requests. |

The README links to these channels with badges. The release and crates.io
badges report their upstream versions; the documentation badge identifies the
published documentation site; and the CI badge reports the `main` workflow
state. Do not treat badges as support, security, download-count, or performance
claims.

## Channels Not Yet Published

Do not add PyPI or Homebrew badges or installation commands until their public
pages resolve and a real user can install from them.

### PyPI

The Python wrapper is source-only until the `oxdoc-python` package is published.
Before adding a PyPI badge or `pip install` instructions:

1. Build and validate the package from `python/`.
2. Publish `oxdoc-python` to PyPI.
3. Confirm the PyPI project page and the package install work.
4. Add a badge linked to that project page and document any requirement for the
   `oxdoc` binary.

### Homebrew

The `spereyra-dev/homebrew-tap` repository and formula are not public yet.
After publishing a GitHub Release:

1. Render the formula with `scripts/render-homebrew-formula.sh <tag> <source-tarball-sha256>`.
2. Commit it to the public tap as `Formula/oxdoc.rb`.
3. Verify `brew tap spereyra-dev/tap` and `brew install oxdoc` on a clean
   machine.
4. Confirm the tap URL resolves, then add the README badge and installation
   link.

## GitHub Repository Metadata

Keep the repository description focused on the evidenced scope: a fast Rust
CLI and library for extracting text, CSV, and metadata from Office Open XML
files. Set the homepage to the published documentation URL.

Current topics are: `rust`, `cli`, `docx`, `xlsx`, `pptx`, `ooxml`, `office`,
`csv`, and `metadata`. Review them when the supported formats or public APIs
change; do not add topics for planned channels or features.

GitHub social preview images are configured in **Settings → General → Social
preview** and cannot be versioned in this repository. Use a 1280×640 image that
contains the project name and the factual scope “OOXML text, CSV, and metadata
extraction”; avoid version numbers, benchmark results, availability claims, or
logos without permission. After uploading, check the repository link in a
social-card debugger and replace the image if the claim becomes stale.

## Release-Time Validation

Before merging a discoverability change or cutting a release:

1. Check every README badge target and every channel URL with an HTTP request.
2. Confirm a GitHub Release is non-draft and its archives and `SHA256SUMS` are
   attached.
3. Confirm the latest `oxdoc-cli` crates.io version matches the release when a
   crate release is intended.
4. Run `make docs-links` to validate Markdown links.
5. Run `make docs-check` to verify the documentation site serves locally.

Record unpublished services here instead of presenting placeholder badges.
