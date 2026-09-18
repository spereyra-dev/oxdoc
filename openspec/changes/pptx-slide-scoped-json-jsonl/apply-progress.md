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
