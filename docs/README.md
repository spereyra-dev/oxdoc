# oxdoc

`oxdoc` is a fast OOXML extractor for `.docx`, `.xlsx`, and document metadata. It opens the ZIP-based Office container, reads only the XML parts that matter, and emits plain text, CSV, or JSON without rendering layout.

The project is designed for shell pipelines, CI jobs, serverless functions, and embedding through a stable Rust library API.

Public documentation URL: [spereyra-dev.github.io/oxdoc](https://spereyra-dev.github.io/oxdoc/).

## What It Does

- Extracts DOCX text from the main document part.
- Extracts PPTX text from slide text boxes and speaker notes.
- Converts XLSX worksheets to CSV.
- Reads core and app metadata from Office documents.
- Reports factual audit signals for governance and intake workflows.
- Emits structured warnings for recoverable parser problems.
- Keeps the CLI simple: extracted data goes to stdout, warnings go to stderr.

## What It Does Not Do

- It does not render Office documents.
- It does not generate PDFs.
- It does not preserve fonts, margins, colors, layout, or pagination.
- It does not mutate or repair input files.
- It does not try to implement the entire OOXML specification.

## Current Status

`oxdoc` is ready for a 1.0 release. The crate split, CLI contract, parser structure, release workflow, installer, and documentation are in place.

From 1.0 onward, public CLI and Rust API changes follow semantic versioning.

## Quick Example

```bash
oxdoc extract text contrato.docx
oxdoc extract text *.docx --format jsonl
oxdoc extract text contrato.docx --format structured-json
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
oxdoc extract csv data.xlsx --sheet-index 2
oxdoc extract csv data.xlsx --list-sheets --include-hidden
oxdoc extract csv data.xlsx --all-sheets --output-dir exported-sheets
oxdoc extract csv data.xlsx --value-mode formatted
oxdoc info report.docx --format json
oxdoc audit report.docx --format json
```

## Crates

- `oxdoc-core`: ZIP/VFS access, relationship discovery, streaming XML parsers, data models, typed errors.
- `oxdoc-cli`: argument parsing, stdout/stderr handling, JSON formatting.

## Documentation Map

- New users should start with [Getting Started](getting-started.md).
- CLI users should read [CLI Reference](cli.md).
- Rust users should read [Library API](library-api.md).
- Contributors should read [Architecture](architecture.md), [Testing Strategy](testing.md), and [Contributing](contributing.md).
- Project planning lives in [Roadmap](roadmap.md).
