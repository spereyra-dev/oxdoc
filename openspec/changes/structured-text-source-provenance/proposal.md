# Proposal — structured text source provenance: versioned schema + header/footer variants (issue #179)

- Change id: `structured-text-source-provenance`
- Status: proposed (SDD propose phase, artifact store: openspec)
- Inputs: `exploration.md` (this change), GitHub issue #179 acceptance criteria,
  parent-resolved product decisions (authoritative, traceability table below),
  `openspec/config.yaml`, `openspec/specs/docx-extraction/spec.md` (canonical spec
  amended by this change), `docs/json-output.md` (JSON versioning policy).
- Delivery: ask-on-risk · review budget 400 changed lines · strict TDD (`cargo test`).

## Intent / problem

`oxdoc extract text --format structured-json` already returns per-source text
blocks (`part_type`, `part_path`, `ordinal`, `text`), but the contract around that
output is under-specified and, for DOCX headers/footers, lossy:

1. **No version marker in the payload.** Tables, rows-jsonl, audit-jsonl,
   diagnostics, and the XLSX schema report all emit `schema_version`, and
   `docs/json-output.md` states that new output fields arrive through a new schema
   version ("instead of silently widening the current contract"). The
   structured-text payload is the one machine contract with no version field,
   so a consumer cannot tell which schema it is validating against.
2. **Header/footer variants are invisible.** `docx-section-order-related-parts`
   (#177) made ordering section-aware and explicitly deferred variant labeling to
   #179. Variant knowledge (`first` / `even` / `default`) is *already computed* in
   `collect_section_references` (`RelatedRefVariant`) purely for sort ranking, and
   then discarded when `plan_related_part_order` compresses the plan to
   `Vec<usize>`. Output today has no way to say "this header block is the
   first-page header"; consumers must reverse-engineer that from `part_path`
   conventions they cannot rely on.
3. **Ordinal semantics and PPTX block meaning are documented only in fragments.**
   `ordinal` is a global 1-based output-order index (not a paragraph ordinal and not
   per-part), and `notes` blocks mean speaker notes from `ppt/notesSlides/…`. Both
   facts are load-bearing for consumers and neither is stated in the schema
   description, `docs/cli.md`, or `docs/formats/pptx.md`.
4. **No PPTX structured snapshot.** Schema validation covers only the DOCX-only
   snapshot `tests/fixtures/snapshots/cli_structured_text_json.json`, so the
   already-shipped PPTX slide/notes behavior (AC 3) is not locked by a
   schema-validated artifact.

The functional delta is small (variant labels + a version marker); the value is
that the structured-text contract becomes explicitly versioned, source-provenance
complete, and documented for both formats.

## Solution shape

### 1. Schema version v2 (first multi-version layout)

- Add `schemas/v2/oxdoc-structured-text.schema.json` with the mirror at
  `docs/schemas/v2/oxdoc-structured-text.schema.json`:
  - draft 2020-12, `$id` `https://github.com/spereyra-dev/oxdoc/schemas/v2/oxdoc-structured-text.schema.json`,
    `additionalProperties: false`;
  - top level requires `schema_version` (`const: 2`), `file`, `document_type`
    (`enum: ["docx", "pptx"]`), `blocks`;
  - block requires `part_type`, `part_path`, `ordinal`, `text`; optional `variant`
    with `enum: ["first", "even", "default"]`;
  - descriptions state the semantics: `ordinal` = 1-based **global output order**
    across the flattened block list; `notes` = speaker notes from a
    `ppt/notesSlides/notesSlideN.xml` part; `variant` = DOCX header/footer variant
    only, omitted for every other block.
- `schemas/v1/**` is **frozen and retained** (it remains the validatable contract
  for already-published outputs); `schemas/v2/` contains only the schema(s) that
  actually changed. This is the repository's first multi-version schema layout, so
  the two hardcoded v1 assumptions move into scope:
  - `Makefile` `docs-schemas-check` gains a `diff -ru schemas/v2 docs/schemas/v2`
    pass (v1 pass stays);
  - `crates/oxdoc-core/tests/schema.rs::read_json_schema` becomes version-aware and
    `schemas_have_stable_public_metadata` asserts the version segment of `$id` per
    directory instead of hardcoding `/schemas/v1/`.
- `docs/json-output.md` points the structured-json row at
  `schemas/v2/oxdoc-structured-text.schema.json`, keeps the v1 link for historical
  outputs, and adds the "Structured text JSON" semantics paragraph (version field,
  variant, global ordinal, slide/notes).

### 2. Variant plumbing (reuse of the #177 data, not a new collection pass)

- `crates/oxdoc-core/src/models.rs`: `TextBlock` gains
  `#[serde(skip_serializing_if = "Option::is_none")] pub variant: Option<String>`
  plus `TextBlock::with_variant(...)`; `TextBlock::new` keeps its current signature
  and produces `variant: None`, so existing callers (including PPTX) compile and
  serialize unchanged.
- `crates/oxdoc-core/src/parsers/docx.rs`: `plan_related_part_order` currently
  throws away the variant it just used for sorting. The plan element becomes a
  variant-carrying entry (relationship index + optional variant label derived from
  the existing `RelatedRefVariant` via a `label()` mapping, where
  missing/unrecognized `w:type` maps to `default`, exactly as #177 already orders
  it). `extract_structured_text` forwards that label into the block;
  `extract_text` and `extract_tables` keep consuming only the relationship index,
  so flat text and tables remain structurally and byte-wise unchanged.
- Labeling rules (all inherited from #177 state, no new parsing):
  - DOCX header/footer blocks referenced by a `sectPr` → `variant` present;
  - orphan header/footer parts (present in rels, referenced by no `sectPr`) →
    `variant` omitted (they have no section-assigned variant);
  - `main`, `footnotes`, `endnotes`, `comments`, and all PPTX `slide`/`notes`
    blocks → `variant` omitted;
  - `include_related_parts = false`, missing rels file, and warnings/errors
    behavior are untouched;
  - when the same resolved part path is referenced twice (dedup at first reference
    by #177), the emitted block carries the **first** reference's variant.
- `ordinal` stays exactly as it is: global 1-based output order, recomputed over
  the final block list. No renumbering, no per-part or slide-scoped counters
  (those belong to #180).

### 3. Payload version marker

- `crates/oxdoc-cli/src/main.rs`: `TextStructuredPayload` gains
  `schema_version: u8` set to `2`, mirroring `TablesPayload` / `AuditJsonlRecord` /
  `RowsJsonlRecord` conventions. Output becomes
  `{"schema_version": 2, "file": …, "document_type": …, "blocks": […]}` for the
  single-input case and the same object per element in the multi-input array.
- `StructuredText` (core) stays unversioned, exactly like `DocxTables`; the
  version marker is a CLI emission concern. Rust library callers that serialize
  `StructuredText` directly keep the v1-shaped body and read variant off the block;
  this is documented as a known, deliberate asymmetry with the CLI payload.

### 4. Tests, fixtures, snapshots

- `tests/fixtures/docx/section-order/expected.json` (the existing hand-authored
  three-path oracle) gains `variant` on section-referenced header/footer parts
  (`header-first.xml` → `first`, `header-even.xml` → `even`,
  `header-default.xml` → `default`, footers likewise) and none on the orphan
  header; `api.rs::orders_docx_structured_blocks_by_section` asserts
  `(part_type, part_path, variant)`, while the text and tables ordering tests keep
  asserting the unchanged two-tuple so #177 parity is pinned.
- A plain-text regression assertion over the same fixture (and the existing
  `docx_basic_text.txt` / `pptx_text.txt` snapshots) proves variant data never
  leaks into flat text (AC 2).
- An ordinal-continuity test over the section-order fixture asserts
  `ordinal == 1..=blocks.len()` in output order, pinning the "paragraph/block
  ordinal" reading documented in the proposal and schema.
- New PPTX structured snapshot (`tests/fixtures/snapshots/cli_structured_text_pptx_json.json`,
  built from `tests/fixtures/corpus/pptx/text/`, which already links
  `notesSlide2.xml` to slide 2), validated against the v2 schema in
  `schema.rs` and compared byte-for-byte by the CLI test; the existing
  `cli.rs::extracts_pptx_text_as_structured_json` is upgraded from minimal field
  assertions to the snapshot comparison.
- `tests/fixtures/snapshots/cli_structured_text_json.json` gains
  `schema_version: 2`; CLI tests assert the marker for DOCX, PPTX, stdin, and
  multi-input payloads.

### 5. Formal amendment of the deferred-prohibition requirement

The canonical requirement **"No public output schema changes"** in
`openspec/specs/docx-extraction/spec.md` forbids variant labels and says
"Variant labeling is deferred to a future change." That prohibition was scoped to
#177's ordering change; #179 is that future change and this proposal performs the
amendment intentionally and visibly by **modifying** (not deleting) the
requirement: `part_type` labels stay unchanged, `DocxTextOptions` gains no option,
`TextBlock`/`DocxTable` gain no field other than `variant`, and variant labels are
now emitted through a versioned schema (`schema_version: 2`) instead of by widening
the v1 contract. The delta spec (spec phase) adds requirements for variant
labeling, payload versioning, plain-text non-leakage, global-ordinal semantics, and
PPTX slide-vs-notes documentation.

### 6. Acceptance-criteria mapping

| Issue #179 criterion | How this proposal satisfies it |
| --- | --- |
| 1. Version a JSON schema describing part/slide, paragraph/block ordinal, and text | `part_path` (part/slide), `ordinal` (global 1-based block ordinal), `text`, plus new optional `variant`; published as `schemas/v2/oxdoc-structured-text.schema.json` with `schema_version: 2` in the payload. |
| 2. Keep plain-text output unchanged | Variant is added only on the structured-block path; `extract_text` output and `docx_basic_text.txt` / `pptx_text.txt` snapshots stay byte-identical and are asserted as regressions. |
| 3. Support DOCX related parts and PPTX slide text vs speaker notes | Already implemented (`header/footer/footnotes/endnotes/comments` labels; `slide` vs `notes`). This change completes it with `variant` labeling, locks it with a PPTX snapshot, and documents `notes` = speaker notes from `ppt/notesSlides/notesSlideN.xml`. |
| 4. Add schema snapshots, fixtures, CLI tests, and documentation | v2 schema + mirror, extended section-order oracle, new PPTX structured snapshot, CLI payload tests, and doc updates (`json-output.md`, `cli.md`, `formats/docx.md`, `formats/pptx.md`, `CHANGELOG.md`). |

The issue's "paragraph/block ordinal" wording is satisfied at **block**
granularity: today a block is the whole text of one non-empty part, and `ordinal`
is its 1-based position in the flattened output list. Per-paragraph blocks and
slide-scoped ordinals are out of scope (per-paragraph granularity would rewrite
every existing snapshot for no requested capability; slide scoping is #180).

## Backwards compatibility

- **Public output contract.** Structured-json output is now governed by schema v2.
  Because every published schema sets `additionalProperties: false`, v1-strict
  validators will reject v2 output: the payload gains the additive top-level
  `schema_version` field and variant-referenced blocks gain the optional `variant`
  field. Consumers must move to `schemas/v2/oxdoc-structured-text.schema.json`;
  the v1 schema remains published unchanged for validating previously captured
  outputs. This is a documented, versioned contract change, not a silent widening,
  and it is recorded in `CHANGELOG.md`.
- **Byte-level continuity.** For single-section documents with no `first`/`even`
  variants (the common case, including every existing fixture except
  `docx/section-order`), the `blocks` array serialization is byte-identical:
  same fields, same field order, same block order, same `ordinal` values, same
  `text`, and `variant` omitted entirely (never `null`). The only byte delta in the
  whole payload is the additive `schema_version: 2` line. **Flat text and tables
  output are byte-identical for every document** — no variant data, ordering, or
  warning changes on those paths. This nuance (blocks byte-identical vs. payload
  gaining one field) is listed in the question round for confirmation, since it
  determines how release notes must phrase the change.
- **Rust API.** `TextBlock` gains an optional field and a `with_variant`
  constructor; `TextBlock::new`, `StructuredText`, `DocxTextOptions`, and all
  extraction function signatures are unchanged. `variant` is omitted from
  serialization when absent, so library-side JSON for untouched documents keeps its
  current shape (minus the CLI-only `schema_version`).
- **Other contracts untouched:** `oxdoc extract text --format json/jsonl`, DOCX
  tables schema v1, audit schemas v1, XLSX schemas v1, plain text, and CLI flags.

## Scope / affected areas

Production:

- `crates/oxdoc-core/src/models.rs` — `TextBlock::variant` + `with_variant`.
- `crates/oxdoc-core/src/parsers/docx.rs` — variant-carrying plan entry,
  `RelatedRefVariant::label`, `push_text_block` variant parameter, structured-path
  forwarding. Text/tables consumers destructure the existing index only.
- `crates/oxdoc-cli/src/main.rs` — `schema_version: 2` on `TextStructuredPayload`.
- `schemas/v2/oxdoc-structured-text.schema.json` + `docs/schemas/v2/…` (new).
- `Makefile` — `docs-schemas-check` also diffs `schemas/v2` ↔ `docs/schemas/v2`.

Tests / fixtures:

- `crates/oxdoc-core/tests/api.rs` — variant assertions, ordinal continuity,
  plain-text non-regression, path-parity preservation.
- `crates/oxdoc-cli/tests/cli.rs` — `schema_version` assertions, PPTX structured
  snapshot comparison.
- `crates/oxdoc-core/tests/schema.rs` — version-aware schema reading; v2 schema
  validation for DOCX and PPTX snapshots; version metadata assertions.
- `tests/fixtures/docx/section-order/expected.json` — `variant` expectations.
- `tests/fixtures/snapshots/cli_structured_text_json.json` — `schema_version: 2`.
- `tests/fixtures/snapshots/cli_structured_text_pptx_json.json` — new.

Docs:

- `docs/json-output.md` — v2 row/link, versioning narrative, structured-text
  semantics (variant, global ordinal, notes = speaker notes).
- `docs/cli.md` — structured-json example with `schema_version`, `variant`, and
  ordinal/slide/notes wording.
- `docs/formats/docx.md` — structured-block variant statement (orphans unlabeled,
  unknown `w:type` = `default`, plain text/tables never carry variant).
- `docs/formats/pptx.md` — new structured-json section (slide vs notes, notes part
  path, ordinal semantics, no variants).
- `CHANGELOG.md` — schema v2 + variant labeling entry.
- `openspec/specs/docx-extraction/spec.md` — composed at archive from the delta spec
  that amends the deferred-prohibition requirement.

## Non-goals (explicit)

1. **Slide-scoped ordinals and slide-scoped JSONL streaming (#180).** No per-slide
   counters, no streaming record shape, no `schema_version` redesign for streaming.
2. **XLSX formulas (#182)** and any formula recalculation or formula reporting.
3. **Changing plain-text output** in any way (ordering, labels, added markers) and
   any variant leakage into flat text.
4. **Changing existing `part_type` vocabulary.** `main`, `header`, `footer`,
   `footnotes`, `endnotes`, `comments`, `slide`, and `notes` stay byte-identical
   labels; `notes` keeps its name and its meaning (speaker notes).
5. **Per-paragraph blocks.** Block granularity stays one block per non-empty source
   part; block ordinals stay global output order.
6. **New CLI flags or `DocxTextOptions` fields** (e.g. an opt-in variant flag).
7. **Honoring `w:evenAndOddHeaders`** (`word/settings.xml` stays unread; the #177
   deferral is unchanged) and any Word rendering emulation.
8. **Versioning or touching any other schema** (tables, audit, XLSX, info, extract
   text) — only structured-text moves to v2.

## Alternatives considered (rejected)

1. **Widen `schemas/v1/oxdoc-structured-text.schema.json` in place** (add optional
   `variant`, add `schema_version` to the same file): rejected — contradicts
   `docs/json-output.md`'s stated policy, gives consumers no way to detect the new
   contract, and `additionalProperties: false` means the fields are a breaking
   shape change regardless, so the version marker is not extra cost.
2. **Encode variants into `part_type` (e.g. `header-first`, `footer-even`)**:
   rejected — mutates the established label vocabulary that v1 consumers switch on
   and would require renaming existing blocks; a separate optional field is additive
   and keeps `part_type` meaning stable.
3. **Emit `variant: null` for unlabeled blocks (no `skip_serializing_if`)**:
   rejected — adds noise to every block of every document, makes the "no change for
   variant-free documents" claim impossible, and offers nothing a missing key does
   not already express.
4. **Put `schema_version` on the core `StructuredText` model**: rejected — diverges
   from the tables/audit/payload convention where versioning is a CLI emission
   concern, and changes a core public struct's construction sites for no library
   benefit. Accepted consequence: direct library serialization stays unversioned,
   documented in `docs/json-output.md`.
5. **Per-paragraph blocks with paragraph ordinals (literal reading of AC 1)**:
   rejected for this change — rewrites block granularity and every existing
   snapshot, and the criterion is already met by the block ordinal; per-paragraph
   provenance is a separate future change if ever requested.
6. **Slide-scoped ordinals now (fold #180 in)**: rejected — two independent schema
   versions in one step multiplies review and migration cost, and #180's streaming
   concerns are record-level, not schema-level (per exploration §9).
7. **New opt-in CLI flag / `DocxTextOptions` field for variants**: rejected — the
   schema is versioned, so gating the field is unnecessary surface area; #177's
   precedent (no option for a completed behavior) applies.
8. **Re-collect variants in a new pass instead of reusing the #177 plan data**:
   rejected — a second `word/document.xml` walk can only drift from the ordering
   pass; the variant is already computed and merely discarded.

## Risks

- **v1-strict consumer breakage (highest).** `additionalProperties: false` means
  v2 output fails v1 validation even for documents with no variants, because of the
  additive `schema_version`. Mitigation: versioned schema file, frozen v1 copy,
  explicit CHANGELOG and `docs/json-output.md` migration note, and the blocks-array
  byte-continuity statement. This is the intended, parent-approved tradeoff; the
  exact phrasing of the byte-identity claim is a question-round item.
- **Three-path lockstep (#177 invariant).** Changing `RelatedPartPlan` touches the
  text, structured, and tables consumers. Mitigation: consumers destructure the
  index explicitly, the shared `expected.json` oracle keeps pinning all three paths,
  and text/tables tests continue to assert the unchanged two-tuple shape.
- **First multi-version schema layout.** `Makefile docs-schemas-check` and
  `schema.rs::read_json_schema` hardcode `schemas/v1`; a missing v2 mirror or an
  unfixed helper fails CI. Both are explicit tasks, and the schema metadata test
  derives the version segment rather than hardcoding it.
- **Snapshot churn.** `cli_structured_text_json.json` and the CLI test expectations
  change; any additional snapshot touched unexpectedly would signal variant leakage
  outside the structured path, which is exactly what the plain-text regression
  assertions are designed to catch.
- **Review budget.** Estimated at ~395–425 changed lines, i.e. at or just over the
  400-line budget (table below). Ask-on-risk delivery: the apply phase must pause
  for a delivery decision (recommended two work units of this same change) rather
  than silently chaining or declaring an exception.
- **Semantic drift of `variant` for shared/deduped parts.** A part referenced as
  two variants keeps the first reference's label. Mitigation: documented in the
  schema description and `docs/formats/docx.md`, and covered by the section-order
  fixture (which already exercises dedup and orphan parts).

## Rollback

Revert the variant field and plan-entry change in `oxdoc-core`, drop
`schema_version` from the CLI payload, delete `schemas/v2/` and `docs/schemas/v2/`,
restore the two snapshots and the `Makefile`/`schema.rs` v1-only tooling, and revert
the doc/CHANGELOG edits. Because `schemas/v1/**` is never modified, v1 consumers are
unaffected by a rollback and previously captured outputs remain valid. No data
migration, no public function signature changes, no persisted state.

## Size estimate (vs 400-line review budget)

| Work unit | Rough lines |
| --- | --- |
| `schemas/v2/oxdoc-structured-text.schema.json` + `docs/schemas/v2/` mirror | ~120–140 |
| Core model: `variant` + `with_variant` | ~10–15 |
| DOCX parser: variant-carrying plan entry + label mapping + forwarding | ~35–45 |
| CLI: `schema_version: 2` on the payload | ~5–10 |
| Tests: oracle variants, ordinal continuity, plain-text regression, v2 schema validation, CLI payload/PPTX snapshot tests | ~85–105 |
| `schema.rs` version-aware helper + metadata assertions | ~25–35 |
| `Makefile` v2 mirror check | ~2 |
| Snapshots (`cli_structured_text_json.json` bump + new PPTX snapshot) | ~20–30 |
| Docs: `json-output.md`, `cli.md`, `formats/docx.md`, `formats/pptx.md`, `CHANGELOG.md` | ~60–80 |
| **Total** | **~395–425** |

The estimate straddles the 400-line budget, so this change is **recommended to ship
as two work units/PRs** (same change id), mirroring the #177 precedent:

- **WU1 — schema contract + DOCX variants (~215 lines):** v2 schema + docs mirror +
  `Makefile` check, `schema.rs` version-aware helper, `TextBlock::variant` + DOCX
  plan plumbing, section-order oracle variants, ordinal-continuity and plain-text
  regression tests, `cli_structured_text_json.json` bump, CLI `schema_version`.
- **WU2 — PPTX snapshot + documentation (~200 lines):** PPTX structured snapshot +
  CLI snapshot test, `docs/json-output.md`, `docs/cli.md`, `docs/formats/docx.md`,
  `docs/formats/pptx.md`, `CHANGELOG.md`.

Each unit is independently reviewable and well under 400 lines, and neither unit
breaks the other (WU1's schema already declares the optional `variant`, which WU2's
PPTX snapshot simply omits). Delivery strategy is `ask-on-risk`: the apply phase
must ask for the delivery decision if the realized diff approaches 400 lines; no
chain strategy or `size:exception` is assumed here. The 95% coverage gate is
unaffected — new production lines (`label()`, plan entry, payload field) arrive with
their tests in WU1.

## Success criteria

1. `schemas/v2/oxdoc-structured-text.schema.json` (and its `docs/schemas/v2/`
   mirror) exists, is published in `docs/json-output.md`, and validates both the
   DOCX and the new PPTX structured snapshots through `schema.rs`; `schemas/v1/**`
   is unmodified.
2. `schema_version: 2` appears in every structured-json payload (single, stdin, and
   multi-input), asserted by CLI tests.
3. DOCX header/footer blocks carry `variant` ∈ {`first`, `even`, `default`} matching
   the `sectPr` reference that produced them, including the shared/orphan/dedup edge
   cases in `tests/fixtures/docx/section-order/expected.json`; no other block type
   or document type emits `variant`.
4. Plain-text and DOCX tables output are byte-identical to the current release for
   the existing fixtures and snapshots, proving variant data does not leak into flat
   text.
5. `ordinal` remains the global 1-based output-order index, asserted contiguous over
   the section-order fixture and documented in the v2 schema, `docs/json-output.md`,
   and `docs/cli.md`.
6. `docs/formats/pptx.md` documents the structured-json section, and the docs state
   explicitly that `notes` blocks carry speaker notes from
   `ppt/notesSlides/notesSlideN.xml` parts.
7. The canonical requirement "No public output schema changes" is amended through
   the delta spec (variant labels now permitted via versioned schema v2; no new
   options, no other `TextBlock`/`DocxTable` fields), rather than silently bypassed.
8. `cargo test --workspace`, `cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `make docs-schemas-check`,
   and the 95% line-coverage gate pass.
9. Realized changed lines stay within the 400-line review budget per PR, or the
   apply phase explicitly asks for a delivery decision (ask-on-risk).

## Parent decision traceability

| Parent decision | Where honored |
| --- | --- |
| 1. Variant labeling ships as JSON schema v2: optional `variant` on header/footer blocks + `schema_version: 2` in the payload; formally AMEND the archived prohibition; plain text and tables unchanged; no variant leakage into flat text | Solution shape §1–§3, §5; Non-goals 3–4; Backwards compatibility; Success criteria 1–4, 7 |
| 2. Keep the existing global 1-based `ordinal` and document it; slide-scoped ordinals are #180; the "paragraph/block ordinal" criterion is satisfied by the block ordinal | Intent 3; Solution shape §2; AC mapping; Non-goals 1, 5; Success criterion 5 |
| 3. `notes` label stays as-is; docs must state it carries speaker notes from notesSlide parts | Intent 3; Solution shape §1, §4; Scope (docs/formats/pptx.md); Success criterion 6 |
| 4. PPTX gets a structured-json snapshot test, schema v2 compatibility, and a `docs/formats/pptx.md` structured-json section | Solution shape §1, §4; Scope (tests/fixtures, `schema.rs`, `cli.rs`, docs); Success criteria 1, 6 |
| 5. Non-goals: #180 slide-scoped JSONL streaming, #182 xlsx formulas, recalculation, plain-text change | Non-goals 1–3; Alternatives 5–6 |

## Proposal question round

The PRD-level decisions above were resolved by the orchestrator, so no user
interview is needed on those. The following **assumptions** were made while
sharpening the proposal and are the ones worth confirming, correcting, or
re-framing before the spec phase turns them into fixtures and requirements:

1. **Byte-identity wording.** We interpret "output bytes for single-section
   documents without first/even variants stay identical" as: the `blocks` array —
   field set, field order, block order, ordinals, text — is byte-identical, and the
   only new bytes in the payload are the additive `schema_version: 2` field, which
   still forces v1-strict validators onto v2. Should release notes and docs phrase
   it that way, or is a stronger claim expected (which would require a different
   version-marker design that we believe is not worth the cost)?
2. **Orphan header/footer parts get no `variant`.** A header part present in the
   rels file but referenced by no `sectPr` has no section-assigned variant, so it is
   emitted without the field. Alternative: label it `default`. We believe omitting is
   more truthful; confirm.
3. **Unknown/missing `w:type` labels as `default`.** Mirrors #177's ordering rule
   silently. Alternative: omit the field for unrecognized types so only explicit
   `w:type` values are surfaced. Which is the better product answer for consumers?
4. **Dedup keeps the first reference's variant.** When one part is referenced as two
   different variants (or by two sections), the single emitted block carries the
   first reference's label. Confirm that positional-first wins over "prefer
   `first`-variant label".
5. **Multi-version schema layout.** v1 stays published and untouched; `schemas/v2/`
   contains only the changed structured-text schema (nine schemas are not copied
   forward), with `Makefile`/`schema.rs` extended to handle both version
   directories. Confirm this layout rather than copying every v1 schema into v2.

These can be answered, corrected, skipped, or followed by a second question round;
the spec phase should not silently bake in assumptions 1–5.
