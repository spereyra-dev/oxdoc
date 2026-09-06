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

The corpus is source-controlled OOXML:

- `tests/fixtures/corpus/` contains minimal `.docx`, `.xlsx`, and `.pptx` package trees.
- `tests/fixtures/files/` contains small application-generated `.docx`, `.xlsx`, and `.pptx` binaries that are consumed as-is.
- `tests/fixtures/provenance/` documents source and redistribution status for each fixture.
- `tests/fixtures/snapshots/` stores the expected text, CSV, and JSON outputs.
- `tests/fixtures/tools/` stores optional generator scripts. CI consumes the checked-in fixtures and does not require these tools.

The current corpus includes hand-authored package trees plus producer-generated fixtures from python-docx, openpyxl, and python-pptx. Every fixture must be generated from repository-authored content or another legally redistributable source, and every fixture needs provenance that states the producer, redistribution status, purpose, and sanitization. The [producer compatibility corpus](compatibility-corpus.md) defines the fixture policy and distinguishes covered producers from planned coverage.

Do not commit private Office files. Microsoft Office, LibreOffice, and Google Workspace exports are welcome only when the content was created for this repository and the provenance note makes redistribution status explicit.

Validate the machine-readable application fixture matrix with:

```bash
make compatibility-corpus-check
```

## Snapshot Tests

Snapshot tests verify that command output does not change accidentally after parser refactors.

This repository uses versioned text snapshots instead of an extra snapshot dependency. The tests read expected output from `tests/fixtures/snapshots/` and compare it directly in CI for both core parser APIs and CLI output.

## Fuzzing

Fuzzing hardens the XML parser entry points that process untrusted OOXML parts.
The `fuzz/` cargo-fuzz project currently covers DOCX document text, PPTX slide
text, XLSX shared strings and worksheets, relationships, and metadata.

```bash
cargo +nightly install cargo-fuzz --locked
cd fuzz
cargo +nightly fuzz run xlsx_sheet -- -max_total_time=300 -timeout=10 -rss_limit_mb=1024
```

See [`fuzz/README.md`](../fuzz/README.md) for the complete target list, bounded
local commands, minimization, and reproduction commands.

The `fuzz` workflow compiles every harness on pull requests that change the
fuzzing surface. It runs every target for five minutes every Monday and may be
started manually with a 1--900 second budget. This keeps the normal Rust and
docs contributor gates fast while continuously checking parser behavior. Each
run has a wall-clock limit, a 1 GiB memory cap, a 10-second per-input timeout,
and a 1 MiB input limit. Crash artifacts are retained only on failed workflow
runs for 14 days.

Fuzz failures must become a focused unit, API, or fixture regression test when
the behavior can be expressed deterministically. Retain the minimized input in
`fuzz/regressions/<target>/<sha256>` only when it adds value beyond that test.
Never commit raw corpus or crash artifacts, private Office files, secrets, or
personal data. Minimize with `cargo fuzz tmin`, inspect and sanitize the result,
then document the fixing issue or advisory and reproduction command in the PR.
The scheduled workflow replays retained inputs as an additional corpus.

## Performance Workbenches

Parser throughput benchmarks live in `crates/oxdoc-core/benches/throughput.rs`
and run with:

```bash
cargo bench -p oxdoc-core
```

Peak-memory baselines use synthetic fixtures and `/usr/bin/time`:

```bash
make memory-baselines
```

The optional competitive workbench compares the release CLI with local tools
such as Apache Tika, `xlsx2csv`, and Mammoth when they are installed:

```bash
make competitor-workbench
```

Competitive results are not part of the merge gate because external tool
availability and versions vary by machine.

The tabular production gate is reproducible and enforced separately:

```bash
make tabular-ci
```

It generates the XLSX edge-case corpus, enforces Arrow/Parquet throughput
ratios, and validates Parquet through pinned DuckDB and PyArrow readers. The
dedicated weekly and path-filtered `tabular` workflow also reports peak RSS,
build size, output size, and row-group counts at two corpus sizes.

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
