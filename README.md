# oxdoc

Fast OOXML extraction without rendering.

`oxdoc` is a Rust workspace for extracting plain text, CSV, and metadata from Office Open XML containers such as `.docx`, `.xlsx`, and, over time, `.pptx`. It is built for automation: shell pipelines, CI jobs, serverless functions, ingestion systems, and future embedding through a stable Rust API.

`oxdoc` is not a document renderer. It does not preserve layout, calculate pagination, generate PDFs, or implement the full OOXML specification. It reads the ZIP-based Office container, targets the XML parts that matter, and emits useful structured output.

## Status

`oxdoc` is pre-1.0 and under active development. The current codebase implements the first MVP slice:

- DOCX text extraction from the main document part.
- DOCX logical text semantics for paragraph breaks, table cell and row separation, tabs, line breaks, and deleted revision text handling.
- XLSX worksheet-to-CSV extraction.
- Metadata extraction from `docProps/core.xml` and `docProps/app.xml`.
- A reusable `oxdoc-core` crate and a CLI-facing `oxdoc-cli` crate.
- Typed errors plus recoverable warnings.
- OSS project scaffolding and Docsify documentation.

APIs and CLI behavior may still change before the first tagged release.

## Documentation

The complete documentation site is built with Docsify and lives in [`docs/`](docs/).

Serve it locally:

```bash
npx docsify-cli@4 serve docs --port 3000
```

Then open:

```text
http://localhost:3000
```

The `docs` GitHub Actions workflow validates that the Docsify site serves correctly. Publishing it through GitHub Pages requires enabling Pages for the repository in GitHub settings.

Key documentation pages:

- [Getting Started](docs/getting-started.md)
- [Installation](docs/installation.md)
- [CLI Reference](docs/cli.md)
- [Library API](docs/library-api.md)
- [Architecture](docs/architecture.md)
- [OOXML Model](docs/ooxml-model.md)
- [Errors and Warnings](docs/errors-and-warnings.md)
- [Performance and Memory](docs/performance.md)
- [Testing Strategy](docs/testing.md)
- [Roadmap](docs/roadmap.md)
- [Security](docs/security.md)

## Quick Start

Build the workspace:

```bash
cargo build --workspace
```

Run the CLI through Cargo:

```bash
cargo run -p oxdoc-cli -- --help
```

Install the CLI locally from source:

```bash
cargo install --path crates/oxdoc-cli
```

## CLI Usage

### Extract DOCX Text

```bash
oxdoc extract text contrato.docx
```

Plain text is written to stdout:

```text
Este es el texto plano extraido velozmente...
```

For integrations:

```bash
oxdoc extract text contrato.docx --format json
```

Output shape:

```json
{
  "file": "contrato.docx",
  "text": "Este es el texto plano extraido velozmente..."
}
```

### Convert XLSX to CSV

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter ","
```

Output:

```csv
id,nombre,monto
1,Cliente A,5000
```

Notes:

- `--sheet` selects a visible workbook sheet name.
- If `--sheet` is omitted, the first workbook sheet is used.
- `--delimiter` must be a single-byte character.
- CSV fields are quoted when needed.
- Sparse cells are padded with empty CSV fields.

### Read Metadata

```bash
oxdoc info report.docx --format json
```

Output shape:

```json
{
  "file": "report.docx",
  "author": "Usuario Falso",
  "created_at": "2024-03-12T10:00:00Z",
  "application": "LibreOffice",
  "has_macros": false,
  "word_count": 1542
}
```

Text output is also available:

```bash
oxdoc info report.docx --format text
```

## Workspace Layout

```text
.
├── crates
│   ├── oxdoc-core
│   └── oxdoc-cli
├── docs
├── .github
└── tests
```

## Crates

### `oxdoc-core`

The reusable library. It owns:

- ZIP-backed OOXML package access.
- Relationship discovery.
- Streaming XML parser state machines.
- Data models such as `Extraction<T>`, `DocumentInfo`, and `XlsxCsvOptions`.
- Error and warning contracts.

### `oxdoc-cli`

The command-line application. It owns:

- Argument parsing with `clap`.
- Routing commands to `oxdoc-core`.
- Writing extraction data to stdout.
- Writing recoverable warnings and hard errors to stderr.
- Formatting JSON with `serde_json`.

## Design Principles

- Tolerant input, strict output.
- Extract useful data; do not render documents.
- Prefer streaming XML parsing over DOM parsing.
- Keep memory bounded for large files.
- Keep warnings recoverable and explicit.
- Keep the library independent from terminal concerns.
- Treat documentation, tests, and fixtures as part of the product.

## Current Capabilities

| Area | Supported now |
| --- | --- |
| DOCX text | Main document text from `<w:t>`, paragraph breaks, tabs, and line breaks. |
| XLSX CSV | Workbook relationship lookup, sheet name selection, shared strings, inline strings, sparse cells, CSV escaping. |
| Metadata | Core/app properties plus basic macro detection. |
| Output | Plain text, CSV, JSON metadata, JSON DOCX text. |
| Errors | Typed library errors, CLI non-zero hard failures. |
| Warnings | Recoverable parser warnings with OOXML part paths. |

## Known MVP Limitations

- DOCX headers, footers, footnotes, comments, and hyperlink details are planned but not complete.
- XLSX shared strings are loaded into memory in the MVP.
- XLSX date, boolean, error, and cached formula interpretation need hardening.
- PPTX text extraction is planned but not implemented yet.
- The public Rust API is not stable before 1.0.

See [ROADMAP.md](ROADMAP.md) and [docs/roadmap.md](docs/roadmap.md).

## Development

Run all standard checks:

```bash
make all
```

Equivalent commands:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features --all-targets
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features --all-targets
cargo test --doc --workspace --all-features
cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only
cargo build --workspace
```

Coverage is gated at 95% line coverage. Install the optional local tool with:

```bash
make install-tools
```

Static Linux build:

```bash
rustup target add x86_64-unknown-linux-musl
make musl
```

Serve documentation:

```bash
make docs
```

## Testing Strategy

The current test suite focuses on parser unit tests. The roadmap includes:

- Real fixture files from Microsoft Office, LibreOffice, Google Docs, and third-party exporters.
- Snapshot tests with `insta`.
- Fuzz targets for XML parser paths.
- Performance benchmarks for cold start, throughput, and peak memory.

See [docs/testing.md](docs/testing.md).

## Security

`oxdoc` parses untrusted Office files. Malformed ZIP/XML input should return errors or warnings, not panic.

Required OOXML ZIP parts are guarded before parsing: encrypted parts are rejected, oversized parts fail predictably, zip-bomb-like compression ratios are blocked, and relationship targets must stay inside the package root.

Do not open public issues for security vulnerabilities. Use GitHub private vulnerability reporting or contact the maintainers privately through GitHub.

See [SECURITY.md](SECURITY.md) and [docs/security.md](docs/security.md).

## Contributing

Contributions are welcome. Good first contributions include:

- Focused parser fixes.
- New unit tests for malformed XML.
- Safe-to-redistribute fixtures with provenance notes.
- Documentation improvements.
- Performance and memory measurement work.

Start with [CONTRIBUTING.md](CONTRIBUTING.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), and [docs/contributing.md](docs/contributing.md).

## Project Governance

`oxdoc` is currently maintainer-led. Larger design changes should start as GitHub issues before implementation.

See [GOVERNANCE.md](GOVERNANCE.md).

## License

`oxdoc` is licensed under the MIT License. See [LICENSE](LICENSE).
