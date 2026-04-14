# Errors and Warnings

`oxdoc` separates hard failures from recoverable parser issues.

## Hard Errors

Hard errors return `OxdocError` from the library and exit non-zero in the CLI.

Current error variants:

| Error | Meaning |
| --- | --- |
| `Io` | The input file could not be opened or read. |
| `CorruptedZip` | The file is not a readable ZIP/OOXML package. |
| `MissingPart` | A required OOXML part is missing. |
| `MissingCoreRelations` | The package did not provide required relationship information. |
| `MalformedXmlNode` | XML parsing failed in a non-recoverable parser path. |
| `InvalidArgument` | A caller supplied an invalid argument. |

CLI hard error example:

```text
error: missing required OOXML part: xl/workbook.xml
```

## Recoverable Warnings

Recoverable warnings are returned in `Extraction<T>`:

```rust
pub struct OutputWarning {
    pub path: String,
    pub message: String,
}
```

CLI warning example:

```text
warning: word/document.xml: stopped after malformed XML: ...
```

## Policy

- Use hard errors when extraction cannot start or a required part is missing.
- Use warnings when partial extraction can still be useful.
- Include the OOXML part path in warnings.
- Do not panic on malformed user input.
