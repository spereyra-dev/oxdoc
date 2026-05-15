# oxdoc-cli

Command-line interface for `oxdoc`.

`oxdoc` extracts text, CSV, and metadata from Office Open XML packages. It is designed for shell pipelines and automation: extracted data is written to stdout, recoverable warnings are written to stderr, and hard failures exit non-zero with stable error codes.

## Install

From GitHub Releases on macOS/Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | sh
```

From crates.io after publication:

```bash
cargo install oxdoc-cli
```

The installed binary is named `oxdoc`.

From a local checkout:

```bash
cargo install --path crates/oxdoc-cli
```

## Usage

```bash
oxdoc extract text contract.docx
oxdoc extract text contract.docx --format json
oxdoc extract text *.docx --format jsonl
oxdoc extract text contract.docx --format structured-json
oxdoc extract text *.docx -o combined.txt
oxdoc extract csv workbook.xlsx --sheet-index 1
oxdoc extract csv workbook.xlsx --list-sheets
oxdoc extract csv workbook.xlsx --list-sheets --include-hidden
oxdoc extract csv workbook.xlsx --all-sheets --output-dir sheets
oxdoc extract csv workbook.xlsx --value-mode formatted
cat contract.docx | oxdoc extract text -
oxdoc info report.docx --format json
oxdoc audit report.docx --format json
```

## Status

The CLI follows semantic versioning from 1.0 onward. Output contracts are documented and tested so shell scripts can depend on them.

## License

MIT
