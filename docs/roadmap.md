# Roadmap

The Arrow/Parquet architecture and benchmark spike is documented in
[XLSX Arrow and Parquet Spike](spikes/xlsx-arrow-parquet.md).

The generic batch-manifest proposal was evaluated and rejected in
[Cross-format Batch Manifest Decision](spikes/batch-manifest.md); existing
operation-specific JSONL contracts remain the supported batch boundary.

The structured-data initiative tracked in issue #127 is complete. Issues
#117–#125 and #128–#131 delivered typed XLSX streaming and JSONL, deterministic
schema inference, worksheet-scoped limits, optional Arrow/Parquet conversion
with DuckDB/PyArrow gates, structured DOCX tables, batch audit JSONL, and
expanded factual audit signals. Issue #126 closed with a documented no-go
decision rather than an implementation.

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

- Add benchmarks for cold start and additional throughput scenarios.
- Expand large XLSX scenarios with inline strings and larger real-world peak-memory probes.
- Add configurable memory and temporary-file policies for high-volume XLSX extraction.
- Keep the competitive workbench current as comparable OOXML extraction tools change.
- Keep peak-memory baseline numbers current in docs and release notes.

## Phase 4: Public API

- Stabilize `oxdoc-core` APIs for embedding in Rust applications.
- Maintain the completed structured XLSX APIs and publishable, opt-in
  `oxdoc-tabular` Arrow/Parquet adapter without changing the default CLI graph.
- Document error types, warning behavior, and streaming sinks.
- Add examples for library consumers.
- Maintain the pure-Python CLI wrapper as the first data-team integration path.
- Evaluate optional WASM, native Python bindings, and FFI boundaries after demand is clear.

## Phase 5: Release Engineering

- Publish versioned binaries for Linux, macOS, and Windows. Implemented through the GitHub Release workflow.
- Publish static Linux builds for `x86_64-unknown-linux-musl`. Implemented through the GitHub Release workflow.
- Add signed checksums to GitHub Releases.
- Publish `oxdoc-core`, `oxdoc-tabular`, and `oxdoc-cli` to crates.io in
  dependency order for the next release.

## Non-Goals

- Rendering pages, slides, or worksheets.
- Generating PDF output.
- Preserving fonts, margins, colors, layout, or pagination.
- Implementing the full OOXML specification.
- Mutating or repairing input documents.
