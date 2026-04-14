# JSON Output

JSON output exists for scripts and integrations that need stable field names instead of human-oriented text.

## Versioned Schemas

Machine-readable schemas live under `schemas/v1/`:

| Command | Schema |
| --- | --- |
| `oxdoc info --format json` | [`schemas/v1/oxdoc-info.schema.json`](https://github.com/spereyra-dev/oxdoc/blob/main/schemas/v1/oxdoc-info.schema.json) |
| `oxdoc extract text --format json` | [`schemas/v1/oxdoc-extract-text.schema.json`](https://github.com/spereyra-dev/oxdoc/blob/main/schemas/v1/oxdoc-extract-text.schema.json) |

The `v1` schemas use JSON Schema draft 2020-12, include stable `$id` values, and set `additionalProperties` to `false`. New output fields are introduced through a new schema version instead of silently widening the current contract.

Within a schema version:

- Required fields remain required.
- Optional fields may be omitted when the source document does not provide them.
- Existing field names and JSON types remain stable.
- Warnings stay on stderr and are not part of CLI JSON payloads.

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
| `text` | string | Extracted plain text. |

## Metadata JSON

Command:

```bash
oxdoc info report.docx --format json
```

Shape:

```json
{
  "file": "report.docx",
  "author": "Usuario Falso",
  "last_modified_by": "Usuario Falso",
  "created_at": "2024-03-12T10:00:00Z",
  "modified_at": "2024-03-13T10:00:00Z",
  "application": "LibreOffice",
  "company": "Example Inc",
  "has_macros": false,
  "word_count": 1542,
  "page_count": 12,
  "slide_count": 0,
  "worksheet_count": 0,
  "revision": "4"
}
```

Fields other than `file` and `has_macros` are optional and omitted when unavailable.

## Warnings

Warnings are emitted to stderr, not embedded in CLI JSON output. Library consumers receive warnings in `Extraction<T>`.

Example warning:

```text
warning[parser/W001]: word/document.xml: stopped after malformed XML: ...
```
