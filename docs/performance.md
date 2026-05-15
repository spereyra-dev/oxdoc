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

## Throughput Benchmarks

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

## Peak Memory Benchmarks

Peak-memory baselines are generated with [`scripts/peak-memory-baselines.py`](../scripts/peak-memory-baselines.py). The script builds the release CLI, creates synthetic OOXML files in a temporary directory, runs each extraction under `/usr/bin/time`, and records peak resident set size.

Run the workflow with:

```bash
make memory-baselines
```

For a quicker local smoke run, use one sample:

```bash
python3 scripts/peak-memory-baselines.py --iterations 1 --output docs/performance-memory-baselines.md
```

The workflow supports macOS and Linux:

- macOS uses `/usr/bin/time -l` and reports `maximum resident set size` in bytes.
- Linux uses `/usr/bin/time -v` and reports `Maximum resident set size (kbytes)`.
- The committed baseline records platform, architecture, Python, Rust, binary path, sample count, fixture size, output size, and peak RSS MiB.

Current cases:

| Case | Covers |
| --- | --- |
| `docx-text-256kb` | DOCX text extraction over a synthetic document with 256 KiB extracted text. |
| `pptx-text-256kb` | PPTX slide text extraction over 256 synthetic slides. |
| `xlsx-shared-strings-spill` | Shared-string-heavy XLSX extraction past the spill-to-disk threshold. |
| `xlsx-wide-sparse` | XLSX rows with sparse far-right cells that force row padding. |

Published baseline numbers live in [Peak Memory Baselines](performance-memory-baselines.md).

### Regression Guidance

Use at least three samples (`--iterations 3`) before treating a memory change as a regression. Compare the median `Peak RSS MiB` for the same machine class, OS, Rust version, and fixture sizes.

Investigate changes when a case grows by more than 20% or by more than 8 MiB, whichever is larger. Expected reasons for an increase include larger fixture parameters, new output buffering, higher shared-string memory thresholds, additional workbook metadata retained in memory, or ZIP library behavior changes.

Do not compare macOS and Linux numbers directly; their RSS accounting differs. Treat each platform as its own baseline family.

### Remaining Gaps

These benchmarks are a stable starting baseline, not a complete performance model. Future work should cover:

- CLI cold start.
- Corrupt-input behavior.
- Larger real-world fixture classes once redistribution rights are clear.

Benchmark results should be tracked in release notes once a baseline exists.
