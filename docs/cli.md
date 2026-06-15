# CLI Reference

`oxdoc` is designed to be predictable in shell pipelines:

- Extraction output goes to stdout.
- Recoverable parser warnings go to stderr as `warning[<category>/<code>]: <path>: <message>` by default.
- Hard failures exit with code `1` and print `error[<code>]: <message>` to stderr.
- Clap usage errors keep their normal exit behavior and are not part of the runtime extraction contract.

## Exit Codes

| Exit Code | Meaning |
| --- | --- |
| `0` | Extraction or metadata read succeeded. Recoverable warnings may still be emitted to stderr. |
| `1` | Hard runtime error. `oxdoc` prints `error[<code>]: <message>` to stderr. |
| `2` | CLI usage error from Clap, such as missing required arguments or conflicting flags. |

## Global Help

```bash
oxdoc --help
```

## Global Options

| Option | Default | Description |
| --- | --- | --- |
| `--quiet`, `-q` | false | Suppress recoverable warnings. |
| `--warnings text\|json\|none` | `text` | Choose warning output format. `json` emits one JSON object per warning to stderr. `none` suppresses warnings. |

JSON warning records include stable machine-readable fields:

```json
{"category":"parser","code":"W001","path":"word/document.xml","message":"stopped after malformed XML: ..."}
```

## Extract DOCX or PPTX Text

```bash
oxdoc extract text <FILES>... [--format text|json|jsonl|structured-json] [-o <PATH>]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILES` | yes | One or more `.docx` or `.pptx` files, or `-` to read one OOXML package from stdin. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--format text` | `text` | Emit plain text. |
| `--format json` | `text` | Emit a JSON object with `file` and `text`; with multiple files, emit a JSON array. |
| `--format jsonl` | `text` | Emit one JSON object per input file, suitable for streaming batch ingestion. |
| `--format structured-json` | `text` | Emit source-aware text blocks with OOXML part metadata. |
| `--output <PATH>`, `-o <PATH>` | stdout | Write extraction output to a file. |

Example:

```bash
oxdoc extract text contrato.docx
```

Presentation example:

```bash
oxdoc extract text deck.pptx
```

JSON example:

```bash
oxdoc extract text contrato.docx --format json
```

Output shape:

```json
{
  "file": "contrato.docx",
  "text": "Plain text..."
}
```

Batch example:

```bash
oxdoc extract text a.docx b.pptx --format json
```

JSONL batch example:

```bash
oxdoc extract text *.docx --format jsonl
```

Each JSONL line contains `file`, `document_type`, and either `text` or `error`. Recoverable parser warnings are embedded in the line as `warnings` and can also be emitted to stderr according to the global `--warnings` setting. Per-file extraction failures are represented as error records and later files continue processing.

Structured JSON example:

```bash
oxdoc extract text contrato.docx --format structured-json
```

Output shape:

```json
{
  "file": "contrato.docx",
  "document_type": "docx",
  "blocks": [
    {
      "part_type": "main",
      "part_path": "word/document.xml",
      "ordinal": 1,
      "text": "Plain text..."
    }
  ]
}
```

Plain text output flattens all supported text-bearing parts into a single stream. Structured output keeps each non-empty source part separate. DOCX blocks can come from `main`, `header`, `footer`, `footnotes`, `endnotes`, or `comments`; PPTX blocks can come from `slide` and speaker `notes`.

Warnings are still written to stderr when regular JSON output is selected. They are not embedded in the JSON payload. Use `--warnings json` when a pipeline needs machine-readable warning records.

PPTX extraction preserves presentation slide order, extracts DrawingML text boxes, and includes linked speaker notes after each slide.

## Extract XLSX CSV

```bash
oxdoc extract csv <FILES>... [--sheet <NAME>|--sheet-index <INDEX>|--list-sheets|--all-sheets --output-dir <DIR>] [--include-hidden] [--delimiter <CHAR>] [--value-mode <MODE>] [-o <PATH>]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILES` | yes | One or more `.xlsx` files, or `-` to read one OOXML package from stdin. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--sheet <NAME>` | first visible workbook sheet | Workbook sheet name to extract. Hidden and very hidden sheets require `--include-hidden`. Mutually exclusive with `--sheet-index` and `--list-sheets`. |
| `--sheet-index <INDEX>` | first visible workbook sheet | 1-based visible sheet index, or 1-based full workbook index with `--include-hidden`. Mutually exclusive with `--sheet` and `--list-sheets`. |
| `--list-sheets` | false | Print sheet names with 1-based indices and exit. Hidden and very hidden sheets require `--include-hidden`. |
| `--all-sheets` | false | Export every visible sheet from a single workbook to separate CSV files, or every workbook sheet with `--include-hidden`. Requires `--output-dir`. Mutually exclusive with `--sheet`, `--sheet-index`, `--list-sheets`, and `--output`. |
| `--include-hidden` | false | Include hidden and very hidden workbook sheets in listing or extraction. |
| `--delimiter <CHAR>` | `,` | Single-byte CSV delimiter. |
| `--value-mode <MODE>` | `raw` | Emit worksheet XML values with `raw`, or deterministic formatted values with `formatted` for supported XLSX number formats. |
| `--output <PATH>`, `-o <PATH>` | stdout | Write CSV or sheet list output to a file. |
| `--output-dir <PATH>` | none | Directory for `--all-sheets` CSV files and `manifest.json`. |

Example:

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
```

Index example:

```bash
oxdoc extract csv data.xlsx --sheet-index 2
```

List sheets example:

```bash
oxdoc extract csv data.xlsx --list-sheets
```

Hidden sheet inventory example:

```bash
oxdoc extract csv data.xlsx --list-sheets --include-hidden
```

All visible sheets example:

```bash
oxdoc extract csv data.xlsx --all-sheets --output-dir exported-sheets
```

Formatted value example:

```bash
oxdoc extract csv data.xlsx --value-mode formatted
```

Output:

```csv
id,nombre,monto
1,Cliente A,5000
```

Fixture-backed example:

```bash
oxdoc extract csv fixtures/xlsx-basic.xlsx --sheet "Sales Q1"
```

stdout:

```csv
id,Cliente A,monto
1,,5000
```

stderr is empty for this fixture. The blank middle field in the second row is intentional; the worksheet has no `B2` cell, so `oxdoc` preserves the sparse column as an empty CSV field.

Notes:

- Sparse cells are padded with empty CSV fields.
- Shared strings, inline strings, booleans, error cells, and cached formula values are supported.
- CSV fields are quoted when they contain the delimiter, quotes, or line breaks.
- The delimiter must be a single-byte character.
- Hidden and very hidden sheets are skipped by default. `--include-hidden` is required to list or extract them.
- With `--include-hidden`, sheet indices count all workbook sheets and `--list-sheets` prints visibility as `visible`, `hidden`, or `veryHidden`.
- `--all-sheets` skips hidden and very hidden sheets unless `--include-hidden` is present. It writes a `manifest.json` file next to the CSV files.
- Duplicate sheet names in the selected visibility scope are rejected; use `--sheet-index` to disambiguate malformed workbooks.

## XLSX Value Modes

`--value-mode raw` is the default and emits the stored worksheet value exactly as represented in the sheet XML after string and boolean decoding. This keeps existing ingestion jobs stable.

`--value-mode formatted` reads workbook styles when present and formats supported numeric cells in a locale-independent way. Dates use ISO output (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SS`), time-only values use `HH:MM:SS`, percentages include `%`, and common decimal and currency formats use the decimal precision declared by the workbook. Unsupported formats fall back to the raw stored value.

## Read Metadata

```bash
oxdoc info <FILE> [--format json|text]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILE` | yes | Path to a `.docx`, `.xlsx`, or `.pptx` OOXML package, or `-` to read from stdin. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--format json` | `json` | Emit structured metadata JSON. |
| `--format text` | `json` | Emit one field per line. |

Example:

```bash
oxdoc info report.docx --format json
```

Output shape:

```json
{
  "oxdoc_version": "1.1.0",
  "file": "report.docx",
  "author": "Ada",
  "created_at": "2024-03-12T10:00:00Z",
  "application": "LibreOffice",
  "has_macros": false,
  "word_count": 1542
}
```

Optional fields are omitted from JSON when they are unavailable.

## Audit Document Signals

```bash
oxdoc audit <FILE> [--format json|text]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILE` | yes | Path to a `.docx`, `.xlsx`, or `.pptx` OOXML package, or `-` to read from stdin. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--format json` | `json` | Emit structured audit JSON. |
| `--format text` | `json` | Emit one field or signal per line. |

Example:

```bash
oxdoc audit workbook.xlsx --format json
```

Audit signals are factual findings, not a risk score. Current signals include macros, custom properties, hidden or protected XLSX workbooks, classified external relationships, embedded packages and OLE objects, suspicious relationship targets, and recoverable parser warnings.
