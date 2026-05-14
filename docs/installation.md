# Installation

`oxdoc` can be installed from GitHub Release binaries or built from source.

## Requirements

- `curl`, `tar`, and `shasum` or `sha256sum` for the shell installer.
- Rust toolchain compatible with the repository `rust-version` when building from source.
- A C toolchain is normally not required for the current dependency set on common targets.
- Optional: Node.js if you want to serve the Docsify site locally.

## Install Script

The installer detects macOS/Linux architecture, downloads the matching GitHub
Release archive, verifies `SHA256SUMS`, and installs `oxdoc` into
`$HOME/.local/bin` by default.

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | sh
```

Install a specific release:

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | OXDOC_VERSION=v1.0.0 sh
```

Choose a different install directory:

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | OXDOC_INSTALL_DIR=/usr/local/bin sh
```

Review `install.sh` before piping it to `sh` in locked-down environments.

## GitHub Release Archives

Download the archive for your platform from GitHub Releases:

- `oxdoc-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `oxdoc-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `oxdoc-vX.Y.Z-x86_64-pc-windows-msvc.zip`

Verify checksums with the attached `SHA256SUMS` file:

```bash
shasum -a 256 -c SHA256SUMS
```

On Linux, `sha256sum -c SHA256SUMS` works too.

## Homebrew

The recommended Homebrew path is a tap. Once `spereyra-dev/homebrew-tap` is
published, users install with:

```bash
brew tap spereyra-dev/tap
brew install oxdoc
```

Maintainers can render the formula for the tap after publishing a release:

```bash
scripts/render-homebrew-formula.sh v1.0.0 <source-tarball-sha256> > Formula/oxdoc.rb
```

The formula builds from the tagged source with Cargo, which is the usual
Homebrew path for Rust CLIs.

## Cargo

After crates.io publication:

```bash
cargo install oxdoc-cli
```

The installed binary is named `oxdoc`.

## Build From Source

```bash
git clone https://github.com/spereyra-dev/oxdoc.git
cd oxdoc
cargo build --release --workspace
```

The release binary is at:

```text
target/release/oxdoc
```

On Windows:

```text
target/release/oxdoc.exe
```

## Install Locally From This Checkout

From the repository root:

```bash
cargo install --path crates/oxdoc-cli
```

This installs the `oxdoc` binary into Cargo's bin directory.

## Static Linux Build

Install the musl target:

```bash
rustup target add x86_64-unknown-linux-musl
```

Build:

```bash
make musl
```

Equivalent Cargo command:

```bash
cargo build --workspace --release --target x86_64-unknown-linux-musl
```

## Documentation Site

Serve the Docsify documentation locally:

```bash
npx docsify-cli@4 serve docs --port 3000
```

Then open:

```text
http://localhost:3000
```

The repository includes a `docs` GitHub Actions workflow that validates the Docsify site, checks internal Markdown links, and confirms the published schema copies match `schemas/v1/`. Publishing the site through GitHub Pages requires enabling Pages for the repository in GitHub settings.
