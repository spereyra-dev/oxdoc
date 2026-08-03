# XLSX Arrow and Parquet Spike

Issue: [#121](https://github.com/spereyra-dev/oxdoc/issues/121)

## Decision

**GO for the separate-crate architecture. Production gates are complete.**

Arrow and Parquet must stay outside `oxdoc-core`. The spike uses an
`oxdoc-tabular` workspace crate with explicit schemas, bounded Arrow
`RecordBatch` production, and incremental Parquet row-group flushing.
`oxdoc-core` and the default `oxdoc` CLI do not link Arrow or Parquet.

The crate is publishable with no default features. Schema inference stays
lightweight, while the `parquet` feature opts consumers into Arrow and Parquet.
The default CLI dependency graph does not enable that feature.

## Public API

```rust
let schema = TabularSchema::new(vec![
    Column::new("id", 0, TabularType::Int64, false),
    Column::new("name", 1, TabularType::Utf8, true),
])?;

let result = write_xlsx_parquet(
    "input.xlsx",
    XlsxSheetOptions::default(),
    &schema,
    8_192,
    output,
)?;
```

The prototype provides path and `Read + Seek` variants for:

- visiting bounded Arrow record batches;
- writing Parquet with one explicitly flushed row group per batch;
- mapping zero-based worksheet columns to an explicit schema;
- rejecting incompatible values with row and column context.

Missing and explicit blank cells become Arrow nulls. Formula cells use cached
worksheet values. Temporal values require formats already recognized by the
XLSX parser.

## Measurements

Measurements were taken on macOS arm64 on June 15, 2026. Both build
measurements used empty, separate `CARGO_TARGET_DIR` directories.

| Measurement | Existing CLI | Tabular example | Delta |
| --- | ---: | ---: | ---: |
| Clean release build | 35.98 s | 56.77 s | +57.8% |
| Build peak RSS | 494,518,272 B | 946,552,832 B | +91.4% |
| Release binary | 4,267,392 B | 4,629,456 B | +8.5% |
| Target directory | 153 MiB | 199 MiB | +30.1% |

These costs are paid by consumers that build `oxdoc-tabular`; the existing CLI
dependency graph and linked binary remain unchanged.

A deterministic 100,000-row, three-column fixture measured:

| Result | Value |
| --- | ---: |
| XLSX input size | 1,698,816 B |
| Conversion wall time | 0.18 s |
| Approximate throughput | 555,556 rows/s |
| Conversion peak RSS | 10,338,304 B |
| Parquet output size | 2,534,171 B |
| Batch/row-group size | 8,192 rows |
| Batches and row groups | 13 |

The runtime result is a directional spike measurement, not a release
benchmark. The generated data is highly regular and the Parquet build uses
minimal features.

## Interoperability

The generated Parquet file was read with DuckDB 1.4.3, independently from the
Rust `parquet` crate. DuckDB verified:

- exactly 100,000 rows;
- `id` range 1 through 100,000;
- 50,000 true boolean values;
- exact string and boolean samples across a row-group boundary;
- 13 distinct row groups.

The Rust tests additionally cover all supported logical types, sparse nulls,
multiple bounded batches, multiple row groups, invalid batch sizes, and schema
mismatches.

## Schema Policy

Explicit schemas are ready for a one-pass adapter. Inferred schemas need a
two-pass workflow:

1. infer and freeze the schema;
2. reopen or seek the XLSX input;
3. convert rows under the frozen schema.

A sampled schema cannot safely mutate after Parquet writing starts. Late
conflicts now follow an explicit policy in `oxdoc-tabular`: full inference
promotes incompatible observed types to UTF-8 and inferred conversion coerces
those columns from formatted worksheet values. Explicit schemas remain strict.
With sampled inference, a conflict found after the sample fails with row and
column context unless the frozen type is already UTF-8; the schema never
mutates after Parquet writing begins.

The path and `Read + Seek` inferred writers implement the deterministic
two-pass workflow. The first pass produces the same versioned report used by
the CLI, the input is reopened or rewound, and the second pass writes bounded
batches under the frozen schema.

## Large Worksheets

The package VFS already supports entry-specific limits. A follow-up core API
should apply an explicit override only to the selected worksheet entry while
leaving workbook metadata, relationships, styles, shared strings, and global
defaults unchanged.

Adding a field to `XlsxSheetOptions` would break existing struct literals.
Prefer a new non-exhaustive read-options type and new visitor entry points.
Large shared-string tables require a separate policy decision.

## Production Gates

The reproducible gate harness lives in `scripts/tabular-ci.py` and runs in the
dedicated `tabular` workflow. It generates dense, mixed, shared-string, sparse,
late-conflict, long-string, and over-limit corpus cases; compares typed-row,
Arrow, and Parquet throughput; and reads the result independently with DuckDB
and PyArrow. Projection and filtered reads are part of the validation.

The workflow runs 10,000-row and 50,000-row cases. `/usr/bin/time -v` reports
clean-build and end-to-end peak RSS, while `du` reports target and executable
sizes. The harness JSON records input rows, output bytes, throughput, ratios,
batches, and row groups. Runtime gates require Arrow batching to reach 70% of
the typed-row sink and Parquet writing to reach 40%; exact-value, null/type,
projection, filtered-read, row-count, and over-limit assertions fail CI.

Local reference runs should use the same isolated reader versions:

```bash
python -m pip install duckdb==1.4.3 pyarrow==22.0.0
python scripts/tabular-ci.py --rows 10000
```

The dedicated CI workflow now enforces these production gates:

- explicit-schema files round-trip through DuckDB and PyArrow in CI;
- inference matches the documented promotion matrix;
- peak RSS stays bounded as row count grows with fixed batch and row-group
  sizes;
- Arrow batching reaches at least 70% of typed-row sink throughput;
- Parquet writing reaches at least 40% of typed-row sink throughput;
- no full worksheet or complete batch set is materialized;
- worksheet limit overrides affect only the selected worksheet entry;
- compile-time and artifact-size costs remain isolated to tabular consumers.

The integration should be reconsidered if deterministic inference requires
schema mutation during writes, bounded memory requires abandoning the streaming
row API, or either independent reader fails interoperability.

## Completed Follow-Ups

Production work was divided into separate issues:

- [#128](https://github.com/spereyra-dev/oxdoc/issues/128): worksheet-specific
  XLSX read limits;
- [#129](https://github.com/spereyra-dev/oxdoc/issues/129): move schema
  inference into `oxdoc-tabular` with explicit conflict policy;
- [#130](https://github.com/spereyra-dev/oxdoc/issues/130): stabilize a
  publishable tabular API with optional Parquet support;
- [#131](https://github.com/spereyra-dev/oxdoc/issues/131): reproducible
  performance and DuckDB/PyArrow interoperability CI.
