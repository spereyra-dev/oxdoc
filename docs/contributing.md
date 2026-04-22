# Contributing

Contributions are welcome. The most useful contributions at this stage are focused parser fixes, tests, fixtures with clear provenance, and documentation improvements. Public contributions should be small enough to review carefully and complete enough to prove the behavior.

## Before You Start

- Read [Architecture](architecture.md).
- Check [Roadmap](roadmap.md).
- Open an issue first for parser semantics, CLI flags, public API changes, release/distribution work, or broad refactors.
- Do not attach private or sensitive Office files.
- Add fixture provenance for every checked-in package tree or binary fixture.

## Contribution Protocol

Use this process for non-trivial changes:

1. Start from an issue with acceptance criteria. If the issue is missing details, ask before building a large patch.
2. Keep the PR focused on one behavior, one bug, or one documentation topic.
3. Add tests for the changed behavior. Parser changes should cover valid input, malformed input, and edge cases when those paths are affected.
4. Update docs when users, integrators, release consumers, or future contributors need to know about the change.
5. Run the local gate and paste the exact commands in the PR.
6. Wait for CI. External contributions require maintainer review before merge.

`main` is protected. Normal changes land through pull requests after the required GitHub checks pass:

- `rust`
- `validate`

Maintainer emergency bypasses should be rare, documented in a public issue or follow-up PR, and cleaned up with the same tests expected from any other change.

## Starter Issues

Good first issues are intentionally small and labeled [`good first issue`](https://github.com/spereyra-dev/oxdoc/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22). Useful starter areas include:

| Area | Example issue | Useful files |
| --- | --- | --- |
| DOCX malformed XML | Parser warning regressions | `crates/oxdoc-core/src/parsers/docx.rs`, `docs/errors-and-warnings.md` |
| XLSX CSV edge cases | Sparse rows, quoting, sheet selection | `crates/oxdoc-core/src/parsers/xlsx.rs`, `docs/formats/xlsx.md` |
| Metadata extraction | Small producer-specific fixtures | `crates/oxdoc-core/src/parsers/metadata.rs`, `docs/formats/metadata.md` |
| CLI docs examples | Reproducible command examples | `docs/cli.md`, `docs/getting-started.md` |
| Safe fixture provenance | Clear source and redistribution notes | `tests/fixtures/README.md`, `tests/fixtures/provenance/` |

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
- Release/distribution changes update [Release Process](release-process.md).
- Large files and private documents are not committed.
- Fixture changes include provenance and redistribution notes.
- Security-sensitive findings are reported privately, not opened as public issues.

## Review and Merge Rules

- CI must be green before merge.
- External contributions need maintainer review.
- Review conversations should be resolved before merge.
- Maintainers should not use direct pushes to `main` for normal work.
- If a change affects warnings, errors, output stability, memory limits, or release artifacts, describe the risk in the PR.

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
