# oxdoc

Fast OOXML extraction without rendering.

`oxdoc` is a Rust workspace for extracting plain text, CSV, and metadata from Office Open XML containers such as `.docx`, `.xlsx`, and `.pptx`. It is built for automation: shell pipelines, CI jobs, serverless functions, ingestion systems, and embedding through a stable Rust API.

`oxdoc` is not a document renderer. It does not preserve layout, calculate pagination, generate PDFs, or implement the full OOXML specification. It reads the ZIP-based Office container, targets the XML parts that matter, and emits useful structured output.

## Status

`oxdoc` has a stable 1.x CLI and Rust API contract for:

- DOCX text extraction from the supported document parts.
- DOCX logical text semantics for paragraph breaks, table cell and row separation, tabs, line breaks, and deleted revision text handling.
- PPTX text extraction from slide text boxes and speaker notes.
- XLSX worksheet-to-CSV extraction.
- XLSX sheet listing and sheet selection by name or index, with explicit opt-in for hidden and very hidden sheets.
- Metadata extraction from core, app, and custom document properties.
- A reusable `oxdoc-core` crate and a CLI-facing `oxdoc-cli` crate.
- Typed errors plus recoverable warnings.
- Stdin, batch extraction, file output, release binaries, and checksum-verified installs.

## Documentation

The complete documentation site is built with Docsify and lives in [`docs/`](docs/). The public site URL is [spereyra-dev.github.io/oxdoc](https://spereyra-dev.github.io/oxdoc/).

Serve it locally:

```bash
npx docsify-cli@4 serve docs --port 3000
```

Then open:

```text
http://localhost:3000
```

The `docs` GitHub Actions workflow validates that the Docsify site serves correctly, checks internal Markdown links, and verifies the published schema copies stay in sync. Publishing it through GitHub Pages requires enabling Pages for the repository in GitHub settings.

Key documentation pages:

- [Getting Started](docs/getting-started.md)
- [Installation](docs/installation.md)
- [CLI Reference](docs/cli.md)
- [Library API](docs/library-api.md)
- [Python Integration](docs/python-integration.md)
- [Architecture](docs/architecture.md)
- [OOXML Model](docs/ooxml-model.md)
- [Errors and Warnings](docs/errors-and-warnings.md)
- [Performance and Memory](docs/performance.md)
- [Peak Memory Baselines](docs/performance-memory-baselines.md)
- [Competitive Workbench](docs/performance-competitors.md)
- [Testing Strategy](docs/testing.md)
- [Roadmap](docs/roadmap.md)
- [Security](docs/security.md)

## Quick Start

Install the latest macOS/Linux release:

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | sh
```

Install with Cargo:

```bash
cargo install oxdoc-cli
```

Or build the workspace from source:

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

## Performance Benchmarks

`oxdoc` keeps parser performance visible through three reproducible workflows:

| Workflow | Command | What it tells you |
| --- | --- | --- |
| Library throughput | `cargo bench -p oxdoc-core` | In-process parser throughput for DOCX text and XLSX CSV paths. |
| Peak memory | `make memory-baselines` | Release CLI peak RSS on synthetic DOCX, PPTX, and XLSX workloads. |
| Competitive workbench | `make competitor-workbench` | Full CLI wall time and peak RSS against optional local extraction tools. |

```bash
cargo bench -p oxdoc-core
make memory-baselines
make competitor-workbench
```

- Criterion throughput benches cover DOCX text extraction, dense XLSX CSV extraction, and shared-string-heavy XLSX extraction.
- Peak-memory baselines measure release CLI RSS on synthetic DOCX, PPTX, and XLSX workloads.
- The competitive workbench compares `oxdoc` with optional local tools such as Apache Tika, `xlsx2csv`, and Mammoth when they are installed.

Comparable tools covered by the workbench:

| Tool | Compared cases | Notes |
| --- | --- | --- |
| Apache Tika | DOCX and PPTX text extraction | Broad document extraction framework; useful as the general-purpose baseline. |
| `xlsx2csv` | XLSX dense, sparse, and shared-string CSV extraction | Closest direct XLSX-to-CSV comparison. |
| Mammoth | DOCX extraction | DOCX-focused converter; output shape differs from plain-text extraction. |

See [Performance and Memory](docs/performance.md), [Peak Memory Baselines](docs/performance-memory-baselines.md), and [Competitive Workbench](docs/performance-competitors.md).

## CLI Usage

For the runtime and usage exit code contract, see [docs/cli.md#exit-codes](docs/cli.md#exit-codes).

### Extract DOCX or PPTX Text

```bash
oxdoc extract text contrato.docx
```

For presentations:

```bash
oxdoc extract text deck.pptx
```

Plain text is written to stdout:

```text
Este es el texto plano extraido velozmente...
```

For integrations:

```bash
oxdoc extract text contrato.docx --format json
```

For streaming batch ingestion:

```bash
oxdoc extract text *.docx --format jsonl
```

For source-aware extraction:

```bash
oxdoc extract text contrato.docx --format structured-json
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

Select by visible sheet order instead:

```bash
oxdoc extract csv data.xlsx --sheet-index 2
```

Inventory hidden sheets and explicitly extract by workbook order when needed:

```bash
oxdoc extract csv data.xlsx --list-sheets --include-hidden
oxdoc extract csv data.xlsx --sheet-index 1 --include-hidden
```

Export every visible sheet to separate CSV files:

```bash
oxdoc extract csv data.xlsx --all-sheets --output-dir exported-sheets
```

Preserve worksheet XML values by default, or opt into deterministic Excel-style formatting for dates, percentages, currency, and decimals:

```bash
oxdoc extract csv data.xlsx --value-mode formatted
```

Output:

```csv
id,nombre,monto
1,Cliente A,5000
```

Notes:

- `--sheet` selects a visible workbook sheet name.
- `--sheet-index` selects a visible workbook sheet by 1-based workbook order.
- `--include-hidden` changes sheet listing and selection to include `visible`, `hidden`, and `veryHidden` sheets in workbook order.
- `--sheet` and `--sheet-index` are mutually exclusive.
- Hidden and very hidden sheets are skipped unless `--include-hidden` is present.
- Duplicate sheet names in the selected visibility scope are rejected; use `--sheet-index` to disambiguate malformed workbooks.
- If no selector is provided, the first visible workbook sheet is used.
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

### Audit Document Signals

```bash
oxdoc audit report.docx --format json
```

Audit output reports factual document intake signals such as macros, custom properties, external hyperlinks and templates, embedded packages and OLE objects, workbook protection, hidden XLSX sheets, and recoverable parser warnings. It does not render, mutate, or assign risk scores to documents.

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
- Data models such as `Extraction<T>`, `DocumentInfo`, `XlsxCsvOptions`, and `XlsxValueMode`.
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
| PPTX text | Slide text boxes and linked speaker notes in presentation order. |
| XLSX CSV | Workbook relationship lookup, sheet name/index selection, hidden-sheet opt-in, shared strings, inline strings, sparse cells, booleans, errors, cached formula values, CSV escaping. |
| Metadata | Core/app properties plus basic macro detection. |
| Audit | Factual signals for macros, custom properties, suspicious relationships, hidden XLSX sheets, and recoverable parser warnings. |
| Output | Plain text, CSV, JSON metadata, JSON text extraction. |
| Errors | Typed library errors, CLI non-zero hard failures. |
| Warnings | Recoverable parser warnings with OOXML part paths. |

## Known Limitations

- DOCX section-aware ordering and some advanced revision/comment semantics need hardening.
- XLSX date and number format interpretation need hardening.
- PPTX extraction does not render slides, synthesize bullets, or preserve visual layout.

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

The test suite combines parser unit tests, fixture corpus tests, and versioned snapshots. Current coverage includes:

- Hand-authored OOXML package trees.
- Application-generated `.docx`, `.xlsx`, and `.pptx` fixtures with provenance.
- Snapshot tests for core parser APIs and CLI output.
- Fuzz targets for XML parser paths.
- Performance benchmarks for throughput and peak memory.
- Optional competitive workbench runs against local extraction CLIs.

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
