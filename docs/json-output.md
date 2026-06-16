# JSON Output

JSON output exists for scripts and integrations that need stable field names instead of human-oriented text.

## Versioned Schemas

Machine-readable schemas live under `schemas/v1/` in the repository and are mirrored into the Docsify site for public access:

| Command | Schema |
| --- | --- |
| `oxdoc info --format json` | [`schemas/v1/oxdoc-info.schema.json`](schemas/v1/oxdoc-info.schema.json) |
| `oxdoc extract text --format json` | [`schemas/v1/oxdoc-extract-text.schema.json`](schemas/v1/oxdoc-extract-text.schema.json) |
| `oxdoc extract text --format structured-json` | [`schemas/v1/oxdoc-structured-text.schema.json`](schemas/v1/oxdoc-structured-text.schema.json) |
| `oxdoc audit --format json` | [`schemas/v1/oxdoc-audit.schema.json`](schemas/v1/oxdoc-audit.schema.json) |
| `oxdoc extract csv --all-sheets --output-dir <DIR>` manifest | [`schemas/v1/oxdoc-all-sheets-manifest.schema.json`](schemas/v1/oxdoc-all-sheets-manifest.schema.json) |
| Each `oxdoc extract rows --format jsonl` line | [`schemas/v1/oxdoc-xlsx-rows-jsonl.schema.json`](schemas/v1/oxdoc-xlsx-rows-jsonl.schema.json) |

`oxdoc extract text --format jsonl` emits newline-delimited records for streaming batch ingestion. Each line is a standalone JSON object with `file`, `document_type`, and either `text` or `error`; successful records may include `warnings`.

`oxdoc extract text --format structured-json` emits ordered text blocks with `part_type`, `part_path`, `ordinal`, and `text` so consumers can distinguish body text from related parts such as comments, headers, speaker notes, and slides.

The `--all-sheets` manifest records each exported XLSX sheet with `index`, `visibility`, `name`, `csv_path`, recoverable `warnings`, and an optional `error`. `visibility` is one of `visible`, `hidden`, or `veryHidden`.

The `v1` schemas use JSON Schema draft 2020-12, include stable `$id` values, and set `additionalProperties` to `false`. New output fields are introduced through a new schema version instead of silently widening the current contract.

Within a schema version:

- Required fields remain required.
- Optional fields may be omitted when the source document does not provide them.
- Existing field names and JSON types remain stable.
- Warnings stay on stderr for regular JSON payloads. JSONL text extraction also embeds recoverable per-file warnings in the record so batch consumers can index them with the extracted text.

## XLSX Rows JSONL

Command:

```bash
oxdoc extract rows workbook.xlsx --sheet "Sales Q1" --format jsonl
```

Each stdout line is a standalone row record:

```json
{"schema_version":1,"file":"workbook.xlsx","sheet_name":"Sales Q1","row_index":2,"cells":[{"column_index":0,"kind":"string","raw":"Widget","value":"Widget","has_formula":false},{"column_index":2,"kind":"number","raw":"42.50","has_formula":true}]}
```

`row_index` and `column_index` are 0-based. `sheet_index`, when requested, is
the 1-based selector passed to the CLI. Sparse cells are omitted rather than
padded. Cell `kind` is `blank`, `string`, `boolean`, `number`, or `error`.
Raw numbers are always JSON strings; they are never converted to JSON numbers.
String and decoded boolean cells may include `value`, while formatted numeric
cells may include `formatted`. Every cell includes `has_formula`.

Rows extraction accepts one XLSX input, including `-` for stdin. Recoverable
warnings are written to stderr so stdout remains a valid JSONL stream.

## DOCX Text JSON

Command:

```bash
oxdoc extract text contrato.docx --format json
```

Shape:

```json
{
  "file": "contrato.docx",
  "text": "Texto extraido..."
}
```

Fields:

| Field | Type | Description |
| --- | --- | --- |
| `file` | string | File name derived from the provided path. |
| `text` | string | Extracted plain text from DOCX or PPTX input. |

## Metadata JSON

Command:

```bash
oxdoc info report.docx --format json
```

Shape:

```json
{
  "oxdoc_version": "1.1.0",
  "file": "report.docx",
  "author": "Usuario Falso",
  "last_modified_by": "Usuario Falso",
  "created_at": "2024-03-12T10:00:00Z",
  "modified_at": "2024-03-13T10:00:00Z",
  "application": "LibreOffice",
  "company": "Example Inc",
  "custom_properties": {
    "Department": "Research & Development",
    "Reviewed": "true"
  },
  "has_macros": false,
  "word_count": 1542,
  "page_count": 12,
  "slide_count": 0,
  "worksheet_count": 0,
  "revision": "4"
}
```

Fields other than `oxdoc_version`, `file`, and `has_macros` are optional and omitted when unavailable.

`custom_properties` contains values from `docProps/custom.xml`. Values are emitted as strings regardless of the OOXML custom property value type.

## Warnings

Warnings are emitted to stderr, not embedded in CLI JSON output. Library consumers receive warnings in `Extraction<T>`.

Example warning:

```text
warning[parser/W001]: word/document.xml: stopped after malformed XML: ...
```

For machine-readable stderr, use:

```bash
oxdoc --warnings json extract text report.docx --format json
```

Each warning is emitted as one JSON object per line:

```json
{"category":"parser","code":"W001","path":"word/document.xml","message":"stopped after malformed XML: ..."}
```
