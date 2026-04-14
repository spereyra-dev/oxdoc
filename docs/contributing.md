# Contributing

Contributions are welcome. The most useful contributions at this stage are focused parser fixes, tests, fixtures with clear provenance, and documentation improvements.

## Before You Start

- Read [Architecture](architecture.md).
- Check [Roadmap](roadmap.md).
- Open an issue first for large features or behavior changes.
- Do not attach private or sensitive Office files.

## Local Checks

The closest local equivalent to the GitHub Actions Rust job is:

```bash
make ci-rust
```

The full local gate adds docs validation and a release build:

```bash
make ci
```

Install optional local tools when needed:

```bash
make install-tools
cargo install cargo-audit --locked
```

If `make` is not available, run the Rust gates directly:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features --all-targets
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features --all-targets
cargo test --doc --workspace --all-features
cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only
```

Docs can be checked separately:

```bash
npx docsify-cli@4 serve docs --port 3000
```

## Pull Request Checklist

- The change is scoped to one problem.
- Tests cover changed parser behavior.
- Warnings remain recoverable where possible.
- Public API changes are documented.
- CLI changes update [CLI Reference](cli.md).
- Large files and private documents are not committed.

## Documentation Changes

Documentation lives in:

```text
docs/
```

Serve it locally:

```bash
npx docsify-cli@4 serve docs --port 3000
```

Root-level project docs such as `ROADMAP.md` and `SECURITY.md` should stay aligned with the Docsify pages.

## Commit Messages

Use short imperative messages:

```text
add xlsx inline string parsing
fix docx paragraph break handling
document metadata output fields
```
