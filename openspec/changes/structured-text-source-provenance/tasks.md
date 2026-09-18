# Tasks — structured text source provenance: versioned schema + header/footer variants (issue #179)

- Change id: `structured-text-source-provenance`
- Status: tasks (SDD tasks phase, artifact store: openspec)
- Inputs: `proposal.md`, `specs/docx-extraction/spec.md`, `specs/structured-text-schema/spec.md`,
  `design.md` (authoritative; three-unit split), `openspec/config.yaml`
- Execution: auto · artifact store openspec · delivery ask-on-risk · review budget 400 lines · strict TDD (`cargo test`)
- Design work units: **WU1** variant plumbing + oracle + regressions (~240–310) ·
  **WU2** versioned contract + CLI + snapshots (~300–380) · **WU3** documentation (~85–100)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | WU1 ~240–310 · WU2 ~300–380 · WU3 ~85–100 · total ~625–790 (design recount, #177 test/fixture multiplier applied) |
| 400-line budget risk | Medium (WU1 Medium, WU2 Medium–High, WU3 Low) |
| Chained PRs recommended | Yes (3 stacked-to-main PRs: WU1 → WU2 → WU3) |
| Suggested split | PR 1 (WU1) → PR 2 (WU2) → PR 3 (WU3) |
| Delivery strategy | ask-on-risk |
| Chain strategy | stacked-to-main (recommended by design; binding selection confirmed by the apply-phase ask, otherwise `pending`) |

Per-unit budget view:

| Work unit | Estimated lines | 400-line budget risk | Reason |
|-----------|-----------------|----------------------|--------|
| WU1 — variant plumbing + oracle + regressions | ~240–310 | Medium | Fixture oracle + API assertions + docx.rs unit tests weigh 1.5–2× first-pass estimates |
| WU2 — versioned contract + CLI + snapshots | ~300–380 | Medium–High | Two ~75–80-line schema files + `schema.rs` restructure + PPTX snapshot + CLI test upgrades |
| WU3 — documentation | ~85–100 | Low | Docs only, no Rust surface |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Medium
```

Delivery notes (ask-on-risk):

- The apply phase must pause for the delivery decision (chaining or not) rather than silently
  chaining or declaring `size:exception`; `exception-ok` is never inferred.
- WU1 alone emits `variant` under an unversioned payload: a legitimate intermediate state that
  violates no locked test (schema tests land in WU2) but **must not ship in a release** — WU2 must
  merge before any tag.
- Pre-declared fallback ladder if a realized unit approaches 400 lines: (1) move the PPTX snapshot +
  its `schema.rs`/`cli.rs` tests into a fourth unit, (2) split WU2 as declared, (3) `size:exception`
  only with explicit user acceptance.

## WU1 — Variant plumbing + oracle + regressions (~240–310 lines)

No schema file, no CLI payload change, no snapshot change in this unit.

### RED

- [x] 1. Add failing unit tests to `crates/oxdoc-core/src/parsers/docx.rs` (`#[cfg(test)]`, next to the existing plan-order tests around line ~2100): `labels_plan_entries_with_section_reference_variants`, `label_maps_variants_to_public_names`, `plans_deduped_part_with_first_reference_variant`. Run `cargo test -p oxdoc-core parsers::docx` and record the compile failure as the RED evidence (`label()` / variant-carrying plan entry do not exist yet). Implements: `docx-extraction` "Section-referenced variants are labeled", "Shared part referenced as two variants", "Orphan header part is unlabeled".
- [x] 2. Extend the hand-authored oracle `tests/fixtures/docx/section-order/expected.json` with `"variant"` on the seven section-referenced rows only — `word/header-first.xml` → `first`, `word/header-even.xml` → `even`, `word/header-default.xml` → `default`, `word/footer1.xml` → `default`, `word/header-titled.xml` → `default`, `word/footer2.xml` → `default` — and no `variant` key on `word/header-orphan.xml`, `main`, `comments`, `footnotes`. The oracle stays hand-authored truth, updated in the same commit as the parser change (never generated from output). Implements: `docx-extraction` "Unknown or missing w:type maps to default", "Orphan header part is unlabeled", "Non-variant blocks omit the field".
- [x] 3. Make the one-word fixture edit in `tests/fixtures/docx/section-order/package/word/document.xml`: section 2's `<w:headerReference r:id="rIdHeaderDefault"/>` gains `w:type="first"`, so the shared part is referenced as two different variants (`default` first in section 1, `first` later) with zero oracle or ordering change (`w:titlePg` stays set and unconsulted, per #177). Implements: `docx-extraction` "Shared part referenced as two variants".
- [x] 4. Extend `crates/oxdoc-core/tests/api.rs`: `orders_docx_structured_blocks_by_section` (line ~637) asserts `(part_type, part_path, variant)` triples from the oracle; add `keeps_non_variant_blocks_without_variant_key` (serialized `main`/`comments`/`footnotes` and PPTX `slide`/`notes` blocks have `.get("variant").is_none()`, never `null`), `ordinals_are_contiguous_in_output_order` (`ordinal == 1..=blocks.len()` in output order over the section-order fixture), and `keeps_plain_text_free_of_variant_metadata` (flat text over the same fixture equals the oracle part texts, with no `word/header` / `word/footer` paths, no variant tokens, no ordinals). Leave `orders_docx_text_related_parts_by_section` (line ~609) and `orders_docx_tables_by_section` (line ~659) asserting the unchanged two-tuples. Run `cargo test -p oxdoc-core --test api` and record RED. Implements: `docx-extraction` "Variant labels are carried by a versioned contract", `structured-text-schema` "Variant data does not leak into flat text", "Tables output unchanged", "Global ordinal semantics are documented and stable".

### GREEN

- [x] 5. In `crates/oxdoc-core/src/models.rs` (~line 187), add `#[serde(skip_serializing_if = "Option::is_none")] pub variant: Option<String>` as the **last** field of `TextBlock` (after `text`, preserving JSON field order), keep `TextBlock::new` at its exact signature producing `variant: None`, and add the consuming builder `with_variant(impl Into<String>)`. Do not add `Deserialize`. Implements: `docx-extraction` "Output shape unchanged except variant on structured header/footer blocks".
- [x] 6. In `crates/oxdoc-core/src/parsers/docx.rs`, change `type RelatedPartPlan = Vec<usize>` (line ~305) to `Vec<RelatedPartPlanEntry>` with `struct RelatedPartPlanEntry { index: usize, variant: Option<RelatedRefVariant> }`, and add `RelatedRefVariant::label(self) -> &'static str` (`First` → `"first"`, `Even` → `"even"`, `Default` → `"default"`). Keep the `Ord` derive and the untouched `sort_by_key((kind, variant))`. Implements: `docx-extraction` "Section-referenced variants are labeled".
- [x] 7. Update the three push sites in `plan_related_part_order` (lines ~466, ~493, ~505) with no algorithmic change: section-referenced loops push `RelatedPartPlanEntry { index, variant: Some(reference.variant) }` (positional-first falls out of the existing `emitted_paths` dedup), orphan and notes loops push `variant: None`. Implements: `docx-extraction` "Shared part referenced as two variants", "Orphan header part is unlabeled".
- [x] 8. Update consumers: `extract_text` (line ~62) and `extract_tables` (line ~189) destructure `entry.index` only (never naming `entry.variant`); `push_text_block` (line ~509) gains a `variant: Option<RelatedRefVariant>` parameter and applies `.with_variant(v.label())` exactly once; the main-part call site (line ~84) passes `None`, the related-part call site (line ~135) passes `entry.variant`. Empty text still returns before any variant attaches. PPTX (`crates/oxdoc-core/src/parsers/pptx.rs`) is not touched. Run `cargo test --workspace` → GREEN, with `docx_basic_text.txt` / `pptx_text.txt` snapshots and tables tests passing unmodified. Implements: `docx-extraction` "Output shape unchanged except variant on structured header/footer blocks", `structured-text-schema` "Tables output unchanged".

### TRIANGULATE

- [x] 9. Prove the edge cases with targeted runs: `cargo test -p oxdoc-core parsers::docx` (label mapping, plan variants, dedup first-reference), `cargo test -p oxdoc-core --test api` (oracle triples incl. `header-default` staying `default` although section 2 re-references it as `first`; `header-titled` `w:type="title"` → `default` with no new warning; orphan unlabeled; ordinal contiguity; plain-text non-leakage). Implements: `docx-extraction` "Unknown or missing w:type maps to default", "Orphan header part is unlabeled", "Non-variant blocks omit the field".
- [x] 10. Confirm the warning pin: the two `warnings` entries in `expected.json` (`rIdGhost`, missing `r:id`) are unchanged and `include_related_parts = false`, missing-rels, and error behavior are untouched by labeling. Implements: `docx-extraction` "Variant label edge cases follow existing dedup and orphan rules".

### REFACTOR

- [x] 11. Keep exactly one variant→string conversion site (`label()` inside `push_text_block`); no `String` allocation on the hot path; run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` clean.

### WU1 final verification

- [x] 12. Run and record: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and the coverage gate `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only`. Capture `git diff --stat` for WU1 and compare against the 240–310 estimate before opening PR 1.

## WU2 — Versioned contract + CLI + snapshots (~300–380 lines)

Depends on WU1 merged (variant field must exist before snapshots can omit it deliberately).

### RED

- [x] 13. Add failing CLI assertions in `crates/oxdoc-cli/tests/cli.rs`: `extracts_text_as_structured_json` (line ~200), `extracts_pptx_text_as_structured_json` (line ~252), and `extracts_structured_json_from_stdin_and_multiple_inputs` (line ~394) each assert the parsed payload's `schema_version == 2` (asserting per element in the multi-input array); the DOCX test additionally asserts `blocks[0].get("variant").is_none()`; upgrade `extracts_pptx_text_as_structured_json` from minimal field asserts to a byte comparison of `stdout(&output).trim_end()` against `fixtures::read_snapshot("cli_structured_text_pptx_json.json")` (the `pptx_text.txt` comparison style) plus a parsed-`Value` equality for failure readability; leave `emits_empty_structured_json_batch_when_no_inputs_succeed` (line ~438) untouched. Run `cargo test -p oxdoc-cli --test cli` and record RED. Implements: `structured-text-schema` "Single-input payload is versioned", "Multi-input payloads are each versioned", "PPTX snapshot locks the contract".
- [x] 14. Add failing schema tests in `crates/oxdoc-core/tests/schema.rs`: switch `representative_structured_text_json_matches_schema` (line ~20) to the v2 schema with the local `output["schema_version"] == schema["properties"]["schema_version"]["const"]` assertion; add `representative_structured_text_pptx_json_matches_schema` validating the new PPTX snapshot the same way; restructure `schemas_have_stable_public_metadata` (line ~235) to iterate `const SCHEMA_VERSIONS: &[(&str, &[&str])]` (`("v1", &[nine names])`, `("v2", &["oxdoc-structured-text.schema.json"])`) asserting `$id.ends_with(&format!("/schemas/{version}/{name}"))`; add the targeted negative test asserting a payload containing `schema_version: 2` fails v1 validation through the `validate_object` undeclared-field path. Run `cargo test -p oxdoc-core --test schema` and record RED (v2 schema file and PPTX snapshot missing). Implements: `structured-text-schema` "Snapshot validates against v2 schema", "v1 stays frozen", "v1-strict consumers must migrate", "Mirror lockstep".

### GREEN

- [x] 15. Create `schemas/v2/oxdoc-structured-text.schema.json` exactly as designed in design.md §3.1: draft 2020-12, `$id` `https://github.com/spereyra-dev/oxdoc/schemas/v2/oxdoc-structured-text.schema.json`, `additionalProperties: false`, top level requiring `schema_version` (`const: 2`), `file`, `document_type` (`enum: ["docx", "pptx"]`), `blocks`, and block requiring `part_type`, `part_path`, `ordinal` (minimum 1), `text` plus optional `variant` (`enum: ["first", "even", "default"]`) — with the semantics descriptions for `ordinal` (global output-order index), `notes` (speaker notes from `ppt/notesSlides/notesSlideN.xml`), and `variant` (missing/unrecognized `w:type` → `default`; section-referenced DOCX header/footer only; omitted never null; first reference wins). Implements: `structured-text-schema` "Structured-text schema version 2".
- [x] 16. Copy it byte-identically to `docs/schemas/v2/oxdoc-structured-text.schema.json` (mirror lockstep), and do **not** modify any file under `schemas/v1/**` or `docs/schemas/v1/**` (verify with `git status --porcelain schemas/v1 docs/schemas/v1`). Implements: `structured-text-schema` "Mirror lockstep", "v1 stays frozen".
- [x] 17. Add the v2 mirror pass to the `docs-schemas-check` target in `Makefile` (line ~146): keep `@diff -ru schemas/v1 docs/schemas/v1` and add `@diff -ru schemas/v2 docs/schemas/v2`. Implements: `structured-text-schema` "Mirror lockstep".
- [x] 18. Make `read_json_schema` (line ~267) version-aware (`read_json_schema(version: &str, name: &str)` joining `schemas/<version>/<name>`) and update its nine existing call sites mechanistically, with the metadata assertion deriving the version segment from the directory table so a future v3 is a one-line edit. Implements: `structured-text-schema` "Structured-text schema version 2".
- [x] 19. Add `schema_version: u8` as the first field of `TextStructuredPayload` in `crates/oxdoc-cli/src/main.rs` (~line 1469) and set `schema_version: 2` at the single construction site (~line 541), which already serves single-input, stdin (`-`), and multi-input through one file loop (multi-input emits the same object per array element). Leave core `StructuredText` unversioned. Implements: `structured-text-schema` "Single-input payload is versioned", "Multi-input payloads are each versioned", "Library serialization stays unversioned".
- [x] 20. Bump `tests/fixtures/snapshots/cli_structured_text_json.json` by adding `"schema_version": 2` as the first key and changing nothing else; the unchanged `blocks` array is the block-level byte-continuity evidence (its blocks carry no `variant`). Implements: `docx-extraction` "Single-section document blocks unchanged".
- [x] 21. Generate `tests/fixtures/snapshots/cli_structured_text_pptx_json.json` once from the implementation output over `tests/fixtures/corpus/pptx/text/` (via the existing `fixtures::build_package("pptx/text", "structured-slide.pptx")` path), then freeze it; review the content against design.md §5 before commit — block 1 `slide` / `ppt/slides/slide2.xml`, block 2 `notes` / `ppt/notesSlides/notesSlide2.xml` ("Speaker note"), block 3 `slide` / `ppt/slides/slide1.xml`, ordinals 1..3, every block without `variant`. The slide2-before-slide1 inversion is the load-bearing proof that `p:sldIdLst` order governs. Implements: `structured-text-schema` "Slide and notes blocks for a two-slide deck", "PPTX snapshot locks the contract".
- [x] 22. Run `cargo test --workspace` and `make docs-schemas-check` → GREEN. Implements: `structured-text-schema` "Snapshot validates against v2 schema", "Mirror lockstep".

### TRIANGULATE

- [x] 23. Verify every emission mode independently: single DOCX, PPTX, stdin, and multi-input each carry `schema_version: 2` (multi per element), the empty-batch behavior is unchanged, and the DOCX `blocks` array is otherwise byte-identical to the previous snapshot. Implements: `structured-text-schema` "Multi-input payloads are each versioned", `docx-extraction` "Single-section document blocks unchanged".
- [x] 24. Confirm the documented failure mode: the targeted negative test shows a v2 payload failing the frozen v1 schema via `additionalProperties: false`, with no file under `schemas/v1/**` / `docs/schemas/v1/**` touched. Implements: `structured-text-schema` "v1-strict consumers must migrate", "v1 stays frozen".

### REFACTOR

- [x] 25. Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` clean; ensure the const assertions stay local to the two structured tests (do not extend `validate_object`).

### WU2 final verification

- [x] 26. Run and record: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `make docs-schemas-check`, and the coverage gate `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only`. Capture `git diff --stat` per unit: if WU2 approaches 400 lines, **pause and ask** for the delivery decision instead of chaining silently or declaring `size:exception`. Note in the delivery ask that WU2 must be in the release before any tag (WU1's unversioned `variant` state must not ship).

## WU3 — Documentation (~85–100 lines)

Depends on WU2 merged (docs describe the shipped v2 contract).

- [ ] 27. Update `docs/json-output.md`: move the structured-json row to `schemas/v2/oxdoc-structured-text.schema.json`, keep a v1 link marked "for outputs captured before schema v2", add the v1-strict migration note, and add the structured-text semantics paragraph (payload `schema_version`, optional `variant`, global 1-based `ordinal`, `slide` vs `notes` with the `ppt/notesSlides/notesSlideN.xml` part path, and the deliberate unversioned-library-serialization asymmetry). Implements: `structured-text-schema` "Schema versioning policy", "v1-strict consumers must migrate", "Library serialization stays unversioned".
- [ ] 28. Update the structured-json example in `docs/cli.md` (~line 149) to show `schema_version: 2`, a header `variant` sample, and the slide/notes/global-ordinal wording. Implements: `structured-text-schema` "Single-input payload is versioned", "Global ordinal semantics are documented and stable".
- [ ] 29. Update `docs/formats/docx.md` with the structured-block variant statement: section-referenced headers/footers carry `variant` from the `sectPr` `w:type`; missing/unrecognized `w:type` → `default` with no warning; orphan header/footer parts are unlabeled; the first reference wins for shared/deduped parts; flat text and tables never carry variant. Implements: `docx-extraction` "Unknown or missing w:type maps to default", "Orphan header part is unlabeled", "Shared part referenced as two variants".
- [ ] 30. Add a "Structured JSON" section to `docs/formats/pptx.md` with the slide vs `notes` meaning, the `ppt/notesSlides/notesSlideN.xml` part path, `p:sldIdLst` presentation order, global-ordinal semantics, "PPTX blocks never carry `variant`", and a structured-json example. Implements: `structured-text-schema` "PPTX structured blocks are labeled slide and notes".
- [ ] 31. Add the `CHANGELOG.md` entry (schema v2, variant labeling, block-level byte continuity, and the migration note pointing v1-strict consumers at `schemas/v2/oxdoc-structured-text.schema.json`). Implements: `structured-text-schema` "v1-strict consumers must migrate".

### WU3 final verification

- [ ] 32. Run and record: `make docs-check docs-links docs-schemas-check`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and the coverage gate `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` (docs touch no Rust, but the full gate must stay green before PR 3).

## Cross-unit guard tasks

- [ ] 33. Confirm the frozen-v1 invariant on the whole change diff: `git diff --name-only` lists no path under `schemas/v1/**` or `docs/schemas/v1/**`. Implements: `structured-text-schema` "v1 stays frozen".
- [ ] 34. Confirm no snapshot outside `tests/fixtures/snapshots/cli_structured_text_json.json` and `tests/fixtures/snapshots/cli_structured_text_pptx_json.json` changed; any other snapshot diff is a stop-and-investigate variant-leakage signal, not a mechanical update. Implements: `structured-text-schema` "Variant data does not leak into flat text", "Tables output unchanged".
- [ ] 35. Preserve work-unit commit boundaries: WU1's oracle edit, `document.xml` edit, and parser/model change land in the same commit; each commit keeps its tests with its code; WU2 merges before any tag/release.
