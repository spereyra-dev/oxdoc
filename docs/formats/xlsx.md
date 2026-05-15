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

## Memory Notes

Worksheet XML is streamed to the caller-provided writer. Shared strings use a bounded store: values stay in memory up to an internal threshold and spill to temporary files after that. Temporary files are created in the OS temporary directory and are removed when the extraction finishes or errors.

The memory bound applies to the shared-string table within the documented ZIP input limits. Memory can still grow with workbook metadata, the largest shared string currently being parsed, the current row width, ZIP library bookkeeping, and the caller's output writer. Very wide rows or sparse cells far to the right can allocate many empty CSV fields before the row is written.

## Planned Improvements

- Configurable large-file memory and temp-file policies.
