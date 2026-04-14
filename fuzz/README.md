# Fuzzing

This directory contains `cargo-fuzz` harnesses for the XML parser entry points in `oxdoc-core`.

Build one target:

```bash
cargo fuzz build docx_text
```

Run one target:

```bash
cargo fuzz run xlsx_sheet
```
