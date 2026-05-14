# Product Marketing Context

*Last updated: 2026-05-14*

## Product Overview

**One-liner:** Fast OOXML extraction without rendering.

**What it does:** `oxdoc` extracts useful data from Office Open XML packages: DOCX/PPTX text, XLSX worksheets as CSV, and document metadata. It reads the ZIP/XML container directly and returns predictable CLI or Rust API output for automation workflows.

**Product category:** Developer tool, Rust CLI, document processing library, OOXML parser.

**Product type:** Open source CLI and Rust library.

**Business model:** MIT-licensed open source.

## Target Audience

**Target companies:** Teams that ingest Office documents in scripts, CI, backend jobs, data pipelines, serverless functions, search indexing, compliance tooling, or internal automation.

**Decision-makers:** Developers, platform engineers, data engineers, security/tooling engineers, and technical founders.

**Primary use case:** Extract machine-usable text, CSV, or metadata from Office files without rendering or installing heavy office suites.

**Jobs to be done:**

- Pull plain text from DOCX/PPTX files for search, indexing, inspection, or ingestion.
- Convert XLSX sheets to CSV for scripts and data pipelines.
- Inspect metadata and macro indicators in untrusted Office files.

**Use cases:**

- Shell pipeline extraction.
- Backend document ingestion.
- CI validation of generated Office files.
- Lightweight metadata inspection.
- Rust applications that need embeddable OOXML parsing.

## Personas

| Persona | Cares about | Challenge | Value we promise |
|---------|-------------|-----------|------------------|
| CLI user | Fast install, predictable stdout/stderr, composability | Office files are awkward in shell scripts | Simple commands with JSON/text/CSV output |
| Rust library user | Typed errors, warnings, embeddable APIs | Needs extraction without shelling out to LibreOffice | `oxdoc-core` APIs with `Read + Seek` support |
| Data engineer | Batch processing, CSV output, sheet selection | Workbooks vary by sheet name/order | Sheet listing, selection, batch extraction, file output |
| Security/tooling engineer | Safe parsing of untrusted files | Office files can be malformed, encrypted, or huge | Bounded ZIP reads, suspicious relationship checks, warnings |

## Problems & Pain Points

**Core problem:** Extracting useful data from Office files often means using heavyweight renderers, brittle scripts, or libraries that do more than the automation actually needs.

**Why alternatives fall short:**

- Office suites are heavy dependencies for serverless/CI environments.
- Rendering-oriented tools preserve layout but add complexity when only data is needed.
- Ad hoc XML scraping breaks on relationships, namespaces, sparse worksheets, malformed XML, and producer differences.

**What it costs them:** Slow jobs, fragile pipelines, bigger containers, hard-to-debug parsing failures, and inconsistent output contracts.

**Emotional tension:** Users want the boring dependable tool: install it, point it at files, get parseable output, move on.

## Competitive Landscape

**Direct:** OOXML parsing crates and document extraction CLIs. They may focus on one format, expose lower-level APIs, or lack a polished CLI contract.

**Secondary:** LibreOffice/headless office conversion. Powerful, but heavy when the task is extraction rather than rendering.

**Indirect:** Custom scripts over zipped XML. Quick to start, brittle across real-world files.

## Differentiation

**Key differentiators:**

- One tool for DOCX/PPTX text, XLSX CSV, and metadata.
- Does not render, which keeps the scope focused and automation-friendly.
- CLI and Rust API share the same parser core.
- Stdin, batch extraction, output files, JSON output, and visible sheet listing are first-class.
- Recoverable warnings are explicit and kept out of machine-readable stdout.

**How we do it differently:** `oxdoc` reads the OOXML package structure directly: ZIP entries, content types, relationships, XML parser state machines, and typed warnings/errors.

**Why that's better:** It is lighter than a renderer and safer than one-off XML scraping.

**Why customers choose us:** They need predictable extraction, not visual fidelity.

## Objections

| Objection | Response |
|-----------|----------|
| Does it render documents or generate PDFs? | No. That is an explicit non-goal; use a renderer when visual fidelity matters. |
| Does it fully implement OOXML? | No. It targets useful extraction paths and documents limitations. |
| Can I trust it with untrusted files? | It rejects encrypted parts, oversized parts, zip-bomb-like ratios, and suspicious relationship targets; malformed XML returns warnings where possible. |

**Anti-persona:** Users who need exact rendered layout, PDF conversion, office document editing, formula recalculation, or complete OOXML fidelity.

## Switching Dynamics

**Push:** Heavy dependencies, fragile extraction scripts, inconsistent output, awkward CI/serverless installs.

**Pull:** Fast Rust binary, simple CLI, embeddable API, documented contracts, release binaries.

**Habit:** Existing LibreOffice or Python scripts already work “well enough.”

**Anxiety:** Parser completeness across real-world Office producers; whether edge cases are documented and handled predictably.

## Customer Language

**How they describe the problem:**

- "I just need the text from this docx."
- "I need a CSV out of this workbook in CI."
- "I do not want to install LibreOffice in this container."

**How they describe us:**

- "Fast OOXML extraction without rendering."
- "A small CLI for Office files in scripts."

**Words to use:** fast, predictable, lightweight, extraction, no rendering, shell pipelines, Rust API, typed errors, recoverable warnings.

**Words to avoid:** renderer, office replacement, full OOXML implementation, PDF converter.

**Glossary:**

| Term | Meaning |
|------|---------|
| OOXML | Office Open XML ZIP/XML document format used by DOCX, XLSX, and PPTX |
| Extraction | Reading useful data from the package without rendering visual layout |
| Recoverable warning | Parser/data issue that is reported while still returning partial useful output |

## Brand Voice

**Tone:** Technical, direct, practical.

**Style:** Clear examples first, implementation detail when useful.

**Personality:** Fast, trustworthy, focused, automation-friendly.

## Proof Points

**Metrics:** CI enforces 95% line coverage. Release workflow builds Linux, macOS, Windows, static Linux artifacts, and checksums.

**Customers:** Early open source users and contributors.

**Testimonials:** None yet.

**Value themes:**

| Theme | Proof |
|-------|-------|
| Automation-ready | stdout/stderr contract, JSON output, stdin, batch extraction |
| Safe parsing | ZIP size checks, relationship validation, encrypted-part rejection |
| Embeddable | `oxdoc-core` exposes path and `Read + Seek` APIs |
| Practical formats | DOCX/PPTX text, XLSX CSV, metadata |

## Goals

**Business goal:** Grow awareness and usage for the 1.0 open source release.

**Conversion action:** Star the repo, install the CLI, try `oxdoc-core`, file issues with safe fixtures.

**Current metrics:** GitHub Release `v0.1.0` exists; 1.0 release and crates.io publication are next.
