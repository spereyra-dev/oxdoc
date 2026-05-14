# XLSX CSV Extraction

XLSX files are OOXML ZIP packages. `oxdoc` reads `xl/workbook.xml`, resolves the selected sheet through workbook relationships, reads shared strings when present, and streams worksheet rows to CSV.

## Current Behavior

The current parser:

- Selects the first visible sheet by default.
- Can select a sheet by visible workbook name with `--sheet`.
- Can select a sheet by 1-based visible workbook order with `--sheet-index`.
- Rejects duplicate visible sheet names instead of guessing.
- Skips hidden and very hidden sheets during selection.
- Stores `xl/sharedStrings.xml` in memory up to an internal threshold, then spills shared-string data to temporary files.
- Supports shared string cells (`t="s"`).
- Supports inline string cells (`t="inlineStr"`).
- Supports boolean cells (`t="b"`) as `TRUE` or `FALSE`.
- Emits error cells (`t="e"`) and cached formula values as their stored workbook values.
- Emits sparse cells as empty CSV fields.
- Escapes CSV fields with delimiters, quotes, or line breaks.

Numeric cells are emitted as the raw stored value from the worksheet XML. Date and time cells are also emitted as their stored serial values; `oxdoc` does not read `styles.xml`, apply number formats, convert Excel date systems, or localize numeric output yet.

Only rows present in `sheetData` are emitted. `oxdoc` pads missing cells within a present row, but it does not synthesize blank CSV rows from worksheet `dimension` ranges or row numbers.

## Example

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
```

Select the second visible sheet:

```bash
oxdoc extract csv data.xlsx --sheet-index 2
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

`--sheet` and `--sheet-index` are mutually exclusive. If a malformed workbook contains duplicate visible sheet names, name selection fails with a stable error instead of selecting an arbitrary match. Use `--sheet-index` to disambiguate those files.

Hidden and very hidden sheets are intentionally skipped by selection. A future explicit opt-in may expose hidden-sheet extraction if a workflow needs it.

## Memory Notes

Worksheet XML is streamed to the caller-provided writer. Shared strings use a bounded store: values stay in memory up to an internal threshold and spill to temporary files after that. Temporary files are created in the OS temporary directory and are removed when the extraction finishes or errors.

The memory bound applies to the shared-string table within the documented ZIP input limits. Memory can still grow with workbook metadata, the largest shared string currently being parsed, the current row width, ZIP library bookkeeping, and the caller's output writer. Very wide rows or sparse cells far to the right can allocate many empty CSV fields before the row is written.

## Planned Improvements

- Date, time, and number-format interpretation.
- Configurable large-file memory and temp-file policies.
- Multiple-sheet export modes.
