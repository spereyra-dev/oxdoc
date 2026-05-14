# Roadmap

The roadmap is intentionally practical: `oxdoc` should become a reliable, fast, embeddable OOXML extractor before it grows a large feature surface.

## Phase 0: OSS Baseline

Status: in progress.

- License, contribution, security, support, governance, and issue templates.
- Docsify documentation site.
- CI for format, lint, and tests.
- Dependabot for Rust and GitHub Actions updates.

## Phase 1: 1.0 Hardening

- Improve DOCX extraction beyond the main document body. Headers, footers, footnotes, endnotes, comments, hyperlink visible text, and deterministic related-part ordering are implemented; section-aware ordering remains future work.
- Improve XLSX CSV extraction for sparse dimensions, date/number formatting, and larger workbooks. Visible sheet selection by name or 1-based index, booleans, errors, cached formula values, and bounded shared-string storage are implemented in the current path.
- Harden PPTX text extraction beyond the current slide text box and speaker notes path.
- Expand metadata coverage across DOCX, XLSX, and PPTX.
- Keep warnings structured and actionable.

## Phase 2: Correctness Corpus

- Keep the checked-in hand-authored and application-generated OOXML corpus growing across DOCX, XLSX, and PPTX.
- Maintain snapshot tests for CLI output and parser output.
- Expand corrupt ZIP/XML fixtures to verify partial extraction and warning behavior.
- Document fixture provenance so contributors can add Microsoft Office, LibreOffice, Google Workspace, and third-party exporter cases safely.

## Phase 3: Performance and Memory

- Add benchmarks for cold start, throughput, and peak memory.
- Expand large XLSX scenarios with shared strings, inline strings, sparse rows, and peak-memory probes.
- Add configurable memory and temporary-file policies for high-volume XLSX extraction.
- Publish baseline benchmark numbers in release notes.

## Phase 4: Public API

- Stabilize `oxdoc-core` APIs for embedding in Rust applications.
- Document error types, warning behavior, and streaming sinks.
- Add examples for library consumers.
- Evaluate optional WASM and FFI boundaries after the Rust API is stable.

## Phase 5: Release Engineering

- Publish versioned binaries for Linux, macOS, and Windows. Implemented through the GitHub Release workflow.
- Publish static Linux builds for `x86_64-unknown-linux-musl`. Implemented through the GitHub Release workflow.
- Add signed checksums to GitHub Releases.
- Publish `oxdoc-core` and `oxdoc-cli` to crates.io for 1.0.

## Non-Goals

- Rendering pages, slides, or worksheets.
- Generating PDF output.
- Preserving fonts, margins, colors, layout, or pagination.
- Implementing the full OOXML specification.
- Mutating or repairing input documents.
