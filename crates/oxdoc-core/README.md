# oxdoc-core

Reusable OOXML extraction library for `oxdoc`.

`oxdoc-core` reads Office Open XML packages such as `.docx`, `.xlsx`, and `.pptx` metadata without rendering pages, slides, or worksheets. It is built for tooling that needs predictable extraction output, typed failures, and recoverable parser warnings.

## Capabilities

- Extract DOCX plain text from the supported document parts.
- Stream XLSX worksheet data to a caller-owned CSV writer.
- Read core and app metadata from OOXML packages.
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

## Status

The crate is pre-1.0. API names and behavior may still change while `oxdoc` hardens parser coverage, memory boundaries, and embedding contracts.

## License

MIT
