# Library API

`oxdoc-core` exposes the reusable Rust API. The CLI is a consumer of this crate.

The API is early and may change before the first stable release.

## Public Functions

Runnable examples live under `crates/oxdoc-core/examples/` and are compiled by the standard workspace test target.

### `extract_docx_text`

```rust
use oxdoc_core::extract_docx_text;

fn main() -> oxdoc_core::Result<()> {
    let extraction = extract_docx_text("contrato.docx")?;
    println!("{}", extraction.value);

    for warning in extraction.warnings {
        eprintln!(
            "warning[{}/{}]: {}: {}",
            warning.category().as_str(),
            warning.code().as_str(),
            warning.path,
            warning.message
        );
    }

    Ok(())
}
```

Returns:

```rust
Extraction<String>
```

### `extract_xlsx_csv`

```rust
use oxdoc_core::{extract_xlsx_csv, XlsxCsvOptions};

fn main() -> oxdoc_core::Result<()> {
    let mut output = Vec::new();

    let extraction = extract_xlsx_csv(
        "data.xlsx",
        XlsxCsvOptions {
            sheet_name: Some("Ventas Q1"),
            sheet_index: None,
            delimiter: b',',
        },
        &mut output,
    )?;

    for warning in extraction.warnings {
        eprintln!(
            "warning[{}/{}]: {}: {}",
            warning.category().as_str(),
            warning.code().as_str(),
            warning.path,
            warning.message
        );
    }

    println!("{}", String::from_utf8_lossy(&output));
    Ok(())
}
```

Returns:

```rust
Extraction<()>
```

The caller owns the writer. This keeps the CSV path stream-friendly.

### `read_info`

```rust
use oxdoc_core::read_info;

fn main() -> oxdoc_core::Result<()> {
    let extraction = read_info("report.docx")?;
    println!("{:#?}", extraction.value);
    Ok(())
}
```

Returns:

```rust
Extraction<DocumentInfo>
```

## Core Types

### `Extraction<T>`

```rust
pub struct Extraction<T> {
    pub value: T,
    pub warnings: Vec<OutputWarning>,
}
```

This separates successful extraction output from recoverable parser warnings.

### `OutputWarning`

```rust
pub struct OutputWarning {
    pub path: String,
    pub message: String,
}
```

`path` is the OOXML part that produced the warning, such as `word/document.xml`.

`OutputWarning` also exposes stable classification helpers:

```rust
warning.category().as_str(); // "parser", "data", ...
warning.code().as_str(); // "W001", "W002", ...
```

The current CLI writes these warnings to stderr and keeps them out of JSON output.

### `XlsxCsvOptions`

```rust
pub struct XlsxCsvOptions<'a> {
    pub sheet_name: Option<&'a str>,
    pub sheet_index: Option<usize>,
    pub delimiter: u8,
}
```

`sheet_name` selects a visible workbook sheet by name. `sheet_index` selects a visible workbook sheet by 1-based workbook order. Set at most one selector; when both are `None`, extraction uses the first visible sheet.

Hidden and very hidden sheets are skipped by selection. Duplicate visible sheet names return an invalid-argument error so callers can retry with `sheet_index`.

Defaults:

```rust
XlsxCsvOptions {
    sheet_name: None,
    sheet_index: None,
    delimiter: b',',
}
```

### `DocumentInfo`

```rust
pub struct DocumentInfo {
    pub file: String,
    pub author: Option<String>,
    pub last_modified_by: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub application: Option<String>,
    pub company: Option<String>,
    pub has_macros: bool,
    pub word_count: Option<u64>,
    pub page_count: Option<u64>,
    pub slide_count: Option<u64>,
    pub worksheet_count: Option<u64>,
    pub revision: Option<String>,
}
```
