# Contributing

Contributions are welcome. The most useful contributions at this stage are focused parser fixes, tests, fixtures with clear provenance, and documentation improvements.

## Before You Start

- Read [Architecture](architecture.md).
- Check [Roadmap](roadmap.md).
- Open an issue first for large features or behavior changes.
- Do not attach private or sensitive Office files.

## Starter Issues

Good first issues are intentionally small and labeled [`good first issue`](https://github.com/spereyra-dev/oxdoc/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22). Current starter areas include:

| Area | Example issue | Useful files |
| --- | --- | --- |
| DOCX malformed XML | [#36](https://github.com/spereyra-dev/oxdoc/issues/36) | `crates/oxdoc-core/src/parsers/docx.rs`, `docs/errors-and-warnings.md` |
| XLSX CSV edge cases | [#37](https://github.com/spereyra-dev/oxdoc/issues/37) | `crates/oxdoc-core/src/parsers/xlsx.rs`, `docs/formats/xlsx.md` |
| Metadata fixtures | [#38](https://github.com/spereyra-dev/oxdoc/issues/38) | `crates/oxdoc-core/src/parsers/metadata.rs`, `docs/formats/metadata.md` |
| CLI docs examples | [#39](https://github.com/spereyra-dev/oxdoc/issues/39) | `docs/cli.md`, `docs/getting-started.md` |
| Safe fixture provenance | [#40](https://github.com/spereyra-dev/oxdoc/issues/40) | `tests/fixtures/README.md`, `tests/fixtures/provenance/` |

Pick one issue, keep the PR scoped to that acceptance criteria, and include the exact check command you ran.

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
