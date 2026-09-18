# Delta for docx-extraction

Change: `structured-text-source-provenance` (issue #179)

## MODIFIED Requirements

### Requirement: No public output schema changes

The system MUST NOT change `part_type` labels (`"main" | "header" | "footer" |
"footnotes" | "endnotes" | "comments"`), MUST NOT add fields to `DocxTable`,
MUST NOT add options to `DocxTextOptions`, and MUST NOT add fields to `TextBlock`
other than the optional `variant` field. Variant labeling, deferred by change
`docx-section-order-related-parts`, is now delivered by this change through the
versioned structured-text contract (`schema_version: 2` and schema
`schemas/v2/oxdoc-structured-text.schema.json`, see the `structured-text-schema`
domain spec); it MUST NOT be emitted by widening the v1 contract. Flat text and
tables output MUST NOT change in any way.
(Previously: the requirement prohibited adding any field to `TextBlock` and
emitting any variant labels, deferring variant labeling to a future change; this
change is that future change and permits exactly one additive optional `variant`
field on header/footer blocks via schema v2.)

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

## ADDED Requirements

### Requirement: Header and footer variant labeling in structured output

The system MUST label structured `header` and `footer` blocks with an optional
`variant` field whose value is one of `first`, `even`, or `default`, derived from
the `w:type` of the `sectPr` reference that positioned the part. Variant data
MUST be sourced from the existing section-reference collection (no second
parsing pass). A `w:type` that is missing or unrecognized MUST be labeled
`default`, matching the existing ordering rule, with no warning. Blocks other
than DOCX `header` and `footer` — `main`, `footnotes`, `endnotes`, `comments`,
and all PPTX blocks — MUST NOT carry `variant`.

#### Scenario: Section-referenced variants are labeled

- GIVEN a section whose `sectPr` references a first-page header, an even header,
  and a default footer
- WHEN structured extraction runs
- THEN the emitted blocks carry `variant` values `first`, `even`, and `default`
  respectively, in the section-aware order defined by
  `docx-section-order-related-parts`.

#### Scenario: Unknown or missing w:type maps to default

- GIVEN a `w:footerReference` with `w:type="title"` (or no `w:type`)
- WHEN structured extraction runs
- THEN the footer block carries `variant: "default"` and no warning is emitted
  for the type value.

#### Scenario: Non-variant blocks omit the field

- GIVEN a document with a main body, footnotes, and comments
- WHEN structured extraction runs
- THEN `main`, `footnotes`, and `comments` blocks serialize without a `variant`
  key (the field is omitted, never `null`).

### Requirement: Variant-free documents keep block serialization byte-identical

For single-section documents with no `first` or `even` variant references, the
`blocks` array serialization MUST be byte-identical to the previous release:
same fields, same field order, same block order, same `ordinal` values, same
`text`, and `variant` omitted entirely. The only byte delta in the structured
payload MUST be the additive top-level `schema_version` field.

#### Scenario: Single-section document blocks unchanged

- GIVEN an existing single-section fixture with only default-variant references
- WHEN structured extraction output is compared byte-for-byte with the previous
  release
- THEN the `blocks` array is byte-identical and the payload differs only by the
  new `schema_version: 2` field.

### Requirement: Variant label edge cases follow existing dedup and orphan rules

When the same resolved header/footer part path is referenced by multiple
sections or as multiple variants, the single emitted block MUST carry the
variant of its first referencing position (positional-first wins), consistent
with the first-reference deduplication rule. Orphan header/footer parts that no
`sectPr` references MUST be emitted without a `variant` field, because they have
no section-assigned variant. `include_related_parts = false`, missing rels
files, and existing warning/error behavior MUST be untouched by labeling.

#### Scenario: Shared part referenced as two variants

- GIVEN one header part referenced as `first` by an early section and as
  `default` by a later section
- WHEN structured extraction runs
- THEN the part is emitted once at its first referencing position carrying
  `variant: "first"`.

#### Scenario: Orphan header part is unlabeled

- GIVEN a rels file listing `header3.xml` that no `sectPr` references
- WHEN structured extraction runs
- THEN the orphan header block is emitted after all section-referenced
  headers/footers with no `variant` field.
