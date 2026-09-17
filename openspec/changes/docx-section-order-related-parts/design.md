# Design — docx: order headers and footers by section (issue #177)

- Change id: `docx-section-order-related-parts`
- Status: designed 2026-06 (SDD design phase, artifact store: openspec)
- Inputs: `proposal.md`, `specs/docx-extraction/spec.md`, `exploration.md`, current code
  on `main` (post PR #199), `openspec/config.yaml`.
- Execution: auto · artifact store openspec · review budget 400 lines · ask-on-risk ·
  strict TDD (`cargo test`, fmt, clippy, 95% line-coverage gate).

## 1. Goal and constraints (recap)

Headers and footers are ordered by the sections that reference them in
`word/document.xml` (`sectPr` document order, then headers-before-footers, then
`first`/`even`/`default`), instead of `word/_rels/document.xml.rels` order.
Default-on, no new options, no public schema change. Existing warning texts,
codes, partial extraction, `SuspiciousRelationshipTarget`, and the
`include_related_parts = false` fast path stay byte-identical. Two new warnings
only (missing `r:id`, unknown `r:id`).

Verified code facts this design builds on:

- All three extraction paths (`extract_text`, `extract_structured_text`,
  `extract_tables` in `crates/oxdoc-core/src/parsers/docx.rs`) share the same
  loop shape: main part first → early return when `!include_related_parts` →
  read rels (silently main-only when missing) → iterate
  `parse_relationships(...)` in rels file order.
- `parse_relationships` (mod.rs) preserves rels document order and already
  accepts relationships without an `Id` (`id: None`).
- `parse_relationship_map` (mod.rs) already exists: `HashMap<String, Relationship>`
  keyed by relationship id — reusable for `r:id` lookup with **no mod.rs change**.
- `resolve_relationship_target(parent_dir(&document_path), ...)` produces the
  normalized package path and raises `SuspiciousRelationshipTarget` for
  external/escape/NUL targets (unchanged hard error).
- Warning construction style: `OutputWarning::new(&relationships_path, message)`
  → `WarningCode::Custom` (W999), exactly like the existing
  `skipped related DOCX text part {path}: missing part` warnings.
- Repo-wide `grep sectPr|headerReference|footerReference|titlePg`: zero matches;
  no current fixture contains a `sectPr`.

## 2. Data structures and parsing (design question 1)

### 2.1 New types (all private to `docx.rs`)

```rust
/// Order rank: headers before footers within a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RelatedRefKind { Header, Footer }

/// Canonical variant rank: first, even, default (unknown/missing -> Default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RelatedRefVariant { First, Even, Default }

/// One `w:headerReference` / `w:footerReference` element, in sectPr element order.
#[derive(Debug)]
struct SectionReference {
    kind: RelatedRefKind,
    variant: RelatedRefVariant,
    rid: Option<String>, // None => element had no r:id (warning + skip at plan time)
}

/// The iteration order shared by all three extraction paths.
/// Indices into the `parse_relationships` result vector.
type RelatedPartPlan = Vec<usize>;
```

Deriving `Ord` on the two enums gives the canonical ranking for free
(`Header < Footer`, `First < Even < Default`); per-section sorting is a stable
`sort_by_key` over `(kind, variant)` so ties keep sectPr element order.

### 2.2 sectPr collector (streaming, graceful on malformed XML)

```rust
fn collect_section_references<R: BufRead>(source: R) -> Vec<Vec<SectionReference>>
```

- quick_xml event loop in the established style of `extract_xml_text`
  (`trim_text(false)`, `read_event_into`, `buf.clear()`), but **not** an
  `Extraction`: it never fails. On a parse error it stops and returns the
  sections collected so far (deterministic prefix). Rationale: today a
  malformed `document.xml` yields W001 + partial main text and extraction still
  succeeds with related parts; planning must not turn that into a hard error.
  The main-text pass emits the W001 warning exactly as today; the collector
  degrades silently.
- Depth tracking (two `usize` counters and one bool):
  - `paragraph_depth` — incremented on `w:p` Start, decremented on `w:p` End.
  - `in_sectpr: bool` — true between `sectPr` Start and End; references are
    only collected while true.
- Section discovery:
  - `sectPr` Start while `paragraph_depth > 0` → mid-body section close
    (`w:p/w:pPr/w:sectPr`); a section is pushed when its `sectPr` closes.
  - `sectPr` Start while `paragraph_depth == 0` → body-level final section
    (last child of `w:body`).
  - `sectPr` as an Empty element (`<w:sectPr/>`) → section with no references.
  - Document order is preserved by push order; sections appear in walk order.
- Reference collection (Start or Empty of `headerReference`/`footerReference`
  while `in_sectpr`):
  - `kind` from the element local name.
  - `variant`: `attr_value(element, "type")` matched against `first`/`even`
    (case-sensitive, per OOXML); anything else — including missing or
    unrecognized values such as `title` — is `Default`, silently (spec: no
    warning for `w:type`).
  - `rid`: `attr_value(element, "r:id")` — attribute local name `id`; safe
    because `headerReference`/`footerReference` have only `w:type` and `r:id`
    attributes, so the local-name match cannot hit a foreign `w:id`.
- `w:titlePg` and any other `sectPr` children are parsed past and ignored (the
  titlePg no-op is enforced by not modeling it at all).

Documented edge cases (behavior, not code branches):

| Situation | Behavior |
| --- | --- |
| `sectPr` inside a `w:pPr` of a paragraph inside a table cell | Treated as a mid-body section close at that position (Word does not produce this; harmless, deterministic). |
| Two `sectPr` at body level, or section-break XML oddities | Each sectPr contributes one section in document order; no dedup across sections at collection time. |
| Duplicate `Id` values in the rels file | `parse_relationship_map` last-wins semantics; deterministic and unchanged from existing rels handling. |
| `r:id` resolves to a relationship whose type is not header/footer (e.g. an image) | Reference skipped silently, no warning, nothing emitted (a non-header/footer part must not be emitted under a header `part_type`; documented in `docs/formats/docx.md`). |
| `headerReference`/`footerReference` outside `sectPr` | Ignored (collector only reads inside `sectPr`). |

### 2.3 Shared ordering plan (design questions 1 + 2)

```rust
fn plan_related_part_order<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    document_path: &str,
    relationships: &[Relationship],
    relationships_path: &str,
    options: DocxTextOptions,
    warnings: &mut Vec<OutputWarning>,
) -> Result<Vec<usize>>
```

Lives in `docx.rs` (private). **Decision: docx.rs, not mod.rs.** All three
consumers are in `docx.rs`; the OOXML sectPr knowledge is DOCX-parser detail;
`mod.rs` needs zero changes because `parse_relationship_map` already exists.
Keeps the diff minimal and the helper untestable-from-outside pressure low
(unit tests live in `docx.rs`'s existing `#[cfg(test)]` module).

Algorithm:

1. `parse_relationships` is called **once by the caller** (preserving today's
   error propagation for malformed rels XML); the plan takes the parsed slice.
2. Read `document.xml` a second time via `package.with_entry` → BufReader →
   `collect_section_references`. (The first read is the text/tables pass,
   which returns extracted text, not XML. One extra sequential read of the
   main part is the accepted cost; no caching, no signature changes.)
3. Build `HashMap<String, usize>` rid → relationship index from
   `relationships` (duplicates: last wins, matching `parse_relationship_map`).
4. Walk sections in document order; per section, stable-sort references by
   `(kind, variant)`, then for each reference in order:
   - `rid: None` → push warning `skipped DOCX headerReference: missing r:id`
     (or `footerReference`), continue.
   - rid not in the map → push warning
     `skipped DOCX headerReference {rid}: unknown relationship id`, continue.
   - rid present but the relationship's type does not classify as
     header/footer → skip silently.
   - otherwise `resolve_relationship_target(parent_dir(&document_path), rel,
     relationships_path)?` — `SuspiciousRelationshipTarget` propagates as the
     unchanged hard error — then dedup by resolved package path:
     emitted once, at first reference; mark the relationship index referenced.
5. Orphans: iterate `relationships` in rels file order; for each relationship
   that classifies as header/footer and was never named by any valid rid, and
   whose resolved path was not already emitted (rid-alias dedup), append its
   index.
6. Then append, still in rels file order, every relationship whose
   `related_docx_text_part_type(...)` is `Some` for footnotes/endnotes
   (comments only when `options.include_comments`). Their mutual order is
   untouched; they now strictly follow all headers/footers (rule 5).
7. Return the index plan; push all planning warnings onto `warnings` in
   reference-walk order. Extraction-loop warnings (missing/malformed part)
   follow in plan order, as today.

The plan is options-aware only for comments filtering; `DocxTextOptions`
gains no field (Non-goal 5).

## 3. Integration with the three extraction paths (design question 2)

Each path replaces exactly one line and adds two:

```rust
// before (all three paths, identical shape):
for relationship in parse_relationships(&relationships_xml, &relationships_path)? {

// after:
let relationships = parse_relationships(&relationships_xml, &relationships_path)?;
let plan = plan_related_part_order(
    package, &document_path, &relationships, &relationships_path, options, &mut warnings,
)?;
for index in plan {
    let relationship = &relationships[index];
```

The per-part body of every loop (resolve, extract, `append_related_text` /
`push_text_block` / `public_tables_for_part`, `MissingPart` warning with the
existing text/table wording, `Err(err) => return Err(err)`) is **unchanged** —
only the iteration source changes. This is the lockstep guarantee: the order
is computed in exactly one place and all three paths consume it as indices.

Fast path: the plan is computed strictly after the `!include_related_parts`
early return and after the rels `MissingPart` early return, so:

- `include_related_parts = false` never reads rels nor document.xml a second
  time — untouched fast path (AC 4).
- Missing `document.xml.rels` still yields main-part output only, no warning.

`extract_text` keeps using `is_related_docx_text_part` for skip classification;
structured/tables keep `related_docx_text_part_type` for labels. Every plan
entry classifies to `Some(_)` under the caller's options, so behavior inside
the loop is unchanged.

## 4. Warning integration (design question 3)

Exact constructions, both with `path = word/_rels/document.xml.rels`
(`relationships_path`), matching the existing skipped-part warning style:

```rust
// missing r:id (kind from the element local name)
OutputWarning::new(
    relationships_path,
    format!("skipped DOCX {element}Reference: missing r:id"),
)

// unknown r:id (verbatim rid text, e.g. rIdGhost)
OutputWarning::new(
    relationships_path,
    format!("skipped DOCX {element}Reference {rid}: unknown relationship id"),
)
```

- Both are `WarningCode::Custom` (W999) via `OutputWarning::new` — same as the
  existing `skipped related DOCX ...` warnings; no `WarningCode`/category
  changes (keeps the schema test green: new *messages* under Custom, no new
  codes).
- Warning order in output: main-part warnings first (unchanged), then the two
  new reference warnings in reference-walk order, then per-part warnings in
  plan order.
- Stability: no existing fixture or inline test package contains a `sectPr`,
  so no existing warning assertion can observe the new warnings; existing
  message texts are not touched by this change (only the iteration source in
  the loop changes). The missing-part / malformed-part / external-target /
  missing-rels behaviors are covered by the unchanged tests listed in §6.

## 5. Fixture plan (design question 4)

### 5.1 Existing related-parts fixture (`tests/fixtures/docx/related-parts/`)

Make it prove section ordering rather than coincidentally matching it:

- `package/word/document.xml`: add a mid-body `w:p/w:pPr/w:sectPr` referencing
  `rIdFooter` (footer default) and a body-level `w:sectPr` referencing
  `rIdHeader` (header default). No visible text is added (`sectPr` has no
  `w:t`), so main-part text output is unchanged.
- New emitted order (rels order for header/footer is header-then-footer, so
  this is a genuine order reversal):
  1. `main` → `word/document.xml` ("Main table")
  2. `footer` → `word/footer1.xml` ("Footer table")
  3. `header` → `word/header1.xml` ("Header table")
  4. `comments` → `word/comments.xml` ("Comment table")
  5. `footnotes` → `word/footnotes.xml` ("Footnote table")
  6. `endnotes` → `word/endnotes.xml` ("Endnote table")
- `expected.json` updated to that order (same schema: `parts` with
  `part_type`/`part_path`/`table_ordinal`/`text`, `warnings: []`).
- Provenance `tests/fixtures/provenance/docx-related-parts-tables.md` updated:
  purpose line now says section-order traversal; the "relationship-order
  traversal" wording is removed.

### 5.2 New hand-authored fixture: `tests/fixtures/docx/section-order/`

Full package tree (no binary committed; `build_package` ZIPs it
deterministically), deliberately out-of-order rels:

- `word/document.xml` — two sections:
  - Section 1: mid-body `w:p/w:pPr/w:sectPr` with, in element order:
    `headerReference w:type="default" r:id="rIdHeaderDefault"`,
    `headerReference w:type="first" r:id="rIdHeaderFirst"`,
    `headerReference w:type="even" r:id="rIdHeaderEven"`,
    `footerReference w:type="default" r:id="rIdFooterOne"`,
    `headerReference r:id="rIdGhost"` (unknown rid),
    `footerReference` with **no** `r:id` (missing rid), and `w:titlePg` set.
  - Section 2: body-level final `w:sectPr` with
    `headerReference r:id="rIdHeaderDefault"` (shared part → dedup),
    `headerReference w:type="title" r:id="rIdHeaderTitled"` (unrecognized type
    → default, silent), `footerReference w:type="default" r:id="rIdFooterTwo"`.
- `word/_rels/document.xml.rels` — deliberately scrambled order:
  `rIdFooterTwo, rIdHeaderFirst, rIdComments, rIdHeaderEven, rIdHeaderDefault,
  rIdHeaderOrphan, rIdFootnotes, rIdHeaderTitled, rIdFooterOne` (plus nothing
  referencing the orphan from document.xml).
- Parts: `header-first.xml`, `header-even.xml`, `header-default.xml` (shared),
  `header-titled.xml`, `header-orphan.xml` (no sectPr reference), `footer1.xml`,
  `footer2.xml` (contains one small table), `footnotes.xml`, `comments.xml`.

Expected oracle (part order after the main body):

| # | part_type | part_path | note |
| --- | --- | --- | --- |
| 2 | header | `word/header-first.xml` | `first` before `even`/`default` |
| 3 | header | `word/header-even.xml` | even variant, no evenAndOddHeaders consult |
| 4 | header | `word/header-default.xml` | emitted here (first referencing section) |
| 5 | footer | `word/footer1.xml` | headers before footers within section 1 |
| 6 | header | `word/header-titled.xml` | `w:type="title"` → default variant slot, section 2 |
| 7 | footer | `word/footer2.xml` | body-level final section |
| 8 | header | `word/header-orphan.xml` | orphan, rels order, after referenced parts |
| 9 | comments | `word/comments.xml` | rels order among notes; after all h/f |
| 10 | footnotes | `word/footnotes.xml` | rels order among notes |

This pins in one oracle: variant ordering (first/even/default regardless of
attribute order), dedup-by-path at first reference, ties, orphans-last,
rule 5, titlePg no-op, and (via warnings below) both new warnings.

Warnings in `expected.json`, in order:

1. `{path: "word/_rels/document.xml.rels", message: "skipped DOCX headerReference rIdGhost: unknown relationship id"}`
2. `{path: "word/_rels/document.xml.rels", message: "skipped DOCX footerReference: missing r:id"}`

`expected.json` schema mirrors the related-parts oracle (`parts` + `warnings`);
parts without tables omit the table fields, parts with tables carry
`table_ordinal`. Consumed by api.rs tests for **all three extractions** from
the same JSON (flat text = join of part texts; structured = part_path sequence;
tables = part_path sequence with table contents).

Provenance: new `tests/fixtures/provenance/docx-section-order.md`
(hand-authored, no producer, synthetic text, purpose = section-order oracle).

### 5.3 Application-generated fixture (decision: do not extend in this change)

- The existing `tests/fixtures/files/docx/python-docx-basic.docx` (python-docx
  1.2.0, provenance `docx-python-docx-basic.md`) is single-section with
  default header/footer and **no sectPr** → under the new rule its
  headers/footers are orphans in rels order = today's order; its snapshot
  `docx_python_docx_text.txt` holds **byte-identical** (spec scenario:
  single-section snapshot stability).
- Rationale for not extending the generator: python-docx cannot express
  even-variant headers and multi-section authoring is fragile; the generated
  fixture's job here is the "no change for ordered single-section rels"
  scenario, which the current fixture already satisfies. Extending it would
  add ~40 lines of generator/provenance/snapshot churn against the budget
  with no new rule coverage. The application-generated *with provenance*
  acceptance criterion (AC 3) is already met by the existing record;
  re-verification of its SHA-256 in the WU2 check step documents that it was
  not regenerated. If the orchestrator wants a generated multi-section
  fixture, that is a follow-up (issue #179 territory or a new change).

## 6. Test plan (design question 5) — strict TDD

### 6.1 Unit tests (`docx.rs` `#[cfg(test)]` — direct collector/plan coverage)

| Test | Scenario pinned |
| --- | --- |
| `collects_sections_in_document_order_with_variant_ranking` | Mid-body sectPr + body-level sectPr; per-section output sorted `first, even, default`, headers before footers, ties keep element order; section order preserved. |
| `treats_missing_and_unrecognized_type_as_default` | `w:type="title"`, absent `w:type` → `Default`, no diagnostics. |
| `keeps_reference_without_rid_as_none` | `footerReference` with no `r:id` → `rid: None`. |
| `stops_collection_gracefully_on_malformed_xml` | Malformed XML after the first sectPr → sections collected so far, no panic, no error. |
| `plans_orphans_and_notes_in_relationship_order` | Plan builder: orphan headers/footers appended in rels order; footnotes/endnotes/comments appended after, mutual rels order; comments filtered when `include_comments = false`. |
| `plan_skips_missing_and_unknown_reference_ids_with_warnings` | Exact warning texts (both kinds × both cases) + plan skips + extraction-would-continue semantics. |
| `plan_dedups_by_resolved_path_at_first_reference` | Shared part across sections + duplicate rid within one section + two rids aliasing one path. |
| `plan_propagates_suspicious_target` | External target referenced from sectPr → `SuspiciousRelationshipTarget` hard error unchanged. |

### 6.2 Integration tests (`crates/oxdoc-core/tests/api.rs`)

| Test | Spec scenario |
| --- | --- |
| `extracts_docx_text_from_related_parts_in_relationship_order` → rename to `keeps_unreferenced_docx_related_parts_in_relationship_order` (no sectPr package; assertions unchanged) | Orphan/`no-sectPr` degenerate case; proves rels-order fallback without the old contract name. |
| `orders_docx_text_related_parts_by_section` | Multi-section + out-of-order rels (flat text, fixture oracle). |
| `orders_docx_structured_blocks_by_section` | Same fixture, asserts the block `part_path` sequence (three-paths-agree + no variant labels). |
| `orders_docx_tables_by_section` | Same fixture, asserts `DocxTable.part_path` sequence incl. the `footer2.xml` table and unchanged `table_ordinal` semantics. |
| `warns_on_missing_and_unknown_docx_reference_ids` | Dangling rid + missing rid: extraction succeeds, exact texts, extraction continues. |
| `preserves_docx_fast_path_with_sectpr_documents` | `include_related_parts = false` on the section-order fixture → body-only, no rels read warnings. |
| `keeps_partial_docx_text_when_document_xml_malformed_with_sectpr` | Malformed `document.xml` with sectPr → W001 + partial text + related parts still planned from the collected prefix (graceful-degradation pin). |
| Existing missing-part / malformed-part / external-target / every-extraction tests | Unchanged (byte-identical behavior pins for AC 4). |

### 6.3 Snapshot / schema

- `crates/oxdoc-core/tests/schema.rs`: **no changes** — no field, `part_type`
  value, or option changes; the two new warnings are `Custom` messages and the
  warning schema is message-opaque.
- CLI snapshots: unchanged in WU1. WU2 re-verifies
  `tests/fixtures/snapshots/docx_python_docx_text.txt` stays identical (§5.3).
  No new CLI snapshots (API-level coverage keeps budget); if the reviewer
  wants one, it is a ~2-line fixture-list addition to `oxdoc-cli` snapshot
  tests — deferred rather than absorbed silently.

### 6.4 TDD sequencing

- **RED 1 (WU1)**: rename the two `..._in_relationship_order` tests, update the
  related-parts `expected.json`, add the §6.1/§6.2 new tests → `cargo test`
  fails.
- **GREEN 1**: implement `RelatedRefKind`/`RelatedRefVariant`/`SectionReference`,
  `collect_section_references`, `plan_related_part_order`, wire the three
  paths → `cargo test --workspace` green.
- **REFACTOR 1**: if the three wiring sites duplicate more than the shared
  plan-call snippet, extract a tiny local helper inside `docx.rs`.
- **TRIANGULATE (WU2)**: add the `section-order` fixture + oracle tests (second
  producer-orthogonal encoding of the same rule; would catch an ordering bug
  tuned to the inline packages). Docs/CHANGELOG last.

## 7. File changes and size estimate (design questions 6)

| File | Change | Est. lines |
| --- | --- | --- |
| `crates/oxdoc-core/src/parsers/docx.rs` | types + collector + plan + 3 wirings (non-test) | ~100 |
| `crates/oxdoc-core/src/parsers/docx.rs` (tests) | §6.1 unit tests | ~110 |
| `crates/oxdoc-core/tests/api.rs` | renames (0 net) + §6.2 new tests | ~110 |
| `tests/fixtures/docx/related-parts/` | document.xml sectPr + expected.json reorder + provenance wording | ~25 |
| `tests/fixtures/docx/section-order/` | XML package + `expected.json` + provenance | ~140 |
| `docs/formats/docx.md` | two related-parts rows + determinism statement (rule, titlePg, evenAndOddHeaders deferral, rid-to-non-h/f note, notes repositioning) + remove Planned-Improvements bullet | ~35 |
| `CHANGELOG.md` | default-on ordering change entry | ~10 |
| Snapshots / schema / mod.rs / models.rs | none | 0 |
| **Total** | | **~530** |

**Honest verdict: this exceeds the 400-line budget by roughly 90–130 lines if
implemented as specified.** The proposal's 300–390 estimate did not count the
new fixture's full XML tree + JSON oracle at realistic size. Per the
`ask-on-risk` delivery strategy this is a budget risk to surface, not to
absorb: WU1 (parser + tests + related-parts oracle ≈ 345 lines) fits
comfortably under 400 on its own; the pause point is **after GREEN 1, before
WU2**, where the orchestrator/user decides between:

1. Two-PR chain of the same change (WU1 PR, then WU2 fixture+docs PR) — keeps
   each review ≤ 400; needs explicit delivery-strategy selection (deferred in
   preflight, so it must be chosen then, not now);
2. Trimming scope inside WU2 (smallest lever: drop `endnotes` from the
   section-order fixture — rule 5 is still pinned by comments-before-
   footnotes — saving ~35 lines; plus compressing the fixture XML), targeting
   ≤ 450–470 total, still over;
3. `size:exception` — requires explicit user acceptance, never inferred.

No silent chain and no inferred exception; ask at the pause point.

## 8. Risks to existing fixtures and oracle truthfulness (design question 7)

- **related-parts `expected.json` is a hand-authored design oracle, not
  parser output** — updating it is legitimate but must stay truthful:
  - The oracle is updated in the **same commit** as the parser change (WU1),
    never generated from parser output to "make tests pass".
  - The new order in §5.1 is derived from the proposal's ordering rule by
    hand (section 1 → footer, body-level → header, notes in rels order), and
    the fixture gains sectPr so the oracle actually exercises the rule.
  - The provenance record is updated in the same commit to describe the new
    traversal so provenance never contradicts the oracle.
- **Inline `create_ooxml` tests** (missing part, malformed part, external
  target, unrelated parts, fast path): all their packages have no `sectPr`,
  so the new rule degenerates to orphan rels order + notes rels order, which
  reproduces today's outputs exactly — verified case-by-case in §6.2; they
  should pass unmodified (except the two renames). If any unexpectedly
  differs, that is a parser bug, not an oracle problem.
- **python-docx snapshot**: expected byte-identical (§5.3); verification is a
  test run, not a hand-wave — if it drifts, stop and investigate before
  touching the snapshot.
- **Three-path drift**: eliminated structurally (one plan helper); the
  fixture oracle is consumed by all three tests so any drift fails CI.
- **Coverage gate**: every new non-test line lands in WU1 with its unit tests;
  the graceful-degradation branch of the collector is explicitly tested
  (`stops_collection_gracefully_on_malformed_xml` + integration pin), so the
  95% gate is not at risk.
- **Memory/perf**: one extra streaming read of `document.xml` per extraction
  call; no full-file buffering (collector is BufRead-based like
  `extract_part_text`).

## 9. Rollout and verification

- Work unit 1 (same change): parser + tests + related-parts oracle. Gate:
  `cargo test --workspace`, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo llvm-cov --workspace --all-features --all-targets --fail-under-lines 95 --summary-only`.
- Pause (ask-on-risk): report measured WU1 size; get delivery decision for WU2.
- Work unit 2 (same change): section-order fixture + docs + CHANGELOG. Same
  gates; snapshot stability check for the python-docx CLI snapshot.
- Rollback: revert the parser plan wiring + collector (single-crate), restore
  oracle; no schema/API/migration surface.

## 10. Decision log

| Decision | Choice | Rationale |
| --- | --- | --- |
| Helper placement | `docx.rs`, private | All consumers in-file; mod.rs stays untouched (`parse_relationship_map` already suffices for rid lookup). |
| Plan representation | `Vec<usize>` indices into parsed rels | No clones, preserves classification reuse in each loop. |
| sectPr parsing | Dedicated streaming collector, infallible with graceful truncation | Preserves today's malformed-`document.xml` partial-extraction behavior. |
| Malformed rels XML | Still a hard error from `parse_relationships` at the caller (unchanged) | Existing behavior; plan never re-parses rels. |
| rid → non-header/footer relationship | Silent skip, documented | No honest existing warning fits; emitting image content under `part_type: "header"` would be wrong. |
| python-docx generator | Not extended this change | Snapshot-stability scenario is already met; budget protection; follow-up possible via #179. |
| New fixture oracle | Shared JSON consumed by all three extraction tests | Enforces the one-oracle-across-paths requirement (spec). |
