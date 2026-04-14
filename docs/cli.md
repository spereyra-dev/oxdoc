# CLI Reference

`oxdoc` is designed to be predictable in shell pipelines:

- Extraction output goes to stdout.
- Recoverable parser warnings go to stderr.
- Hard failures exit non-zero and print `error: ...` to stderr.

## Global Help

```bash
oxdoc --help
```

## Extract DOCX Text

```bash
oxdoc extract text <FILE> [--format text|json]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILE` | yes | Path to a `.docx` file. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--format text` | `text` | Emit plain text. |
| `--format json` | `text` | Emit a JSON object with `file` and `text`. |

Example:

```bash
oxdoc extract text contrato.docx
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

## Extract XLSX CSV

```bash
oxdoc extract csv <FILE> [--sheet <NAME>] [--delimiter <CHAR>]
```

Arguments:

| Name | Required | Description |
| --- | --- | --- |
| `FILE` | yes | Path to a `.xlsx` file. |

Options:

| Option | Default | Description |
| --- | --- | --- |
| `--sheet <NAME>` | first workbook sheet | Visible workbook sheet name to extract. |
| `--delimiter <CHAR>` | `,` | Single-byte CSV delimiter. |

Example:

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
```

Output:

```csv
id,nombre,monto
1,Cliente A,5000
```

Notes:

- Sparse cells are padded with empty CSV fields.
- Shared strings and inline strings are supported in the MVP.
- CSV fields are quoted when they contain the delimiter, quotes, or line breaks.
- The delimiter must be a single-byte character.

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
