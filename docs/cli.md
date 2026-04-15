# CLI Reference

`oxdoc` is designed to be predictable in shell pipelines:

- Extraction output goes to stdout.
- Recoverable parser warnings go to stderr as `warning[<category>/<code>]: <path>: <message>`.
- Hard failures exit with code `1` and print `error[<code>]: <message>` to stderr.
- Clap usage errors keep their normal exit behavior and are not part of the runtime extraction contract.

## Global Help

```bash
oxdoc --help
```

## Extract DOCX or PPTX Text

```bash
oxdoc extract text <FILE> [--format text|json]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILE` | yes | Path to a `.docx` or `.pptx` file. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--format text` | `text` | Emit plain text. |
| `--format json` | `text` | Emit a JSON object with `file` and `text`. |

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

Warnings are still written to stderr when JSON output is selected. They are not embedded in the JSON payload.

PPTX extraction preserves presentation slide order, extracts DrawingML text boxes, and includes linked speaker notes after each slide.

## Extract XLSX CSV

```bash
oxdoc extract csv <FILE> [--sheet <NAME>|--sheet-index <INDEX>] [--delimiter <CHAR>]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILE` | yes | Path to a `.xlsx` file. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--sheet <NAME>` | first visible workbook sheet | Visible workbook sheet name to extract. Mutually exclusive with `--sheet-index`. |
| `--sheet-index <INDEX>` | first visible workbook sheet | 1-based visible workbook sheet index to extract. Mutually exclusive with `--sheet`. |
| `--delimiter <CHAR>` | `,` | Single-byte CSV delimiter. |

Example:

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
```

Index example:

```bash
oxdoc extract csv data.xlsx --sheet-index 2
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
- Hidden and very hidden sheets are skipped by default and by both sheet selectors.
- Duplicate visible sheet names are rejected; use `--sheet-index` to disambiguate malformed workbooks.

## Read Metadata

```bash
oxdoc info <FILE> [--format json|text]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILE` | yes | Path to a `.docx`, `.xlsx`, or `.pptx` OOXML package. |

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
  "file": "report.docx",
  "author": "Ada",
  "created_at": "2024-03-12T10:00:00Z",
  "application": "LibreOffice",
  "has_macros": false,
  "word_count": 1542
}
```

Optional fields are omitted from JSON when they are unavailable.
