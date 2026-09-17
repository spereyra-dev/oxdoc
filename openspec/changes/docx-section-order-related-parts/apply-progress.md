# Apply Progress — docx-section-order-related-parts

## Session 1 (2026-06 apply) — Work Unit 1 (tasks 1–7)

- Status consumed: native `gentle-ai.sdd-status` v2, `nextRecommended: apply`,
  `applyState: ready`, `actionContext.mode: repo-local`, allowed edit roots =
  repo root. Preflight: execution `auto`, store `openspec`, delivery
  `ask-on-risk`, budget 400 lines. No blockers.
- Skills: `work-unit-commits` and `chained-pr` loaded from parent-injected paths
  (`skill_resolution: paths-injected`). Strict TDD active (`cargo test`).

### Completed tasks (persisted checkboxes updated in tasks.md)

- [x] 1–7 (WU1). Tasks 8–14 (WU2) remain unchecked and untouched.

### Files changed (WU1, measured vs main, excluding the pre-existing .gitignore tweak)

| File | Changed lines (add+del) |
| --- | --- |
| `crates/oxdoc-core/src/parsers/docx.rs` | 718 (+709/−9) |
| `crates/oxdoc-core/tests/api.rs` | 120 (+117/−3) |
| `tests/fixtures/docx/related-parts/expected.json` | 18 (+9/−9) |
| `tests/fixtures/docx/related-parts/package/word/document.xml` | 10 (+10/−0) |
| `tests/fixtures/provenance/docx-related-parts-tables.md` | 2 (+1/−1) |
| **WU1 total** | **868 changed lines** (git: 848 insertions, 22 deletions incl. unrelated .gitignore +2) |

### Commits (feature branch `issue-177-docx-section-ordering`, not pushed)

1. `f68638b` — test(docx): rename relationship-order pins to unreferenced-part intent
2. `345f542` — feat(docx): order related header and footer parts by section references
3. `80ea65d` — test(docx): pin section-order reference warnings and graceful sectPr degradation

### TDD cycle evidence (strict TDD, cargo test)

| Task | RED signal observed | GREEN |
| --- | --- | --- |
| 1 (rename) | none expected — renamed tests pass unchanged on pre-change parser (orphan degenerate case); run confirmed both pass | ✅ pre-GREEN |
| 2 (unit) | `cargo test -p oxdoc-core --lib` → compile failure `unresolved imports super::collect_section_references, plan_related_part_order, RelatedRefKind, RelatedRefVariant, SectionReference` | all 8 unit tests + `fixture_related_parts_oracle_orders_sections_across_all_paths` green |
| 3 (integration) | `warns_on_missing_and_unknown_docx_reference_ids` FAILED (expected 2 warnings, got 0); `keeps_partial_..._malformed_with_sectpr` and `preserves_docx_fast_path_with_sectpr_documents` passed pre-GREEN only because rels order coincided with section order — they pin the post-change behavior | green |
| 4 (oracle) | RED observable only through a consumer test: none existed for `expected.json`, so `fixture_related_parts_oracle_orders_sections_across_all_paths` (unit test, all three extraction paths vs the hand-authored oracle) was added; RED was demonstrated via the related behavior pin `keeps_unreferenced_docx_table_parts_in_relationship_order` failing pre-GREEN (rels order `comments→header` vs section order `header→comments`) | oracle consumed by all three paths; `expected.json` hand-authored, never regenerated |
| 5 (GREEN) | — | `cargo test -p oxdoc-core`: 100 lib + 73 api + 10 schema + 2 fixtures all pass |
| 6 (refactor) | — | extracted `section_reference` helper; collapsed `if` arms for clippy; tests/clippy re-run green |
| 7 (gate) | — | `cargo fmt --all -- --check` OK; `cargo clippy --workspace --all-targets -- -D warnings` 0 errors; `cargo test --workspace` 318 tests pass, 0 fail; `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` exit 0, total line coverage 96.16% (docx.rs 97.72% lines, 100% functions) |

### Spec acceptance checks (WU1)

- "Output shape unchanged": no changes to `models.rs`, `mod.rs`, `schema.rs`,
  `part_type` labels, `TextBlock`/`DocxTable` fields, or `DocxTextOptions`
  (git diff touches only docx.rs, api.rs, fixture XML/JSON/provenance).
- Existing behaviors byte-identical: missing-part / malformed-part /
  external-target / missing-rels / fast-path tests pass unmodified.
- python-docx single-section fixture and its snapshot untouched (fixture not
  regenerated; full re-verification happens in WU2 task 11).
- `include_related_parts = false` fast path: verified untouched
  (`preserves_docx_fast_path_with_sectpr_documents`).

### Deviations from design (disclosed)

1. **Measured WU1 size is ~868 lines, not the ~345 estimate.** The design's
   estimates (docx.rs ~100 impl + ~110 tests, api.rs ~110, fixture ~25) proved
   optimistic: the eight §6.1 unit tests plus their package/rid helpers are
   ~430 lines, the §6.2 integration tests ~112, the implementation ~205 (with
   required doc comments), and the related-parts oracle consumer ~90. Tests,
   comments, and blank lines were never compressed or deleted to reach the
   budget. WU1 is one cohesive work unit (one shared ordering rule); no honest
   split puts both halves under 400. Budget decision needed (see pause below).
2. **`paragraph_depth` counter omitted** from the collector: the design listed
   a `w:p` depth counter, but push-on-sectPr-close behavior is identical with
   or without it (no behavioral branch depends on it); keeping it would be
   dead state. Documented here as the §2.2 deviation.
3. **Tables relationship-order test assertion updated with its rename** — spec
   rule 5 moves notes strictly after headers/footers, so the renamed
   `keeps_unreferenced_docx_table_parts_in_relationship_order` assertion now
   expects main → header → comments (documented with an inline comment).
   The flat-text rename kept assertions unchanged as specified.
4. **Oracle consumer test added in WU1** (`fixture_related_parts_oracle_
   orders_sections_across_all_paths`): task 4's "RED against current parser
   output" requires a consumer; the oracle was previously unconsumed. It runs
   all three extraction paths against `expected.json`, which also anticipates
   the spec's "three paths agree on one fixture oracle" requirement.

### Warnings / status

- New warnings implemented with exact texts, both `WarningCode::Custom` on
  `word/_rels/document.xml.rels`: `skipped DOCX {header|footer}Reference:
  missing r:id` and `skipped DOCX {header|footer}Reference {rid}: unknown
  relationship id`.
- `rid` → non-header/footer relationship: silent skip (documented in design;
  docs wording lands in WU2).

### Remaining tasks

- [ ] 8. WU2 fixture (`tests/fixtures/docx/section-order/` package XML)
- [ ] 9. WU2 oracle (`expected.json` + provenance record)
- [ ] 10. WU2 three-path fixture-oracle tests in api.rs
- [ ] 11. Snapshot stability check (python-docx SHA-256 + snapshot)
- [ ] 12. Docs (`docs/formats/docx.md` rewrite)
- [ ] 13. CHANGELOG entry
- [ ] 14. WU2 verification gates

### Workload / PR boundary

- WU1 = PR 1 of the stacked-to-main chain (commits f68638b, 345f542, 80ea65d;
  868 changed lines). WU2 (~185 estimated) = PR 2 targeting main after PR 1
  merges.
- ⏸ PAUSE (ask-on-risk): WU1 measured size (868) exceeds the 400-line budget
  and the ~345 estimate. The WU2 delivery decision must be made with the real
  WU1 number in hand: (a) keep the planned two-PR chain with WU1 PR over
  budget → explicit `size:exception`; (b) three-PR chain (WU1a: parser+unit
  tests ≈ 520, WU1b: integration+oracle ≈ 350 — WU1a alone still exceeds
  400); (c) trim WU2 scope; or (d) maintainer accepts `size:exception` for
  both. No silent chain and no inferred exception — decision escalated to the
  orchestrator/user before starting WU2.

### Structured status

- Consumed: native v2 status at session start (applyState ready, no blockers).
- Produced: taskProgress after WU1 = 7/14 complete; `nextRecommended` remains
  apply (WU2 tasks 8–14), paused pending the delivery decision.
