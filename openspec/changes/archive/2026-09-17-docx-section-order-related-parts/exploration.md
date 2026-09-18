# Exploration — docx: order headers and footers by section (issue #177)

- Change id: `docx-section-order-related-parts`
- Status: explored 2026-06 (SDD explore phase, artifact store: openspec)
- Inputs: GitHub issue #177 acceptance criteria (verbatim, below); current code on `main` after PR #199.

## Acceptance criteria (verbatim)

1. Define and document deterministic section-aware ordering semantics.
2. Cover first/even/default header/footer variants and missing/malformed references.
3. Add hand-authored and application-generated fixtures with provenance.
4. Preserve warnings and partial-extraction behavior.

## 1. Where related parts are enumerated today

All in `crates/oxdoc-core/src/parsers/docx.rs`:

- `extract_text` (text), `extract_structured_text` (blocks), and `extract_tables`
  each follow the same shape:
  1. Find the main part via `find_office_document_path(package, "word/document.xml")`
     (`crates/oxdoc-core/src/parsers/mod.rs`), which follows the root
     `officeDocument` relationship in `_rels/.rels` with a `word/document.xml` fallback.
  2. If `options.include_related_parts` is false, return early (no rels read at all).
  3. Read `word/_rels/document.xml.rels` (path via `rels_path_for`); if the rels
     part is missing, return the main-part result silently (no warning).
  4. Iterate `parse_relationships(...)` results **in relationship file order**
     and, for each relationship whose `Type` ends with `/header`, `/footer`,
     `/footnotes`, `/endnotes` (or `/comments` when `include_comments`), resolve
     the target with `resolve_relationship_target(parent_dir(&document_path), ...)`
     and extract text/tables, appending after the main part.
- Part-type classification: `related_docx_text_part_type` returns the string
  labels `"header" | "footer" | "footnotes" | "endnotes" | "comments"`. There is
  **no header/footer variant discrimination** today — every header maps to the
  same `"header"` label regardless of `w:type`.
- Output ordering semantics:
  - Flat text: main body first, then each related part appended with
    `append_related_text` (newline-joined) in enumeration order.
  - Structured text: `push_text_block` assigns 1-based `TextBlock.ordinal` by
    encounter order; `part_path` is the normalized package path.
  - Tables: `public_tables_for_part` renumbers `table_ordinal` per part, and
    tables extend in enumeration order. Each `DocxTable` carries
    `part_type` + `part_path` but no variant/section info.
- Failure behavior (must be preserved per AC 4):
  - Missing related part → warning
    `"skipped related DOCX text part {path}: missing part"` (tables:
    `"... table part ..."`), extraction continues.
  - Malformed XML inside a part → `OutputWarning::malformed_xml` (W001) and
    partial text; other parts still processed.
  - External/escape/NUL targets → `OxdocError::SuspiciousRelationshipTarget`
    (hard error, unchanged).
  - Missing `document.xml.rels` → main-part output only, no warning.

Shared helpers in `parsers/mod.rs`: `parse_relationships` preserves document
order of `<Relationship>` elements; `resolve_relationship_target` enforces
package-internal targets only.

## 2. How section properties appear in document.xml (current support: none)

- `grep` for `sectPr|headerReference|footerReference|titlePg` across the
  repository: **zero matches**. The parser never reads `w:body/w:sectPr` or
  `w:pPr/w:sectPr` (section breaks mid-body create per-paragraph `sectPr`).
- OOXML facts the design must handle:
  - References: `w:headerReference`/`w:footerReference` with `w:type`
    (`default` | `even` | `first`) and `r:id` linking to
    `word/_rels/document.xml.rels` relationship ids.
  - `w:titlePg` enables the first-page header/footer for that section.
  - `w:evenAndOddHeaders` (settings.xml) enables even variants globally; it
    lives in `word/settings.xml`, a part the DOCX text extractor currently
    never reads.
  - Multiple sections are legal; final `sectPr` sits at the end of `w:body`,
    earlier sections attach to the last paragraph of the section.
  - Word fallback semantics when a variant is referenced but absent: Word
    inherits (default header used when even/first is missing). Section-aware
    ordering must decide whether to emulate that inheritance or emit each
    referenced part once.

## 3. Current output ordering semantics (documented)

- `docs/formats/docx.md` "Logical Text Contract" row *Related text parts*:
  "main document body is emitted first, then headers, footers, footnotes,
  endnotes, and comments are appended in `word/_rels/document.xml.rels`
  relationship order. Missing related parts are skipped with a warning."
- Same statement duplicated for tables under "Structural Table Model" →
  Decisions → Related parts.
- The doc already lists the change under **Planned Improvements**:
  "Section-aware ordering for headers and footers." Docs will need both rows
  rewritten plus a new determinism statement.

## 4. Fixtures today

Hand-authored (unpackaged XML + JSON oracle), `tests/fixtures/docx/`:
- `related-parts/` — complete package tree (`document.xml`, one
  `header1.xml`, `footer1.xml`, `footnotes.xml`, `endnotes.xml`,
  `comments.xml`; rels deliberately **out of order**). Its `expected.json`
  encodes current relationship-order output; **it will need a new oracle or a
  new fixture** once ordering changes (it is a design oracle, not parser
  output). Provenance: `tests/fixtures/provenance/docx-related-parts-tables.md`.
- `table-semantics/`, `malformed-table/` — not affected.
- Read by `docx.rs` unit tests (`read_fixture`) and via
  `tests/fixtures/mod.rs::build_package` (deterministic sorted ZIP, Stored
  compression) in `crates/oxdoc-core/tests/api.rs`.

Corpus packages (sorted-ZIP by `build_package`), `tests/fixtures/corpus/docx/`:
- `basic/`, `external-target/`, `policies/` — none contain headers, footers,
  or `sectPr` references. `policies/` covers the PR #199 policy flags.

Application-generated: `tests/fixtures/files/docx/python-docx-basic.docx`
(regenerable via `tests/fixtures/tools/generate_application_fixtures.py`;
python-docx 1.2.0; provenance record `docx-python-docx-basic.md` with SHA-256).
It exercises a default header/footer but only a single section with no variant
references.

Test consumption paths:
- `crates/oxdoc-core/tests/api.rs`: `extracts_docx_text_from_related_parts_in_relationship_order`,
  `extracts_docx_tables_from_related_parts_in_relationship_order`,
  `extracts_docx_structured_text_blocks_with_related_part_sources`,
  `warns_on_missing_docx_related_part`,
  `keeps_partial_related_docx_text_and_warns_on_malformed_part`,
  `rejects_external_docx_related_part_targets`,
  `skips_unrelated_docx_parts_and_reports_missing_parts_for_every_extraction`,
  `omits_related_parts_from_docx_structured_text_and_tables`.
  Tests build packages inline with `create_ooxml` and via `build_package`.
- `crates/oxdoc-cli/tests/cli.rs` + snapshots in `tests/fixtures/snapshots/`
  (e.g. `docx_python_docx_text.txt`) pin CLI output for the python-docx fixture.

## 5. Gaps vs acceptance criteria

| AC | Current state | Gap |
| --- | --- | --- |
| 1. Deterministic section-aware ordering semantics | Pure relationship-enumeration order; docs say so | No `sectPr` parsing, no ordering rule, no docs. Need a defined rule (proposal phase): candidate — emit main, then per first-referencing-section pass over `sectPr` elements in document order, headers before footers, variants in `first`, `even`, `default` order, then any related header/footer part not referenced by any `sectPr` (in rels order) so nothing silently disappears; footnotes/endnotes/comments keep rels order. Decide dedup policy (a part referenced by multiple sections) and whether unreferenced orphan parts keep old placement. |
| 2. first/even/default variants + missing/malformed references | No variant parsing; existing missing/malformed/external tests exist | Need `w:type` parsing, `r:id` → relationship-id → part resolution, `titlePg`/`evenAndOddHeaders` handling, per-variant labels (e.g. extend `part_type` or add a variant field — note `TextBlock`/`DocxTable` are `#[non_exhaustive]` Serialize types, so adding a field is a serialization-visible change needing schema test updates), missing `r:id`, unknown `r:id`, duplicate `r:id` per type, missing target part, malformed header XML. |
| 3. Hand-authored + application-generated fixtures with provenance | related-parts fixture has provenance; python-docx fixture lacks header variants | New hand-authored package with two sections, `titlePg`, even/odd header, missing reference, plus updated `expected.json` oracle and provenance `.md`. Application-generated: extend the python-docx generator (different first page header/footer via `section.different_first_page_header_footer`) and refresh SHA-256 + provenance + snapshots. |
| 4. Warnings + partial extraction preserved | Already implemented and tested | Must keep identical warning text/codes for missing and malformed parts; new warnings (e.g. dangling `r:id`) need a stable message format and coverage; ensure `include_related_parts = false` fast path unchanged. |

## 6. Risks / open questions for proposal phase

1. **Breaking-change boundary**: current order is documented contract; the
   issue implies changing it. `DocxTextOptions` is `#[non_exhaustive]`, so a
   new option (e.g. opt-in section ordering with rels-order default) is
   feasible, but the issue's goal suggests section order becomes the
   behavior. Decide: version as breaking (release notes) or gate behind an
   option.
2. **Variant labeling**: adding variant info to `part_type` (`"header"` →
   `"header-first"`) vs adding a field changes JSON schema snapshots
   (`tests/schema.rs`, `schema` command) — must be deliberate.
3. **settings.xml dependency**: honoring `w:evenAndOddHeaders` requires
   reading `word/settings.xml`, a new package read path and its own
   missing/malformed handling.
4. **Word inheritance emulation** (missing even/first fall back to default)
   vs literal-references-only: affects output count and is the core semantic
   decision; fixtures must encode whichever is chosen.
5. **Footnotes/endnotes/comments placement**: keep rels order (after
   section-ordered headers/footers) vs interleaving; keep as rels order to
   limit blast radius.
6. **python-docx limits**: python-docx cannot author multiple sections with
   even/odd headers easily; generator may need section manipulation or a
   second producer; provenance record must reflect the actual producer.
7. Review budget is 400 lines: parser changes + 3 extraction paths + fixtures
   + docs + snapshots will be tight; fixtures and docs can be split into
   later work units of the same change.

## 7. Suggested next phase input

Proposal should pin down: (a) the exact ordering rule and dedup policy,
(b) whether behavior is default-on or option-gated, (c) variant representation
in public output, (d) new warning message formats, (e) fixture list (names +
provenance records), (f) whether `w:evenAndOddHeaders` is in scope for this
change or deferred.
