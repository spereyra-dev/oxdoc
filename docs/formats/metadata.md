# Metadata Extraction

`oxdoc info` reads Office package metadata from:

- `docProps/core.xml`
- `docProps/app.xml`

It also checks for common macro project parts.

## Command

```bash
oxdoc info report.docx --format json
```

## JSON Fields

| Field | Type | Source | Description |
| --- | --- | --- | --- |
| `file` | string | CLI path | File name from the input path. |
| `author` | string | `dc:creator` | Document author. |
| `last_modified_by` | string | `cp:lastModifiedBy` | Last modifier. |
| `created_at` | string | `dcterms:created` | Creation timestamp as stored in OOXML. |
| `modified_at` | string | `dcterms:modified` | Modification timestamp as stored in OOXML. |
| `application` | string | `Application` | Producing application. |
| `company` | string | `Company` | Company metadata. |
| `has_macros` | boolean | package parts | Whether a known `vbaProject.bin` part exists. |
| `word_count` | number | `Words` | Word count when provided. |
| `page_count` | number | `Pages` | Page count when provided. |
| `slide_count` | number | `Slides` | Slide count when provided. |
| `worksheet_count` | number | `Worksheets` | Worksheet count when provided. |
| `revision` | string | `cp:revision` | Revision metadata. |

## Macro Detection

The MVP checks for:

- `word/vbaProject.bin`
- `xl/vbaProject.bin`
- `ppt/vbaProject.bin`

This is intentionally simple and may be expanded later.

## Text Output

```bash
oxdoc info report.docx --format text
```

Text output prints one available field per line.
