# Proposal — docx: order headers and footers by section (issue #177)

- Change id: `docx-section-order-related-parts`
- Status: proposed 2026-06 (SDD propose phase, artifact store: openspec)
- Inputs: `exploration.md` (this change), GitHub issue #177, parent-resolved product
  decisions (authoritative, listed in traceability below), `openspec/config.yaml`.
- Delivery: ask-on-risk · review budget 400 lines · strict TDD (`cargo test`).

## Intent / problem

Today every DOCX extraction (`extract_text`, `extract_structured_text`,
`extract_tables`) appends header/footer related parts in
`word/_rels/document.xml.rels` **relationship-file order**. Relationship order is a
packaging artifact: it has no connection to which section of the document each
header/footer belongs to, so multi-section documents surface header/footer text in
an order a reader cannot reconstruct or reason about. `docs/formats/docx.md`
documents this relationship-order contract today, and the same doc already lists
"Section-aware ordering for headers and footers" under Planned Improvements.

This change makes related-part extraction **section-aware by default**: headers and
footers are ordered by the sections that reference them in `word/document.xml`
(`sectPr` document order, then variant type), instead of relationship enumeration
order. The ordering change **is** the requested feature (issue #177); it ships
default-on with no new option.

## Ordering rule (deterministic)

Applied to header/footer related parts only; footnotes, endnotes, and comments keep
current relationship-file order (see Non-goals).

1. **Collect sections in document order.** Walk `w:body` children in document
   order. A `w:p/w:pPr/w:sectPr` closes a section at that paragraph position; the
   body-level `w:sectPr` (last child of `w:body`) is the final section. This
   yields the section sequence exactly as a reader encounters section breaks.
2. **Normalize references within a section.** For each `sectPr`, collect
   `w:headerReference` / `w:footerReference` elements and order them:
   headers before footers; within each kind, variants in `first`, `even`,
   `default` order; ties (same kind + variant) keep `sectPr` element order.
   Canonical variant ordering is chosen over raw attribute order so the rule does
   not depend on producer XML serialization habits.
3. **Resolve and dedup.** Walk the normalized reference list in order, resolving
   each `r:id` through `word/_rels/document.xml.rels`. A resolved part is emitted
   **once, at its first reference position** (dedup by resolved package path).
   A part shared by several sections appears only at its first referencing section.
4. **Orphans last.** Header/footer parts present in the rels file but not
   referenced by any `sectPr` are appended after all section-referenced parts, in
   rels file order, so nothing silently disappears from output.
5. **Non-header/footer parts unchanged in placement relative to each other.**
   After all headers and footers (section-referenced, then orphans), footnotes,
   endnotes, and comments (when `include_comments`) are appended in rels file
   order. Note: this moves footnotes/endnotes/comments strictly after all
   headers/footers; today they interleave with them in rels order. This is a
   documented consequence of the section pass, not a separate behavior change.

The main document body is still emitted first, unchanged.

### Missing, malformed, duplicate, and degenerate references

| Situation | Behavior |
| --- | --- |
| `headerReference`/`footerReference` without `r:id` | Skip reference; new warning (exact strings below). |
| `r:id` not present in `document.xml.rels` (dangling) | Skip reference; new warning. |
| Duplicate `r:id` within one section, or same part referenced by multiple sections | Dedup by resolved part path; emitted once at first reference (rule 3). |
| Two different `r:id`s with same kind + variant in one section | Both emitted, in `sectPr` element order (rule 2 tie-break). |
| `w:type` missing or unrecognized | Treated as `default`; documented, no warning. |
| Resolved target is external / has escape / NUL chars | Existing `SuspiciousRelationshipTarget` hard error, unchanged. |
| Resolved part missing from package | Existing `skipped related DOCX text part {path}: missing part` warning, extraction continues. |
| Malformed XML inside a referenced part | Existing W001 `malformed_xml` warning + partial text, other parts still processed. |
| `document.xml.rels` missing entirely | Existing behavior: main-part output only, no warning (no references can resolve anyway). |
| `sectPr` present but malformed | Covered by existing malformed-XML handling of `document.xml` (main part). |

### New warning messages (stable format, aligned with existing style)

- Missing `r:id`: `skipped DOCX headerReference: missing r:id` /
  `skipped DOCX footerReference: missing r:id`
- Dangling `r:id`: `skipped DOCX headerReference {rid}: unknown relationship id` /
  `skipped DOCX footerReference {rid}: unknown relationship id`

Existing warning texts, codes, and the partial-extraction behavior are preserved
**exactly** (AC 4). The `include_related_parts = false` fast path is untouched.

### `titlePg` handling

`w:titlePg` is **not** consulted for ordering or filtering. A first-variant
(`w:type="first"`) header/footer is emitted whenever a `sectPr` references it,
whether or not `titlePg` is set. Rationale: extraction orders document content; it
does not emulate Word page rendering. Gating on `titlePg` would remove content
from output based on a render flag — a semantic change beyond ordering. This is
stated explicitly in the docs so the behavior is not mistaken for a bug.

## Scope / affected areas

- `crates/oxdoc-core/src/parsers/docx.rs` — new sectPr/reference collection
  helper; ordering merge applied identically in the three extraction paths
  (`extract_text`, `extract_structured_text`, `extract_tables`); two new warnings.
- `crates/oxdoc-core/src/parsers/mod.rs` — only if shared helpers need a small
  addition (e.g. rels-id lookup); no signature changes elsewhere.
- `crates/oxdoc-core/tests/api.rs` — update tests that pin relationship order
  (`..._in_relationship_order` become section-order tests with renamed intent);
  add coverage for the ordering rule table above.
- Fixtures: update `tests/fixtures/docx/related-parts/` oracle (its rels are
  deliberately out of order — now provably section-driven); add a new
  hand-authored multi-section package (two sections, `titlePg`, first/even/default
  references, one dangling `r:id`, one orphan part) with provenance record;
  optionally extend the python-docx generator with
  `different_first_page_header_footer` and refresh SHA-256 + snapshot.
- `docs/formats/docx.md` — rewrite the two "Related text parts" rows
  (Logical Text Contract + Structural Table Model) and add the determinism
  statement (rule above, titlePg note, evenAndOddHeaders deferral note).
- `CHANGELOG.md` — document the default-on ordering behavior change.

## Non-goals (explicit)

1. **No variant labels in public output.** `part_type` stays
   `"header" | "footer" | ...`; no `header-first`-style labels, no new fields on
   `TextBlock`/`DocxTable`. Variant labeling is deferred to issue #179
   (structured-text work); structured-text block paths and part labels stay
   compatible.
2. **No `w:evenAndOddHeaders` honoring.** That flag lives in
   `word/settings.xml`, a part the DOCX extractor never reads today; adding the
   read path plus its own missing/malformed handling is a half-measure risk.
   **Deferred explicitly**: even/odd *variant references* still order by their
   `w:type` label (rule 2), but the settings.xml enablement flag is not consulted.
   Documented in `docs/formats/docx.md`.
3. **No Word inheritance emulation.** Missing even/first variants do not fall
   back to emitting the default header again; only literally referenced parts are
   emitted.
4. **Footnotes/endnotes/comments keep relationship-file order** among themselves;
   only their position relative to headers/footers changes (rule 5).
5. **No opt-in option.** Section ordering is the default and only behavior;
   `DocxTextOptions` gains no new field.
6. **No rendering emulation** beyond ordering (no page numbers, no section-boundary
   markers in output).

## Alternatives considered (rejected)

1. **Option-gated ordering** (new `DocxTextOptions` flag, rels order default):
   rejected — issue #177's goal is section order as the behavior; an option would
   leave the default contrary to the request and add public API surface. Parent
   decision 1 confirms default-on.
2. **Emulate Word rendering semantics** (`titlePg` gating, `evenAndOddHeaders`,
   missing-variant inheritance): rejected for this change — requires a new
   `word/settings.xml` read path, mixes rendering into text extraction, and
   triples the fixture matrix. Even/odd honoring deferred per parent decision 4;
   inheritance rejected (Non-goal 3).
3. **Add variant labels to `part_type` or new output fields**: rejected —
   serialization-visible schema change to `#[non_exhaustive]` public types with
   snapshot/schema-test fallout; belongs to #179 (parent decision 3).
4. **Emit per reference, not per part** (duplicate text when a part is shared
   across sections): rejected — duplicated blocks corrupt ordinals and table
   numbering; first-reference dedup (rule 3) keeps output bijective with parts.
5. **Interleave footnotes/endnotes/comments within the section pass**: rejected —
   no ordering information exists for them in `sectPr`; keeping rels order limits
   blast radius (parent decision 2, exploration risk 5).

## Risks

- **Documented-contract break**: rels-order output is documented today; consumers
  relying on it will see different block/table ordering for multi-section or
  out-of-order-rels packages. Mitigated by CHANGELOG + docs rewrite; single-section
  documents with rels order matching section order see no change (e.g. the
  python-docx fixture likely keeps its snapshot).
- **Three extraction paths must stay in lockstep**: the ordering merge must be one
  shared helper consumed by all three paths, or flat text / structured text /
  tables can drift. Strict TDD with a shared fixture oracle across all three
  extractions.
- **Budget pressure**: parser + tests + fixtures + docs ≈ 330–390 lines (below).
  If implementation trends past 400, ask-on-risk pauses for a delivery decision
  (split fixtures/docs into a second work unit of the same change) — no silent
  chain or exception.

## Rollback

Single-crate behavior change behind no schema/API change: revert the parser
ordering merge and restore the relationship-order tests; fixtures and docs revert
in the same commit range. No data migration, no public type changes, no CLI flag
changes. Rollback risk is low because output *shape* is unchanged — only order.

## Size estimate (vs 400-line review budget)

| Work unit | Rough lines |
| --- | --- |
| sectPr/reference collection + shared ordering merge (parser) | ~70–90 |
| Test updates + new ordering/warning tests (`api.rs`) | ~120–150 |
| New multi-section fixture (XML + expected.json + provenance) + related-parts oracle update | ~80–100 |
| Docs (`docx.md` two rows + determinism statement) + CHANGELOG | ~30 |
| Snapshot refresh (only if python-docx ordering changes) | ~0–10 |
| **Total** | **~300–390** |

Fits the 400-line budget, but tight. Recommended split into two work units of this
same change: (1) parser + tests + oracle updates; (2) new fixture + docs +
CHANGELOG + snapshots. Coverage gate (95%) is unaffected — new parser lines arrive
with their tests in unit 1.

## Success criteria

1. Deterministic section-aware ordering implemented and documented exactly as the
   rule above; all three extraction paths agree on one fixture oracle. (AC 1)
2. first/even/default variants, missing `r:id`, dangling `r:id`, duplicate
   references, orphan parts, and `titlePg`-present documents covered by tests.
   (AC 2)
3. New hand-authored multi-section fixture and (updated) application-generated
   fixture carry provenance records. (AC 3)
4. Existing warning texts/codes, W001 partial extraction,
   `SuspiciousRelationshipTarget`, and the `include_related_parts = false` fast
   path are byte-identical in behavior; only the two new reference warnings are
   added. (AC 4)
5. `part_type` labels and `TextBlock`/`DocxTable` shape unchanged; footnotes/
   endnotes/comments mutual order unchanged; no new options. (parent decisions)
6. `cargo test --workspace`, `cargo fmt --check`, `cargo clippy -D warnings`, and
   the 95% coverage gate pass.

## Parent decision traceability

| Parent decision | Where honored |
| --- | --- |
| 1. Default-on section ordering, no option; CHANGELOG + docs | Intent; Non-goal 5; Scope (docs/CHANGELOG) |
| 2. Footnotes/endnotes/comments keep rels order | Ordering rule 5; Non-goal 4 |
| 3. Keep `part_type` labels; defer variant labeling to #179 | Non-goal 1 |
| 4. Defer `evenAndOddHeaders` unless settings.xml trivially available | Non-goal 2 (deferred explicitly; even/odd variant fixtures in scope) |
| 5. Preserve warnings/partial extraction; orphan parts append deterministically | Reference table; Ordering rule 4; Success criterion 4 |

## Proposal question round

The product decisions for this change were resolved by the orchestrator, so no
user interview is needed on those. The following assumptions were made during
proposal drafting and are the ones worth a quick confirm or correction before the
specs phase — they sharpen the PRD by pinning edge-case semantics that fixtures
will encode as oracles:

1. **Dedup policy** — a header/footer part referenced by two sections is emitted
   once, at its first referencing section. Assumption: duplicated text blocks
   would be worse than positional loss. Correct?
2. **`titlePg` is a no-op for output** — a first-page header is emitted whenever
   referenced, even without `titlePg`. Assumption: extraction must not hide
   document content based on render flags. Correct, or should unreferenced-by-
   -render parts be suppressed?
3. **Unknown/missing `w:type` maps to `default` silently** (no warning).
   Assumption: warning noise on producer quirks would hurt more than a documented
   default. Correct?
4. **New warning wording** (`skipped DOCX headerReference {rid}: unknown
   relationship id`, etc.) — acceptable, or is there an existing message
   convention these should match more closely?
5. **evenAndOddHeaders deferral is final for this change** — confirm the deferral
   should be documented as deferred (not planned-elsewhere) in
   `docs/formats/docx.md`, or whether it should get a tracking issue reference.

These assumptions need review; answer, correct the framing, skip, or request a
second question round before the specs phase consumes this proposal.
