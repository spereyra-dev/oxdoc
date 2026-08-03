# Cross-format Batch Manifest Decision

Issue: [#126](https://github.com/spereyra-dev/oxdoc/issues/126)

## Decision

**NO-GO for a generic `oxdoc batch` command and persistent cross-format
manifest.** The existing operation-specific JSONL contracts are the stable
integration boundary. A new generic manifest would duplicate them while
mixing inventory, audit, extraction, and orchestration semantics.

This decision can be revisited only when a concrete consumer requires a
durable run ledger spanning more than one operation.

## Contract decisions

- Inventory and audit use `oxdoc audit FILES... --format jsonl`.
- Text ingestion uses `oxdoc extract text FILES... --format jsonl`.
- Worksheet exports retain their operation-specific `manifest.json`.
- Per-file failures remain records in JSONL and do not fail an otherwise
  completed batch. A command fails when no input succeeds or setup fails.
- SHA-256 stays outside stable output. It adds a second full-file read and is
  best computed by an orchestrator only when deduplication or provenance
  requires it.
- Durations stay outside deterministic schemas. Operational telemetry belongs
  in the runner, where clocks, retries, and concurrency have defined meaning.
- Stdin retains the identity `-`; size and hash are unavailable unless a caller
  explicitly buffers the stream.
- Document-specific counts remain inside operation-specific payloads rather
  than being flattened into a weak common record.

## Hash cost benchmark

Run the reproducible standard-library benchmark with:

```bash
python3 scripts/batch-manifest-spike.py --files 256 --file-size 262144 --iterations 5
```

The script creates a deterministic 64 MiB temporary corpus, compares metadata
inventory with full SHA-256 reads, and emits machine-readable JSON. Results are
environment-dependent and deliberately reported rather than enforced: hashing
cost scales with total bytes, while metadata inventory scales mainly with file
count.

Reference run on August 3, 2026 (macOS 26.5.2 arm64, Python 3.12.9):

| Measurement | Result |
| --- | ---: |
| Files | 256 |
| Bytes per file | 262,144 |
| Total corpus | 67,108,864 bytes |
| Median metadata inventory | 0.000567 s |
| Median inventory with SHA-256 | 0.037901 s |
| Incremental hashing time | 0.037333 s |
| Effective SHA-256 throughput | 1,688.62 MiB/s |

The numbers are directional rather than a performance gate. Storage speed,
filesystem cache state, CPU implementation, and corpus shape affect absolute
timings; the invariant is that hashing requires reading every byte.

The benchmark supports the no-go decision: hashes are useful optional
orchestration metadata, but charging every batch operation for a second read
would violate the principle that extra full-file work must be explicit.

## Revisit criteria

Open a new design issue only with all of the following:

1. a named consumer that cannot use the existing JSONL contracts;
2. a precise operation and partial-failure policy;
3. a versioned schema proposal that excludes nondeterministic fields by
   default;
4. measurements for hashing, stdin buffering, and large file counts;
5. separate implementation issues for core models, CLI behavior, schemas,
   tests, and documentation.
