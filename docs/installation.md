# Installation

`oxdoc` is currently built from source. Binary releases and crates.io publication are planned after the MVP is hardened.

## Requirements

- Rust toolchain compatible with the repository `rust-version`.
- A C toolchain is normally not required for the current dependency set on common targets.
- Optional: Node.js if you want to serve the Docsify site locally.

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

## Install Locally With Cargo

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
