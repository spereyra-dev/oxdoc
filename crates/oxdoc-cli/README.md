# oxdoc-cli

Command-line interface for `oxdoc`.

`oxdoc` extracts text, CSV, and metadata from Office Open XML packages. It is designed for shell pipelines and automation: extracted data is written to stdout, recoverable warnings are written to stderr, and hard failures exit non-zero with stable error codes.

## Install From Source

```bash
cargo install oxdoc-cli
```

The installed binary is named `oxdoc`.

## Usage

```bash
oxdoc extract text contract.docx
oxdoc extract text contract.docx --format json
oxdoc extract csv workbook.xlsx --sheet-index 1
oxdoc info report.docx --format json
```

## Status

The CLI is pre-1.0. Output contracts are documented and tested, but new extraction surfaces may still change before the first stable release.

## License

MIT
