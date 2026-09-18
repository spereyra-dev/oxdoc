# XLSX CSV Extraction

XLSX files are OOXML ZIP packages. `oxdoc` reads `xl/workbook.xml`, resolves the selected sheet through workbook relationships, reads shared strings when present, and streams worksheet rows to CSV.

## Current Behavior

The current parser:

- Selects the first visible sheet by default.
- Can select a sheet by visible workbook name with `--sheet`.
- Can select a sheet by 1-based visible workbook order with `--sheet-index`.
- Can inventory and explicitly extract hidden or very hidden sheets with `--include-hidden`.
- Rejects duplicate sheet names in the selected visibility scope instead of guessing.
- Skips hidden and very hidden sheets during selection unless `--include-hidden` is present.
- Stores `xl/sharedStrings.xml` in memory up to an internal threshold, then spills shared-string data to temporary files.
- Supports shared string cells (`t="s"`).
- Supports inline string cells (`t="inlineStr"`).
- Supports boolean cells (`t="b"`) as `TRUE` or `FALSE`.
- Emits error cells (`t="e"`) and cached formula values as their stored workbook values.
- Emits sparse cells as empty CSV fields.
- Escapes CSV fields with delimiters, quotes, or line breaks.
- Uses raw worksheet XML values by default.
- Can opt into deterministic formatted values for supported date, time, percentage, currency, and decimal formats with `--value-mode formatted`.

Raw mode keeps numeric cells as the stored worksheet XML value. Formatted mode reads `xl/styles.xml` when present, converts the Excel 1900 and 1904 date systems, preserves Excel's serial 60 leap-year compatibility as `1900-02-29`, and emits locale-independent output. Unsupported number formats fall back to the raw stored value.

Only rows present in `sheetData` are emitted. `oxdoc` pads missing cells within a present row, but it does not synthesize blank CSV rows from worksheet `dimension` ranges or row numbers.

## Example

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
```

Select the second visible sheet:

```bash
oxdoc extract csv data.xlsx --sheet-index 2
```

Export every visible sheet:

```bash
oxdoc extract csv data.xlsx --all-sheets --output-dir exported-sheets
```

This writes one CSV file per visible sheet plus `manifest.json` in the output directory. CSV filenames use the visible sheet index plus a sanitized sheet name, for example `001-sales-q1.csv`.

Inventory hidden sheets:

```bash
oxdoc extract csv data.xlsx --list-sheets --include-hidden
```

When `--include-hidden` is present, sheet indices count all workbook sheets and list output includes each sheet visibility.

Format supported Excel cell values:

```bash
oxdoc extract csv data.xlsx --value-mode formatted
```

Output:

```csv
id,nombre,monto
1,Cliente A,5000
```

## Delimiters

The delimiter must be a single-byte character:

```bash
oxdoc extract csv data.xlsx --delimiter ";"
```

Multi-byte delimiters are rejected.

## Sheet Selection

Sheet indexes are 1-based and count only visible sheets in workbook order. For example, `--sheet-index 2` extracts the second visible sheet, even if the package contains hidden sheets before it.

With `--include-hidden`, sheet indexes are 1-based and count all workbook sheets in workbook order. This makes hidden extraction explicit and auditable:

```bash
oxdoc extract csv data.xlsx --sheet-index 3 --include-hidden
```

`--sheet` and `--sheet-index` are mutually exclusive. If a malformed workbook contains duplicate sheet names in the selected visibility scope, name selection fails with a stable error instead of selecting an arbitrary match. Use `--sheet-index` to disambiguate those files.

Hidden and very hidden sheets are intentionally skipped by default. Use `--include-hidden` to list or extract them, including `veryHidden` sheets.

`--all-sheets` also skips hidden and very hidden sheets unless `--include-hidden` is present. Its manifest records the sheet index, visibility, original sheet name, CSV path, recoverable warnings, and any sheet-level export error.

## Value Modes

`raw` is the default. It is the safest choice for repeatable ingestion because it preserves stored values such as `44927`, `0.125`, and `42.5`.

`formatted` is useful for analysts and downstream tools that expect spreadsheet-like values without custom postprocessing. Supported conversions include:

| XLSX stored value | Style kind | CSV output |
| --- | --- | --- |
| `44927` | date | `2023-01-01` |
| `44927.25` | date and time | `2023-01-01T06:00:00` |
| `0.1234` | percentage with two decimals | `12.34%` |
| `9.5` | currency with two decimals | `$9.50` |

Formula cells use their cached workbook value. If the workbook does not contain a cached value, the CSV field remains empty.

## Formula Provenance

Formulas are **never recalculated**. `oxdoc` performs no evaluation, builds no dependency graph, and never derives a value from a formula expression: every emitted value is the workbook's stored value exactly as written in the worksheet XML. A formula cell's CSV field is its cached `<v>` value; a formula cell without a cached value yields an empty CSV field, as before.

The stored expression text is available through typed rows (`oxdoc extract rows --format jsonl` and the library's `visit_xlsx_rows`), not through CSV. Each formula cell in a row record carries `formula` (the stored `<f>` text) and `formula_cached` (whether the workbook carried a cached value). Three cases are distinguishable in typed rows:

| Case | `formula` | `formula_cached` | `kind` |
| --- | --- | --- | --- |
| Cached formula | stored expression | `true` | value kind with the cached value |
| Uncached formula | stored expression | `false` | `blank` |
| Empty-but-cached value | stored expression | `true` | `blank` |

Cells without an `<f>` element carry no formula fields at all — no expression is ever synthesized.

### Shared formulas

Shared formulas (`<f t="shared" si="N">`) are resolved per worksheet from a bounded, first-wins table keyed by the raw `si` attribute text:

- A master cell keeps its own expression text; each slave resolves to the master's text **verbatim** — no reference translation, no rewriting, no evaluation.
- The first master registered for an `si` wins; duplicate masters never overwrite it.
- A slave whose master appears later in the worksheet stream (or not at all) is reported with `has_formula: true`, no `formula` fields, and one warning per affected cell: `unresolved shared formula index '{si}': formula expression omitted`. The cell, its `kind`, and its cached value are still emitted.

The shared-formula table holds one entry per distinct `si` group in memory and is bounded at 1 MiB with saturating checks, consistent with the shared-string memory conventions; it never spills to temporary files. When an insertion would exceed the bound, the entry is not recorded and one warning is latched per worksheet: `shared formula table limit reached: expressions beyond it are omitted`. Cells whose expressions are then unresolvable still emit the per-cell unresolved warning above.

Because the same parser backs CSV and rows extraction, CSV extraction of a workbook with unresolvable shared formulas now surfaces these warnings on stderr; the CSV bytes themselves are unchanged.

### Array formulas

An array formula master (`<f t="array" ref="…">EXPR</f>`) captures its expression and cache flag exactly like a normal formula. Cells inside the `ref` region that carry no `<f>` element of their own stay non-formula — no formula is ever synthesized for a cell whose XML carries none. Every other `<f>` with text (for example `t="dataTable"`) captures its text as-is; `t="shared"` with an `si` attribute is the only special case.

## Memory Notes

Worksheet XML is streamed to the caller-provided writer. Shared strings use a bounded store: values stay in memory up to an internal threshold and spill to temporary files after that. Temporary files are created in the OS temporary directory and are removed when the extraction finishes or errors.

The memory bound applies to the shared-string table within the documented ZIP input limits. Memory can still grow with workbook metadata, the largest shared string currently being parsed, the current row's populated cells, ZIP library bookkeeping, and the caller's output writer. Sparse cells far to the right are kept sparse until the row is written; the CSV output still contains the required delimiters for missing fields.

## Planned Improvements

- Configurable large-file memory and temp-file policies.
