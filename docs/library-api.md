# Library API

`oxdoc-core` exposes the reusable Rust API. The CLI is a consumer of this crate.

From 1.0 onward, public API changes follow semantic versioning.

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

### `extract_pptx_text`

```rust
use oxdoc_core::extract_pptx_text;

fn main() -> oxdoc_core::Result<()> {
    let extraction = extract_pptx_text("deck.pptx")?;
    println!("{}", extraction.value);
    Ok(())
}
```

Returns:

```rust
Extraction<String>
```

The text value includes slide text boxes in presentation order and linked speaker notes after their slide text.

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

### `read_audit`

```rust
use oxdoc_core::read_audit;

fn main() -> oxdoc_core::Result<()> {
    let extraction = read_audit("report.docx")?;
    for signal in extraction.value.signals {
        println!(
            "{} {} {}: {}",
            signal.severity, signal.kind, signal.path, signal.message
        );
    }
    Ok(())
}
```

Returns:

```rust
Extraction<DocumentAudit>
```

Audit signals are factual findings for intake and governance workflows, such as macros, custom properties, hidden XLSX sheets, suspicious relationship targets, and recoverable parser warnings.

## `Read + Seek` Entry Points

Embedding applications can pass any `Read + Seek` source, such as `std::fs::File`, `std::io::Cursor<Vec<u8>>`, or a seekable in-memory buffer:

```rust
use std::io::Cursor;

use oxdoc_core::{
    extract_docx_text_from_reader, extract_pptx_text_from_reader, extract_xlsx_csv_from_reader,
    read_audit_from_reader, read_info_from_reader, XlsxCsvOptions,
};

fn main() -> oxdoc_core::Result<()> {
    let docx_bytes = std::fs::read("contrato.docx")?;
    let text = extract_docx_text_from_reader(Cursor::new(docx_bytes))?;
    println!("{}", text.value);

    let pptx_bytes = std::fs::read("deck.pptx")?;
    let slide_text = extract_pptx_text_from_reader(Cursor::new(pptx_bytes))?;
    println!("{}", slide_text.value);

    let xlsx = std::fs::File::open("data.xlsx")?;
    let mut csv = Vec::new();
    extract_xlsx_csv_from_reader(xlsx, XlsxCsvOptions::default(), &mut csv)?;

    let info_bytes = std::fs::read("deck.pptx")?;
    let info = read_info_from_reader(Cursor::new(info_bytes), "deck.pptx")?;
    println!("{:#?}", info.value);

    let audit_bytes = std::fs::read("report.docx")?;
    let audit = read_audit_from_reader(Cursor::new(audit_bytes), "report.docx")?;
    println!("{:#?}", audit.value.signals);

    Ok(())
}
```

The reader APIs return the same `Extraction<T>` and `OxdocError` values as the path helpers. `read_info_from_reader` and `read_audit_from_reader` require a display file name because no filesystem path is available for deriving output file fields.

### Document type and sheets

```rust
use oxdoc_core::{detect_document_type, list_xlsx_sheets, DocumentType};

fn main() -> oxdoc_core::Result<()> {
    match detect_document_type("renamed-package.bin")? {
        DocumentType::Docx => println!("word document"),
        DocumentType::Pptx => println!("presentation"),
        DocumentType::Xlsx => {
            for sheet in list_xlsx_sheets("renamed-package.bin")?.value {
                println!("{}: {}", sheet.index, sheet.name);
            }
        }
        DocumentType::Unknown => println!("unknown OOXML package"),
    }

    Ok(())
}
```

`detect_document_type` inspects `[Content_Types].xml`, so it works even when a package has no useful filename extension. `list_xlsx_sheets` reports visible sheets with 1-based indices without opening worksheet data.

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

`extract_xlsx_csv` uses raw worksheet XML values. Use `extract_xlsx_csv_with_value_mode` or `extract_xlsx_csv_from_reader_with_value_mode` with `XlsxValueMode::Formatted` to apply supported workbook number formats for dates, times, percentages, currency, and decimals with locale-independent output.

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

### `DocumentAudit`

```rust
pub struct DocumentAudit {
    pub file: String,
    pub document_type: String,
    pub metadata: DocumentInfo,
    pub signals: Vec<AuditSignal>,
}
```

### `AuditSignal`

```rust
pub struct AuditSignal {
    pub kind: String,
    pub severity: String,
    pub path: String,
    pub message: String,
}
```
