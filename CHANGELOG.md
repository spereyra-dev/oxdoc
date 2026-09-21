# Changelog

All notable changes to this project will be documented in this file.

The format is based on human-readable release notes.

## Unreleased

### Added

- Optional UTF-8 BOM for XLSX CSV output: `oxdoc extract csv --bom` (including
  `--all-sheets` per-sheet exports) prefixes the CSV bytes with `EF BB BF` for
  Excel-on-Windows compatibility, `XlsxCsvOptions::bom` exposes the same option
  to library consumers (BOM bytes are written before any sheet rows), and the
  Python wrapper's `extract_csv(bom=True)` passes the flag through and strips
  the BOM from the returned text.
- CRLF line terminator for XLSX CSV output: `oxdoc extract csv --crlf`
  (including `--all-sheets` per-sheet exports) ends CSV rows with `\r\n`
  instead of `\n` for Excel-classic compatibility, and the new
  `XlsxCsvOptions::line_terminator` field (`CsvLineTerminator::Lf` by default,
  `CsvLineTerminator::Crlf` for CRLF) exposes the same option to library
  consumers; only the row terminator changes, never content inside quoted
  fields. The Python wrapper's `extract_csv(crlf=True)` passes the flag
  through.
- Quote mode for XLSX CSV output: `oxdoc extract csv --quote-mode all`
  (including `--all-sheets` per-sheet exports) quotes every CSV field,
  including empty ones (`""`), matching common QUOTE_ALL semantics for strict
  ingestion pipelines, and the new `XlsxCsvOptions::quote_mode` field
  (`CsvQuoteMode::Minimal` by default, `CsvQuoteMode::All` for QUOTE_ALL) and
  re-exported `CsvQuoteMode` enum expose the same option to library consumers.
  The default `minimal` quoting is unchanged, and the Python wrapper's
  `extract_csv(quote_mode=...)` always passes the flag through.

## 2.0.0 - 2026-09-18

Released as `oxdoc-core` 2.0.0, `oxdoc-tabular` 0.2.0, and `oxdoc-cli` 2.0.0.

### Added

- XLSX formula provenance in typed rows: the stored `<f>` expression text is
  captured per cell (with the same entity/CDATA decoding used for cell values),
  each formula cell reports whether the workbook carried a cached value, and
  the contract guarantees formulas are never recalculated, evaluated, or
  rewritten — every emitted value is the workbook's stored value. Shared
  formulas resolve from a bounded (1 MiB), first-wins per-worksheet table;
  slaves carry the master's text verbatim with no reference translation.
  Formula cells without a resolvable expression warn per cell with
  `unresolved shared formula index '{si}': formula expression omitted`, and an
  overflowing table warns once per worksheet with `shared formula table limit
  reached: expressions beyond it are omitted`; the cell, its kind, and its
  cached value are still emitted in both cases. Array-formula masters capture
  their expression; region cells stay non-formula. CSV output is unchanged
  apart from the new stderr warnings on workbooks with unresolvable shared
  formulas.

- Deferred: an openpyxl-generated formula workbook for the
  `tests/fixtures/compatibility-matrix.json` compatibility corpus (the formula
  case matrix is covered by hand-authored runtime-zipped fixtures instead).

- New `oxdoc extract slides` CLI subcommand for slide-scoped PPTX extraction:
  one record per slide with `slide_id` (the `p:sldId/@id` integer, omitted
  when absent), `slide_ordinal` (the 1-based `p:sldIdLst` position, with gaps
  kept after skips), `slide_path`, body `text`, and optional speaker `notes`.
  `--format json` (the default) emits a single document validated by the new
  `schemas/v1/oxdoc-pptx-slides.schema.json` contract (`schema_version: 1`)
  with embedded warnings also mirrored to stderr; `--format jsonl` emits one
  compact record per slide on stdout with warnings on stderr only. Missing
  slide or notes targets degrade to per-slide skip warnings and extraction
  continues; the existing `extract text`/`structured-json` PPTX output and
  the structured-text v1/v2 schemas are unchanged.

### Changed

- XLSX rows JSONL (`oxdoc extract rows --format jsonl`) is now versioned as
  schema v2 (`schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`): payloads carry
  `schema_version: 2` and formula cells may carry the optional `formula` and
  `formula_cached` fields as two trailing keys (always together, omitted for
  non-formula cells). Formula-free workbooks produce records identical to v1
  apart from the `schema_version` key. **Migration note:** this intentionally
  breaks strict v1 validation of rows-jsonl payloads — because v1 sets
  `additionalProperties` to `false`, any v2 payload fails v1 validation
  through its undeclared-field rule, even for formula-free workbooks. The v1
  schema is frozen and kept only for previously captured outputs; point
  consumers at `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`. Mirroring the
  structured-text v2 migration, this break is documented and precedented.

- The `oxdoc-core` `XlsxCell` type gains a `formula: Option<XlsxFormula>`
  field, a source break for external struct literals. **Migration note:** add
  `formula: None` to `XlsxCell` struct literals (or read the field when
  constructing parser output) to restore compilation; `has_formula`,
  `XlsxCellValue`, and all value semantics are unchanged, and `formula` is
  `None` exactly when the cell has no `<f>` element or its shared-formula
  expression is unresolvable.

- `oxdoc extract text --format structured-json` output is now versioned as
  schema v2: payloads carry a top-level `schema_version: 2` field and blocks
  may carry an optional `variant` field (`first`, `even`, or `default`) on
  section-referenced DOCX `header` and `footer` blocks, sourced from the
  `w:type` of the `sectPr` reference that positioned the part. Missing or
  unrecognized `w:type` values are labeled `default` with no warning; orphan
  header/footer parts and all other block kinds (including every PPTX block)
  omit the field, never emitting `null`; when one part is referenced as
  multiple variants, the first reference's variant wins. The `blocks` arrays
  of variant-free documents are byte-identical to v1 output apart from the
  new `schema_version` key. **Migration note:** this intentionally breaks
  strict v1 validation of structured-json payloads — because v1 sets
  `additionalProperties` to `false`, any v2 payload fails v1 validation
  through its undeclared-field rule, even for variant-free documents. The v1
  schema is frozen and kept only for previously captured outputs; point
  consumers at `schemas/v2/oxdoc-structured-text.schema.json`. PPTX
  structured output (slide and notes blocks in `p:sldIdLst` presentation
  order) is unchanged but is now locked by a snapshot and documented under
  the v2 schema.

- DOCX headers and footers are now ordered by the sections that reference
  them in `word/document.xml` instead of `word/_rels/document.xml.rels`
  relationship-file order: sections in document order, headers before
  footers, `first`/`even`/`default` variants, dedup by resolved part path at
  the first reference, orphan header/footer parts appended in relationship
  order, and footnotes/endnotes/comments moved strictly after all headers and
  footers (keeping their mutual relationship order). Consumers relying on
  relationship-file order for multi-section packages will see different text,
  block, and table ordering; single-section documents with ordered rels are
  unchanged. No public output schema or option changes. Two new recoverable
  warnings are added for section references: a dangling `r:id` emits
  `skipped DOCX headerReference|footerReference {rid}: unknown relationship
  id`, and a reference without `r:id` emits `skipped DOCX
  headerReference|footerReference: missing r:id`; extraction continues in
  both cases.

## 1.2.0 - 2026-08-04

### Added

- Reusable XLSX schema inference and deterministic two-pass inferred Parquet
  conversion in `oxdoc-tabular`.
- Publishable `oxdoc-tabular` packaging with dependency-free-by-default schema
  inference and opt-in Arrow/Parquet conversion.
- Configurable Parquet writer properties, atomic path output, serializable
  conversion reports, and stable tabular error categories.

### Fixed

- Coerce promoted inferred UTF-8 columns from formatted XLSX values so a type
  conflict discovered during the inference pass remains writable under the
  frozen schema.
- Upgrade `quick-xml` and `crossbeam-epoch` to releases that address
  RUSTSEC-2026-0194, RUSTSEC-2026-0195, and RUSTSEC-2026-0204.

### Changed

- Updated the Rust dependency graph to the latest stable releases compatible
  with Rust 1.88, including SHA-2 0.11 and Parquet 59.1, and refreshed the
  Python build backend and GitHub Actions installer pins.
- Adapted updater checksum streaming to the SHA-2 0.11 API without
  materializing release archives in memory.

### Documentation

- Documented the tabular schema promotion, coercion, late-conflict, and
  two-pass conversion policies.

## 1.1.0 - 2026-05-15

### Added

- Optional competitive workbench for comparing `oxdoc` with local OOXML extraction CLIs such as Apache Tika, `xlsx2csv`, and Mammoth.
- Documentation for competitive benchmark setup, fixture cases, and interpretation.
- Python wrapper package for calling the `oxdoc` CLI from orchestration and data workflows.

### Changed

- Documented XLSX sparse-row memory behavior after sparse CSV row buffering improvements.
- Reduced XML text decoding allocations across DOCX, PPTX, metadata, XLSX, and shared-string parser paths.
- Kept XLSX CSV rows sparse until write time to avoid allocating empty strings for far-right cells.

## 1.0.0 - 2026-05-14

### Added

- Initial Rust workspace with `oxdoc-core` and `oxdoc-cli`.
- DOCX text extraction from the main document part.
- XLSX CSV extraction with shared strings and sparse cell padding.
- Metadata extraction from `docProps/core.xml` and `docProps/app.xml`.
- Basic CI, Makefile, and OSS project files.
- Docsify documentation site with usage, architecture, API, testing, roadmap, and security pages.
- Makefile and CI coverage gate with `cargo-llvm-cov` at 95% line coverage.
- XLSX CSV visible sheet selection by name or 1-based index, with explicit hidden-sheet and duplicate-name behavior.
- Crate-local READMEs, core API examples, and crates.io dry-run publishing guidance.
- Versioned JSON schemas for `info` and `extract text` machine-readable output.
- Additional CLI integration coverage for warning isolation and XLSX sheet-selection failures.
- Security advisory automation with RustSec scanning and local `make audit` support.
- `make ci-rust` for a local Rust gate aligned with GitHub Actions plus clearer contributor workflow docs.
- `Read + Seek` library entry points for DOCX text, XLSX CSV, and OOXML metadata extraction.
- Documented DOCX logical text semantics with parser tests for list, field, and hidden text policy.
- Initial Criterion benchmark suite for DOCX text throughput and XLSX row throughput.
- Manual GitHub Release workflow for Linux, macOS, Windows, musl Linux, and SHA256 checksum artifacts.
- Metadata extraction for `docProps/custom.xml` custom properties and macro detection from `[Content_Types].xml`.
- Regression coverage for DOCX malformed XML warnings and XLSX CSV sparse-field escaping.
- CLI documentation example copied from a real XLSX fixture run.
- Stronger fixture provenance checks and explicit XLSX archive generation notes.
- Fixture coverage for optional XLSX app metadata fields.
- Docs link checking plus published JSON schema copies for the Docsify site.
- PPTX text extraction for slide text boxes and speaker notes.
- `install.sh` for checksum-verified GitHub Release installs on macOS and Linux.
- Release workflow support for pushed `v*` tags, native smoke checks, and packaged `install.sh`.
- Homebrew formula rendering script for maintaining a tap.
