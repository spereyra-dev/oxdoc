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
| PPTX text | Builds a `String` result from slide and notes text parts. |
| XLSX CSV | Streams worksheet output to a caller-provided writer. |
| Shared strings | Kept in memory up to an internal threshold, then spilled to temporary data and index files. |

Default ZIP part guardrails:

- Maximum uncompressed required part size: 64 MiB.
- Compression ratio check: 200:1 once a part is at least 4 MiB uncompressed.
- Encrypted required parts are rejected because `oxdoc` does not decrypt password-protected documents.

## Bounded Memory Contract

For XLSX CSV extraction, bounded shared-string memory means memory should not grow with the full shared-string table after spill-to-disk is active. It can still grow with workbook metadata, the configured in-memory shared-string threshold, the largest single shared string currently being parsed, the current row width, the caller's output writer, and ZIP library bookkeeping.

Shared-string temporary files are created in the OS temporary directory and removed on success or error. Disk use can grow with spilled shared-string data and the fixed-width lookup index.

Default ZIP part guardrails still apply. Large shared-string tables must fit within the configured OOXML part limits and pass compression-ratio checks; bounded memory is not a bypass for suspicious or oversized ZIP input.

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
| `xlsx_shared_string_throughput` | End-to-end `extract_xlsx_csv_from_reader` over shared-string-heavy worksheets below and above the spill threshold. | CSV bytes emitted. |

The DOCX benchmark includes ZIP package opening, relationship fallback, XML
event parsing, and the returned `String` allocation. The XLSX benchmark includes
ZIP package opening, workbook and workbook relationship parsing, worksheet XML
event parsing, shared-string lookup, CSV escaping, and writing to a `Vec<u8>`.

### Benchmark Limits

These benchmarks are intended as a stable starting baseline, not a complete
performance model. They do not yet measure CLI cold start, malformed-input
latency, or peak resident memory. Criterion reports timing and throughput, but
it does not report peak memory; use an external profiler or platform tool when
validating large-file memory behavior.

Future benchmarks should cover:

- CLI cold start.
- DOCX throughput by input size.
- XLSX throughput by row count, shared-string count, and very wide rows.
- Peak memory on large worksheets.
- Corrupt-input behavior.

Benchmark results should be tracked in release notes once a baseline exists.
