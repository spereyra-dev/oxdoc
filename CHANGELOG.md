# Changelog

All notable changes to this project will be documented in this file.

The format is based on human-readable release notes. This project has not published a versioned release yet.

## Unreleased

### Added

- Initial Rust workspace with `oxdoc-core` and `oxdoc-cli`.
- DOCX text extraction from the main document part.
- XLSX CSV extraction with shared strings and sparse cell padding.
- Metadata extraction from `docProps/core.xml` and `docProps/app.xml`.
- Basic CI, Makefile, and OSS project files.
- Docsify documentation site with usage, architecture, API, testing, roadmap, and security pages.
- Makefile and CI coverage gate with `cargo-llvm-cov` at 95% line coverage.
- XLSX CSV visible sheet selection by name or 1-based index, with explicit hidden-sheet and duplicate-name behavior.
- Crate-local READMEs, core API examples, and crates.io dry-run publishing guidance.
- Versioned JSON schemas for `info` and `extract text` machine-readable output.
- Additional CLI integration coverage for warning isolation and XLSX sheet-selection failures.
- Security advisory automation with RustSec scanning and local `make audit` support.
