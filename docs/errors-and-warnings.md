# Errors and Warnings

`oxdoc` separates hard failures from recoverable parser issues.

## Hard Errors

Hard errors return `OxdocError` from the library and exit non-zero in the CLI.

`OxdocError::code()` exposes the stable error code for callers that need to branch on the failure class.

Current error variants:

| Error | Code | Meaning |
| --- | --- | --- |
| `Io` | `E001` | The input file could not be opened or read. |
| `CorruptedZip` | `E002` | The file is not a readable ZIP/OOXML package. |
| `MissingPart` | `E003` | A required OOXML part is missing. |
| `UnsupportedEncryptedPart` | `E004` | A required OOXML part is encrypted and cannot be read without password support. |
| `PartTooLarge` | `E005` | A ZIP part exceeds the configured uncompressed byte limit. |
| `SuspiciousZipEntry` | `E006` | A ZIP entry has an unsafe shape, such as a directory where a part is required or a zip-bomb-like compression ratio. |
| `SuspiciousRelationshipTarget` | `E007` | A relationship target is external, malformed, or escapes the OOXML package root. |
| `MissingCoreRelations` | `E008` | The package did not provide required relationship information. |
| `MalformedXmlNode` | `E009` | XML parsing failed in a non-recoverable parser path. |
| `InvalidArgument` | `E010` | A caller supplied an invalid argument. |

CLI hard error example:

```text
error[E003]: missing required OOXML part: xl/workbook.xml
```

Unsafe package example:

```text
error[E007]: suspicious OOXML relationship target in _rels/.rels: https://example.invalid/document.xml: external relationship targets are not supported
```

## Recoverable Warnings

Recoverable warnings are returned in `Extraction<T>`:

```rust
pub struct OutputWarning {
    pub path: String,
    pub message: String,
}
```

Warnings also expose stable categories and codes through methods:

```rust
warning.category().as_str(); // "parser" or "data"
warning.code().as_str(); // "W001", "W002", ...
```

CLI warning example:

```text
warning[parser/W001]: word/document.xml: stopped after malformed XML: ...
```

## Exit Codes

| Exit code | Meaning |
| --- | --- |
| `0` | Success. |
| `1` | Runtime extraction or IO error. |
| `2` | CLI usage error reported by `clap`. |

## Policy

- Use hard errors when extraction cannot start or a required part is missing.
- Use warnings when partial extraction can still be useful.
- Include the OOXML part path in warnings.
- Treat encrypted required parts, oversized parts, zip-bomb-like ratios, and unsafe relationship targets as hard errors.
- Keep warnings out of CLI JSON output. They stay on stderr.
- Do not panic on malformed user input.
