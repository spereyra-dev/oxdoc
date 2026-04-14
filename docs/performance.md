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
| ZIP access | Reads targeted entries through `zip::ZipArchive`. |
| XML parsing | Uses `quick-xml` event readers. |
| DOCX text | Builds a `String` result for the library API. |
| XLSX CSV | Streams worksheet output to a caller-provided writer. |
| Shared strings | Loaded into memory for MVP. |

## Known MVP Tradeoff

XLSX shared strings are currently loaded into memory. This is correct for the first implementation but not the final large-file strategy.

Future options:

- Disk-backed temporary store.
- Indexed shared-string offsets.
- Adaptive in-memory store with a size threshold.
- Separate API for high-volume streaming use cases.

## Benchmarking Plan

Benchmarks should cover:

- CLI cold start.
- DOCX throughput by input size.
- XLSX throughput by row count and shared-string count.
- Peak memory on large worksheets.
- Corrupt-input behavior.

Benchmark results should be tracked in release notes once a baseline exists.
