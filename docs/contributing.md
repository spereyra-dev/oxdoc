# Contributing

Contributions are welcome. The most useful contributions at this stage are focused parser fixes, tests, fixtures with clear provenance, and documentation improvements.

## Before You Start

- Read [Architecture](architecture.md).
- Check [Roadmap](roadmap.md).
- Open an issue first for large features or behavior changes.
- Do not attach private or sensitive Office files.

## Local Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Or:

```bash
make all
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
