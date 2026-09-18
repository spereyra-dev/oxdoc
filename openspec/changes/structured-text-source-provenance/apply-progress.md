# Apply Progress — structured text source provenance

- Change id: `structured-text-source-provenance`
- Branch: `issue-179-structured-text-wu1` (stacked-to-main PR 1 of 3)
- Artifact store: openspec · strict TDD (`cargo test`) · review budget 400 lines

## WU1 — Variant plumbing + oracle + regressions (tasks 1–12) — COMPLETE

### TDD Cycle Evidence

| Cycle | Step | Evidence |
| --- | --- | --- |
| R1 RED | Failing unit tests in `docx.rs` (`label_maps_variants_to_public_names`, `labels_plan_entries_with_section_reference_variants`, `plans_deduped_part_with_first_reference_variant`) + api.rs triple assertion | `cargo test -p oxdoc-core parsers::docx` compile failure: `RelatedPartPlanEntry` not found (×6, E0422), `label` method missing (×3, E0599), `no field 'variant' on type '&TextBlock'` (E0609) |
| R1 RED | api.rs oracle-triple test | same compile failure: `error[E0609]: no field 'variant' on type '&TextBlock'` at `crates\oxdoc-core\tests\api.rs:651` |
| R1 GREEN | models.rs `variant` field + `with_variant`; docx.rs `RelatedPartPlanEntry` + `label()` + three push sites + `push_text_block` re-signature; existing plan assertions destructured to `entry.index` | `cargo test --workspace`: 8 suites, all `ok`, 0 failed (25+99+103+79+10+9+2+0 = 327 tests) |
| R1 TRIANGULATE | targeted `cargo test -p oxdoc-core parsers::docx` → 35 passed (incl. label mapping, plan variants, dedup first-reference, unknown/missing `w:type` → default, orphan/notes ordering, warning pins) | ok |
| R1 TRIANGULATE | targeted `cargo test -p oxdoc-core --test api` → 79 passed (incl. `orders_docx_structured_blocks_by_section` triples with `header-default` staying `default` though section 2 re-references it as `first`; `header-titled` → `default`; orphan unlabeled; `keeps_non_variant_blocks_without_variant_key` incl. PPTX; `ordinals_are_contiguous_in_output_order`; `keeps_plain_text_free_of_variant_metadata`) | ok |
| REFACTOR | single `label()` conversion site inside `push_text_block`, `&'static str` (no hot-path allocation); removed now-dead two-tuple `oracle_parts` helper; `cargo fmt --all` + `cargo clippy --workspace --all-targets -- -D warnings` clean | ok |

### Files changed (WU1)

- `crates/oxdoc-core/src/models.rs` — `TextBlock.variant: Option<String>` (last field, `skip_serializing_if`), `new()` keeps signature with `variant: None`, consuming builder `with_variant`.
- `crates/oxdoc-core/src/parsers/docx.rs` — `RelatedPartPlanEntry { index, variant }` + `RelatedRefVariant::label()`; section-referenced/orphan/notes push sites carry variant; `extract_text`/`extract_tables` destructure `entry.index` only; `push_text_block` gains `variant` parameter and applies `.with_variant(v.label())` exactly once; existing plan tests updated to entry assertions.
- `crates/oxdoc-core/tests/api.rs` — structured oracle test asserts `(part_type, part_path, variant)` triples; new `keeps_plain_text_free_of_variant_metadata`, `keeps_non_variant_blocks_without_variant_key` (DOCX + PPTX), `ordinals_are_contiguous_in_output_order`; text/tables tests keep unchanged two-tuple shape.
- `tests/fixtures/docx/section-order/expected.json` — `"variant"` on the six section-referenced rows (`header-first`→`first`, `header-even`→`even`, `header-default`→`default` first-reference-wins, `footer1`/`header-titled`/`footer2`→`default`); no `variant` key on `main`, `comments`, `footnotes`, `header-orphan`; the two warning pins (`rIdGhost`, missing `r:id`) unchanged.
- `tests/fixtures/docx/section-order/package/word/document.xml` — section 2's `headerReference r:id="rIdHeaderDefault"` gains `w:type="first"` (shared part referenced as two variants; zero ordering/oracle churn).

### Verification evidence (WU1 final gate)

- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean (after removing dead `oracle_parts` helper)
- `cargo test --workspace` → 327 passed, 0 failed; flat-text/tables snapshot tests (`docx_basic_text.txt`, `pptx_text.txt`) pass unmodified
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → exit 0 (line coverage 96.22% ≥ 95)
- `git diff main --stat` (excluding .gitignore): 5 files, 307 insertions(+), 33 deletions(−) = **340 changed lines** vs estimate ~240–310 — slightly above the honest estimate (test bodies + entry-assertion rework), within the 400-line budget; no exception needed.

### Deviations from design

- None behavioral. One mechanical addition: the existing four plan-order unit tests (`collects_sections_in_document_order_with_variant_ranking`, `plans_orphans_and_notes_in_relationship_order`, `plan_skips_missing_and_unknown_reference_ids_with_warnings`, `plan_dedups_by_resolved_path_at_first_reference`) had their bare-index assertions updated to destructure `entry.index` (with variant tuples added to the ranking test) because the plan element type changed from `usize` to `RelatedPartPlanEntry` — this is the compile-pinned consequence of task 6, not extra scope.
- `keeps_plain_text_free_of_variant_metadata` pins non-leakage via the oracle text equality plus absence of `word/header` / `word/footer` / `variant` / `ordinal` tokens (the fixture texts legitimately contain the words "First"/"Even", so standalone variant-token absence is pinned at serialization level by `keeps_non_variant_blocks_without_variant_key` instead).

### Remaining tasks

- WU2 (tasks 13–26): versioned contract + CLI + snapshots — unchecked.
- WU3 (tasks 27–32): documentation — unchecked.
- Cross-unit guards (tasks 33–35): pending with each unit's diff review; WU1 diff touches no `schemas/v1/**`, no snapshot file, and lands oracle + fixture + parser in one commit.

### Sequencing note

WU1 merged alone emits `variant` under an unversioned payload — legitimate intermediate state violating no locked test, but WU2 must merge before any tag/release.
