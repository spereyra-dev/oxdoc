# Apply Progress — pptx-slide-scoped-json-jsonl

- Store: openspec
- Branch: `issue-180-pptx-slides-wu1` (PR 1 of a 5-PR stacked-to-main chain)
- Base at apply start: `d4427b2` (`main`)
- Mode: strict TDD (`cargo test`), review budget 400 changed lines, `ask-on-risk`

## WU1 — Fixtures, provenance, and output-neutral `@id` parse

### Completed tasks (tasks.md updated to `[x]`)

| Task | Result |
| --- | --- |
| 1 | `tests/fixtures/corpus/pptx/missing-target/` created per design §5.2: 4 `p:sldId` entries (`256`/`rId101`, `257`/`rId999` dangling, `258`/`rId102`→absent part, `259`/`rId103`), rels `rId101`→slide1, `rId102`→absent.xml, `rId103`→slide3, slide1 "Intact Slide", slide3 "Notes Slide" + notesSlide rel to sibling `ppt/notesSlides/notesSlide3.xml`, content-type overrides only for existing parts. Hand-authored XML only; no `tests/fixtures/files/` entry. |
| 2 | `tests/fixtures/corpus/pptx/malformed-xml/` created per design §5.3: 2 slides, `slide2.xml` truncated mid-tag after `<a:t>Partial Text`, complete presentation/rels/content-types. |
| 3 | `tests/fixtures/provenance/pptx-missing-target.md` + `pptx-malformed-xml.md` written following the `pptx-text.md` label set (`Source:`/`Producer:`/`Redistribution:`/`Purpose:`/`Archive generation:`, no `Sanitization:` stated); both names added to `fixture_provenance_notes_are_present` arrays in `crates/oxdoc-core/tests/api.rs` and `crates/oxdoc-cli/tests/cli.rs`. |
| 4 | `cargo test -p oxdoc-core fixture_provenance_notes_are_present` → 1 passed; `cargo test -p oxdoc-cli fixture_provenance_notes_are_present` → 1 passed; `python scripts/check-compatibility-corpus.py` → "compatibility corpus validation passed (3 fixtures)"; `git status --porcelain tests/fixtures/files tests/fixtures/compatibility-matrix.json` → no changes. |
| 5 | Throwaway test `crates/oxdoc-core/tests/wu1-corpus-throwaway.rs` built both packages via `fixtures::build_package`, asserted archive part lists and absence of `ppt/slides/absent.xml`/`ppt/notesSlides/notesSlide3.xml`, confirmed `extract_pptx_text` errors on `missing-target.pptx` and yields `"Before Truncation\nPartial Text\n"` on `malformed-xml.pptx`; DELETED after confirmation (not in any commit). |
| 6 | RED: parser tests added for `slide_id_value`/`parse_slide_references` (`@id` absent → `None`, `@id="abc"` → `None`, `@id="256"` → `Some(256)`, truncated presentation XML → 1 partial ref + `W001` path `ppt/presentation.xml`); `cargo test -p oxdoc-core --lib parsers::pptx` failed with `E0425 cannot find function parse_slide_references` (RED confirmed). |
| 7 | Regression tests `extracts_pptx_sldid_without_id_attribute_unchanged` and `extracts_pptx_sldid_with_id_attribute_unchanged` added in `crates/oxdoc-core/tests/api.rs` with inline expected literals; both passed pre-refactor (hazard pins) and post-refactor. |
| 8 | Baseline captured at base `d4427b2` with task-1–5 fixtures present: `cargo test --workspace` all green (cli bin 25, cli 99, core lib 103, api 79, schema 12, tabular lib 9 + 2 + 0). Frozen snapshots untouched throughout. |
| 9 | GREEN: `SlideReference { position, relation_id, slide_id }` added; `parse_slide_relation_ids` replaced by `parse_slide_references` (position increments for every `p:sldId` start/empty element; exact `ignored presentation slide without relationship id` warning and path kept); `slide_id_value(&BytesStart)` matches only unprefixed `id` and parses `u32` (quick-xml 0.42: `attr.value` is `Cow<str>`); `extract_text`/`extract_structured_text` call sites read only `relation_id`; `relationship_id_value` untouched. |
| 10 | Existing parser unit test updated to new return type: `position` 1/2, `relation_id` `rId2`/`rId1`, `slide_id` `Some(256)`/`Some(257)`; `cargo test -p oxdoc-core --lib parsers::pptx` → 12 passed. |
| 11 | TRIANGULATE: (a) `p:sldId` without `r:id` warns once and shifts `position` (ordinals 1 and 3 asserted); (b) `p:sldId` with `@id` but no `r:id` emits only the pre-existing ignored-rel warning (no `slide_id`-related warning); (c) `@id="0"` → `Some(0)`; (d) duplicate `@id` values both carried (no dedup). |
| 12 | REFACTOR: `parse_slide_references` + `slide_id_value` clippy-clean; verified by inspection that no existing entry point references `slide_id` (`grep` across `crates/oxdoc-core/src` shows reads only in `parsers/pptx.rs` and its tests). |
| 13 | WU1 gate: `cargo test --workspace` all green (frozen snapshots byte-identical); `cargo fmt --all -- --check` OK; `cargo clippy --workspace --all-targets -- -D warnings` OK; `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → line coverage 96.35% TOTAL, exit 0; `diff -ru schemas/v1 docs/schemas/v1` and `diff -ru schemas/v2 docs/schemas/v2` silent; `python scripts/check-compatibility-corpus.py` passed. No snapshot churn → no stop-and-investigate event. |

### Files changed (vs `main`)

- `crates/oxdoc-core/src/parsers/pptx.rs` (+~208/−17): `SlideReference`, `slide_id_value`, `parse_slide_references`, call-site adaptation, 9 new parser tests.
- `crates/oxdoc-core/tests/api.rs` (+86): 2 refactor-hazard regression tests + 2 provenance list entries.
- `crates/oxdoc-cli/tests/cli.rs` (+2): 2 provenance list entries.
- `tests/fixtures/corpus/pptx/missing-target/**` (7 files), `tests/fixtures/corpus/pptx/malformed-xml/**` (6 files): new hand-authored XML trees.
- `tests/fixtures/provenance/pptx-missing-target.md`, `tests/fixtures/provenance/pptx-malformed-xml.md`.

### Commits

1. `877cd45` `test(pptx): add missing-target and malformed-xml slide corpora with provenance` (fixtures + provenance + list updates; 17 files, 209 insertions)
2. `3f62aea` `refactor(pptx): carry @id and sldIdLst position in shared slide-reference parse` (parser refactor + regression tests; 2 files, 292 insertions, 17 deletions)

### TDD Cycle Evidence (strict TDD)

| Cycle | RED | GREEN | Evidence |
| --- | --- | --- | --- |
| Parser `@id` parse | 4 new tests referencing nonexistent `parse_slide_references` → compile fail `E0425` (`cargo test -p oxdoc-core --lib parsers::pptx`) | `SlideReference` + `parse_slide_references` + `slide_id_value` implemented | 12 parser lib tests pass |
| Refactor hazard | Task-7 api regression tests written pre-refactor (passed against old code — they pin, not drive) | refactor applied; both still pass | `cargo test -p oxdoc-core --test api` 81 passed |
| TRIANGULATE | position/duplicate/zero/`@id`-without-`r:id` cases added with GREEN batch | all pass | included in the 12 lib tests |

### Test commands run

- `cargo test -p oxdoc-core --test wu1-corpus-throwaway` → 2 passed (deleted afterwards)
- `cargo test -p oxdoc-core --lib parsers::pptx` → RED (compile) then 12 passed
- `cargo test -p oxdoc-core --test api extracts_pptx_sldid` → 2 passed
- `cargo test -p oxdoc-core fixture_provenance_notes_are_present` → 1 passed
- `cargo test -p oxdoc-cli fixture_provenance_notes_are_present` → 1 passed
- `cargo test --workspace` → all 8 test targets `ok` (frozen snapshots untouched)
- `cargo fmt --all -- --check` → OK
- `cargo clippy --workspace --all-targets -- -D warnings` → OK
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → lines 96.35%, exit 0
- `python scripts/check-compatibility-corpus.py` → passed (3 fixtures)
- `make` unavailable on this machine; `make compatibility-corpus-check`, `make docs-schemas-check`, `make coverage` equivalents run directly (python script, `diff -ru`, `cargo llvm-cov`).

### Deviations from design

- Design §1.3 shows `slide_id_value` reading `attr.value` as bytes (`std::str::from_utf8(&attr.value)`); quick-xml 0.42 already yields `Cow<str>`, so the helper parses `attr.value.trim().parse::<u32>()` directly. Same matching rule (unprefixed `id` only), same `None` semantics.
- Task 14's "open PR 1" was NOT executed: no push, no PR (parent instructed commits only, no PRs), and the realized diff exceeds the budget (see below).

## ⚠ STOP — review budget exceeded (ask-on-risk)

Measured at WU1 exit (task 14 measurement step):

```
git diff main --shortstat -- . ':(exclude).gitignore'
18 files changed, 417 insertions(+), 17 deletions(-)   → 434 changed lines
```

434 > 400 review-budget lines. WU1's estimate was ~240–300 realized; the extra weight is the
high-fidelity fixture XML (13 files, ~120 lines) plus parser tests (~180 lines) that carry the
strict-TDD evidence. Per `ask-on-risk`, apply paused instead of opening PR 1 or slicing further:

- No commit was shrunk (no comments/tests/docs deleted or compressed).
- Each individual commit is under 400 lines (209 and 275); the PR diff (both commits vs `main`) is 434.
- Nothing has been pushed; no PRs created.

Decision needed from the maintainer (any one):

1. `size:exception` — accept the 434-line PR 1 as a cohesive work unit (fixtures + output-neutral parse are one unit per design §7).
2. Re-slice WU1 into two stacked PRs (fixtures 209 / parser 275), making the chain 6 PRs.
3. Drop some fixture fidelity (e.g. single-slide variants) — not recommended; fixtures drive R2/R5/R6/R8/R12 scenarios.

## Remaining tasks (unchecked)

- [ ] 14. WU1 exit: re-measure realized changed lines (`git diff --stat` against the WU1 base). Expect ~240–300; if > 400, pause and ask before opening PR 1. Then open **PR 1 (WU1)** against the current main and note in its description that no user-visible behavior changed. — measurement DONE (434 > 400 → paused); PR opening awaits the budget decision above.
- WU2 tasks 15–31, WU3 tasks 32–40, WU4a tasks 41–51, WU4b tasks 52–57, final verification tasks 58–60 (all unchecked, later units).

### Structured status

- Consumed native `gentle-ai.sdd-status` v2 before work: change `pptx-slide-scoped-json-jsonl`, `applyState: ready`, `nextRecommended: apply`, `actionContext.mode: repo-local`, `allowedEditRoots: [repo root]` — no blockers, no warnings.
- After WU1: `taskProgress` moves from 0/60 to 13/60; apply remains in progress; budget decision pending (native status must be re-read before further apply work).


---

## WU2 — Core per-slide API (this session)

- Store: openspec
- Branch: `issue-180-pptx-slides-wu2` (PR 3 of the 6-PR stacked-to-main chain)
- Base at apply start: `b1b9fb8` (`main` after WU1 merged as #217 + #218)
- WU1 resolution recorded: the maintainer resolved WU1's 434-line budget stop by
  splitting it into two stacked PRs (option 2), merged as #217 (fixtures,
  `2be69d1`) and #218 (output-neutral `@id` parse, `b1b9fb8`). Task 14 marked
  `[x]` accordingly.

### Completed tasks (tasks.md updated to `[x]`)

| Task | Result |
| --- | --- |
| 15 | RED (model serde): `models.rs` test asserting `serde_json::to_value(&PptxSlideText { slide_id: None, .. })` omits `slide_id`, `notes: None` omits `notes`, `notes: Some("")` serializes `"notes": ""`, `text: String::new()` serializes `"text": ""`, and no key is ever `null`. `cargo test -p oxdoc-core --lib models` compiled RED with `E0432 unresolved import crate::models::PptxSlideText`. |
| 16 | GREEN: `pub struct PptxSlideText` with `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]` and `skip_serializing_if = "Option::is_none"` on `slide_id: Option<u32>` and `notes: Option<String>` (design §1.1); re-exported in `lib.rs`'s `pub use models::{...}` list. `cargo test -p oxdoc-core --lib models` -> 8 passed. |
| 17 | RED (api): 4 happy-path tests — text corpus order/ids/paths (`256`/`ppt/slides/slide2.xml` first, notes `Some("Speaker note\n")` on record 1 only, `slide_path` == structured `part_path`), basic corpus record with no `notes` key and integer `slide_id` (no nulls), reader + reader-with-limits variants byte-equal to the path variant. `cargo test -p oxdoc-core --test api extracts_pptx_slides` -> compile RED `E0425 cannot find function extract_pptx_slides`. One test-assertion correction during GREEN: `corpus/pptx/basic`'s single slide is textless (`<p:spTree/>`), so the record asserts a string `text` field and `slide_id: 256` rather than non-empty text (task 17's wording: "no notes key and no slide-id/notes nulls"). |
| 18 | GREEN: `pptx::extract_slides` loop (main part via `find_office_document_path` first, presentation rels, per-reference loop pushing `PptxSlideText` always), `read_notes_text_for_slides` (rels missing -> `None`, no notesSlide rel -> `None`, readable -> `Some(joined)` via `append_part_text`, rel-id sort kept), and the three lib entry points delegating exactly like the PPTX text family. Initial GREEN deliberately used old-path semantics (hard errors) so cycle 3's skip tests are genuine RED; `has_notes_relationships.then_some(text)` (not text emptiness) is the `Some("")` discriminator. api `extracts_pptx_slides` -> 4 passed. |
| 19 | RED: missing-target fixture test asserting the three exact spec strings, warning paths (`ppt/presentation.xml` / `ppt/slides/absent.xml` / `ppt/notesSlides/notesSlide3.xml`), `Ok`, intact slides present, ordinals `1,4` (gap `2,3` not renumbered). RED: `Err(MissingPart("rId999"))` panic. |
| 20 | RED: inline package without `ppt/_rels/presentation.xml.rels` -> per-`r:id` unknown-relationship-id warnings only, no missing-rels warning, `Ok`, empty record set (RED: `Err(MissingPart("ppt/_rels/presentation.xml.rels"))`); inline all-skipped deck -> `Ok`, empty records, 1 warning (RED: `Err(MissingPart("rId404"))`; also fixed an `rId401`->`rId404` test typo). |
| 21 | GREEN: skip branches implemented at the single emission site in `extract_slides` with inline `format!` warnings (design §2 step 3): unknown rel id -> warn + `continue`; `read_text_part` `MissingPart` -> warn + `continue`; `read_notes_text_for_slides` `Err(MissingPart(path))` -> warn with the notes path + record with `notes: None`; presentation rels `MissingPart` -> empty map, no warning. Suspicious-target and other errors propagate. `cargo test -p oxdoc-core --test api` -> 88 passed. |
| 22 | RED (R6): malformed-xml fixture test asserting record 1 intact, record 2 emitted with recovered partial text + exactly one `W001`/`malformed_xml` warning at `ppt/slides/slide2.xml`, ordinals `1,2` with no gap. RED revealed a **WU1 fixture authoring defect**: `slide2.xml` ended after `<a:t>Partial Text` with the element merely unclosed at EOF — quick-xml tolerates that, so W001 could never fire (investigated: no existing test pinned the fixture's extraction behavior; provenance note unchanged in meaning). Fix per design §5.3's "cuts off mid-tag": appended an incomplete `<a:` so the truncation is mid-tag; recoverable text stays `"Partial Text\n"`, old-path output unchanged. Test then green. |
| 23 | RED (R4 textless): inline empty-`txBody` slide -> record emitted with `text == ""`, not dropped. PASSED on first run against the already-correct unconditional push (pin, not driver — it pins task 24's "record pushed unconditionally"). |
| 24 | GREEN: `read_text_part` warnings (incl. `W001`) merged into the record's extraction warnings and a readable-but-malformed part is never converted into a skip — pinned by the task-22 fixture test (record 2 emitted, 1 warning, no gap). |
| 30 | WU2 gate: `cargo test -p oxdoc-core` -> lib 112 / api 90 / schema 12 / doc-compat 2 passed; `cargo test --workspace` -> all 8 targets ok (frozen snapshots untouched); `cargo fmt --all -- --check` OK; `cargo clippy --workspace --all-targets -- -D warnings` OK; `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` -> **lines 96.34% TOTAL, exit 0** (pptx.rs lines 96.20%); `python scripts/check-compatibility-corpus.py` -> passed (3 fixtures); `git status --porcelain tests/fixtures/compatibility-matrix.json tests/fixtures/files` -> no changes. |
| 31 | WU2 exit measurement: `git diff main --shortstat -- . ':(exclude).gitignore'` -> `5 files changed, 455 insertions(+), 3 deletions(-)` = **458 changed lines > 400**. Pre-agreed fallback applied (parent-authorized): tasks 25-29 (security/hard-error + wrapper test group) move WHOLE (never split) into a follow-up stacked unit; no coverage relaxation, no test splitting, no commit after the overage. PR opening is parent-owned (session contract: no push, no PRs); WU2 maps to PR 3 of the 6-PR chain. |

### TDD Cycle Evidence (strict TDD)

| Cycle | RED | GREEN | Evidence |
| --- | --- | --- | --- |
| Model serde (15-16) | `E0432 unresolved import PptxSlideText` (`--lib models`) | model + re-export | 8 lib models tests pass |
| Entry points + happy path (17-18) | `E0425 cannot find function extract_pptx_slides[_from_reader[_with_limits]]` | `extract_slides` minimal loop + `read_notes_text_for_slides` + 3 lib wrappers | 4 api tests pass |
| Skip-with-warning (19-21) | 3 tests fail on real behavior: `Err(MissingPart("rId999"))`, `Err(MissingPart("ppt/_rels/presentation.xml.rels"))`, `Err(MissingPart("rId404"))` | skip branches + empty-rels map at the single emission site | 88 api tests pass incl. exact warning strings and ordinals `1,4` |
| Malformed partial text (22-24) | warnings `0` vs `1` on the fixture (WU1 fixture defect found and fixed) | fixture corrected to mid-tag truncation; warning-merge and unconditional push pinned | 90 api tests pass |

### Files changed (vs `main`)

- `crates/oxdoc-core/src/models.rs` (+60): `PptxSlideText` + serde unit test.
- `crates/oxdoc-core/src/lib.rs` (+25/-3): 3 entry points + `PptxSlideText` re-export.
- `crates/oxdoc-core/src/parsers/pptx.rs` (+111): `extract_slides`, `read_notes_text_for_slides`.
- `crates/oxdoc-core/tests/api.rs` (+261): 11 new WU2 tests.
- `tests/fixtures/corpus/pptx/malformed-xml/ppt/slides/slide2.xml` (+1): mid-tag truncation fix (WU1 fixture defect; no digest/matrix/snapshot impact).

### Commits

1. `aa1e61b` `feat(pptx): add slide-scoped PptxSlideText model and per-slide extraction API` (model + entry points + minimal loop + happy-path tests; 4 files, 245 insertions, 3 deletions)
2. `1672cad` `feat(pptx): skip missing slide targets with locked per-slide warnings` (skip branches + api tests + malformed-xml fixture fix)

### Test commands run

- `cargo test -p oxdoc-core --lib models` -> RED (compile) then 8 passed
- `cargo test -p oxdoc-core --test api extracts_pptx_slides` -> RED (compile) then 4 passed
- `cargo test -p oxdoc-core --test api` -> RED (3 skip tests, 1 malformed test) then 88/90 passed
- `cargo test --workspace` -> all 8 targets ok
- `cargo test -p oxdoc-core` -> 112/90/12/2 passed
- `cargo fmt --all -- --check` -> OK
- `cargo clippy --workspace --all-targets -- -D warnings` -> OK
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` -> lines 96.34% TOTAL, exit 0
- `python scripts/check-compatibility-corpus.py` -> passed (3 fixtures)
- `make` unavailable on this machine; gate equivalents run directly as above.

### Deviations from design

- **Task 17 basic-corpus expectation:** design/spec R4's "both carry non-empty text" scenario is not satisfiable by `corpus/pptx/basic` (its single slide is textless). Task 17's authoritative wording ("no notes key and no slide-id/notes nulls") is what the test asserts; the textless -> `text: ""` behavior is separately covered by the inline textless test.
- **malformed-xml fixture amended (WU1 fix):** the merged WU1 `slide2.xml` could not produce W001 (unclosed-at-EOF is silently tolerated by quick-xml). Corrected to a genuine mid-tag truncation matching design §5.3; recoverable text and old-path output unchanged; no digest/matrix/snapshot impact.
- **Fallback realized:** tasks 25-29 moved whole to a follow-up stacked unit (see below); hard-error propagation for suspicious targets and missing `ppt/presentation.xml` already exists in the implemented loop, but its slides-path regression tests land with the follow-up unit.
- Tasks 30/31 marked `[x]`; the "open PR 2" clause is parent-owned (session forbids push/PR creation); WU2 maps to PR 3 of the 6-PR chain.

## Remaining tasks (unchecked after WU2)

- [ ] 25. RED (R7): suspicious-target hard-error api tests
- [ ] 26. GREEN: propagation unchanged (lands with the 25-29 tests)
- [ ] 27. RED (R8): old-path `MissingPart` asymmetry regression tests
- [ ] 28. TRIANGULATE/RED: three-slide gap `1,3`, notes-readable-but-empty `Some("")`, absent slide `.rels` -> `notes: None`
- [ ] 29. GREEN/REFACTOR: notes-resolution dedup pin (`read_notes_text_for_slides` stays the only notes path)
- WU3 tasks 32-40, WU4a tasks 41-51, WU4b tasks 52-57, final verification tasks 58-60 (all unchecked, later units)

### Follow-up unit needed (fallback applied)

**WU2b — security/hard-error + wrapper regression tests (tasks 25-29, moved whole).**
New stacked unit after WU2 (next PR position in the chain; later units shift by one).
Content: only the task 25-29 api tests plus any small production adjustment they
drive (none expected — propagation is already implemented). Reason: WU2 realized
458 lines > 400 budget; the group was never split; coverage unaffected (96.34%
TOTAL >= 95 gate). Per `ask-on-risk` the parent should confirm this unit boundary
when opening PRs.

### Structured status (WU2)

- Consumed native `gentle-ai.sdd-status` v2 before WU2 work: `applyState: ready`, `nextRecommended: apply`, `blockedReasons: []`, `actionContext.mode: repo-local`, `allowedEditRoots: [repo root]`.
- After WU2: `taskProgress` 26/60 completed (tasks 14, 15-24, 30, 31 checked; 25-29 + WU3/WU4/final unchecked). Apply remains in progress for the follow-up unit; the budget fallback pre-authorized by the parent instruction ("move the security/wrapper test group whole, never split its tests") was used instead of an `ask-on-risk` pause; no `size:exception` used or needed for this unit.

---

## WU2c — Security / hard-error / wrapper regression tests (tasks 25-29, moved whole by the WU2 fallback)

- Store: openspec
- Branch: `issue-180-pptx-slides-wu2c` (PR 4 of the stacked-to-main chain; parent session applies the WU2 fallback as its own unit)
- Base at apply start: `a7d047c` (`main` after WU1 merged as #217 + #218 and WU2 merged as #219 + #220)
- Scope: exactly tasks 25-29 (the security/hard-error + wrapper test group), moved WHOLE per the pre-agreed WU2 fallback; never split. No production code was expected or written.

### Completed tasks (tasks.md updated to `[x]`)

| Task | Result |
| --- | --- |
| 25 | RED (R7): 4 hard-error tests in `crates/oxdoc-core/tests/api.rs` — external slide target (`TargetMode="External"` → `SuspiciousRelationshipTarget` with path `ppt/_rels/presentation.xml.rels`, target `https://example.invalid/slide1.xml`, reason contains `external`), escaping slide target (`../../outside.xml` → suspicious, path + target pinned), NUL in slide target (`Target="slides/slide&#0;1.xml"` decoded via `&#0;` → suspicious reason contains `NUL`), and a package without `ppt/presentation.xml` → `Err(MissingPart("ppt/presentation.xml"))` with no partial record set. Tests written before any change; see TDD evidence note below. |
| 26 | GREEN: verified by inspection + tests that `resolve_relationship_target` errors propagate unchanged (never downgraded to a skip) and `find_office_document_path(package, "ppt/presentation.xml")?` remains the first hard failure in `extract_slides`. No production edit was needed; `cargo test -p oxdoc-core --test api` → 99 passed. |
| 27 | RED (R8): `old_pptx_paths_still_hard_error_on_missing_targets_while_slides_extract` — `extract_pptx_text` AND `extract_pptx_structured_text` on `missing-target.pptx` both return `Err(MissingPart("rId999"))`, while the same package still extracts via `extract_pptx_slides` with records `1,4` and 3 warnings — the asymmetry is asserted, not just implemented. |
| 28 | TRIANGULATE (R2 gaps + notes presence rule): `keeps_ordinal_gap_when_middle_slide_of_three_is_skipped` (second of three skipped → ordinals `1,3`, one missing-part warning); `reads_empty_readable_notes_part_as_present_empty_string` (readable empty notes part → `notes: Some("")`, serialized as `"notes": ""`); `omits_notes_without_warning_when_slide_rels_part_is_absent` (absent slide `.rels` → `notes: None` omitted key, no warning). |
| 29 | GREEN/REFACTOR: verified `read_notes_text_for_slides` remains the only notes-resolution path for slides (no duplication; grep confirms `read_notes_for_slide`/`read_notes_blocks_for_slide` serve only the old text/structured paths, untouched), rel-id sort order and deterministic `append_part_text` concatenation unchanged. No production edit needed. |
| (wrapper parity) | Parent's WU2c scope also names reader-wrapper/limits parity: added `reads_missing_target_slides_identically_through_reader_wrappers` asserting `extract_pptx_slides_from_reader` and `..._with_limits(OoxmlLimits::default())` produce value AND warnings identical to the path variant on the `missing-target` fixture (task 17 already covered the happy-path variants). |

### TDD Cycle Evidence (strict TDD)

| Cycle | RED | GREEN | Evidence |
| --- | --- | --- | --- |
| Security hard errors (25-26) | Tests written first; no RED failure was achievable because WU2's `extract_slides` already propagates `SuspiciousRelationshipTarget`/`MissingPart` correctly (task 21 GREEN). These are characterization pins in the task-23/task-7 precedent: breaking them would require artificially breaking correct production code, which was not staged. | No production change; tests pass and pin the propagation contract. | 9 new api tests pass |
| Old-path asymmetry (27) | Same pin status: WU1 refactored without changing old-path error semantics, and WU2's skip branches are scoped to `extract_slides` only. Test asserts the asymmetry on the real fixture (both old paths `MissingPart("rId999")`, slides path `Ok`). | passes | included in the 9 |
| Ordinal gap + notes rule (28-29) | Same pin status; exercises the implemented skip/`Some("")`/`None` branches with fresh inline packages. | passes | included in the 9 |

### Files changed (vs `main`)

- `crates/oxdoc-core/tests/api.rs` (+274): 9 new WU2c tests (4 security/hard-error, 1 old-path asymmetry, 2 notes-presence triangulation, 1 ordinal gap, 1 reader-wrapper parity).

### Test commands run

- `cargo test -p oxdoc-core --test api slides_` → 8 passed (first partial run)
- `cargo test -p oxdoc-core --test api -- rejects_external_pptx rejects_pptx_slide rejects_nul fails_hard_when_pptx old_pptx_paths keeps_ordinal_gap reads_empty_readable_notes omits_notes_without_warning reads_missing_target` → 9 passed
- `cargo test --workspace` → all 8 test targets ok (cli bin 25, cli 99, core lib 112, api 99, schema 12, tabular 9 + 2 + 0); frozen snapshots untouched
- `cargo fmt --all -- --check` → OK (after one mechanical fmt pass on the new tests)
- `cargo clippy --workspace --all-targets -- -D warnings` → OK
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → lines 96.37% TOTAL, exit 0 (pptx.rs lines 96.75%)
- `python scripts/check-compatibility-corpus.py` → passed (3 fixtures); `git status --porcelain tests/fixtures/compatibility-matrix.json tests/fixtures/files` → no changes

### WU2c gate + exit measurement

- Measured `git diff main --shortstat -- . ':(exclude).gitignore'` → `1 file changed, 274 insertions(+)` = **274 changed lines ≤ 400 budget**. No pause needed; no `size:exception`.
- No stop-and-investigate event: no snapshot churn, no fixture/matrix changes, no newly-failing existing test.

### Deviations from design

- None in production (no production change at all). The 27-task RED and 28-task TRIANGULATE tests pass on first run against the already-correct WU2 implementation; per the task-23 precedent they are recorded as regression pins rather than drivers, since a genuine RED would require artificially breaking correct propagation that task 21 already landed.
- `OxdocError` does not implement `PartialEq`, so the `MissingPart` assertions use `matches!(err, OxdocError::MissingPart(path) if path == ...)` instead of `assert_eq!` (matches the file's existing error-assertion style).

### Commits

1. (this commit) `test(pptx): pin slide-extraction security hard errors and notes leniency` — api tests + openspec artifact updates.

### Structured status (WU2c)

- Consumed native `gentle-ai.sdd-status` v2 before work: change `pptx-slide-scoped-json-jsonl`, `applyState: ready`, `nextRecommended: apply`, `taskProgress 26/60`, `blockedReasons: []`, `actionContext.mode: repo-local`, `allowedEditRoots: [repo root]`.
- After WU2c: tasks 25-29 marked `[x]`; `taskProgress` moves to 31/60. Remaining: WU3 (32-40), WU4a (41-51), WU4b (52-57), final verification (58-60).

---

## WU3 — Versioned schema contract and snapshots (this session)

- Store: openspec
- Branch: `issue-180-pptx-slides-wu3` (PR position 5 of the stacked-to-main chain after the WU2 split)
- Base at apply start: `8152760` (`main` after WU1 #217/#218, WU2 #219/#220, WU2c #221)
- Scope: exactly tasks 32-40 (schema + mirror, oneOf-aware harness, snapshots, validation + negative tests). Strict TDD active.

### Completed tasks (tasks.md updated to `[x]`)

| Task | Result |
| --- | --- |
| 32 | RED (harness, R11): `"oxdoc-pptx-slides.schema.json"` added to `SCHEMA_VERSIONS` v1 list; `assert_schema_metadata` made oneOf-aware (for `$ref` branches it asserts `additionalProperties == false` on every resolved branch; schemas with inline oneOf discrimination branches — `oxdoc-audit-jsonl` — keep the top-level strictness assertion, discovered when the first cut broke that test). `cargo test -p oxdoc-core --test schema` → FAILED `schemas_have_stable_public_metadata` (schema file absent) while the other 11 tests passed: genuine RED. |
| 33 | GREEN: `schemas/v1/oxdoc-pptx-slides.schema.json` created per design §3.1 — draft 2020-12, stable `$id`, top-level `type: object` + `oneOf` over `$defs.documentPayload`/`$defs.jsonlRecord`, shared `$defs.slide` with property-level `$ref`s (`#/$defs/slide/properties/…`) in the record branch, `additionalProperties: false` on both branches + `$defs.slide`, `schema_version` const 1, `document_type` const "pptx", `slide_id` integer, `slide_ordinal` integer minimum 1, `warning` def mirroring `oxdoc-docx-tables`'s, ordinal-gap rule in `description` fields. 15 metadata tests green (12 previous + registration). |
| 34 | Mirror copied byte-identically to `docs/schemas/v1/oxdoc-pptx-slides.schema.json`; `diff -ru schemas/v1 docs/schemas/v1` and `diff -ru schemas/v2 docs/schemas/v2` both silent; `cargo test -p oxdoc-core --test schema` → 12 passed. |
| 35 | Snapshots generated from WU2's real core output via a throwaway test `crates/oxdoc-core/tests/wu3-snapshot-generator.rs` (`cargo test -p oxdoc-core --test wu3-snapshot-generator` → 1 passed): built `corpus/pptx/text` as `slides-deck.pptx` with `fixtures::build_package`, called the real `extract_pptx_slides`, serialized (a) the pretty payload `{schema_version:1, file:"slides-deck.pptx", document_type:"pptx", slides, warnings:[]}` → `tests/fixtures/snapshots/cli_pptx_slides_json.json` (trailing newline) and (b) compact records `{schema_version:1, file:"slides-deck.pptx", ...slide}` via `#[serde(flatten)]` → `cli_pptx_slides_jsonl.jsonl`. Content verified against expectations: record 1 `slide_id 256`, ordinal 1, `ppt/slides/slide2.xml`, text `"First Slide\nAlpha\tBeta & Co\nGamma < Delta\n"`, notes `"Speaker note\n"`; record 2 `slide_id 257`, ordinal 2, `ppt/slides/slide1.xml`, text `"Second Slide\n"`, no `notes` key; `"warnings": []` present in the payload. Generator DELETED after generation (provenance: this apply-progress entry + the generation command in this table; no duplicate payload type left behind). |
| 36 | RED (R11): tests `representative_pptx_slides_json_payload_matches_schema` + `representative_pptx_slides_jsonl_record_matches_schema_shape` (payload snapshot with local assertions `schema_version == 1`/`document_type == "pptx"`; JSONL first line; inline record with `slide_id` omitted to pin optionality). RED: compile error `E0425 cannot find function validate_slides_object`. |
| 37 | GREEN: `validate_object` body extracted into `validate_against(definition, output, root)` (delegating wrapper unchanged in behavior; property-level local `#`-refs resolved against the schema root — the audit-jsonl schema's external same-dir `$ref` `"oxdoc-audit.schema.json"` is deliberately NOT resolved, preserving its old `type: object` check), plus `validate_slides_object` picking the branch whose `required` fields are all present. All 14 tests pass (nine pre-existing schema tests untouched and passing). Clippy `-D warnings` clean after fixing one `needless_borrow`. |
| 38 | Negative test `slides_payload_fails_frozen_structured_text_validation`: `cli_pptx_slides_json.json` FAILS validation against `schemas/v2/oxdoc-structured-text.schema.json` AND `schemas/v1/oxdoc-structured-text.schema.json` using the `catch_unwind` pattern (undeclared `slides` field). `cargo test -p oxdoc-core --test schema` → 15 passed. |

### Test commands run

- `cargo test -p oxdoc-core --test schema` → RED (schema file missing), RED (`E0425 validate_slides_object`), then 12 → 14 → 15 passed
- `cargo test -p oxdoc-core --test wu3-snapshot-generator` → 1 passed (throwaway, deleted)
- `cargo test --workspace` → all 8 targets ok (cli bin 25, cli 99, core lib 112, api 99, schema 15, tabular 9 + 2 + 0); frozen snapshots untouched
- `cargo fmt --all -- --check` → OK
- `cargo clippy --workspace --all-targets -- -D warnings` → OK
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` → **lines 96.37% TOTAL, exit 0**
- `diff -ru schemas/v1 docs/schemas/v1` / `diff -ru schemas/v2 docs/schemas/v2` → silent (docs-schemas-check)
- `python scripts/check-compatibility-corpus.py` → passed (3 fixtures); no change under `tests/fixtures/files/` or `tests/fixtures/compatibility-matrix.json`
- `git status --porcelain schemas docs/schemas` → only the two new `oxdoc-pptx-slides.schema.json` files (frozen-schema guard respected)

### TDD Cycle Evidence (strict TDD)

| Cycle | RED | GREEN | Evidence |
| --- | --- | --- | --- |
| Harness registration + oneOf metadata (32-33) | `schemas_have_stable_public_metadata` fails (schema file missing) | schema created; first harness cut broke `oxdoc-audit-jsonl`'s inline-branch oneOf → corrected to `$ref`-branch resolution + top-level strictness for inline branches | 12 tests pass |
| Snapshot validation (36-37) | `E0425 cannot find function validate_slides_object` (compile RED) | `validate_against`/`validate_object`/`validate_slides_object` refactor + branch picking | 14 tests pass, 9 pre-existing untouched |
| Negative vs structured-text (38) | test written first (no RED achievable: it pins the frozen schemas' rejection, which already holds) | passes; asserts failure against BOTH v2 and v1 | 15 tests pass |

### Files changed (vs `main`, staged but NOT committed — see STOP below)

- `schemas/v1/oxdoc-pptx-slides.schema.json` (+129): new v1 contract.
- `docs/schemas/v1/oxdoc-pptx-slides.schema.json` (+129): byte-identical mirror.
- `crates/oxdoc-core/tests/schema.rs` (+157/−6): registration, oneOf-aware metadata, `validate_against`/`validate_slides_object`, 3 new tests.
- `tests/fixtures/snapshots/cli_pptx_slides_json.json` (+21), `cli_pptx_slides_jsonl.jsonl` (+2): snapshots from real core output.
- `openspec/.../tasks.md`, `apply-progress.md`: artifact updates.

## ⚠ STOP — WU3 review budget exceeded (ask-on-risk)

Measured at WU3 exit (task 40 measurement, changes staged):

```
git diff main --shortstat -- . ':(exclude).gitignore'
6 files changed, 440 insertions(+), 14 deletions(-)          (incl. openspec artifacts)
git diff main --shortstat -- . ':(exclude).gitignore' ':(exclude)openspec'
5 files changed, 432 insertions(+), 6 deletions(-)           = 438 changed lines
```

438 > 400 review-budget lines (WU3 estimate was ~330–380). Breakdown: schema 129 + byte-identical
mirror 129 (spec-mandated `docs-schemas-check` lockstep, review-trivial copy) + harness/tests 163 net
+ snapshots 23. The pre-agreed fallback ("move the two snapshot files into WU4a") is NOT viable as
written: it only removes ~23 lines (→ ~415, still over) and tasks 36-38 read those snapshot files
(`read_snapshot` panics), so the snapshot files cannot leave WU3 while its validation tests stay.
Per `ask-on-risk`, apply is PAUSED before committing; the WU3 changes are staged (not committed),
nothing pushed, no PRs.

Decision needed from the maintainer (any one):

1. `size:exception` — accept the 438-line PR (the byte-identical schema mirror is 129 of the lines
1. `size:exception` — accept the 438-line PR (the byte-identical schema mirror is 129 of the lines
   and cannot be split from the schema; schema + tests + snapshots are one cohesive contract unit).
2. Re-slice WU3 into two stacked PRs (chain grows to 8 PRs): PR 5a = schema + mirror + registration/
   metadata harness (~300 lines); PR 5b = validation tests + snapshots + negative tests (~140 lines).
   Both are individually green (`cargo test` passes at each point).
3. No other honest cut exists: the schema files, mirror, harness, and snapshots are spec-mandated
   deliverables of R11; no comments/docs/tests were compressed to approach the number.

### Remaining tasks (unchecked after WU3 pause)

- [ ] 40. WU3 exit — measurement DONE (438 > 400 → paused); commit/PR awaits the budget decision above.
- WU4a tasks 41-51, WU4b tasks 52-57, final verification tasks 58-60 (all unchecked, later units).

### Structured status (WU3)

- Consumed native `gentle-ai.sdd-status` v2 before work: change `pptx-slide-scoped-json-jsonl`, `applyState: ready`, `nextRecommended: apply`, `taskProgress 31/60`, `blockedReasons: []`, `actionContext.mode: repo-local`, `allowedEditRoots: [repo root]` — no blockers, no warnings.
- After WU3: tasks 32-39 marked `[x]`; `taskProgress` moves to 39/60; apply remains in progress with the budget decision pending.
- Verification-gate extras run this session (parent-prompt gate): `cargo llvm-cov` line coverage 96.37% (>= 95 gate, exit 0); compatibility-corpus script passed; `git diff main` measured and reported above.

---

## WU4a — `extract slides` CLI subcommand and CLI tests (this session)

- Store: openspec
- Branch: `issue-180-pptx-slides-wu4a` (PR position 6 of the stacked-to-main chain after the WU2/WU3 splits; parent session owns PR opening)
- Base at apply start: `5e4d98b` (`main` after WU1 #217/#218, WU2 #219/#220, WU2c #221, WU3 #222/#223)
- Scope: exactly tasks 41-51 (CLI subcommand + CLI tests). Strict TDD (`cargo test`) active. WU4b docs (52-57) left unchecked. Task 40 (WU3 exit / PR 3) remains parent-owned; WU3 landed as #222/#223.

### Completed tasks (tasks.md updated to `[x]`)

| Task | Result |
| --- | --- |
| 41 | RED (R9 JSON payload shape): `extracts_pptx_slides_as_json_snapshot` in `crates/oxdoc-cli/tests/cli.rs` — `oxdoc extract slides <slides-deck.pptx>` with NO `--format` flag; asserts success, empty stderr, stdout byte-equal to `cli_pptx_slides_json.json`, top-level keys (order locked by the byte compare; Value keys asserted as a set because serde_json `Value` maps sort alphabetically), exactly two `slides` records, `notes` on exactly one record. RED: `assertion failed: output.status.success()` (clap "unrecognized subcommand 'slides'", exit 2). |
| 42 | GREEN: `ExtractCommand::Slides { file, format }` with `#[arg(long, value_enum, default_value_t = SlidesFormat::Json)]`, `enum SlidesFormat { Json, Jsonl }`, dispatch arm `extract_slides_command(&file, format, warning_format)?`, `SlidesPayload { schema_version: u8, file: &str, document_type: &'static str, slides: &[PptxSlideText], warnings: Vec<OwnedWarningPayload> }`, pptx-only `document_type_for_input` gate (Docx/Unknown -> "cannot extract slides from a DOCX document", Xlsx -> "...XLSX workbook"), and `Input::extract_pptx_slides` (modeled on `extract_pptx_structured_text`, always `cli_limits().ooxml`). JSON branch: `emit_warnings` first, then `to_writer_pretty` + `println!`. File label = `display_file_name` (stdin -> `<stdin>` per the amended spec; NO `slides_file_label` helper). Byte snapshot compare passed first run after GREEN. |
| 43 | Pin (RED/GREEN not achievable: all-skipped + quiet behavior is shared task-42 emission code): `emits_slides_json_payload_when_all_skipped` — inline package with empty presentation rels -> exit 0, `"slides": []`, 2 embedded warnings with exact `skipped PPTX slide {rid}: unknown relationship id` wording and path `ppt/presentation.xml`; `--quiet` -> empty stderr with the embedded `warnings` intact. |
| 44 | RED (R10 JSONL): `extracts_pptx_slides_as_jsonl_snapshot` — `--format jsonl` on `corpus/pptx/text` -> stdout byte-equal to `cli_pptx_slides_jsonl.jsonl`, 2 compact records, `notes` on record 1 only, record values match the JSON `slides` array plus the `schema_version`/`file` envelope. RED: command failed (jsonl arm was a deliberate transient `InvalidArgument` placeholder so the RED was genuine). |
| 45 | GREEN: `SlidesJsonlRecord<'a> { schema_version: u8, file: &'a str, #[serde(flatten)] slide: &'a PptxSlideText }`; per-slide `serde_json::to_writer` + `\n`, `writer.flush()?`, then `emit_warnings` AFTER the flush (rows-jsonl precedent: stdout stays a pure record stream). Both snapshot tests green. |
| 46 | Pin (shared emission code): `keeps_slides_jsonl_stdout_clean_under_warnings` on `missing-target` — every stdout line parses as JSON, no warning text on stdout, 3 `warning[...]` lines on stderr, ordinals `1,4` gap asserted; `--warnings json` -> 3 stderr JSON warning payloads whose messages are exactly the three spec-locked wordings. |
| 47 | Pin (read_input/stdin handled by task 42): `extracts_pptx_slides_jsonl_from_stdin` — piped package -> same records/values as the file form; `file` field asserted as `<stdin>` explicitly (amended spec locked). |
| 48 | Pin (pptx-only gate was part of the task-42 command implementation per design §4.2 step 2): `rejects_slides_from_non_pptx_packages` — DOCX -> exit 1, empty stdout, `cannot extract slides from a DOCX document`; XLSX -> exit 1, `cannot extract slides from an XLSX workbook` (`CliError::InvalidArgument`, E010). |
| 49 | Pin (hard errors already propagate via `CliError::Core`): `reports_slides_hard_errors_without_partial_output` — external slide target package and a package without `ppt/presentation.xml` both exit 1 through the `error[...]` handler with empty stdout (no partial payload/records). |
| 50 | WU4a gate: `cargo test -p oxdoc-cli` -> bin 25 + cli 106 passed; `cargo test --workspace` -> all 8 targets ok (cli bin 25, cli 106, core lib 112, api 99, schema 15, tabular 9 + 2 + 0), frozen snapshots untouched; `cargo fmt --all -- --check` OK; `cargo clippy --workspace --all-targets -- -D warnings` OK; `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` -> **lines 96.39% TOTAL, exit 0**; `python scripts/check-compatibility-corpus.py` -> "compatibility corpus validation passed (3 fixtures)". No snapshot churn; no change under `tests/fixtures/files/` or `compatibility-matrix.json`. |

### TDD Cycle Evidence (strict TDD)

| Cycle | RED | GREEN | Evidence |
| --- | --- | --- | --- |
| JSON payload (41-42) | clap "unrecognized subcommand 'slides'" -> `assertion failed: output.status.success()` | subcommand + gate + payload + `Input::extract_pptx_slides` | byte snapshot compare passes; 106 cli tests |
| JSONL stream (44-45) | command failed with the transient `--format jsonl is not implemented yet` placeholder arm | `SlidesJsonlRecord` + flush-then-warn stream | byte snapshot compare passes |
| Pins (43, 46-49) | No RED achievable: the exercised behaviors (shared emission path, quiet suppression, stdin buffering, type gate, hard-error propagation) were landed by tasks 42/45 as the minimal correct command implementation (design §4.2); per the task-23/task-25-29 precedent they are recorded as regression pins | pass on first run | 5 tests in the 106 |

### Files changed (vs `main`)

- `crates/oxdoc-cli/src/main.rs` (+118): `ExtractCommand::Slides`, `SlidesFormat`, `extract_slides_command`, `SlidesPayload`, `SlidesJsonlRecord`, `Input::extract_pptx_slides`.
- `crates/oxdoc-cli/tests/cli.rs` (+262): 7 new WU4a tests.

### Test commands run

- `cargo test -p oxdoc-cli --test cli extracts_pptx_slides_as_json_snapshot` -> RED (clap exit 2), then 1 passed
- `cargo test -p oxdoc-cli --test cli extracts_pptx_slides_as_jsonl_snapshot` -> RED (transient jsonl placeholder), then 1 passed
- `cargo test -p oxdoc-cli --test cli slides` -> 7 passed
- `cargo test -p oxdoc-cli` -> bin 25 passed, cli 106 passed
- `cargo test --workspace` -> all 8 targets ok (frozen snapshots untouched)
- `cargo fmt --all -- --check` -> OK (one mechanical fmt pass on new tests)
- `cargo clippy --workspace --all-targets -- -D warnings` -> OK
- `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only` -> lines 96.39% TOTAL, exit 0
- `python scripts/check-compatibility-corpus.py` -> passed (3 fixtures)

### WU4a gate + exit measurement (task 51)

- `git diff main --shortstat -- . ':(exclude).gitignore'` -> `2 files changed, 380 insertions(+)` = **380 changed lines <= 400 budget** (matches the ~300-380 estimate). No pause needed; no `size:exception`.
- The `.gitignore` modification and untracked `.gga`/`.pi/` present in the working tree are pre-existing local Pi runtime state, NOT part of this unit and excluded from the commit.
- No stop-and-investigate event: no snapshot churn, no fixture/matrix changes, no newly-failing existing test.

### Deviations from design

- Design §4.2's `-` stdin label callout is superseded (as recorded in tasks.md reconciliation notes): the `<stdin>` label from `display_file_name` is used, per the amended spec; no `slides_file_label` helper exists.
- No other deviations: payload/record struct shapes, stderr warning ordering (JSON: warnings before stdout; JSONL: after flush), gate messages, and exit codes follow design §4.2 exactly.

### Commits

1. (this commit) `feat(cli): add extract slides subcommand for slide-scoped JSON and JSONL` — subcommand + CLI tests + openspec artifact updates. Single cohesive work-unit commit (task 51's "open PR 4" clause is parent-owned; session forbids push/PR creation).

### Remaining tasks (unchecked after WU4a)

- [ ] 40. WU3 exit (parent-owned; WU3 landed as PRs #222/#223)
- WU4b tasks 52-57 (docs only), final verification tasks 58-60

### Structured status (WU4a)

- Consumed native `gentle-ai.sdd-status` v2 before work: change `pptx-slide-scoped-json-jsonl`, `applyState: ready`, `nextRecommended: apply`, `taskProgress 39/60`, `blockedReasons: []`, `actionContext.mode: repo-local`, `allowedEditRoots: [repo root]` — no blockers, no warnings.
- After WU4a: tasks 41-51 marked `[x]`; `taskProgress` moves to 50/60. Remaining: task 40 (parent-owned), WU4b (52-57), final verification (58-60).
