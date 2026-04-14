# Contributing

Thanks for helping improve `oxdoc`. The project is early, so small focused changes are easier to review than broad rewrites.

## Ground Rules

- Keep `oxdoc` focused on extraction, not rendering.
- Prefer streaming and bounded-memory designs.
- Keep warnings recoverable and errors explicit.
- Add tests for parser behavior, especially when fixing malformed OOXML input.
- Do not add sample documents unless they are safe to redistribute.

## Development Setup

Install a current Rust toolchain and run:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

The Makefile wraps the same checks:

```bash
make all
```

## Starter Issues

Issues labeled [`good first issue`](https://github.com/spereyra-dev/oxdoc/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22) are scoped for focused first PRs. Good starter areas include parser warning regressions, CSV edge cases, metadata fixtures, CLI docs examples, and safe fixture provenance.

Start with the issue's acceptance criteria and keep the PR limited to that slice.

## Pull Requests

- Open an issue first for large features or behavior changes.
- Keep PRs scoped to one problem.
- Include a clear summary and the checks you ran.
- Add or update tests when behavior changes.
- Keep generated files, large binaries, and private documents out of the repo.

## Fixtures

Future fixture files should include a short note explaining:

- Which tool generated the file.
- Whether it is safe to redistribute.
- What behavior the fixture is meant to cover.

## Commit Style

Use short imperative commit messages, for example:

```text
add xlsx inline string parsing
fix docx paragraph break handling
```
