# Roadmap

This roadmap is intentionally practical: `oxdoc` should become a reliable, fast, embeddable OOXML extractor before it grows a large feature surface.

## Project Principles

- Extract useful data; do not render documents.
- Be tolerant on input and strict on output.
- Keep memory bounded for large Office files.
- Prefer streaming parsers and explicit data contracts.
- Make regressions visible through fixtures, snapshots, benchmarks, and fuzzing.

## Phase 0: OSS Baseline

- Keep license, contribution, security, support, and issue templates in the repository.
- Document project scope and non-goals clearly.
- Run formatting, linting, and tests in CI for every PR.
- Track dependency updates with Dependabot.

## Phase 1: MVP Hardening

- Improve DOCX extraction beyond the main document body. Headers, footers, footnotes, endnotes, comments, hyperlink visible text, and deterministic related-part ordering are implemented; section-aware ordering remains future work.
- Improve XLSX CSV extraction for sparse dimensions, date/number formatting, and larger workbooks. Visible sheet selection by name or 1-based index, booleans, errors, cached formula values, and bounded shared-string storage are implemented in the MVP path.
- Add PPTX text extraction for slide text boxes and speaker notes.
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

- Publish versioned binaries for Linux, macOS, and Windows.
- Publish static Linux builds for `x86_64-unknown-linux-musl`.
- Add signed checksums to GitHub Releases.
- Decide whether and when to publish crates to crates.io.

## Non-Goals

- Rendering pages, slides, or worksheets.
- Generating PDF output.
- Preserving fonts, margins, colors, layout, or pagination.
- Implementing the full OOXML specification.
- Mutating or repairing input documents.
