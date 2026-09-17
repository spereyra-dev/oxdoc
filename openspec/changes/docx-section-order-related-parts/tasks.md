# Tasks — docx: order headers and footers by section (issue #177)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~530 (WU1 parser + tests + related-parts oracle ≈ 345; WU2 fixture + docs + CHANGELOG ≈ 185) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 = WU1 (parser ordering + tests + related-parts oracle) → PR 2 = WU2 (section-order fixture + docs + CHANGELOG) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main|feature-branch-chain|size-exception|pending
400-line budget risk: High
```

Budget note: WU1 alone fits under 400 lines. The measured pause point is after
WU1 GREEN: per `ask-on-risk`, report the measured WU1 size and ask the
orchestrator/user for the WU2 delivery decision (two-PR chain, scope trim, or
explicit `size:exception`). No silent chain, no inferred exception.

Global constraints for every task: no public output schema changes (`part_type`
labels, `TextBlock`/`DocxTable` fields, `DocxTextOptions` options all stay
unchanged — spec scenario "Output shape unchanged"); existing warning texts,
codes, W001 partial extraction, `SuspiciousRelationshipTarget`, and the
`include_related_parts = false` fast path stay byte-identical (spec scenario
set under "Existing warning and error behavior is preserved exactly").

---

## Work Unit 1 — parser ordering + tests + related-parts oracle (~345 lines)

- [x] 1. RED (rename): rename
  `extracts_docx_text_from_related_parts_in_relationship_order` to
  `keeps_unreferenced_docx_related_parts_in_relationship_order` in
  `crates/oxdoc-core/tests/api.rs` (assertions unchanged — the test package has
  no `sectPr`, so the new rule degenerates to rels order). Spec scenario:
  orphan/no-sectPr degenerate case under "Unreferenced header and footer parts
  are appended, not dropped". Run `cargo test -p oxdoc-core --test api` to
  confirm the renamed test still passes unchanged.
- [x] 2. RED (unit): add the `#[cfg(test)]` unit tests from design §6.1 to
  `crates/oxdoc-core/src/parsers/docx.rs`, one per pinned behavior:
  `collects_sections_in_document_order_with_variant_ranking` (spec scenarios
  "Multi-section document with out-of-order relationships" + "Variant ordering
  within one section"), `treats_missing_and_unrecognized_type_as_default`
  (scenario "Unrecognized w:type"), `keeps_reference_without_rid_as_none`
  (scenario "Missing r:id attribute"),
  `stops_collection_gracefully_on_malformed_xml` (partial-extraction behavior
  under "Existing warning and error behavior is preserved exactly"),
  `plans_orphans_and_notes_in_relationship_order` (scenarios "Orphan header
  part with no sectPr reference" + "Footnotes and comments among themselves"),
  `plan_skips_missing_and_unknown_reference_ids_with_warnings` (scenarios
  "Dangling relationship id" + "Missing r:id attribute"),
  `plan_dedups_by_resolved_path_at_first_reference` (scenario "Part referenced
  by two sections"), `plan_propagates_suspicious_target` (scenario "External
  target"). Run `cargo test -p oxdoc-core --lib` → RED (compile failure on
  missing types/collectors is the expected RED signal).
- [x] 3. RED (integration): add the new integration tests from design §6.2 to
  `crates/oxdoc-core/tests/api.rs`:
  `warns_on_missing_and_unknown_docx_reference_ids` (scenarios "Dangling
  relationship id" + "Missing r:id attribute", exact warning texts, extraction
  continues), `preserves_docx_fast_path_with_sectpr_documents`
  (`include_related_parts = false` untouched, spec "Existing warning and error
  behavior is preserved exactly"), `keeps_partial_docx_text_when_document_xml_malformed_with_sectpr`
  (W001 + partial text + related parts still planned from the collected
  prefix). Use inline `create_ooxml` packages with sectPr per design §5. Run
  `cargo test -p oxdoc-core --test api` → RED.
- [x] 4. RED (oracle truthfulness): in `tests/fixtures/docx/related-parts/`,
  edit `package/word/document.xml` to add a mid-body `w:p/w:pPr/w:sectPr`
  referencing `rIdFooter` and a body-level `w:sectPr` referencing `rIdHeader`
  (no visible text added; design §5.1), and hand-author the new expected order
  in `expected.json`: main → footer1 → header1 → comments → footnotes →
  endnotes (genuine order reversal of the rels order). Spec scenarios:
  "Multi-section document with out-of-order relationships" + "Footnotes and
  comments among themselves". Update
  `tests/fixtures/provenance/docx-related-parts-tables.md` in the same edit:
  purpose line becomes section-order traversal, the "relationship-order
  traversal" wording is removed (spec "One shared ordering helper across all
  extraction paths" fixture-oracle agreement). Run the related-parts tests →
  RED against current parser output.
- [x] 5. GREEN: implement in `crates/oxdoc-core/src/parsers/docx.rs` (all
  private, design §2):
  - `RelatedRefKind`, `RelatedRefVariant` (derived `Ord`:
    `Header < Footer`, `First < Even < Default`), `SectionReference`,
    `RelatedPartPlan = Vec<usize>`.
  - `collect_section_references` — streaming quick_xml collector, infallible
    with graceful truncation on malformed XML; `w:type` missing/unrecognized →
    `Default` silently; `titlePg` parsed past and ignored (spec "titlePg is a
    no-op for output").
  - `plan_related_part_order` — rid map (last-wins), per-section stable sort by
    `(kind, variant)`, missing/unknown-rid warnings with the exact texts
    `skipped DOCX headerReference|footerReference: missing r:id` and
    `skipped DOCX headerReference|footerReference {rid}: unknown relationship
    id` (`OutputWarning::new` → `WarningCode::Custom`, spec "Missing and
    dangling reference ids produce stable warnings"), silent skip for rid →
    non-header/footer relationship, dedup by resolved package path at first
    reference, `SuspiciousRelationshipTarget` propagated unchanged, orphans
    appended in rels order, then footnotes/endnotes/(comments when enabled) in
    rels order (spec scenarios: "Part referenced by two sections", "External
    target", "Orphan header part with no sectPr reference", "Footnotes and
    comments among themselves").
  - Wire `extract_text`, `extract_structured_text`, `extract_tables` to call
    the plan after the `!include_related_parts` and missing-rels early returns
    and iterate `&relationships[index]` (spec "One shared ordering helper
    across all extraction paths"; fast path preserved per "Existing warning
    and error behavior is preserved exactly"). `mod.rs` needs no changes.
  Run `cargo test --workspace` → all WU1 tests green.
- [x] 6. REFACTOR: if the three wiring sites in `docx.rs` duplicate more than
  the shared plan-call snippet, extract a small local helper inside `docx.rs`
  (design §3, §6.4). Re-run `cargo test --workspace` and
  `cargo clippy --workspace --all-targets -- -D warnings`. No behavior change,
  no signature changes outside `docx.rs`.
- [x] 7. WU1 verification + budget gate: run
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, and
  `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only`
  (coverage gate: every new non-test line already carries its unit test,
  including the graceful-degradation collector branch). Measure the changed
  line count for WU1 and report it at the ask-on-risk pause point before
  starting WU2. Spec acceptance: WU1 scenarios all green, "Output shape
  unchanged" confirmed (no schema, option, or `part_type` changes).

  ⏸ PAUSE (ask-on-risk): get the delivery decision for WU2 (two-PR chain of
  the same change, scope trim inside WU2, or explicit `size:exception`)
  before continuing.

## Work Unit 2 — section-order fixture + docs + CHANGELOG (~185 lines, TRIANGULATE)

- [x] 8. RED/TRIANGULATE (fixture): hand-author
  `tests/fixtures/docx/section-order/` exactly per design §5.2:
  `word/document.xml` (two sections; section 1 with default/first/even header
  refs in that attribute order, a footer ref, an unknown `rIdGhost`, a
  footer reference with no `r:id`, and `w:titlePg` set; body-level final
  section sharing `rIdHeaderDefault`, a `w:type="title"` header ref, and
  `rIdFooterTwo`), scrambled `word/_rels/document.xml.rels` ordering
  (`rIdFooterTwo, rIdHeaderFirst, rIdComments, rIdHeaderEven,
  rIdHeaderDefault, rIdHeaderOrphan, rIdFootnotes, rIdHeaderTitled,
  rIdFooterOne`), the nine part XML files (including `header-orphan.xml` with
  no sectPr reference and a small table in `footer2.xml`). Spec scenarios:
  "Multi-section document with out-of-order relationships", "Variant ordering
  within one section", "Part referenced by two sections", "Orphan header part
  with no sectPr reference", "First variant referenced without titlePg"
  (titlePg present but still a no-op), "Unrecognized w:type", "Footnotes and
  comments among themselves".
- [x] 9. RED/TRIANGULATE (oracle): hand-author
  `tests/fixtures/docx/section-order/expected.json` mirroring the
  related-parts oracle schema (`parts` + `warnings`) with the design §5.2
  order: header-first → header-even → header-default (first referencing
  section) → footer1 → header-titled → footer2 → header-orphan → comments →
  footnotes, plus the two warnings in order (`skipped DOCX headerReference
  rIdGhost: unknown relationship id`, `skipped DOCX footerReference: missing
  r:id`). Add the provenance record
  `tests/fixtures/provenance/docx-section-order.md` (hand-authored, no
  producer, synthetic text, purpose = section-order oracle). Spec scenario:
  "Multi-section document with out-of-order relationships".
- [x] 10. TRIANGULATE (three-path tests): add to
  `crates/oxdoc-core/tests/api.rs` the three fixture-oracle consumers from
  design §6.2 — `orders_docx_text_related_parts_by_section` (flat text =
  join of part texts), `orders_docx_structured_blocks_by_section` (block
  `part_path` sequence; no variant labels, spec "Output shape unchanged"),
  `orders_docx_tables_by_section` (`DocxTable.part_path` sequence including
  the `footer2.xml` table, unchanged `table_ordinal` semantics). All three
  consume the same `expected.json` (spec "Three paths agree on ordering").
  Run `cargo test -p oxdoc-core --test api` → green against WU1 (second,
  producer-orthogonal encoding of the same rule; a failure here exposes an
  ordering bug tuned to the inline packages — fix in `docx.rs`, never in the
  oracle).
- [x] 11. Snapshot stability: verify
  `tests/fixtures/snapshots/docx_python_docx_text.txt` for the python-docx
  single-section fixture (`tests/fixtures/files/docx/python-docx-basic.docx`,
  no sectPr) is byte-identical after the change (spec "python-docx
  single-section fixture snapshot holds"). If it drifts, stop and
  investigate the parser before touching the snapshot; do not regenerate.
  Re-verify the recorded SHA-256 in
  `tests/fixtures/provenance/docx-python-docx-basic.md` was not invalidated
  (fixture not regenerated, design §5.3).
- [x] 12. Docs: rewrite `docs/formats/docx.md` — the two "Related text parts"
  rows (Logical Text Contract + Structural Table Model) to describe
  section-order traversal, plus the determinism statement: full ordering
  rule, `titlePg` no-op note, `evenAndOddHeaders` deferral note (spec
  "evenAndOddHeaders flag is not consulted (deferred)"), rid →
  non-header/footer silent-skip note, and the footnotes/endnotes/comments
  repositioning (rule 5); remove the "Section-aware ordering for headers and
  footers" bullet from Planned Improvements. Spec scenarios: "First variant
  referenced without titlePg" (documented), "evenAndOddHeaders set but not
  honored", "Unrecognized w:type".
- [x] 13. CHANGELOG: add the default-on section-ordering behavior-change entry
  to `CHANGELOG.md` (consumers relying on rels order for multi-section
  packages will see different block/table ordering; single-section ordered
  rels documents unchanged; only the two new reference warnings added). Spec
  scenario: "Output shape unchanged" (behavior change documented, schema
  unchanged).
- [x] 14. WU2 verification: run
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, and
  `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only`.
  Confirm `crates/oxdoc-core/tests/schema.rs` needed no changes and no CLI
  snapshots changed except the verified-stable python-docx one. Spec
  acceptance: all delta-spec scenarios green; success criteria 1–6 from the
  proposal satisfied.
