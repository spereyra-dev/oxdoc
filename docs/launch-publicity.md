# 1.0 Launch And Publicity

This page is a maintainer checklist and copy bank for publishing `oxdoc` 1.0.

## Launch Goals

- Make `oxdoc` easy to install from GitHub Releases and Cargo.
- Give Rust and CLI users a clear reason to try it: fast OOXML extraction without rendering.
- Point early users toward issues, fixtures, and edge cases that improve parser correctness.
- Keep the launch honest about non-goals: no rendering, no PDF generation, no layout preservation.

## Release Channels

### GitHub Release

Publish `v1.0.0` with generated release notes plus this short top section:

~~~markdown
`oxdoc` 1.0 is the first stable release of a fast OOXML extractor for `.docx`, `.pptx`, and `.xlsx` automation.

It ships:

- `oxdoc extract text` for DOCX/PPTX text extraction
- `oxdoc extract csv` for XLSX-to-CSV workflows
- `oxdoc info` for metadata and macro detection
- stdin, batch extraction, file output, JSON output, and visible sheet listing
- a reusable `oxdoc-core` Rust API
- release binaries for Linux, macOS, Windows, and static Linux

Install:

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | sh
cargo install oxdoc-cli
```
~~~

### Crates.io

Publish in this order:

```bash
cargo publish -p oxdoc-core --dry-run
cargo publish -p oxdoc-cli --dry-run
cargo publish -p oxdoc-core
cargo publish -p oxdoc-cli
```

If `oxdoc-cli` dry-run cannot resolve `oxdoc-core`, publish `oxdoc-core` first, then rerun the `oxdoc-cli` dry-run before publishing the CLI crate.

Crate summary:

> `oxdoc` extracts text, CSV, and metadata from Office Open XML files without rendering. Use it from shell pipelines with the `oxdoc` CLI or embed the parser with `oxdoc-core`.

### Homebrew Tap

After the GitHub Release is public, compute the source tarball checksum and render:

```bash
scripts/render-homebrew-formula.sh v1.0.0 <source-tarball-sha256> > Formula/oxdoc.rb
```

Then publish it in `spereyra-dev/homebrew-tap`.

## Announcement Copy

### Short Social Post

I just released `oxdoc` 1.0: a fast Rust CLI/library for extracting useful data from Office files without rendering them.

It handles DOCX/PPTX text, XLSX-to-CSV, metadata, stdin, batch extraction, JSON output, and visible sheet listing.

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | sh
oxdoc extract text contract.docx
oxdoc extract csv workbook.xlsx --list-sheets
```

Repo: https://github.com/spereyra-dev/oxdoc

### Hacker News / Reddit

Title:

```text
Show HN: oxdoc, a Rust CLI/library for extracting text and CSV from Office files
```

Body:

~~~markdown
I built `oxdoc`, a Rust-based OOXML extractor for automation workflows.

It does not render Office files or try to preserve layout. Instead, it reads the ZIP/XML package directly and emits data that is useful in scripts and ingestion systems:

- DOCX and PPTX text extraction
- XLSX worksheet-to-CSV extraction
- metadata and macro detection
- stdin support with `-`
- batch extraction
- JSON output for text and metadata
- visible sheet listing for workbooks
- typed Rust API through `oxdoc-core`

Install:

```bash
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | sh
cargo install oxdoc-cli
```

Examples:

```bash
oxdoc extract text report.docx
cat deck.pptx | oxdoc extract text -
oxdoc extract csv workbook.xlsx --sheet "Data" -o data.csv
oxdoc info report.docx --format json
```

The project is MIT licensed and I am especially interested in safe-to-share fixtures from different Office producers so the parser can keep getting more reliable.

Repo: https://github.com/spereyra-dev/oxdoc
Docs: https://spereyra-dev.github.io/oxdoc/
~~~

### LinkedIn

```text
I released oxdoc 1.0, a Rust CLI and library for extracting text, CSV, and metadata from Office Open XML files.

The idea is simple: many automation pipelines do not need to render a document. They need predictable text, workbook data, metadata, warnings, and errors that can be handled in scripts or services.

oxdoc supports DOCX/PPTX text extraction, XLSX-to-CSV, metadata and macro detection, stdin, batch extraction, JSON output, file output, and a reusable Rust API.

Install:
curl -fsSL https://raw.githubusercontent.com/spereyra-dev/oxdoc/main/install.sh | sh
cargo install oxdoc-cli

Repo: https://github.com/spereyra-dev/oxdoc
Docs: https://spereyra-dev.github.io/oxdoc/
```

## Outreach Checklist

- Pin the `v1.0.0` GitHub Release.
- Add repository topics: `rust`, `cli`, `docx`, `xlsx`, `pptx`, `ooxml`, `office`, `csv`, `metadata`.
- Publish `oxdoc-core` and `oxdoc-cli` to crates.io.
- Publish or update the Homebrew tap.
- Post the short social copy.
- Post the Show HN / Reddit copy.
- Ask early users for fixtures with provenance notes.
