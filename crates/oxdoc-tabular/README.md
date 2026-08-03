# oxdoc-tabular

Experimental XLSX-to-Arrow and Parquet adapter for `oxdoc`.

The crate keeps Arrow and Parquet outside `oxdoc-core` and supports both
explicit schemas and deterministic two-pass inferred conversion. Inferred
conversion scans and freezes the schema, rewinds or reopens the workbook, and
then writes bounded Arrow batches and Parquet row groups.

## Inferred conversion policy

- Inference promotes `int64` plus `float64` to `float64`.
- Other incompatible observed types promote the column to `utf8` and emit a
  `type_conflict` warning.
- Explicit-schema conversion is strict: an incompatible worksheet value
  returns `SchemaMismatch` with row and column context.
- Inferred `utf8` columns use formatted coercion so earlier values remain
  writable after a later conflict promotes the frozen column to text.
- Sampled inference is approximate. A value after the sample that conflicts
  with a non-`utf8` frozen column fails deterministically with
  `SchemaMismatch`; schemas never mutate while Parquet is being written.

## Example

```rust,no_run
use oxdoc_core::XlsxSheetOptions;
use oxdoc_tabular::write_xlsx_parquet_with_inferred_schema;

fn main() -> oxdoc_tabular::Result<()> {
    let output = std::fs::File::create("output.parquet")?;
    write_xlsx_parquet_with_inferred_schema(
        "input.xlsx",
        XlsxSheetOptions::default(),
        None,
        8_192,
        output,
    )?;
    Ok(())
}
```

The crate remains unpublished and experimental. See
[`docs/spikes/xlsx-arrow-parquet.md`](../../docs/spikes/xlsx-arrow-parquet.md)
for architecture, measurements, and production gates.
