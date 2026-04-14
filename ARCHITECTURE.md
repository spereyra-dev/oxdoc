# Architecture

`oxdoc` is a targeted OOXML extractor. It reads ZIP-based Office containers and extracts plain text, CSV, and metadata without rendering layout.

## Crates

- `oxdoc-core`: reusable parsing library. It owns ZIP access, OOXML part discovery, XML parsing, data models, and typed errors.
- `oxdoc-cli`: command-line application. It owns argument parsing, stdout/stderr behavior, and JSON formatting.

## Data Flow

1. The CLI validates arguments and opens the input file.
2. `oxdoc-core` opens the OOXML ZIP container through the VFS abstraction.
3. Relationship files identify the main document/workbook part when present.
4. XML parts are parsed with `quick-xml` event readers.
5. Extracted values are written to the caller-provided sink or returned as structured data.
6. Recoverable parser problems are emitted as warnings.

## Error Model

Hard failures use typed errors from `OxdocError`. Recoverable parser issues should be returned as warnings with the relevant OOXML part path.

## Memory Model

The parser should stream large XML parts where possible. The current MVP keeps XLSX shared strings in memory; the storage boundary should be kept isolated so a disk-backed implementation can replace it later.

## Extension Points

- Additional OOXML part parsers under `oxdoc-core/src/parsers`.
- New CLI output formats in `oxdoc-cli`.
- Future benchmark and fuzz targets outside the runtime crates.

