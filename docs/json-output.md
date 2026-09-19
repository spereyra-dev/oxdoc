# JSON Output

JSON output exists for scripts and integrations that need stable field names instead of human-oriented text.

## Versioned Schemas

Machine-readable schemas live under `schemas/v1/` in the repository and are mirrored into the Docsify site for public access:

| Command | Schema |
| --- | --- |
| `oxdoc info --format json` | [`schemas/v1/oxdoc-info.schema.json`](schemas/v1/oxdoc-info.schema.json) |
| `oxdoc extract text --format json` | [`schemas/v1/oxdoc-extract-text.schema.json`](schemas/v1/oxdoc-extract-text.schema.json) |
| `oxdoc extract text --format structured-json` | [`schemas/v2/oxdoc-structured-text.schema.json`](schemas/v2/oxdoc-structured-text.schema.json) |
| `oxdoc extract tables --format json` | [`schemas/v1/oxdoc-docx-tables.schema.json`](schemas/v1/oxdoc-docx-tables.schema.json) |

The v1 schema for structured-json output remains available at [`schemas/v1/oxdoc-structured-text.schema.json`](schemas/v1/oxdoc-structured-text.schema.json) for outputs captured before schema v2. It stays frozen: new output fields are never added to it.
| `oxdoc audit --format json` | [`schemas/v1/oxdoc-audit.schema.json`](schemas/v1/oxdoc-audit.schema.json) |
| Each `oxdoc audit --format jsonl` line | [`schemas/v1/oxdoc-audit-jsonl.schema.json`](schemas/v1/oxdoc-audit-jsonl.schema.json) |
| `oxdoc extract csv --all-sheets --output-dir <DIR>` manifest | [`schemas/v1/oxdoc-all-sheets-manifest.schema.json`](schemas/v1/oxdoc-all-sheets-manifest.schema.json) |
| `oxdoc extract rows --format jsonl` | [`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`](schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json) |
| `oxdoc infer schema FILE` | [`schemas/v1/oxdoc-xlsx-schema.schema.json`](schemas/v1/oxdoc-xlsx-schema.schema.json) |
| `oxdoc extract slides --format json` | [`schemas/v1/oxdoc-pptx-slides.schema.json`](schemas/v1/oxdoc-pptx-slides.schema.json) |
| Each `oxdoc extract slides --format jsonl` line | [`schemas/v1/oxdoc-pptx-slides.schema.json`](schemas/v1/oxdoc-pptx-slides.schema.json) |

`oxdoc extract text --format jsonl` emits newline-delimited records for streaming batch ingestion. Each line is a standalone JSON object with `file`, `document_type`, and either `text` or `error`; successful records may include `warnings`.

`oxdoc extract text --format structured-json` emits ordered text blocks with `part_type`, `part_path`, `ordinal`, and `text` so consumers can distinguish body text from related parts such as comments, headers, speaker notes, and slides.

Structured-json payloads also carry a top-level `schema_version` field (`2` today). Schema version 2 adds one optional block field: `variant`, a DOCX header/footer label (`first`, `even`, or `default`) sourced from the `w:type` of the `sectPr` reference that positioned the part. Missing or unrecognized `w:type` values are labeled `default` with no warning. `variant` is present only on section-referenced DOCX `header` and `footer` blocks; it is omitted (never `null`) for orphan header/footer parts and for every other block kind and document type, including all PPTX blocks. When one header/footer part is referenced by multiple sections as different variants, the first reference's variant wins. The `ordinal` field is a 1-based global output-order index across the flattened block list — not a paragraph ordinal and not scoped per part or per slide; PPTX `slide` blocks are followed by the `notes` block holding their speaker notes, extracted from the `ppt/notesSlides/notesSlideN.xml` part.

The Rust `StructuredText` library type itself stays unversioned: serializing it directly produces the block body without the CLI's top-level `schema_version` marker. The versioned contract is a CLI output convention, mirroring the tables and rows-jsonl payloads.

Strict v1 validation of structured-json payloads is intentionally broken by schema v2: because v1 sets `additionalProperties` to `false`, any v2 payload (including variant-free ones, which only add `schema_version: 2` at the top level) fails v1 validation through its undeclared-field rule. If you validate structured-json output against a schema, point consumers at [`schemas/v2/oxdoc-structured-text.schema.json`](schemas/v2/oxdoc-structured-text.schema.json); the v1 schema is kept only for previously captured outputs. The v2 `blocks` arrays of variant-free documents are byte-identical to v1 output apart from the new `schema_version` key.

`oxdoc extract tables --format json` emits DOCX tables with source part
metadata, row and cell ordinals, grid offsets, column spans, vertical merge
states, paragraph blocks, nested table blocks, completion flags, and embedded
recoverable warnings.

The `--all-sheets` manifest records each exported XLSX sheet with `index`, `visibility`, `name`, `csv_path`, recoverable `warnings`, and an optional `error`. `visibility` is one of `visible`, `hidden`, or `veryHidden`.

`oxdoc audit --format jsonl` emits one record per input file with either an
`audit` object or an `error` object. It is the batch-friendly audit format; the
regular JSON audit contract stays optimized for one document or a small
in-memory array.

The `v1` and `v2` schemas use JSON Schema draft 2020-12, include stable `$id` values, and set `additionalProperties` to `false`. New output fields are introduced through a new schema version instead of silently widening the current contract: whenever a command's output gains a field, the payload's `schema_version` moves to a new version and the previous version's schema stays frozen. Existing `schema_version: 1` payloads remain valid against their published v1 schemas.

Within a schema version:

- Required fields remain required.
- Optional fields may be omitted when the source document does not provide them.
- Existing field names and JSON types remain stable.
- Warnings stay on stderr for legacy regular JSON payloads. JSONL text extraction and DOCX table JSON embed recoverable per-file warnings in the record so batch consumers can index them with the extracted content.

## XLSX Rows JSONL

Command:

```bash
oxdoc extract rows workbook.xlsx --sheet "Sales Q1" --format jsonl
```

Each stdout line is a standalone row record:

```json
{"schema_version":2,"file":"workbook.xlsx","sheet_name":"Sales Q1","row_index":2,"cells":[{"column_index":0,"kind":"string","raw":"Widget","value":"Widget","has_formula":false},{"column_index":2,"kind":"number","raw":"42.50","has_formula":true,"formula":"B2*2","formula_cached":true}]}
```

`row_index` and `column_index` are 0-based. `sheet_index`, when requested, is
the 1-based selector passed to the CLI. Sparse cells are omitted rather than
padded. Cell `kind` is `blank`, `string`, `boolean`, `number`, or `error`.
Raw numbers are always JSON strings; they are never converted to JSON numbers.
String and decoded boolean cells may include `value`, while formatted numeric
cells may include `formatted`. Every cell includes `has_formula`.

Formula cells additionally carry two trailing keys, `formula` and
`formula_cached`, always together and both omitted for non-formula cells:

- `formula` is the stored expression text exactly as written in the workbook's
  `<f>` element. It is **never recalculated, evaluated, or rewritten**; the
  emitted value is always the workbook's stored value, never a computed one.
  For shared formulas, the master's expression text is repeated verbatim on
  each slave cell (no coordinate rewriting).
- `formula_cached` reports whether the workbook stored a cached `<v>` value for
  the cell: `true` when a cache exists, `false` when the formula has no cached
  value (such a cell is `kind: "blank"` with the expression still present).

A workbook whose shared-formula references cannot be resolved (for example a
slave cell whose `si` master is missing) emits the cell with `has_formula:
true` and without the two formula fields, plus a per-cell warning on stderr:
`unresolved shared formula index '{si}': formula expression omitted`. When a
worksheet's shared-formula table exceeds the internal 1 MiB bound, one latched
warning names the worksheet: `shared formula table limit reached: expressions
beyond it are omitted`. Warnings never contaminate the JSONL stream.

Rows extraction accepts one XLSX input, including `-` for stdin. Recoverable
warnings are written to stderr so stdout remains a valid JSONL stream.

Rows-jsonl payloads are versioned as schema v2
([`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`](schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json)).
The previous v1 schema remains frozen at
[`schemas/v1/oxdoc-xlsx-rows-jsonl.schema.json`](schemas/v1/oxdoc-xlsx-rows-jsonl.schema.json)
for previously captured payloads: it stays valid for v1 records, and new
fields are never added to it. Strict v1 validation of rows-jsonl payloads is
intentionally broken by schema v2 — because v1 sets `additionalProperties` to
`false`, any v2 payload fails v1 validation through its undeclared-field rule,
even for formula-free workbooks whose records differ from v1 only by
`schema_version: 2`. If you validate rows-jsonl output against a schema, point
consumers at the v2 schema.

## XLSX Inferred Schema JSON

Command:

```bash
oxdoc infer schema workbook.xlsx --sheet "Sales Q1"
```

The experimental command emits JSON only:

```json
{
  "schema_version": 1,
  "experimental": true,
  "file": "workbook.xlsx",
  "sheet_name": "Sales Q1",
  "scan": {
    "mode": "full",
    "examined_rows": 250
  },
  "header_policy": "none",
  "columns": [
    {
      "column_index": 0,
      "name": "A",
      "logical_type": "int64",
      "nullable": false,
      "observed_types": ["int64"]
    },
    {
      "column_index": 1,
      "name": "B",
      "logical_type": "utf8",
      "nullable": true,
      "observed_types": ["null", "utf8"]
    }
  ],
  "warnings": []
}
```

`sheet_name` or `sheet_index` is included when the corresponding selector is
used; they are mutually exclusive. `sheet_index` is 1-based and
`column_index` is 0-based.

The scan defaults to `mode: "full"`. With `--sample-rows N`, the scan uses
`mode: "sampled"` and includes both `sample_rows` and `examined_rows`; sampled
results are approximate. No row is treated as a header. `header_policy` is
always `none`, so column names are Excel letters such as `A`, `B`, and `AA`.

The only logical and observed types are `null`, `bool`, `int64`, `float64`,
`date`, `time`, `datetime`, and `utf8`. Date/time inference requires supporting
workbook style information. Incompatible types promote to `utf8`, and numeric
inference makes no decimal precision or scale claim. Report warnings are
objects with `code`, `message`, and an optional 0-based `column_index`.

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
  "oxdoc_version": "2.0.0",
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

## PPTX Slides JSON / JSONL

Command:

```bash
oxdoc extract slides deck.pptx
oxdoc extract slides deck.pptx --format jsonl
```

`--format json` (the default) emits a single pretty-printed document with
`schema_version` (`1`), `file`, `document_type` (`"pptx"`), a `slides` array
in `p:sldIdLst` order, and a top-level `warnings` array that is present and
`[]` when empty. `--format jsonl` emits one compact record per slide, each
carrying `schema_version` (`1`), `file`, and the slide fields; there is no
wrapping array and the records appear in the same order as the JSON `slides`
array.

Each slide record carries `slide_id` (the `p:sldId/@id` integer, omitted when
absent or unparsable), `slide_ordinal` (the 1-based `p:sldIdLst` position —
gaps after skips are kept, so a skipped second slide of three yields ordinals
`1` and `3`), `slide_path` (the resolved slide part path), `text` (body text,
possibly `""`), and `notes` (speaker notes, present — possibly `""` — when a
notes part was read successfully, omitted otherwise; never `null`).

Missing slide or notes targets degrade to per-slide skip warnings while
extraction continues; skipped slides are simply absent from `slides` (or from
the JSONL stream), and a deck whose slides are all skipped still exits `0`
with a valid payload (`"slides": []` plus the warnings).

Warning channels follow the format: `--format json` embeds warnings in the
payload and mirrors them to stderr subject to `--warnings`/`--quiet` (embedded
warnings are never suppressed by `--quiet`); `--format jsonl` writes warnings
to stderr only so stdout stays a valid JSONL stream. The exact skip wordings
are documented in [Formats: PPTX](formats/pptx.md#slide-scoped-json--jsonl).

This is a NEW versioned contract (`schema_version: 1` at
[`schemas/v1/oxdoc-pptx-slides.schema.json`](schemas/v1/oxdoc-pptx-slides.schema.json)),
not a widening of structured-text: the structured-text v1 and v2 schemas stay
frozen, and a slides payload does not validate against them.

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
