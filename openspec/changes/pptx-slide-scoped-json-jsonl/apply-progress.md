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
