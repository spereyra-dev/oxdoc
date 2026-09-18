# DOCX Extraction Specification

## Purpose

Define deterministic, text-safe content extraction from DOCX packages: which
related text parts (headers, footers, footnotes, endnotes, comments) are
emitted, in what order they appear after the main document body, and what
warnings or errors occur when references or parts are missing or malformed.
This spec covers the section-aware ordering of headers and footers introduced
by change `docx-section-order-related-parts`.

## Requirements

### Requirement: Section-aware ordering of header and footer parts

The system MUST order header and footer related parts by the sections that
reference them in `word/document.xml`, not by `word/_rels/document.xml.rels`
relationship-file order. The system MUST walk `w:body` children in document
order to collect sections: a `w:p/w:pPr/w:sectPr` closes a section at that
paragraph position, and the body-level `w:sectPr` (last child of `w:body`) is
the final section. Within each section, the system MUST order references with
headers before footers, and within each kind order variants `first`, `even`,
`default`; ties on the same kind and variant MUST keep `sectPr` element order.
The main document body MUST still be emitted first, unchanged.

#### Scenario: Multi-section document with out-of-order relationships

- GIVEN a DOCX package whose `word/document.xml` has two sections and whose
  `word/_rels/document.xml.rels` lists a footer relationship before a header
  relationship referenced by an earlier section
- WHEN any extraction with related parts runs
- THEN headers and footers are emitted in section document order (headers
  before footers per section), regardless of relationship-file order.

#### Scenario: Variant ordering within one section

- GIVEN a section whose `sectPr` serializes `w:headerReference` elements with
  `w:type` values `default`, `first`, `even` in that attribute order
- WHEN extraction orders that section's references
- THEN the header parts are emitted in `first`, `even`, `default` order.

### Requirement: Deduplication by resolved part path at first reference

The system MUST emit each resolved header/footer part exactly once, at its
first referencing position, deduplicating by resolved package part path. A
part shared by multiple sections (including duplicate `r:id` references within
one section) MUST appear only at its first reference. Two different `r:id`s
with the same kind and variant in one section MUST both be emitted in `sectPr`
element order.

#### Scenario: Part referenced by two sections

- GIVEN two sections whose `sectPr` elements both reference the same default
  header relationship id
- WHEN extraction orders related parts
- THEN the shared header text is emitted once, at the position of the first
  referencing section, and no duplicate blocks appear.

### Requirement: Unreferenced header and footer parts are appended, not dropped

The system MUST append header/footer parts present in
`word/_rels/document.xml.rels` but not referenced by any `sectPr` after all
section-referenced header/footer parts, in relationship-file order, so no
related part silently disappears from output.

#### Scenario: Orphan header part with no sectPr reference

- GIVEN a rels file listing `header3.xml` that no `sectPr` references
- WHEN extraction runs
- THEN `header3.xml` text is emitted after all section-referenced
  headers/footers, in relationship-file order among the orphans.

### Requirement: Footnotes, endnotes, and comments keep relationship order

The system MUST emit footnotes, endnotes, and comments (when
`include_comments` is enabled) after all header and footer parts, and MUST
keep their mutual order identical to relationship-file order. Their placement
relative to headers/footers MAY change (they now follow strictly after), but
their ordering among themselves MUST NOT change.

#### Scenario: Footnotes and comments among themselves

- GIVEN a rels file listing `comments.xml` before `footnotes.xml`
- WHEN extraction runs with comments enabled
- THEN comments text is emitted before footnotes text (relationship order) and
  both are emitted after all headers and footers.

### Requirement: Missing and dangling reference ids produce stable warnings

The system MUST skip a `w:headerReference`/`w:footerReference` without an
`r:id` and emit a warning with the exact text
`skipped DOCX headerReference: missing r:id` or
`skipped DOCX footerReference: missing r:id` (matching the reference kind).
The system MUST skip a reference whose `r:id` is not present in
`word/_rels/document.xml.rels` and emit a warning with the exact text
`skipped DOCX headerReference {rid}: unknown relationship id` or
`skipped DOCX footerReference {rid}: unknown relationship id`. Extraction
MUST continue after either warning. A `w:type` attribute that is missing or
unrecognized MUST be treated as `default` silently, with no warning.

#### Scenario: Dangling relationship id

- GIVEN a `sectPr` referencing `r:id="rIdX"` that is absent from the rels file
- WHEN extraction runs
- THEN extraction succeeds, the reference is skipped, and the warning
  `skipped DOCX headerReference rIdX: unknown relationship id` (or the footer
  equivalent) is emitted.

#### Scenario: Missing r:id attribute

- GIVEN a `w:headerReference` element with no `r:id`
- WHEN extraction runs
- THEN the reference is skipped and the warning
  `skipped DOCX headerReference: missing r:id` is emitted.

#### Scenario: Unrecognized w:type

- GIVEN a `w:headerReference` with `w:type="title"` (or no `w:type`)
- WHEN extraction orders the section references
- THEN the reference is treated as a `default` variant and no warning is
  emitted for the type value.

### Requirement: Existing warning and error behavior is preserved exactly

The system MUST preserve existing warning texts, codes, and behavior exactly:
a referenced part missing from the package emits
`skipped related DOCX text part {path}: missing part` (table variant
`... table part ...`) and extraction continues; malformed XML inside a
referenced part emits the `malformed_xml` warning (W001) with partial text and
other parts still processed; `SuspiciousRelationshipTarget` remains a hard
error for external, escape, or NUL targets; a missing
`word/_rels/document.xml.rels` yields main-part output only with no warning;
the `include_related_parts = false` fast path is untouched. Only the two new
reference warnings defined above may be added.

#### Scenario: Missing referenced part

- GIVEN a header reference whose resolved target part is absent from the
  package
- WHEN extraction runs
- THEN the warning `skipped related DOCX text part {path}: missing part` is
  emitted with the exact existing wording, and remaining parts are still
  extracted.

#### Scenario: Malformed referenced part

- GIVEN a referenced header part containing malformed XML
- WHEN extraction runs
- THEN the W001 `malformed_xml` warning is emitted, whatever text could be
  recovered is included, other parts are still processed, and extraction
  succeeds.

#### Scenario: External target

- GIVEN a relationship with an external target mode
- WHEN extraction runs
- THEN `SuspiciousRelationshipTarget` is raised as a hard error, unchanged.

### Requirement: titlePg is a no-op for output

The system MUST NOT consult `w:titlePg` for ordering or filtering. A
first-variant (`w:type="first"`) header/footer MUST be emitted whenever a
`sectPr` references it, whether or not `titlePg` is set on that section. This
behavior MUST be documented so it is not mistaken for a bug.

#### Scenario: First variant referenced without titlePg

- GIVEN a section referencing a first-page header but with no `w:titlePg`
- WHEN extraction runs
- THEN the first-page header text is emitted in output.

### Requirement: evenAndOddHeaders flag is not consulted (deferred)

The system MUST NOT read `word/settings.xml` or honor `w:evenAndOddHeaders`
for ordering or filtering. Even and odd variant references are still ordered
by their `w:type` label. The deferral MUST be documented in
`docs/formats/docx.md` as deferred for this change.

#### Scenario: evenAndOddHeaders set but not honored

- GIVEN a package with `w:evenAndOddHeaders` enabled in settings.xml and an
  even-variant header reference
- WHEN extraction runs
- THEN the even-variant header is emitted in `first`/`even`/`default` variant
  position without consulting settings.xml, and no new warning appears.

### Requirement: One shared ordering helper across all extraction paths

The system MUST apply the section-aware ordering through a single shared
helper consumed identically by `extract_text`, `extract_structured_text`, and
`extract_tables`, so the three paths cannot drift. All three paths MUST agree
on one fixture oracle for the same package.

#### Scenario: Three paths agree on ordering

- GIVEN the multi-section fixture package
- WHEN `extract_text`, `extract_structured_text`, and `extract_tables` run
- THEN the relative order of related parts is identical across all three
  outputs.

### Requirement: Deterministic output for single-section documents with ordered rels

For single-section documents whose relationship-file order already matches the
section reference order, the change MUST produce output identical to the
previous relationship-order behavior (no change beyond the footnotes/
endnotes/comments repositioning defined above).

#### Scenario: python-docx single-section fixture snapshot holds

- GIVEN the application-generated single-section python-docx fixture with
  rels order matching section order
- WHEN extraction runs
- THEN output content and order are unchanged relative to the previous
  release (snapshot stability), modulo the documented footnote/comment
  repositioning.

### Requirement: No public output schema changes

The system MUST NOT change `part_type` labels (`"main" | "header" | "footer" |
"footnotes" | "endnotes" | "comments"`), MUST NOT add fields to `DocxTable`,
MUST NOT add options to `DocxTextOptions`, and MUST NOT add fields to `TextBlock`
other than the optional `variant` field. Variant labeling, deferred by change
`docx-section-order-related-parts`, is now delivered by change
`structured-text-source-provenance` through the
versioned structured-text contract (`schema_version: 2` and schema
`schemas/v2/oxdoc-structured-text.schema.json`, see the `structured-text-schema`
domain spec); it MUST NOT be emitted by widening the v1 contract. Flat text and
tables output MUST NOT change in any way.

#### Scenario: Output shape unchanged except variant on structured header/footer blocks

- GIVEN any DOCX package before and after this change
- WHEN extraction output is compared structurally
- THEN the set of fields on each table and the allowed `part_type` values are
  identical, `DocxTextOptions` is unchanged, the only new `TextBlock` field is
  the optional `variant`, and flat text and tables output are byte-identical.

#### Scenario: Variant labels are carried by a versioned contract

- GIVEN a DOCX header block that carries a `variant` label
- WHEN the structured-json output is validated
- THEN it validates against `schemas/v2/oxdoc-structured-text.schema.json` with
  top-level `schema_version: 2`, and `schemas/v1/**` remains unmodified for
  validating previously captured v1 outputs.
