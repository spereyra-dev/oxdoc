# Audit Signals

`oxdoc audit` reports factual signals that help document intake, compliance, security review, and data pipeline triage. It does not render documents, mutate input files, or assign a risk score.

## Command

```bash
oxdoc audit report.docx --format json
```

Text output is also available:

```bash
oxdoc audit report.docx --format text
```

## JSON Shape

The versioned JSON Schema lives at [`schemas/v1/oxdoc-audit.schema.json`](schemas/v1/oxdoc-audit.schema.json).

```json
{
  "oxdoc_version": "1.1.0",
  "file": "workbook.xlsx",
  "document_type": "xlsx",
  "metadata": {
    "file": "workbook.xlsx",
    "application": "Excel",
    "has_macros": false
  },
  "signals": [
    {
      "kind": "hidden_sheet",
      "severity": "warning",
      "path": "xl/workbook.xml",
      "message": "worksheet 'Model Inputs' is hidden"
    }
  ]
}
```

## Signal Fields

| Field | Description |
| --- | --- |
| `kind` | Stable signal category, such as `macros`, `custom_properties`, `hidden_sheet`, `relationship_target`, or `parser_warning`. |
| `severity` | Factual severity bucket. Current values are `info`, `warning`, and `high`. |
| `path` | OOXML package part related to the signal. |
| `message` | Human-readable detail. |

## Current Signals

| Kind | Severity | Meaning |
| --- | --- | --- |
| `macros` | `high` | VBA macro content is present or declared. |
| `custom_properties` | `info` | Custom document properties are present. |
| `hidden_sheet` | `warning` | An XLSX worksheet is hidden or very hidden. |
| `relationship_target` | `warning` | A relationship target is external or otherwise suspicious. |
| `parser_warning` | `warning` | A recoverable parser warning occurred while collecting audit data. |

Warnings are still emitted to stderr according to the global `--warnings` option. The `signals` array keeps audit findings in stdout for JSON consumers.
