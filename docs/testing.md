# Testing Strategy

Parser correctness needs multiple test layers because OOXML files vary across producers.

## Unit Tests

Unit tests should feed focused XML snippets directly into parsers.

Current examples:

- DOCX `<w:t>` extraction.
- DOCX malformed XML partial output.
- XLSX shared strings.
- XLSX sparse CSV rows.
- Metadata fields.

## Fixture Tests

Fixture tests use a checked-in corpus under `tests/fixtures/`.

The corpus is source-controlled OOXML, not opaque binaries:

- `tests/fixtures/corpus/` contains minimal `.docx`, `.xlsx`, and `.pptx` package trees.
- `tests/fixtures/provenance/` documents source and redistribution status for each fixture.
- `tests/fixtures/snapshots/` stores the expected text, CSV, and JSON outputs.

The current corpus is hand-authored and redistributable. It is intentionally not attributed to Microsoft Office, LibreOffice, or Google Workspace outputs unless such a fixture is explicitly added later with clear provenance.

## Snapshot Tests

Snapshot tests verify that command output does not change accidentally after parser refactors.

This repository uses versioned text snapshots instead of an extra snapshot dependency. The tests read expected output from `tests/fixtures/snapshots/` and compare it directly in CI.

## Fuzzing

Fuzzing is required for parser hardening. Planned approach:

```bash
cargo install cargo-fuzz
cargo fuzz init
```

High-value fuzz targets:

- DOCX document XML parser.
- XLSX shared string parser.
- XLSX sheet parser.
- Relationship parser.
- Metadata parser.

Fuzz failures should become regression tests when possible.

## CI Checks

Current CI runs:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features --all-targets
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features --all-targets
cargo test --doc --workspace --all-features
cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only
```

The coverage gate is 95% line coverage.

## Coverage Gate

Coverage is part of the merge contract. Do not lower the 95% threshold to merge parser or CLI changes.

Run the local gate with:

```bash
make coverage
```

Use this command when a change needs gap analysis by crate or parser module:

```bash
cargo llvm-cov report --show-missing-lines
```

When coverage drops, add focused tests for the public behavior or parser branch that changed. Parser changes should usually add a unit test for the XML state machine and, when the behavior is user-visible, a CLI or public API test.
