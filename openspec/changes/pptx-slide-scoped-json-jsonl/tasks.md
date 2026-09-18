# Tasks — pptx-slide-scoped-json-jsonl (#180)

- Change: `pptx-slide-scoped-json-jsonl`
- Delivery: `ask-on-risk` · review budget 400 changed lines · strict TDD (`cargo test`) · 95% line-coverage gate
- Authoritative design: `design.md` §7 split (WU1 → WU2 → WU3 → WU4), amended by the two parent corrections below.
- Spec: `specs/pptx-slide-extraction/spec.md` (11 requirements, 28 scenarios).

Parent corrections folded into this plan:

1. **Spec amendment (applied).** The stdin file label is `<stdin>` (repo-wide
   `display_file_name` precedent), not `-`. Scenarios updated in
   `specs/pptx-slide-extraction/spec.md` (JSON document contract, JSONL stream
   contract, CLI stdin scenario). Consequence: **no `slides_file_label` helper**
   is written; `extract_slides_command` reuses `display_file_name`
   (`crates/oxdoc-cli/src/main.rs`). Design §4.2's `-` callout is superseded.
2. **WU4 split.** WU4 at ~400–460 realized lines is over budget, so it becomes
   **WU4a — CLI subcommand + CLI tests** and **WU4b — docs only**, producing a
   **5-PR chain stacked to main**.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1,330–1,590 realized (raw ~1,000 before the 1.5–2× test/fixture multiplier) |
| 400-line budget risk | High (whole change) — units WU2/WU3/WU4a sit at the edge |
| Chained PRs recommended | Yes — 5 PRs |
| Suggested split | PR 1 (WU1) → PR 2 (WU2) → PR 3 (WU3) → PR 4 (WU4a) → PR 5 (WU4b), stacked to main |
| Delivery strategy | ask-on-risk (pause and ask per unit before any oversized diff) |
| Chain strategy | stacked-to-main |

Per-unit estimates (raw × 1.5–2× multiplier on test/fixture weight, post-#177 lesson):

| Work unit | Content | Raw | Realistic (1.5–2×) | 400-line budget risk |
| --- | --- | --- | --- | --- |
| WU1 — fixtures, provenance, output-neutral `@id` parse | 2 corpus trees, 2 provenance notes, provenance-list updates, `SlideReference`/`slide_id_value` refactor + refactor-hazard regression tests | ~150–200 | ~240–300 | Low |
| WU2 — core per-slide API | `PptxSlideText`, `pptx::extract_slides` loop, `read_notes_text_for_slides`, 3 lib entry points, R1–R8 `api.rs` tests | ~250 | ~360–400 | Medium (at edge) |
| WU3 — schema + snapshots | `schemas/v1/oxdoc-pptx-slides.schema.json` + `docs/schemas/v1` mirror, oneOf-aware `schema.rs` harness, validation/negative tests, 2 snapshots | ~250 | ~330–380 | Medium |
| WU4a — CLI subcommand + CLI tests | `ExtractCommand::Slides`, `SlidesFormat`, `extract_slides_command`, payload/record structs, `Input::extract_pptx_slides`, R9/R10 `cli.rs` tests, snapshot byte-compares | ~250 | ~300–380 | High (at edge) |
| WU4b — docs only | `docs/json-output.md`, `docs/cli.md`, `docs/formats/pptx.md`, `README.md`, `CHANGELOG.md` | ~100–130 | ~100–130 | Low |
| **Total** | | **~1,000** | **~1,330–1,590** | **High** |

Fallbacks already agreed in `design.md` §7 (use before relaxing anything):

- WU2 realized > 400 → move its third commit group (security/hard-error + wrapper
  tests, tasks 18–21) into a follow-up stacked unit; never relax coverage.
- WU3 realized > 400 → move the two snapshot files into WU4a (WU3 then ~300).
- WU4a realized > 400 → do **not** merge docs back in; docs already live in WU4b,
  so re-slice WU4a CLI tests instead and pause to ask.

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
```

## Cross-unit guards (apply to every task below)

- **Frozen outputs.** `tests/fixtures/snapshots/pptx_text.txt`,
  `cli_structured_text_pptx_json.json`, `cli_structured_text_json.json`,
  `pptx_basic_info.json`, and all DOCX tables / XLSX rows / audit / info / plain
  text snapshots MUST be byte-identical (`spec.md` → Existing PPTX extraction
  stays byte-identical → Pre-existing snapshots unchanged).
- **Frozen schemas.** No file under `schemas/v1/**`, `schemas/v2/**`, or
  `docs/schemas/**` other than the two new `oxdoc-pptx-slides.schema.json` files
  may be modified or deleted (`spec.md` → Versioned slides schema contract →
  Structured-text schemas stay frozen).
- **Stop-and-investigate.** Any unexpected snapshot churn, any diff in
  `tests/fixtures/compatibility-matrix.json` or `tests/fixtures/files/**`, or any
  newly-failing existing test means STOP: investigate and report rather than
  updating the snapshot, regenerating a digest, or regenerating output.
- **No blast radius.** No change to `TextBlock`, `StructuredText`,
  `DocxTables`, `DocxTextOptions`, `Extraction<T>`, `--format structured-json`,
  `extract text|tables|rows|csv|info|audit`, limits, or dependencies.
- **No new CLI surface.** No `--output`/`--output-dir`, no multi-input batch, no
  per-line error contract for `extract slides`.
- **Per-unit pause.** Measure realized `additions + deletions` at each unit gate;
  if a unit would exceed 400 lines, pause and ask (ask-on-risk) instead of
  shipping the oversized diff.

---

## WU1 — Fixtures, provenance, and output-neutral `@id` parse

PR 1. No user-visible feature. Goal: all new fixtures exist, and the shared
slide-reference parse carries `@id` + `p:sldIdLst` position while every existing
output stays byte-identical.

### Fixtures and provenance

- [x] 1. Create the runtime-zipped corpus tree `tests/fixtures/corpus/pptx/missing-target/` exactly as `design.md` §5.2 specifies: `[Content_Types].xml` (overrides only for parts that exist), `_rels/.rels`, `ppt/presentation.xml` with four `p:sldId` entries (`@id` 256/`r:id` `rId101`, `@id` 257/`rId999` dangling, `@id` 258/`rId102` → absent part, `@id` 259/`rId103`), `ppt/_rels/presentation.xml.rels` (`rId101`→`slides/slide1.xml`, `rId102`→`slides/absent.xml`, `rId103`→`slides/slide3.xml`), `ppt/slides/slide1.xml` ("Intact Slide"), `ppt/slides/slide3.xml` ("Notes Slide"), `ppt/slides/_rels/slide3.xml.rels` (notesSlide rel targeting the sibling ZIP part `ppt/notesSlides/notesSlide3.xml`). Hand-authored XML only, no binary, no `tests/fixtures/files/` entry. Drives R2 (gap `1,4`), R5 (three skip scenarios), R8 (old paths hard-error), R12.
- [x] 2. Create the runtime-zipped corpus tree `tests/fixtures/corpus/pptx/malformed-xml/` per `design.md` §5.3: two slides, `ppt/slides/slide2.xml` truncated mid-tag after opening `p:sld/cSld/spTree/sp/txBody/a:p/a:r/a:t` with `Partial Text`; `[Content_Types].xml`, `_rels/.rels`, `ppt/presentation.xml`, `ppt/_rels/presentation.xml.rels` complete. Drives R6 and R12.
- [x] 3. Add `tests/fixtures/provenance/pptx-missing-target.md` and `tests/fixtures/provenance/pptx-malformed-xml.md` following the `tests/fixtures/provenance/pptx-text.md` label set (`Source:`, `Producer:`, `Redistribution:`, `Purpose:`, `Archive generation:`, stated no-`Sanitization:` justification), and add both file names to the `fixture_provenance_notes_are_present` arrays in `crates/oxdoc-core/tests/api.rs` and `crates/oxdoc-cli/tests/cli.rs` (R12 → Provenance and corpus gates pass).
- [x] 4. Verify tasks 1–3: `cargo test -p oxdoc-core fixture_provenance_notes_are_present`, `cargo test -p oxdoc-cli fixture_provenance_notes_are_present`, `make compatibility-corpus-check`, and confirm `git status --porcelain` shows no change under `tests/fixtures/files/` or `tests/fixtures/compatibility-matrix.json`.
- [x] 5. Prove the new corpora are actually load-bearing before writing production code: add a temporary ignored/debug check (or a throwaway `cargo test -p oxdoc-core --test api` run) that builds both trees with `fixtures::build_package("pptx/missing-target", "missing-target.pptx")` and `fixtures::build_package("pptx/malformed-xml", "malformed-xml.pptx")` and asserts the archives open and list the expected part names; delete the throwaway check once the trees are confirmed.

### RED — parser tests and refactor-hazard regression tests

- [x] 6. RED (parser unit): in `crates/oxdoc-core/src/parsers/pptx.rs` add tests for `slide_id_value`: `@id` absent → `None`, `@id="abc"` → `None`, `@id="256"` → `Some(256)`, plus a truncated-XML warning case for `ppt/presentation.xml` (path `ppt/presentation.xml`, partial list, `W001` — sliding through the same `parse_slide_references` API). Still no shared parser available → `cargo test -p oxdoc-core --lib parsers::pptx` fails.
- [x] 7. RED (refactor hazard, `spec.md` → "p:sldId without @id still extracts" and → "Old paths hard-error on missing targets"): add `crates/oxdoc-core/tests/api.rs` tests that (a) an inline package whose `p:sldId` has a valid `r:id` but no `@id` still yields the same `extract_pptx_text` and `extract_pptx_structured_text` output as before the refactor, and (b) `extract_pptx_text` still extracts a `p:sldId` with an `@id` exactly as today. Record the current expected strings as inline literals (no new snapshot files).
- [x] 8. Capture the byte-identity baseline for the existing paths before touching shared code: run `cargo test --workspace` and record the passing snapshot set (or `git rev-parse HEAD` + `cargo test -p oxdoc-cli --test cli` output) in the PR description so any later churn is attributable.

### GREEN — shared slide-reference parse

- [x] 9. GREEN: in `crates/oxdoc-core/src/parsers/pptx.rs` add the private
  `SlideReference { position: usize, relation_id: String, slide_id: Option<u32> }`
  struct, replace `parse_slide_relation_ids` with `parse_slide_references`
  (increment `position` for every `p:sldId` start/empty element, keep the exact
  `ignored presentation slide without relationship id` warning and path), add
  `slide_id_value(&BytesStart)` matching only unprefixed `id` and `parse::<u32>()`,
  and adapt `extract_text`/`extract_structured_text` call sites to read only
  `relation_id`. `relationship_id_value` must stay untouched.
- [x] 10. GREEN: update the existing parser unit test to the new return type
  (assert `position` 1/2, `relation_id` `rId2`/`rId1`, `slide_id`
  `Some(256)`/`Some(257)` for `tests/fixtures/corpus/pptx/text/ppt/presentation.xml`)
  and run `cargo test -p oxdoc-core --lib parsers::pptx` → green.
- [x] 11. TRIANGULATE: add the missing cases that the refactor could silently
  break — (a) `p:sldId` without `r:id` still warns once and shifts `position`
  (so ordinals count every element), (b) `p:sldId` with `@id` but no `r:id` emits
  no `slide_id`-related warning, (c) `@id="0"` parses as `Some(0)` (integer
  passthrough, no validation fiction), (d) two identical `@id` values are both
  carried through (no dedup invented by the parser).

### REFACTOR + WU1 gate

- [x] 12. REFACTOR: keep `parse_slide_references` and `slide_id_value` free of
  unused surface (`clippy -D warnings`), keep `slide_id` reachable only by the
  new contract (no existing consumer reads it), and confirm by inspection that
  no existing entry point references `slide_id`.
- [x] 13. WU1 gate — run and record: `cargo test --workspace`,
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `make coverage` (≥ 95% lines), `make docs-schemas-check`,
  `make compatibility-corpus-check`. Diff the frozen snapshot list from task 8:
  any diff is a stop-and-investigate event, not a snapshot update.
- [ ] 14. WU1 exit: re-measure realized changed lines
  (`git diff --stat` against the WU1 base). Expect ~240–300; if > 400, pause and
  ask before opening PR 1. Then open **PR 1 (WU1)** against the current main and
  note in its description that no user-visible behavior changed.

---

## WU2 — Core per-slide API

PR 2, stacked on PR 1. Goal: `PptxSlideText` + `extract_pptx_slides*` fully
implemented with the exact spec warnings and R1–R8 covered.

### RED/GREEN — model

- [ ] 15. RED (model, R4): in `crates/oxdoc-core/src/models.rs` add a test module
  case asserting `serde_json::to_value(&PptxSlideText { slide_id: None, … })`
  omits the `slide_id` key, that `notes: None` omits the `notes` key, that
  `notes: Some("".into())` serializes `"notes": ""`, that `text: String::new()`
  serializes `"text": ""`, and that no key is ever `null`. Fails to compile until
  the model exists.
- [ ] 16. GREEN: add `pub struct PptxSlideText { slide_id: Option<u32>,
  slide_ordinal: usize, slide_path: String, text: String, notes: Option<String> }`
  with `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]` and
  `skip_serializing_if = "Option::is_none"` on `slide_id` and `notes`
  (`design.md` §1.1). Re-export it from `crates/oxdoc-core/src/lib.rs` in the
  existing `pub use models::{…}` list. `cargo test -p oxdoc-core --lib models` →
  green.

### RED/GREEN — three lib entry points + happy path

- [ ] 17. RED (R1/R2/R3/R4, `crates/oxdoc-core/tests/api.rs`): tests that
  `extract_pptx_slides` on `corpus/pptx/text` yields ordinals `1,2`, slide ids
  `256,257`, `slide_path` `ppt/slides/slide2.xml` then `ppt/slides/slide1.xml`,
  `notes: Some("Speaker note\n")` on the first record only, and that each record's
  `slide_path` equals the `part_path` the structured path reports; plus
  `corpus/pptx/basic` → one record with no `notes` key and no slide-id/notes
  nulls. Also add the reader variants coverage: `extract_pptx_slides_from_reader`
  and `extract_pptx_slides_from_reader_with_limits` on the same bytes.
- [ ] 18. GREEN: add `pptx::extract_slides<R: Read + Seek>(package) -> Result<Extraction<Vec<PptxSlideText>>>`
  (`design.md` §2 steps 1–3) and `pub(crate) fn extract_pptx_slides[_from_reader[_with_limits]]`
  in `crates/oxdoc-core/src/lib.rs` delegating exactly like the PPTX text family.
  `cargo test -p oxdoc-core --test api` → green for the happy path.

### RED/GREEN — skip-with-warning loop and locked warning texts

- [ ] 19. RED (R5, path `ppt/presentation.xml`): `api.rs` test on the
  `missing-target` fixture asserting the exact strings
  `skipped PPTX slide rId999: unknown relationship id`,
  `skipped related PPTX slide part ppt/slides/absent.xml: missing part`
  (warning path = `ppt/slides/absent.xml`),
  `skipped related PPTX notes part ppt/notesSlides/notesSlide3.xml: missing part`
  (warning path = `ppt/notesSlides/notesSlide3.xml`), that the extraction returns
  `Ok`, that the intact slides are present, and that ordinals are `1,4` (gap
  `2,3` is intentional and not renumbered).
- [ ] 20. RED (R5, missing rels part + all-skipped): inline zip package with no
  `ppt/_rels/presentation.xml.rels` → only
  `skipped PPTX slide {rid}: unknown relationship id` per `r:id`, no
  missing-rels warning of its own, `Ok`, empty record set; and an inline
  all-skipped deck asserting `warnings` non-empty with `value.len() == 0`.
- [ ] 21. GREEN: implement the loop's skip branches at the single emission site in
  `extract_slides` with inline `format!` warnings (`design.md` §2 step 3): unknown
  relationship id → warn + `continue`; `MissingPart` from `read_text_part` → warn
  + `continue`; `read_notes_text_for_slides` returning `Err(MissingPart)` → warn
  and emit the record with `notes: None`; missing presentation rels →
  `Err(OxdocError::MissingPart(_))` mapped to an empty relationship map with no
  warning. `cargo test -p oxdoc-core --test api` → green.

### RED/GREEN — malformed partial text and textless slides

- [ ] 22. RED (R6): test on the `malformed-xml` fixture asserting record 1 is
  intact, record 2 is emitted with recovered partial text plus exactly one
  `W001`/`malformed_xml` warning whose path is `ppt/slides/slide2.xml`, and
  ordinals are `1,2` with no gap.
- [ ] 23. RED (R4 textless): inline package with an empty `txBody` slide →
  record emitted with `text == ""` and the slide is not dropped.
- [ ] 24. GREEN: ensure slide reads merge the `read_text_part` warnings (including
  `W001`) into the record's extraction warnings and never convert a readable
  malformed part into a skip; confirm the record is pushed unconditionally.
  `cargo test -p oxdoc-core --test api` → green.

### RED/GREEN — security, hard errors, and leniency scoping

- [ ] 25. RED (R7): `api.rs` tests mirroring the existing PPTX suspicious-target
  test — a slide rel with an external/escaping/NUL target returns
  `Err(OxdocError::SuspiciousRelationshipTarget { … })` from
  `extract_pptx_slides` with no records; a package without `ppt/presentation.xml`
  returns a hard error (no partial record set).
- [ ] 26. GREEN: ensure `resolve_relationship_target` errors propagate unchanged
  (never downgraded to a skip) and `find_office_document_path(package,
  "ppt/presentation.xml")?` stays the first hard failure. `cargo test -p oxdoc-core --test api` → green.
- [ ] 27. RED (R8 / spec → Old paths hard-error on missing targets):
  `extract_pptx_text` **and** `extract_pptx_structured_text` on
  `missing-target.pptx` both return `Err(OxdocError::MissingPart(_))`, and the
  same package still extracts successfully (with skips) through
  `extract_pptx_slides` — the asymmetry is asserted, not just implemented.
- [ ] 28. TRIANGULATE/RED (R2 gaps + notes presence rule): a deck where the
  second of three slides is skipped → ordinals `1,3`; a slide whose notes part is
  readable but empty → `notes: Some("")`; a slide whose `.rels` part is absent →
  `notes: None` with no warning.
- [ ] 29. GREEN/REFACTOR: satisfy 27–28 without duplicating the notes resolution
  logic (`read_notes_text_for_slides` stays the only notes path), keep the
  rel-id sort order for notes relationships, and keep the notes concatenation
  deterministic (`append_part_text`). `cargo test -p oxdoc-core --test api` → green.

### WU2 gate

- [ ] 30. WU2 gate — run and record: `cargo test -p oxdoc-core`,
  `cargo test --workspace` (frozen snapshots must still pass untouched),
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `make coverage`
  (new parser branches in tasks 19–28 must be covered ≥ 95%).
- [ ] 31. WU2 exit: re-measure realized changed lines. Expect ~360–400; if > 400,
  apply the agreed fallback (move the task 25–29 test group to a follow-up
  stacked unit) or pause and ask — never relax coverage and never inline the
  group into an oversized PR. Then open **PR 2 (WU2)** stacked on PR 1.

---

## WU3 — Versioned schema contract and snapshots

PR 3, stacked on PR 2. Goal: the new v1 contract exists, is mirrored, is
registered in the harness, validates both new snapshots, and rejects
structured-text schemas.

### RED/GREEN — registration and oneOf-aware harness

- [ ] 32. RED (harness, R11): in `crates/oxdoc-core/tests/schema.rs` add
  `"oxdoc-pptx-slides.schema.json"` to `SCHEMA_VERSIONS`'s `v1` list and make
  `assert_schema_metadata` oneOf-aware (when the schema declares a top-level
  `oneOf`, assert top-level `"type": "object"` and assert
  `additionalProperties == false` on **every** branch; keep the `$schema`/`$id`
  checks unchanged). `cargo test -p oxdoc-core --test schema` fails because the
  schema file does not exist yet.
- [ ] 33. GREEN: create `schemas/v1/oxdoc-pptx-slides.schema.json` with the exact
  structure in `design.md` §3.1 — draft 2020-12, `$id`
  `https://github.com/spereyra-dev/oxdoc/schemas/v1/oxdoc-pptx-slides.schema.json`,
  top-level `type: object` + `oneOf` over `$defs.documentPayload` and
  `$defs.jsonlRecord`, shared `$defs.slide` (record branch `$ref`s
  `#/$defs/slide/properties/…`), `additionalProperties: false` on both branches
  and on `$defs.slide`, `schema_version` `const: 1`,
  `document_type` `const: "pptx"`, `slide_id` integer, `slide_ordinal` integer
  `minimum: 1`, `warning` object mirroring
  `schemas/v1/oxdoc-docx-tables.schema.json`'s `$defs.warning`, and the
  ordinal-gap rule in `description`.
- [ ] 34. GREEN: copy the schema byte-identically to
  `docs/schemas/v1/oxdoc-pptx-slides.schema.json`, then run
  `make docs-schemas-check` (`diff -ru schemas/v1 docs/schemas/v1` must be
  silent) and `cargo test -p oxdoc-core --test schema` → green for metadata.

### Snapshots with documented provenance

- [ ] 35. Generate the two snapshots from WU2's real core output and commit them:
  add a throwaway generator test in `crates/oxdoc-core/tests/` that builds
  `corpus/pptx/text` with `fixtures::build_package("pptx/text", "slides-deck.pptx")`,
  calls `extract_pptx_slides`, and serializes (a) the pretty payload
  `{schema_version: 1, file: "slides-deck.pptx", document_type: "pptx", slides, warnings: []}`
  to `tests/fixtures/snapshots/cli_pptx_slides_json.json` with a trailing newline
  and (b) one compact record per slide, each
  `{schema_version: 1, file: "slides-deck.pptx", …slide}`, to
  `tests/fixtures/snapshots/cli_pptx_slides_jsonl.jsonl`. Expected content: record
  1 `slide_id 256`, `slide_ordinal 1`, `ppt/slides/slide2.xml`,
  text `"First Slide\nAlpha\tBeta & Co\nGamma < Delta\n"`,
  notes `"Speaker note\n"`; record 2 `slide_id 257`, `slide_ordinal 2`,
  `ppt/slides/slide1.xml`, text `"Second Slide\n"`, no `notes` key. Record the
  generation command in the PR description (the snapshot's provenance), then
  delete the throwaway generator so no duplicate payload type is left behind.
- [ ] 36. RED (R11 → Snapshots validate against the new schema): add
  `validate_against(schema_branch, object)` extracted from `validate_object`, a
  delegating `validate_object`, and `validate_slides_object(schema, output)` that
  picks the branch whose `required` fields are all present (`design.md` §3.2);
  assert the `cli_pptx_slides_json.json` snapshot validates with local assertions
  `schema_version == 1` and `document_type == "pptx"`, and validate the first
  line of `cli_pptx_slides_jsonl.jsonl` plus an inline record with `slide_id`
  omitted (optionality pinned).
- [ ] 37. GREEN: make 36 pass; all nine pre-existing schema tests must still pass
  with the oneOf-aware harness (no existing assertion weakened).
- [ ] 38. RED/GREEN negative test (R11 → Slides payload is not a structured-text
  payload): assert `cli_pptx_slides_json.json` FAILS validation against
  `schemas/v2/oxdoc-structured-text.schema.json` and
  `schemas/v1/oxdoc-structured-text.schema.json` using the existing
  `catch_unwind` pattern from `v2_payload_fails_frozen_v1_validation`.

### WU3 gate

- [ ] 39. WU3 gate — run and record: `cargo test -p oxdoc-core --test schema`,
  `cargo test --workspace`, `make docs-schemas-check`,
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `git status --porcelain schemas docs/schemas` showing only the two new
  `oxdoc-pptx-slides.schema.json` files (frozen-schema guard).
- [ ] 40. WU3 exit: re-measure realized changed lines (expect ~330–380; if > 400,
  apply the agreed fallback of moving both snapshot files into WU4a, or pause and
  ask). Then open **PR 3 (WU3)** stacked on PR 2.

---

## WU4a — `extract slides` CLI subcommand and CLI tests

PR 4, stacked on PR 3. Goal: `oxdoc extract slides` json/jsonl works from file
and stdin, warnings follow the channel rules, and the snapshots byte-compare.

### RED/GREEN — subcommand, JSON payload

- [ ] 41. RED (R9 JSON document contract → JSON payload shape): in
  `crates/oxdoc-cli/tests/cli.rs` add a test running
  `oxdoc extract slides <slides-deck.pptx>` with **no** `--format` flag,
  asserting success, empty stderr, stdout byte-equal to
  `cli_pptx_slides_json.json`, top-level keys
  `schema_version`(`1`)/`file`/`document_type`(`"pptx"`)/`slides`/`warnings`,
  exactly two `slides` records, and `notes` present on exactly the
  notes-bearing record.
- [ ] 42. GREEN: in `crates/oxdoc-cli/src/main.rs` add
  `ExtractCommand::Slides { file: PathBuf, #[arg(long, value_enum, default_value_t = SlidesFormat::Json)] format: SlidesFormat }`,
  `#[derive(Debug, Clone, Copy, ValueEnum)] enum SlidesFormat { Json, Jsonl }`,
  the dispatch arm calling `extract_slides_command(&file, format, warning_format)?`,
  the `SlidesPayload` struct (`schema_version`, `file`, `document_type`,
  `slides`, `warnings`), and the `Input::extract_pptx_slides` method (modeled on
  `Input::extract_pptx_structured_text`, always passing `cli_limits().ooxml`).
  Emit stderr warnings first via `emit_warnings`, then
  `serde_json::to_writer_pretty(io::stdout().lock(), &payload)?` plus trailing
  newline. Reuse `display_file_name` for the file label — **no** `slides_file_label`
  helper (`<stdin>` per the amended spec). `cargo test -p oxdoc-cli --test cli` → green.
- [ ] 43. RED/GREEN (R9 → All slides skipped still yields a payload): inline
  all-skipped package → exit code `0`, payload with `"slides": []` and embedded
  warnings, and `--quiet` leaves the embedded `warnings` intact while producing
  empty stderr.

### RED/GREEN — JSONL stream

- [ ] 44. RED (R10 → One record per slide, same order as JSON): test asserting
  `--format jsonl` on `corpus/pptx/text` produces stdout byte-equal to
  `cli_pptx_slides_jsonl.jsonl`, one compact record per line, no wrapping array,
  and the same field values/order as the JSON `slides` array.
- [ ] 45. GREEN: add `SlidesJsonlRecord<'a> { schema_version: u8, file: &'a str,
  #[serde(flatten)] slide: &'a PptxSlideText }`, write each record with
  `serde_json::to_writer` + `\n`, `flush()` stdout, then call `emit_warnings`
  **after** the flush (rows-jsonl precedent: stdout stays a pure record stream).
- [ ] 46. RED/GREEN (R10 → Valid stream under warnings): `--format jsonl` on the
  `missing-target` fixture → every stdout line parses as JSON with no warning text
  on stdout, all three warnings appear on stderr, and the emitted ordinals show
  the `1,4` gap; `--warnings json` renders the stderr lines as JSON warning
  payloads (`OwnedWarningPayload::from_output_warning`).
- [ ] 47. RED/GREEN (R10 CLI stdin scenario, amended): `oxdoc extract slides - --format jsonl`
  with the package piped in → same records as the file argument and the `file`
  field is `<stdin>`; assert the label explicitly so the amended spec is locked.

### RED/GREEN — type gate and hard errors

- [ ] 48. RED/GREEN (CLI → DOCX input rejected): `oxdoc extract slides <report.docx>`
  fails with `CliError::InvalidArgument` in the same style as
  `extract_docx_tables`' rejections (message `cannot extract slides from a DOCX
  document`), exit `1`, no slide output; add the XLSX analogue
  (`cannot extract slides from an XLSX workbook`).
- [ ] 49. RED/GREEN (hard errors at CLI level): a package with a suspicious slide
  target and a package without `ppt/presentation.xml` both exit non-zero through
  the existing `error[…]` handler with no partial payload/records on stdout.

### WU4a gate

- [ ] 50. WU4a gate — run and record: `cargo test -p oxdoc-cli`,
  `cargo test --workspace` (frozen snapshots untouched), `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `make coverage`,
  `make compatibility-corpus-check`.
- [ ] 51. WU4a exit: re-measure realized changed lines (expect ~300–380). If > 400,
  do not merge docs back in (they are WU4b) — re-slice the CLI tests or pause and
  ask. Then open **PR 4 (WU4a)** stacked on PR 3.

---

## WU4b — Documentation and changelog

PR 5, stacked on PR 4. Docs-only: no production code, no new tests beyond
link/schema gates.

- [ ] 52. `docs/formats/pptx.md`: add the "Slide-scoped JSON / JSONL" section —
  record fields, `slide_id`/`slide_ordinal` identity rules including gaps after
  skips, the `notes` presence rule, `slide_path` provenance, the three exact
  warning wordings with their channels, and the `extract slides` command
  (R14 / spec → Docs describe the contract).
- [ ] 53. `docs/json-output.md`: add the schema table rows
  (`| oxdoc extract slides --format json | schemas/v1/oxdoc-pptx-slides.schema.json |`
  and the JSONL row), the slide payload/JSONL semantics, the warning-channel
  statement (JSONL stderr-only; JSON embedded plus stderr mirror subject to
  `--warnings`/`--quiet`), and the version-policy note that this is a **new**
  contract rather than a structured-text widening (v1/v2 structured-text frozen).
- [ ] 54. `docs/cli.md`: add `extract slides` usage, a runnable example, the
  default `json` format, the `-` stdin form with the `<stdin>` file label, and the
  ordinal/skip semantics (`1..=N` over `p:sldIdLst`, gaps after skips).
- [ ] 55. `README.md`: add an `oxdoc extract slides` command example alongside the
  existing tables/rows examples; `CHANGELOG.md`: record the new subcommand and the
  new `schemas/v1/oxdoc-pptx-slides.schema.json` contract.
- [ ] 56. Confirm no existing documentation of `extract text`, `structured-json`,
  or the v2 schema had its meaning altered (`git diff docs README.md` review).
- [ ] 57. WU4b gate: `make docs-links` (no link rot),
  `make docs-schemas-check`, `make docs-check`, and `make ci` end-to-end. Then open
  **PR 5 (WU4b)** stacked on PR 4.

---

## Final verification (after PR 5)

- [ ] 58. Run the full local gate on the completed stack: `make ci`
  (`fmt-check`, `check`, `clippy`, `test`, `doctest`, `coverage` ≥ 95%,
  `scripts-test`, `docs-check`, `docs-links`, `docs-schemas-check`,
  `docs-playground-check`, `build-release`) plus
  `make compatibility-corpus-check` and `python-test`.
- [ ] 59. Confirm success criteria 1–11 from `proposal.md` end-to-end, including:
  byte-identical pre-existing snapshots, `schemas/v1/**`/`schemas/v2/**` frozen
  except the two new files, `extract_pptx_text`/`extract_pptx_structured_text`
  still `MissingPart` on `missing-target`, and no change to
  `tests/fixtures/compatibility-matrix.json` or any fixture digest.
- [ ] 60. Confirm no task created ownership metadata, delivery gates, or
  `size:exception`; per-unit realized line counts stay ≤ 400 (or each overage was
  an explicit, asked-for decision).

---

## Spec/design reconciliation notes for apply

- **`<stdin>` label supersedes design §4.2.** Reuse `display_file_name`
  (`crates/oxdoc-cli/src/main.rs`, returns `<stdin>` for `-`); do not add
  `slides_file_label`. Snapshot/CLI expectations must therefore read
  `"file": "<stdin>"` for the stdin scenario (tasks 41, 47).
- **`missing-target` has four `p:sldId` entries** (`design.md` §5.2 is
  authoritative over the proposal's "three slides" wording): emitted ordinals are
  `1` and `4`, with all three warnings asserted (task 19).
- **`@id`-less slides are emitted** with `slide_id` omitted and no warning
  (spec R1 overrides the proposal's earlier skip idea) — nothing in WU1–WU4a may
  turn that into a skip.
