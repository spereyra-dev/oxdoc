# Architecture

`oxdoc` is a targeted OOXML extractor. It reads ZIP-based Office containers and extracts plain text, CSV, and metadata without rendering layout.

## Workspace Layout

```text
.
â”œâ”€â”€ crates
â”‚   â”œâ”€â”€ oxdoc-cli
â”‚   â””â”€â”€ oxdoc-core
â”œâ”€â”€ docs
â”œâ”€â”€ .github
â””â”€â”€ tests
```

## Crates

### `oxdoc-core`

Responsibilities:

- Open OOXML ZIP packages.
- Resolve package relationships.
- Parse XML parts with `quick-xml`.
- Expose extraction models.
- Return typed errors and recoverable warnings.

Important modules:

| Module | Responsibility |
| --- | --- |
| `vfs` | ZIP-backed virtual file system abstraction. |
| `parsers` | DOCX, XLSX, metadata, relationship, and shared XML helpers. |
| `models` | Public data structures returned by the library. |
| `error` | `OxdocError` and `Result<T>`. |

### `oxdoc-cli`

Responsibilities:

- Parse commands with `clap`.
- Route commands to `oxdoc-core`.
- Write extraction output to stdout.
- Write warnings and hard errors to stderr.
- Format JSON with `serde_json`.

## Data Flow

```text
CLI args
  -> open input file
  -> OoxmlPackage
  -> relationship discovery
  -> streaming XML parser
  -> extraction model or writer sink
  -> stdout

recoverable parser issue
  -> OutputWarning
  -> stderr in the CLI
```

## Relationship Discovery

OOXML packages use relationship files to point to the main document or workbook part. `oxdoc` checks `_rels/.rels` and falls back to conventional paths such as `word/document.xml` and `xl/workbook.xml`.

## Parser Strategy

- Use event-based XML parsing instead of DOM parsing.
- Keep parser state explicit.
- Emit partial results when malformed XML appears after useful content.
- Keep large-output flows writer-based where possible.

## Output Strategy

The library returns `Extraction<T>`:

```rust
pub struct Extraction<T> {
    pub value: T,
    pub warnings: Vec<OutputWarning>,
}
```

The CLI then decides whether `value` becomes plain text, CSV, JSON, or human-readable text.
