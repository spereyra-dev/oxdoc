# Metadata Extraction

`oxdoc info` reads Office package metadata from:

- `docProps/core.xml`
- `docProps/app.xml`
- `docProps/custom.xml`

It also checks for common macro project parts and macro content types in `[Content_Types].xml`.

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
| `custom_properties` | object | `docProps/custom.xml` | Custom document properties as string values. |
| `has_macros` | boolean | package parts, `[Content_Types].xml` | Whether VBA macro content is present or declared. |
| `word_count` | number | `Words` | Word count when provided. |
| `page_count` | number | `Pages` | Page count when provided. |
| `slide_count` | number | `Slides` | Slide count when provided. |
| `worksheet_count` | number | `Worksheets` | Worksheet count when provided. |
| `revision` | string | `cp:revision` | Revision metadata. |

## Macro Detection

Macro detection checks known VBA project part paths:

- `word/vbaProject.bin`
- `xl/vbaProject.bin`
- `ppt/vbaProject.bin`

It also checks `[Content_Types].xml` for `application/vnd.ms-office.vbaProject`, which catches macro-enabled packages that declare VBA content under a non-standard part path.

## Custom Properties

Custom properties are read from `docProps/custom.xml` and emitted as a JSON object:

```json
{
  "custom_properties": {
    "Department": "Research & Development",
    "Reviewed": "true"
  }
}
```

OOXML custom property values can have several value types. `oxdoc` preserves their textual value as a string so the JSON shape stays stable for integrations.

## Text Output

```bash
oxdoc info report.docx --format text
```

Text output prints one available scalar field per line. Use JSON output for custom properties.
