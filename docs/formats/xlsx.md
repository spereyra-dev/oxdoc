# XLSX CSV Extraction

XLSX files are OOXML ZIP packages. `oxdoc` reads `xl/workbook.xml`, resolves the selected sheet through workbook relationships, reads shared strings when present, and streams worksheet rows to CSV.

## Current Behavior

The MVP parser:

- Selects the first sheet by default.
- Can select a sheet by visible workbook name with `--sheet`.
- Reads `xl/sharedStrings.xml` into memory.
- Supports shared string cells (`t="s"`).
- Supports inline string cells (`t="inlineStr"`).
- Emits sparse cells as empty CSV fields.
- Escapes CSV fields with delimiters, quotes, or line breaks.

## Example

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
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

## Memory Notes

Worksheet XML is streamed to the caller-provided writer. Shared strings are loaded into memory in the MVP. The code keeps that responsibility isolated so a future large-file implementation can switch to a disk-backed or indexed shared-string store.

## Planned Improvements

- Sheet selection by index.
- Date and time interpretation.
- Boolean and error values.
- Cached formula output.
- Large shared-string storage.
- Multiple-sheet export modes.
