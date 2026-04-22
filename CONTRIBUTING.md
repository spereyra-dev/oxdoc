# Contributing

Thanks for helping improve `oxdoc`. The project is early, so small focused changes are easier to review than broad rewrites. The standard is simple: every change should be understandable, tested, and safe to publish.

## Ground Rules

- Keep `oxdoc` focused on extraction, not rendering.
- Prefer streaming and bounded-memory designs.
- Keep warnings recoverable and errors explicit.
- Add tests for parser behavior, especially when fixing malformed OOXML input.
- Do not add sample documents unless they are safe to redistribute.

## Contribution Protocol

1. Open an issue first for parser semantics, CLI flags, public API changes, release/distribution work, or broad refactors. Tiny documentation fixes and typo fixes can go straight to a PR.
2. Keep each PR scoped to one problem and describe the expected behavior before the implementation details.
3. Add or update tests for changed behavior. Parser fixes should include malformed-input and edge-case coverage when relevant.
4. Update documentation for user-facing behavior, warning/error contracts, installation changes, or release changes.
5. Run the local gate before requesting review and paste the exact commands in the PR.
6. Wait for required CI checks before merge. External contributions require maintainer review.

`main` is a protected branch. Changes should land through pull requests, not direct pushes. Required checks are the Rust gate (`rust`) and docs gate (`validate`).

## Development Setup

Install a current Rust toolchain and run:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features --all-targets
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features --all-targets
```

The Makefile wraps the same checks:

```bash
make ci
```

## Starter Issues

Issues labeled [`good first issue`](https://github.com/spereyra-dev/oxdoc/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22) are scoped for focused first PRs. Good starter areas include parser warning regressions, CSV edge cases, metadata fixtures, CLI docs examples, and safe fixture provenance.

Start with the issue's acceptance criteria and keep the PR limited to that slice.

## Pull Requests

- Link the issue or explain why no issue is needed.
- Include a clear summary, risk notes, and the checks you ran.
- Keep generated files, large binaries, and private documents out of the repo.
- Mark breaking changes, public API changes, or release/distribution changes clearly.
- Use draft PRs for early feedback when the design is still moving.

## Fixtures

Future fixture files should include a short note explaining:

- Which tool generated the file.
- Whether it is safe to redistribute.
- What behavior the fixture is meant to cover.
- Whether it was sanitized, minimized, or generated from repository-authored content.

Never commit private, customer, confidential, or secret-bearing documents. Prefer a minimal reproduction generated specifically for this repository.

## Commit Style

Use short imperative commit messages, for example:

```text
add xlsx inline string parsing
fix docx paragraph break handling
```
