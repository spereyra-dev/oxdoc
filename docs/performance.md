# Performance and Memory

Performance is a core part of `oxdoc`, not an afterthought.

## Goals

- Fast CLI cold start.
- Bounded memory for large Office files.
- Streaming XML parsing.
- Writer-based output for large extraction results.
- Static Linux builds for deployment in minimal containers and CI jobs.

## Current Implementation

| Area | Current behavior |
| --- | --- |
| ZIP access | Reads targeted entries through `zip::ZipArchive` after encrypted-part, size, and compression-ratio guardrails. |
| XML parsing | Uses `quick-xml` event readers. |
| DOCX text | Builds a `String` result for the library API. |
| XLSX CSV | Streams worksheet output to a caller-provided writer. |
| Shared strings | Loaded into memory for MVP. |

Default ZIP part guardrails:

- Maximum uncompressed required part size: 64 MiB.
- Compression ratio check: 200:1 once a part is at least 4 MiB uncompressed.
- Encrypted required parts are rejected because `oxdoc` does not decrypt password-protected documents.

## Known MVP Tradeoff

XLSX shared strings are currently loaded into memory. This is correct for the first implementation but not the final large-file strategy.

Future options:

- Disk-backed temporary store.
- Indexed shared-string offsets.
- Adaptive in-memory store with a size threshold.
- Separate API for high-volume streaming use cases.

## Benchmarking Plan

The repository now carries an initial reproducible Criterion suite for parser
throughput. Run it locally with:

```bash
cargo bench -p oxdoc-core
```

For a faster compile-only verification, use:

```bash
cargo bench -p oxdoc-core --no-run
```

The first suite lives in `crates/oxdoc-core/benches/throughput.rs` and builds
safe synthetic OOXML packages in memory for each benchmark case. No external
Office files or checked-in binary fixtures are required.

Current benchmark groups:

| Group | Measures | Throughput unit |
| --- | --- | --- |
| `docx_text_throughput` | End-to-end `extract_docx_text_from_reader` over minimal DOCX packages with generated paragraphs. | Extracted text bytes. |
| `xlsx_row_throughput` | End-to-end `extract_xlsx_csv_from_reader` over minimal XLSX packages with dense numeric worksheets. | Worksheet rows emitted as CSV. |

The DOCX benchmark includes ZIP package opening, relationship fallback, XML
event parsing, and the returned `String` allocation. The XLSX benchmark includes
ZIP package opening, workbook and workbook relationship parsing, worksheet XML
event parsing, CSV escaping, and writing to a `Vec<u8>`.

### Benchmark Limits

These benchmarks are intended as a stable starting baseline, not a complete
performance model. They do not yet measure CLI cold start, malformed-input
latency, shared-string-heavy worksheets, or peak resident memory. Criterion
reports timing and throughput, but it does not report peak memory; use an
external profiler or platform tool when validating large-file memory behavior.

Future benchmarks should cover:

- CLI cold start.
- DOCX throughput by input size.
- XLSX throughput by row count and shared-string count.
- Peak memory on large worksheets.
- Corrupt-input behavior.

Benchmark results should be tracked in release notes once a baseline exists.
