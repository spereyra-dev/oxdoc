# Apply Progress — structured text source provenance

- Change id: `structured-text-source-provenance`
- Branch: `issue-179-structured-text-wu1` (stacked-to-main PR 1 of 3) · WU2 on `issue-179-structured-text-wu2` (stacked-to-main PR 2 of 3, WU1 merged as #213)
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

## WU2 — Versioned contract + CLI + snapshots (tasks 13–26) — COMPLETE

### TDD Cycle Evidence

| Cycle | Step | Evidence |
| --- | --- | --- |
| R3/R5/R7 RED | cli.rs marker asserts (`extracts_text_as_structured_json` incl. `blocks[0].get("variant").is_none()`, stdin/multi per-element `schema_version == 2`, PPTX test upgraded to byte snapshot comparison) + schema.rs v2/PPTX validation tests, version-aware `read_json_schema`, `SCHEMA_VERSIONS` metadata table, negative v1 test | `cargo test -p oxdoc-core --test schema`: 3 failed (v2 schema file missing → path NotFound; metadata table fails on missing v2 dir), 9 passed · `cargo test -p oxdoc-cli --test cli`: 3 failed (schema_version `Null != 2` ×2; PPTX snapshot file missing), 96 passed |
| R3/R5/R7 GREEN | `schemas/v2/oxdoc-structured-text.schema.json` (design §3.1 verbatim) + byte-identical `docs/schemas/v2/` mirror + Makefile v2 diff pass + CLI `TextStructuredPayload.schema_version: 2` at the single construction site + DOCX snapshot bump (one key) + frozen PPTX snapshot + cli.rs/schema.rs tests | `cargo test --workspace`: 329 passed, 0 failed (was 327; +2 new schema tests) · schema suite 12 passed · cli suite 99 passed |
| TRIANGULATE | targeted `cargo test -p oxdoc-cli --test cli -- structured` (7 passed: single DOCX, PPTX byte-snapshot, stdin, multi per-element, empty-batch unchanged) · `git diff main -- cli_structured_text_json.json` shows exactly one added key · `cargo test -- v2_payload` (negative v1 breakage) · `cargo test -- representative_structured` (both snapshots validate vs v2 + const) · `git diff main --name-only -- schemas/v1 docs/schemas/v1` → empty · `git diff main --name-only -- tests/fixtures/snapshots` → exactly the two sanctioned snapshots | ok |
| REFACTOR | version-driven tooling only (`read_json_schema(version, name)`, metadata derives version from the directory table — v3 is a one-line table edit); `validate_object` untouched (const asserts local to the two structured tests, mirroring the rows-jsonl precedent); `cargo fmt --all` + clippy clean | ok |

### Files changed (WU2)

- `schemas/v2/oxdoc-structured-text.schema.json` (new) — draft 2020-12, `$id .../schemas/v2/oxdoc-structured-text.schema.json`, `additionalProperties: false`, required `schema_version` (const 2) / `file` / `document_type` (docx|pptx) / `blocks`; block items require `part_type` / `part_path` / `ordinal` (minimum 1) / `text` with optional `variant` enum `first|even|default` and the ordinal/notes/variant semantics descriptions.
- `docs/schemas/v2/oxdoc-structured-text.schema.json` (new) — byte-identical mirror (`cmp` verified).
- `Makefile` — `docs-schemas-check` now diffs v1 **and** v2 mirrors.
- `crates/oxdoc-core/tests/schema.rs` — `read_json_schema(version, name)` + 9 mechanical call-site updates; `schemas_have_stable_public_metadata` iterates `SCHEMA_VERSIONS` (v1 ×9, v2 ×1) deriving `$id` version from the directory; structured test switched to v2 with const assertion; new `representative_structured_text_pptx_json_matches_schema`; new `v2_payload_fails_frozen_v1_validation` (documented intentional v1-strict breakage via the undeclared-field panic path, with a temporary no-op panic hook); PPTX test added.
- `crates/oxdoc-cli/src/main.rs` — `TextStructuredPayload` gains `schema_version: u8` as first field; set to 2 at the single construction site serving single/stdin/multi modes. Core `StructuredText` unversioned (asymmetric convention preserved, mirrors TablesPayload).
- `tests/fixtures/snapshots/cli_structured_text_json.json` — `"schema_version": 2` added as first key; `blocks` array byte-identical (diff reviewed).
- `tests/fixtures/snapshots/cli_structured_text_pptx_json.json` (new) — generated once from implementation output over `pptx/text` corpus (`build_package` path), reviewed against design §5 before freezing: block 1 `slide`/`ppt/slides/slide2.xml`, block 2 `notes`/`ppt/notesSlides/notesSlide2.xml` ("Speaker note"), block 3 `slide`/`ppt/slides/slide1.xml`, ordinals 1..3, no `variant` — the slide2-before-slide1 inversion locks `p:sldIdLst` presentation order.
- `crates/oxdoc-cli/tests/cli.rs` — marker asserts for all emission modes (multi per element), `variant`-omission assert on the DOCX main block, PPTX test upgraded from minimal field asserts to byte comparison + parsed-Value equality; `emits_empty_structured_json_batch_when_no_inputs_succeed` untouched.

### Verification evidence (WU2 final gate)

- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo test --workspace` → 329 passed, 0 failed
- `make docs-schemas-check` → `make` unavailable on Windows; ran the exact underlying commands instead: `diff -ru schemas/v1 docs/schemas/v1` and `diff -ru schemas/v2 docs/schemas/v2` → both clean (mirror byte-identical, `cmp`-verified)
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → exit 0 (line coverage 96.22% ≥ 95)
- `git diff main --stat` (excluding .gitignore): 8 files, 263 insertions(+), 54 deletions(−) = **317 changed lines** vs estimate ~300–380 — within the 400-line budget; no delivery ask needed. WU1+WU2 cumulative ≈ 657 lines across two stacked PRs.
- Cross-unit guards (WU2 portion): no `schemas/v1/**` / `docs/schemas/v1/**` path in the change diff; only the two sanctioned structured snapshots changed; `validate_object` untouched.

### Commits (WU2)

- `22e4770` feat(schema): publish structured-text schema v2 with mirror lockstep (schema + mirror + Makefile + version-aware tooling + negative v1 pin; verified green as its own tree via stash-run)
- `e89d735` feat(cli): stamp structured-json payloads with schema version 2 (CLI marker + DOCX snapshot bump + frozen PPTX snapshot + cli.rs/schema.rs structured tests)

### Deviations from design

- None behavioral. The PPTX snapshot was generated from implementation output as designed and matches the §5 block table exactly. The negative v1 test suppresses the panic hook during `catch_unwind` to keep CI output clean; the assertion goes through the same `validate_object` undeclared-field panic path the design specifies.

### Remaining tasks

- WU3 (tasks 27–32): documentation — unchecked.
- Cross-unit guards (tasks 33–35): v1-frozen and snapshot-scope verified for WU1+WU2 (tasks 33–34 evidence above); task 35 work-unit boundaries preserved (WU2 in two commits, tests with code).

### Sequencing note

WU2 carries the version marker, so after this PR the transient "unversioned variant" state from WU1 is resolved; WU2 must be in a release before any tag (still holds — release sequencing unchanged).

## WU3 — Documentation (tasks 27–35) — COMPLETE

Branch: `issue-179-structured-text-wu3` (stacked-to-main PR 3 of 3; WU1 merged as #213, WU2 as #214). Docs only — no code, no schema files.

### TDD Cycle Evidence

Docs-only unit; strict TDD applies to code, so verification = docs link validation + the full unchanged Rust gate:

| Check | Evidence |
| --- | --- |
| Markdown link validation | `npx --yes markdown-link-check@3 --config .markdown-link-check.json` over `docs/json-output.md`, `docs/cli.md`, `docs/formats/docx.md`, `docs/formats/pptx.md`, `CHANGELOG.md` (plus README/docs set) → exit 0, 11 links checked, all `[✓]` (new `schemas/v2/…` links resolve; v1 link retained and resolves) |
| `cargo fmt --all -- --check` | clean (docs touch no Rust) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | 329 passed, 0 failed (25+99+103+79+12+9+2) — unchanged from WU2 |
| `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` | exit 0 (line coverage 96.22% ≥ 95) |

### Files changed (WU3)

- `docs/json-output.md` — structured-json row moved to `schemas/v2/oxdoc-structured-text.schema.json`; v1 link retained marked "for outputs captured before schema v2"; versioning policy sentence extended to cover v1+v2 and the new-field-requires-new-version rule; new structured-text semantics paragraph (`schema_version: 2`, optional `variant` with default/first-reference-wins/orphan-omitted semantics, global 1-based `ordinal`, `slide` vs `notes` with the `ppt/notesSlides/notesSlideN.xml` path); v1-strict migration note (v1 frozen; `additionalProperties: false` means v2 payloads — even variant-free ones — fail v1 validation); deliberate unversioned-library-serialization asymmetry documented.
- `docs/cli.md` — structured-json example shows `schema_version: 2` and a header block with `"variant": "default"`; prose explains variant labeling, global ordinals spanning parts/document types, slide/notes, and links the v2 schema.
- `docs/formats/docx.md` — new "Structured blocks and variants" subsection after Related-part ordering: variant from the section-reference `w:type`, missing/unrecognized `w:type` → `default` with no warning, first reference wins for shared/deduped parts, orphans unlabeled (key omitted, never null), flat text and tables never carry variant.
- `docs/formats/pptx.md` — new "Structured JSON" section: `slide` vs `notes` meaning (notes from `ppt/notesSlides/notesSlideN.xml`, notes block follows its slide), `p:sldIdLst` presentation-order resolution (not file names), global-ordinal semantics, "PPTX blocks never carry `variant`", and a structured-json example with `schema_version: 2`.
- `CHANGELOG.md` — Unreleased entry: schema v2 + variant labeling + block-level byte continuity + intentional v1-strict breakage migration note pointing at `schemas/v2/oxdoc-structured-text.schema.json`; PPTX structured output unchanged but now snapshot-locked.

### Cross-unit guards (tasks 33–35)

- Task 33: `git diff --name-only 8724d87..56266fb -- schemas/v1 docs/schemas/v1` → empty; the WU3 diff touches no v1 paths. Frozen-v1 invariant holds across the whole change.
- Task 34: whole-change snapshot diffs are exactly `cli_structured_text_json.json` and `cli_structured_text_pptx_json.json` (both sanctioned); the WU3 diff touches no snapshots.
- Task 35: WU1's oracle edit, `document.xml` edit, and parser/model change landed in one commit (d6e270f / PR #213); WU2's tests shipped with its code across its two commits; `git tag --contains 56266fb` → empty, so WU2 merged before any tag. WU3 lands docs + CHANGELOG in one commit.

### Verification evidence (WU3 final gate) and measured size

- `git diff main --stat` (excluding `.gitignore`): 5 files, 99 insertions(+), 3 deletions(−) = **102 changed lines** vs estimate ~85–100 — within the 400-line budget; no exception needed. PR 3 is the final stacked slice.
- `make` remains unavailable on this Windows host; `make docs-links` was exercised through its exact underlying `markdown-link-check` command with the repo config, and `docs-schemas-check` underlying `diff` passes were verified in WU2 (both mirrors byte-identical, untouched since).

### Commits (WU3)

- (this unit) docs: commit with the five files above.

### Remaining tasks

- None — all 35 tasks complete. Next: verify (optional) and archive per native status.
