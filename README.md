# oxdoc

`oxdoc` is a fast OOXML extractor for `.docx`, `.xlsx`, and document metadata. It is not a renderer: it ignores presentation layout and styling, and focuses on useful text, CSV, and JSON output.

The project is split from day one into a reusable core crate and a CLI crate:

- `oxdoc-core`: ZIP/VFS access plus streaming XML parsers.
- `oxdoc-cli`: command-line routing, stdout/stderr handling, and JSON formatting.

## CLI

```bash
oxdoc extract text contrato.docx
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
oxdoc info report.docx --format json
```

`oxdoc` writes extraction results to stdout and parser warnings to stderr.

## Design Notes

- The ZIP container is accessed through targeted entry reads instead of unpacking the full OOXML package to disk.
- XML is parsed with `quick-xml` in event mode.
- `.xlsx` sheet parsing streams rows to the caller-provided writer. Shared strings are currently indexed in memory for the MVP, with the storage boundary isolated for a future temp-file backed implementation.
- "Zero dependencies" is treated as zero runtime/system dependencies for deployment. The Rust build still uses focused crates for ZIP, XML, CLI, JSON, and typed errors.

## Development

```bash
make fmt
make lint
make test
make build
```

For a static Linux build:

```bash
rustup target add x86_64-unknown-linux-musl
make musl
```

## MVP Coverage

- DOCX text extraction from the main document part, resolving the office document relationship when present.
- XLSX CSV extraction with workbook relationship lookup, optional sheet name selection, shared strings, inline strings, sparse cell padding, and CSV escaping.
- Metadata extraction from `docProps/core.xml` and `docProps/app.xml`, plus basic macro detection.

