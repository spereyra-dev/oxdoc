# oxdoc-core

Reusable OOXML extraction library for `oxdoc`.

`oxdoc-core` reads Office Open XML packages such as `.docx`, `.xlsx`, and `.pptx` without rendering pages, slides, or worksheets. It is built for tooling that needs predictable extraction output, typed failures, and recoverable parser warnings.

## Capabilities

- Extract DOCX plain text from the supported document parts.
- Extract structured DOCX tables with source part metadata, spans, merges, and nested table blocks.
- Extract PPTX plain text from slide text boxes and speaker notes.
- Stream XLSX worksheet data to a caller-owned CSV writer.
- Visit sparse, typed XLSX rows through a bounded-memory callback API.
- Read core and app metadata from OOXML packages.
- Read factual audit signals for governance and intake workflows.
- Use path helpers or `Read + Seek` entry points for embedding.
- Return stable error codes through `OxdocError`.
- Return recoverable parser/data warnings alongside successful output.

## Example

```rust
use oxdoc_core::{extract_xlsx_csv, XlsxCsvOptions};

fn main() -> oxdoc_core::Result<()> {
    let mut csv = Vec::new();

    extract_xlsx_csv("data.xlsx", XlsxCsvOptions::default(), &mut csv)?;

    println!("{}", String::from_utf8_lossy(&csv));
    Ok(())
}
```

Typed rows are also streamed one at a time:

```rust
use oxdoc_core::{XlsxRowControl, XlsxSheetOptions, XlsxValueMode};

fn main() -> oxdoc_core::Result<()> {
    oxdoc_core::visit_xlsx_rows(
        "data.xlsx",
        XlsxSheetOptions::default(),
        XlsxValueMode::Raw,
        |row| {
            println!("row {} has {} cells", row.row_index, row.cells.len());
            Ok(XlsxRowControl::Continue)
        },
    )?;

    Ok(())
}
```

## Status

The crate follows semantic versioning from 1.0 onward. Public API changes that break callers should ship in a new major version.

## License

MIT
