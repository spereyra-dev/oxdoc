# Getting Started

This page walks through a local checkout and the three primary extraction flows.

## Clone

```bash
git clone https://github.com/spereyra-dev/oxdoc.git
cd oxdoc
```

## Build

```bash
cargo build --workspace
```

The CLI binary is named `oxdoc` and is provided by the `oxdoc-cli` crate.

```bash
cargo run -p oxdoc-cli -- --help
```

## Extract DOCX Text

```bash
cargo run -p oxdoc-cli -- extract text contrato.docx
```

Default output is plain text on stdout. Recoverable parser warnings are written to stderr.

JSON text output is available for integrations:

```bash
cargo run -p oxdoc-cli -- extract text contrato.docx --format json
```

## Convert XLSX to CSV

```bash
cargo run -p oxdoc-cli -- extract csv data.xlsx
```

Select a sheet by visible workbook name:

```bash
cargo run -p oxdoc-cli -- extract csv data.xlsx --sheet "Ventas Q1"
```

Use a different single-byte delimiter:

```bash
cargo run -p oxdoc-cli -- extract csv data.xlsx --delimiter ";"
```

## Read Metadata

```bash
cargo run -p oxdoc-cli -- info report.docx --format json
```

Text output is also available:

```bash
cargo run -p oxdoc-cli -- info report.docx --format text
```

## Run Checks

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Or use the Makefile:

```bash
make all
```
