# corpus/xlsx/formatted-locale

- Source: repository-authored OOXML fixture using date, elapsed-time, localized decimal, and currency format codes interoperable with Excel and LibreOffice.
- Producer: no application binary; the compact package is assembled by `tests/fixtures/mod.rs` for deterministic parser coverage.
- Redistribution: permitted; all workbook values and XML are authored in this repository.
- Purpose: formatted XLSX regression coverage for locale directives, non-US currency symbols, elapsed time, and predictable raw fallbacks.
- Sanitization: no external links, macros, remote data, or personal data are included.
