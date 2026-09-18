# Design — structured text source provenance: versioned schema + header/footer variants (issue #179)

- Change id: `structured-text-source-provenance`
- Status: designed (SDD design phase, artifact store: openspec)
- Inputs: `proposal.md`, `specs/docx-extraction/spec.md`, `specs/structured-text-schema/spec.md`,
  `exploration.md`, code on `main` (post archived change
  `2026-09-17-docx-section-order-related-parts`, #177), `openspec/config.yaml`.
- Execution: auto · artifact store openspec · review budget 400 lines · ask-on-risk ·
  strict TDD (`cargo test`, fmt, clippy, 95% line-coverage gate).

## 0. Verified code facts this design builds on

- `crates/oxdoc-core/src/models.rs` (~line 187): `StructuredText { document_type, blocks }`;
  `TextBlock { part_type, part_path, ordinal, text }` with `TextBlock::new(...)`. The only
  `TextBlock` struct literal in the workspace is the definition itself — every construction
  goes through `TextBlock::new`, so adding a field is non-breaking for call sites.
- `crates/oxdoc-core/src/parsers/docx.rs`: `RelatedPartPlan = Vec<usize>` (line ~305);
  `RelatedRefVariant { First, Even, Default }` (line ~288) already carries the variant used
  only for sort ranking; `related_ref_variant` maps unknown/missing `w:type` → `Default`
  silently (line ~328). `plan_related_part_order` pushes bare indices in three places:
  section-referenced header/footer (line ~466), orphans (line ~493), notes (line ~505).
  Consumers: `extract_text` (`for index in plan` + `is_related_docx_text_part`, line ~62),
  `extract_structured_text` (`related_docx_text_part_type` + `push_text_block`, line ~118),
  `extract_tables` (`related_docx_text_part_type` + `public_tables_for_part`, line ~189).
- `push_text_block` (line ~509) skips empty text and numbers ordinals by encounter order;
  it has exactly two DOCX call sites (main at line ~84, related at line ~135). PPTX has its
  own private `push_text_block` in `pptx.rs` — renaming or re-signaturing the DOCX one does
  not touch PPTX.
- `crates/oxdoc-core/src/parsers/pptx.rs::extract_structured_text` (line ~52) pushes
  `slide` blocks in `p:sldIdLst` order then `notes` blocks per slide, with
  `renumber_blocks` keeping global ordinals contiguous. It never sees the DOCX plan and
  needs zero changes for variant (its blocks simply keep `variant: None`).
- `crates/oxdoc-cli/src/main.rs`: `TextStructuredPayload { file, #[serde(flatten)] structured }`
  (line ~1469) has exactly one construction site (line ~541) inside the shared file loop that
  already handles single-input, stdin (`-`), and multi-input uniformly. Tables/rows-jsonl/audit
  payloads all carry `schema_version: 1` at the CLI layer while their core models stay
  unversioned — the convention this change mirrors.
- `crates/oxdoc-core/tests/schema.rs`: `read_json_schema` hardcodes `schemas/v1` and is called
  9× by name; `schemas_have_stable_public_metadata` hardcodes `/schemas/v1/` in the `$id`
  assertion for the nine v1 schemas. `representative_structured_text_json_matches_schema`
  validates the DOCX-only snapshot. There is no PPTX structured snapshot anywhere.
- `Makefile` line 146: `docs-schemas-check: @diff -ru schemas/v1 docs/schemas/v1` (v1 only).
- `tests/fixtures/docx/section-order/` (from #177): two-section package whose
  `expected.json` oracle is hand-authored and consumed by api.rs tests for flat text,
  structured, and tables from one JSON. Section 1 references `header-first` (`first`),
  `header-even` (`even`), `header-default` (`default`), `footer1` (`default`), plus the two
  warning pins (`rIdGhost` unknown rid, missing rid). Section 2 re-references
  `rIdHeaderDefault` **without** `w:type`, references `header-titled` (`w:type="title"` →
  default), and `footer2` (`default`). `header-orphan.xml` is referenced by no `sectPr`.
- PPTX fixture `tests/fixtures/corpus/pptx/text/`: `presentation.xml` `sldIdLst` lists
  `rId2` **before** `rId1`, while `slide2.xml` carries the "First Slide…" content and
  `slide1.xml` carries "Second Slide" — filenames deliberately contradict presentation order,
  which makes the snapshot prove order comes from `sldIdLst`, not file naming. Slide 2's rels
  link `../notesSlides/notesSlide2.xml` ("Speaker note"). So presentation order is:
  slide2.xml → notesSlide2.xml → slide1.xml.
- `docs/json-output.md` line 37 states the versioning policy this change implements for
  structured-json; `docs/formats/pptx.md` has no structured-json section;
  `docs/cli.md` structured example (line ~149) shows the v1-shaped payload.
- #177 sizing lesson (from its archived design): the proposal estimated ~300–390 lines; the
  design recount hit ~530 because fixture XML trees, hand-authored oracles, and test bodies
  weigh 1.5–2× the first-pass estimate. This design sizes with that multiplier applied to
  every test/fixture row.

## 1. Goal and constraints (recap)

Structured-json becomes a versioned contract (schema v2 + `schema_version: 2` in the CLI
payload) and DOCX header/footer blocks gain an optional `variant` label (`first`/`even`/
`default`) sourced from the variant data #177 already computes and discards. Flat text and
tables output stay byte-identical; variant data never leaks into them. `part_type` labels,
`DocxTextOptions`, `DocxTable`, ordinal semantics, warning behavior, and `schemas/v1/**` are
untouched. PPTX structured output gains a snapshot lock and documentation but no behavior
change.

## 2. Design question 1 — exact variant data plumbing

### 2.1 Core model (`crates/oxdoc-core/src/models.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextBlock {
    pub part_type: String,
    pub part_path: String,
    pub ordinal: usize,
    pub text: String,
    /// DOCX header/footer variant (`first` | `even` | `default`); omitted
    /// from serialization for every other block kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

impl TextBlock {
    pub fn new(
        part_type: impl Into<String>,
        part_path: impl Into<String>,
        ordinal: usize,
        text: impl Into<String>,
    ) -> Self {
        Self { part_type: part_type.into(), part_path: part_path.into(),
               ordinal, text: text.into(), variant: None }
    }

    /// Builder for the section-referenced DOCX header/footer path.
    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }
}
```

Decisions baked in:

- `variant` is declared **after `text`** so serde field order keeps existing JSON shape
  byte-stable; when absent, `skip_serializing_if` omits the key entirely (never `null`) —
  this is what makes the "blocks array byte-identical for variant-free documents" claim
  true.
- `TextBlock::new` keeps its exact signature and produces `variant: None`. PPTX, and every
  existing caller, compile and serialize unchanged.
- `with_variant` is a consuming builder (like the established `OutputWarning` style), used
  only by the DOCX structured path.
- No `Deserialize` is added: the model is Serialize-only today and stays so (adding Deserialize
  would be a public API widening outside the approved scope; library consumers construct
  blocks, they do not parse them).

### 2.2 Plan entry (`crates/oxdoc-core/src/parsers/docx.rs`)

The plan element changes from a bare index to a variant-carrying struct:

```rust
/// One entry of the shared related-part iteration order: which relationship
/// to emit, plus the section-assigned variant when the reference that
/// positioned the part carried one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RelatedPartPlanEntry {
    /// Index into the `parse_relationships` result vector.
    index: usize,
    /// `Some(v)` only for header/footer parts positioned by a `sectPr`
    /// reference; `None` for orphans, footnotes, endnotes, and comments.
    variant: Option<RelatedRefVariant>,
}

type RelatedPartPlan = Vec<RelatedPartPlanEntry>;

impl RelatedRefVariant {
    /// Public label for the structured path; matches the ordering rank names.
    fn label(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Even => "even",
            Self::Default => "default",
        }
    }
}
```

`plan_related_part_order` changes in exactly three push sites, with **no algorithmic
change** (the sort key `(reference.kind, reference.variant)`, dedup, orphan walk, and
notes walk are untouched):

1. Section-referenced loop (was `plan.push(index)`):
   `plan.push(RelatedPartPlanEntry { index, variant: Some(reference.variant) })`. Because
   dedup happens at first reference via `emitted_paths.insert(part_path)`, the stored
   variant is automatically the **first** reference's variant — the positional-first rule
   needs no extra logic.
2. Orphan loop: `plan.push(RelatedPartPlanEntry { index, variant: None })` — orphans have
   no section-assigned variant.
3. Notes loop: same, `variant: None` for footnotes/endnotes/comments.

The `sort_by_key` over `(kind, variant)` is untouched: `RelatedRefVariant` keeps its
`Ord` derive for ranking, and `label()` is only called at emission time.

### 2.3 Consumer destructuring — behavior-identical for text and tables

All three loops change one line each; the per-part bodies are unchanged:

```rust
// extract_text (flat text):
for entry in plan {
    let relationship = &relationships[entry.index];
    if !is_related_docx_text_part(relationship.relationship_type.as_deref(), options) {
        continue;
    }
    // ... unchanged body; entry.variant is never read
}

// extract_tables:
for entry in plan {
    let relationship = &relationships[entry.index];
    // ... unchanged body
}

// extract_structured_text:
for entry in plan {
    let relationship = &relationships[entry.index];
    let Some(part_type) = related_docx_text_part_type(...) else { continue };
    // ...
    push_text_block(&mut blocks, part_type, &part_path, entry.variant, part.value);
}
```

`push_text_block` gains one parameter:

```rust
fn push_text_block(
    blocks: &mut Vec<TextBlock>,
    part_type: &str,
    part_path: &str,
    variant: Option<RelatedRefVariant>,
    text: String,
) {
    if text.is_empty() {
        return;
    }
    let block = TextBlock::new(part_type, part_path, blocks.len() + 1, text);
    let block = match variant {
        Some(v) => block.with_variant(v.label()),
        None => block,
    };
    blocks.push(block);
}
```

Call sites: the main-part call passes `None`; the related-part call passes `entry.variant`.
Empty-text parts are skipped before any variant attaches, so a first/even header whose text
extraction yields nothing emits nothing (same as today) and no phantom variant survives.

How variant reaches `TextBlock` **only** in the structured path: `extract_text` and
`extract_tables` destructure `entry.index` and never name `entry.variant`; the compiler
enforces that the field exists but nothing reads it on those paths. Variant conversion to a
public string happens exactly once (`label()` inside `push_text_block`), so flat text and
tables are structurally and byte-wise unchanged — locked by the unchanged two-tuple
assertions in the oracle-driven tests (§5).

`RelatedRefVariant::label` returning `&'static str` (not `String`) keeps the hot path
allocation-free; `with_variant(impl Into<String>)` accepts it directly.

PPTX (`pptx.rs`) needs zero changes: its own `push_text_block` calls `TextBlock::new`,
which now yields `variant: None`, and `serde` omits the key. This is asserted, not assumed,
by the PPTX snapshot (§4) and the api.rs non-leakage test (§5).

## 3. Design question 2 — schema v2 and the first multi-version layout

### 3.1 Full schema: `schemas/v2/oxdoc-structured-text.schema.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/spereyra-dev/oxdoc/schemas/v2/oxdoc-structured-text.schema.json",
  "title": "oxdoc structured text JSON output",
  "description": "Schema for `oxdoc extract text --format structured-json` output in oxdoc schema version 2.",
  "type": "object",
  "additionalProperties": false,
  "required": ["schema_version", "file", "document_type", "blocks"],
  "properties": {
    "schema_version": {
      "type": "integer",
      "const": 2,
      "description": "Schema version of this payload; always 2 for this contract."
    },
    "file": {
      "type": "string",
      "description": "File name derived from the provided input path."
    },
    "document_type": {
      "type": "string",
      "enum": ["docx", "pptx"],
      "description": "Detected text-bearing OOXML document type."
    },
    "blocks": {
      "type": "array",
      "description": "Ordered non-empty text blocks with source part metadata.",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["part_type", "part_path", "ordinal", "text"],
        "properties": {
          "part_type": {
            "type": "string",
            "description": "Logical source kind, such as main, header, footer, footnotes, endnotes, comments, slide, or notes. `notes` blocks carry speaker notes extracted from a `ppt/notesSlides/notesSlideN.xml` part."
          },
          "part_path": {
            "type": "string",
            "description": "OOXML package part path that produced this text block."
          },
          "ordinal": {
            "type": "integer",
            "minimum": 1,
            "description": "1-based global output-order index across the flattened block list — not a paragraph ordinal and not scoped per part or per slide."
          },
          "text": {
            "type": "string",
            "description": "Extracted text for this source block."
          },
          "variant": {
            "type": "string",
            "enum": ["first", "even", "default"],
            "description": "DOCX header/footer variant from the `w:type` of the `sectPr` reference that positioned the part; missing or unrecognized `w:type` values are labeled `default`. Present only on section-referenced DOCX `header` and `footer` blocks; omitted (never null) for orphan header/footer parts and for all other block kinds and document types. When one part is referenced as multiple variants, the first reference's variant wins."
          }
        }
      }
    }
  }
}
```

Placement: `schemas/v2/oxdoc-structured-text.schema.json` plus the exact mirrored copy at
`docs/schemas/v2/oxdoc-structured-text.schema.json`. Only the changed structured-text schema
lives in v2 — the other eight v1 schemas are **not** copied forward (v1 remains the
validatable contract for them and for previously captured structured outputs).

### 3.2 Multi-version tooling (the two hardcoded v1 assumptions)

`Makefile` (both directories must be updated together, enforced per version):

```make
docs-schemas-check:
	@diff -ru schemas/v1 docs/schemas/v1
	@diff -ru schemas/v2 docs/schemas/v2
```

`crates/oxdoc-core/tests/schema.rs` becomes version-aware:

```rust
fn read_json_schema(version: &str, name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas")
        .join(version)
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
```

- The nine existing call sites change from `read_json_schema("x.schema.json")` to
  `read_json_schema("v1", "x.schema.json")` (mechanical, ~9 one-word edits).
- `schemas_have_stable_public_metadata` is restructured to iterate
  `const SCHEMA_VERSIONS: &[(&str, &[&str])]` — `("v1", &[nine names])` and
  `("v2", &["oxdoc-structured-text.schema.json"])` — asserting
  `$id.ends_with(&format!("/schemas/{version}/{name}"))` instead of hardcoding `v1`. The
  version segment is thereby derived from the directory, so adding v3 later is a one-line
  table edit.
- `representative_structured_text_json_matches_schema` switches to the v2 schema (its
  snapshot gains `schema_version: 2`), and additionally asserts
  `output["schema_version"] == schema["properties"]["schema_version"]["const"]` (the shared
  `validate_object` helper is not extended; the const check lives in the two structured
  tests, mirroring how the rows-jsonl test asserts its const).
- New `representative_structured_text_pptx_json_matches_schema` validates the new PPTX
  snapshot against v2 with the same const assertion.

No new dependency: validation stays the hand-rolled `validate_object` walker (house style,
as for v1). `additionalProperties: false` on block objects is asserted by the walker's
"field not declared in schema" panic path, which is exactly how v1 is validated today.

`docs/json-output.md` moves the structured-json row to
[`schemas/v2/oxdoc-structured-text.schema.json`](schemas/v2/oxdoc-structured-text.schema.json),
adds a v1 link marked "for outputs captured before schema v2", and states the v1-strict
migration note (§7).

## 4. Design question 3 — CLI payload version marker

`crates/oxdoc-cli/src/main.rs`:

```rust
#[derive(Debug, serde::Serialize)]
struct TextStructuredPayload {
    schema_version: u8,
    file: String,
    #[serde(flatten)]
    structured: StructuredText,
}
```

- Emission order becomes `schema_version, file, document_type, blocks` (the flattened
  struct's fields land where `#[serde(flatten)]` sits, matching TablesPayload's field
  layout).
- The single construction site at main.rs line ~541 serves single-input, stdin (`-`), and
  multi-input alike — they all flow through the same file loop — so one added field
  (`schema_version: 2,`) covers all three modes. The multi-input mode emits the same object
  per array element. The empty-batch case (`emits_empty_structured_json_batch_when_no_inputs_succeed`)
  is untouched.
- Core `StructuredText` stays unversioned (alternative 4 in the proposal stands): library
  callers serializing `StructuredText` directly get the v1-shaped body plus optional
  `variant`, no `schema_version`. This asymmetry is documented in `docs/json-output.md`.

Snapshots:

- `tests/fixtures/snapshots/cli_structured_text_json.json` gains `"schema_version": 2` as
  the first key; nothing else in it changes (its blocks have no `variant` — main + comments
  only), which *is* the byte-continuity evidence for blocks.
- The v1 snapshot contract: no v1-published artifact is modified. The snapshot file is a
  test fixture for the *current* contract, so bumping it in the same commit as the payload
  change is legitimate and mirrors the tables payload's `schema_version: 1` convention.

CLI tests (`crates/oxdoc-cli/tests/cli.rs`):

- `extracts_text_as_structured_json`, `extracts_pptx_text_as_structured_json`, and the
  stdin/multi test each gain `assert_eq!(actual["schema_version"], 2);`.
- The DOCX structured test additionally asserts
  `actual["blocks"][0].get("variant").is_none()` (serialization omission, not `null`).
- `extracts_pptx_text_as_structured_json` is upgraded to full snapshot comparison (§5).

## 5. Design question 4 — PPTX structured snapshot

- Fixture: the existing `tests/fixtures/corpus/pptx/text/`, built into a temp package by
  the existing `fixtures::build_package("pptx/text", "structured-slide.pptx")` call — no new
  fixture authoring.
- Snapshot: new `tests/fixtures/snapshots/cli_structured_text_pptx_json.json`, generated
  **once** from the implementation output and then frozen (this is the #177-sanctioned
  approach for locking *existing* behavior: the content is reviewed against the known
  structure below, not hand-guessed). Expected shape from the verified fixture facts:

  | # | part_type | part_path | text |
  | --- | --- | --- | --- |
  | 1 | `slide` | `ppt/slides/slide2.xml` | `First Slide\nAlpha\tBeta & Co\nGamma < Delta\n` |
  | 2 | `notes` | `ppt/notesSlides/notesSlide2.xml` | `Speaker note\n` |
  | 3 | `slide` | `ppt/slides/slide1.xml` | `Second Slide\n` |

  The filename-vs-order inversion (slide2.xml first) is the load-bearing detail: the
  snapshot proves `p:sldIdLst` order governs, notes follow their slide, and ordinals are
  globally contiguous — three facts in one artifact. All blocks omit `variant`.
- Test upgrade: `extracts_pptx_text_as_structured_json` keeps its `status`/`stderr`
  assertions and replaces the minimal field asserts with:
  byte-compare `stdout(&output).trim_end()` against
  `fixtures::read_snapshot("cli_structured_text_pptx_json.json")` (the established
  `pptx_text.txt` comparison style), plus a parsed-`Value` equality against the snapshot
  for failure readability. It keeps asserting `schema_version == 2` via the parsed value.
- Schema lock: the new `schema.rs` PPTX test validates the same snapshot against
  `schemas/v2/oxdoc-structured-text.schema.json`.
- Docs: `docs/formats/pptx.md` gains a "Structured JSON" section (slide vs `notes`
  meaning, the `ppt/notesSlides/notesSlideN.xml` part path, global-ordinal semantics,
  "PPTX blocks never carry `variant`") and a structured-json example; `docs/cli.md`'s
  structured example gains `schema_version` and a header `variant` sample plus the
  slide/notes/ordinal sentence.

## 6. Design question 5 — test plan (strict TDD, RED → GREEN per requirement)

### R1 — Variant labeling (docx-extraction delta: MODIFIED prohibition + ADDED labeling)

RED (fails to compile — the strictest RED, since `variant` and `label()` don't exist yet):

| Test | Location | Pins |
| --- | --- | --- |
| `labels_plan_entries_with_section_reference_variants` (extend existing plan unit tests) | `docx.rs #[cfg(test)]` | Section-referenced entries carry `Some(First/Even/Default)` matching the sorted reference; orphans and notes entries carry `None`. |
| `label_maps_variants_to_public_names` | `docx.rs #[cfg(test)]` | `label()` = `first`/`even`/`default` for all three variants. |
| `plans_deduped_part_with_first_reference_variant` | `docx.rs #[cfg(test)]` | One part referenced as `first` by an early section and differently later → entry keeps the first reference's variant. |
| extend `orders_docx_structured_blocks_by_section` | `api.rs` | Asserts `(part_type, part_path, variant)` triples from the oracle; the text and tables tests keep the unchanged two-tuple assertions (#177 parity pinned). |
| `keeps_non_variant_blocks_without_variant_key` | `api.rs` | Serialized `main`/`footnotes`/`comments` blocks (and PPTX `slide`/`notes`) have no `variant` key (`.get("variant").is_none()`), never `null`. |

GREEN: `TextBlock::variant` + `with_variant`, `RelatedPartPlanEntry`, `label()`, the three
push-site changes, `push_text_block` parameter. REFACTOR: none expected (single conversion
site).

Fixture work for R1 (cheap, high-leverage — verified against the existing package):

- `tests/fixtures/docx/section-order/expected.json`: add `"variant"` to the seven
  section-referenced header/footer parts — `header-first.xml` → `first`, `header-even.xml`
  → `even`, `header-default.xml` → `default` (first reference, section 1), `footer1.xml` →
  `default`, `header-titled.xml` → `default` (`w:type="title"` unknown → default, silent),
  `footer2.xml` → `default` — and **no** `variant` key on `header-orphan.xml`, `main`,
  `comments`, `footnotes`. The oracle stays hand-authored truth, updated in the same commit
  as the parser change (never generated from output).
- `package/word/document.xml` one-word edit: section 2's
  `headerReference r:id="rIdHeaderDefault"` gains `w:type="first"`. This makes the shared
  part genuinely referenced as two *different* variants (`default` first, `first` later)
  with zero oracle or ordering change — the shared/deduped-variant scenario is then pinned
  end-to-end at both unit and fixture level. (`w:titlePg` remains set in section 1 and
  unconsulted, per #177.)

R2 — Variant-free documents keep block serialization byte-identical:

| Test | Location | Pins |
| --- | --- | --- |
| existing `extracts_text_as_structured_json` + no-variant-key assert | `cli.rs` | Single-section inline package: field set/order unchanged, no `variant`. |
| `cli_structured_text_json.json` diff review | snapshot | Only `schema_version: 2` added; blocks byte-identical (reviewer-checked diff, plus the byte-compare in R5-style tests for DOCX where applicable). |
| unchanged `docx_basic_text.txt` / `pptx_text.txt` snapshot tests | `cli.rs` | Flat text byte-identity regression (already passing; run unmodified). |

R3 — Payload carries `schema_version` 2:

| Test | Location | Pins |
| --- | --- | --- |
| `schema_version == 2` asserts (single, PPTX, stdin, multi) | `cli.rs` | Every emission mode carries the marker; multi mode carries it per element. |
| `schema.rs` const assertions on both structured snapshots | `schema.rs` | Snapshot `schema_version` equals the schema's `const`. |

R4 — Plain text and tables never carry provenance:

| Test | Location | Pins |
| --- | --- | --- |
| `keeps_plain_text_free_of_variant_metadata` | `api.rs` | Flat text over the section-order fixture equals the oracle's joined part texts and contains no `word/header` / `word/footer` paths, no `first`/`even` variant tokens as standalone markers, and no ordinals. |
| unchanged `orders_docx_text_related_parts_by_section` / `orders_docx_tables_by_section` | `api.rs` | Byte-level three-path parity: these keep asserting the two-tuple shape and pass unmodified. |

R5 — PPTX structured snapshot (structured-text-schema delta):

| Test | Location | Pins |
| --- | --- | --- |
| upgraded `extracts_pptx_text_as_structured_json` | `cli.rs` | Byte-for-byte snapshot comparison incl. `schema_version`, slide/notes order, presentation-order resolution. |
| `representative_structured_text_pptx_json_matches_schema` | `schema.rs` | PPTX snapshot validates against v2. |

R6 — Global ordinal semantics:

| Test | Location | Pins |
| --- | --- | --- |
| `ordinals_are_contiguous_in_output_order` | `api.rs` | Over the section-order fixture (and the PPTX snapshot implicitly), `ordinal == 1..=blocks.len()` spanning main, headers, footers, comments. |

R7 — Versioning policy + mirror lockstep + v1 frozen:

| Check | Location | Pins |
| --- | --- | --- |
| restructured `schemas_have_stable_public_metadata` | `schema.rs` | `$id` version segment derived from the directory for v1 (9) and v2 (1); draft 2020-12; `additionalProperties: false`. |
| `make docs-schemas-check` | `Makefile` | v1 and v2 mirror diff passes. |
| v1 frozen | change diff review | No file under `schemas/v1/**` or `docs/schemas/v1/**` modified (apply/verify checklist item). |
| v1-strict rejection | `schema.rs` (small assertion) | The v2 snapshot payload fails the v1 schema's `additionalProperties: false` via the walker's undeclared-field panic logic — asserted once with a targeted negative test (`output["schema_version"]` not declared in v1 → validation panics), documented rather than load-bearing in CI. |

TDD sequencing:

- **RED 1**: R1 tests + oracle updates → `cargo test` red (compile failure on `variant`).
- **GREEN 1**: core model + docx.rs plumbing → workspace green, flat text/tables untouched.
- **RED 2**: R3/R5 CLI + schema.rs version-aware tests + v2 schema referenced by tests →
  red (missing schema file / missing marker).
- **GREEN 2**: v2 schema + mirror + Makefile + CLI `schema_version: 2` + snapshot bumps +
  PPTX snapshot → workspace green.
- **REFACTOR**: fold doc wording (R7-adjacent) after code green; docs and CHANGELOG last.
- Coverage gate: every new production line (`variant` field, `with_variant`, `label()`,
  plan entry, payload field) lands with its exercising tests in the same unit; the 95%
  line gate is not at risk (no new untested branch — `label()` is total, the plan entry is
  data).

## 7. Design question 6 — work-unit split and honest sizing (400-line budget)

### The #177 lesson, applied

#177's proposal estimated 300–390 lines; its design recount measured ~530, driven by
fixture XML trees, hand-authored oracles, and test bodies running 1.5–2× first-pass
estimates. Applying that multiplier to this proposal's 395–425 estimate gives a realistic
total of **~560–660 changed lines**, and — more importantly — the proposal's *split* is
misbalanced: WU1 (schema + DOCX variants, estimated ~215) actually concentrates the two
heaviest categories (schema files + parser tests + oracle) and lands first.

### Honest recount of the proposal's split

| Item | Proposal est. | Honest est. | Why |
| --- | --- | --- | --- |
| v2 schema + mirror | 120–140 | ~150–160 | Two ~75–80-line JSON files (v1 file is 62 lines; v2 adds `schema_version`, `variant`, and longer descriptions). |
| models.rs variant + `with_variant` | 10–15 | ~15–20 | Field, builder, doc comments. |
| docx.rs plumbing (non-test) | 35–45 | ~45–55 | Entry struct + `label()` + three push sites + `push_text_block` re-signature. |
| docx.rs unit tests | (in tests row) | ~70–100 | Label mapping, plan-variant coverage, dedup-variant — real bodies, not stubs. |
| expected.json variants + document.xml edit | (in fixtures) | ~15–20 | Seven `"variant"` keys + the `w:type="first"` edit. |
| api.rs tests | 85–105 (shared) | ~55–75 | Triple assertion, no-variant-key tests, ordinal continuity, plain-text regression. |
| schema.rs version-aware helper + metadata + v2 tests | 25–35 | ~45–60 | 9 call-site edits + metadata restructure + const asserts + PPTX validation + negative v1 assert. |
| Makefile | 2 | 2 | One added diff line. |
| CLI `schema_version` | 5–10 | ~5 | One field + asserts counted below. |
| cli.rs tests (marker asserts + PPTX snapshot upgrade) | (shared) | ~25–35 | Four marker asserts, no-variant assert, snapshot comparison rewrite. |
| Snapshots: DOCX bump + new PPTX snapshot | 20–30 | ~50–60 | Pretty-printed 3-block PPTX payload ≈ 45–55 lines alone; DOCX bump ≈ 2. |
| Docs (json-output, cli, docx, pptx, CHANGELOG) | 60–80 | ~85–100 | Five files; pptx.md gains a whole section; cli.md example rewrite; migration note. |
| **Total (proposal split)** | **395–425** | **~560–660** | |

Under the proposal split: **WU1 ≈ 430–510 lines (over the 400 budget)** and WU2 ≈ 130–190
(comfortably under). Total is ~1.4–1.6× the proposal estimate — the same direction of error
as #177, concentrated exactly where #177's error was (tests, fixtures, snapshots).

### Recommended split: three work units, each honestly under 400

The fix is not shrinking scope but cutting the cross-product: the versioned contract
(schema + CLI marker + snapshots) is separable from the variant plumbing, because the
plumbing is fully testable at the core/API level before the payload is versioned.

| Work unit | Contents | Honest size |
| --- | --- | --- |
| **WU1 — variant plumbing + oracle + regressions** | models.rs `variant`/`with_variant`; docx.rs plan entry + `label()` + forwarding; `expected.json` variants + `document.xml` `w:type="first"` edit; api.rs variant/oracle/ordinal-continuity/plain-text-regression tests; docx.rs unit tests. **No schema, no CLI change, no snapshot change.** | **~240–310** |
| **WU2 — versioned contract + CLI + snapshots** | `schemas/v2/` + `docs/schemas/v2/` mirror; Makefile v2 pass; `schema.rs` version-aware helper, metadata restructure, v2 validation tests (DOCX + PPTX + negative v1 assert); CLI `schema_version: 2`; `cli_structured_text_json.json` bump + marker asserts; new PPTX snapshot + upgraded `extracts_pptx_text_as_structured_json`. | **~300–380** |
| **WU3 — documentation** | `docs/json-output.md` (v2 row, migration note, structured-text semantics paragraph), `docs/cli.md` (example + wording), `docs/formats/docx.md` (variant statement: orphans unlabeled, unknown `w:type` = default, plain text/tables never carry variant), `docs/formats/pptx.md` (structured-json section), `CHANGELOG.md`. | **~85–100** |

Notes on the intermediate state: WU1 merged alone emits `variant` under an unversioned
payload — it violates no locked test (schema tests are updated in WU2) but is a transient
state that must not ship in a release; WU2 must merge before any tag. This is called out
here so the apply phase states it in the delivery ask rather than discovering it later.
WU1's oracle edit and parser change land in one commit (the #177 oracle-truthfulness rule).
Each unit runs the full gate (`cargo test --workspace`, fmt, clippy,
`docs-schemas-check` from WU2 on, coverage ≥ 95%).

Delivery-strategy implication (ask-on-risk): if the realized WU2 diff approaches 400 lines
at apply time, pause and ask; the pre-declared fallback ladder is (1) move the PPTX
snapshot + its `schema.rs`/`cli.rs` tests into a fourth unit, (2) two-PR chain with WU2
split as declared, (3) `size:exception` only with explicit user acceptance. No chain
strategy or exception is assumed now.

## 8. Design question 7 — rollback and risks

### Rollback

Per unit, in reverse merge order:

- WU3: revert the five doc files. No code surface.
- WU2: delete `schemas/v2/` and `docs/schemas/v2/`, revert the Makefile line, restore
  `schema.rs`'s single-version helper shape, drop `schema_version` from
  `TextStructuredPayload`, restore `cli_structured_text_json.json`, delete the PPTX
  snapshot and its test upgrade.
- WU1: remove `variant` from `TextBlock` (+ `with_variant`), restore
  `RelatedPartPlan = Vec<usize>` and the three push sites, revert `expected.json` and the
  `document.xml` edit, remove the api.rs/docx.rs variant tests.

Because `schemas/v1/**` is never modified, a rollback leaves v1 consumers and previously
captured outputs fully valid. No data migration, no persisted state, no public function
signature changes (the only signature deltas are private-to-crate: `push_text_block` and
the plan type). Rollback is per-PR trivial; no cross-unit coupling beyond merge order.

### Risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| v1-strict consumer breakage: `additionalProperties: false` means v2 output fails v1 validation even for variant-free documents (additive `schema_version`). | Highest, intended | Frozen v1 directory; versioned v2 file; CHANGELOG + `docs/json-output.md` migration note phrased per the proposal's byte-identity wording ("blocks array byte-identical; only new payload bytes are `schema_version: 2`"); negative v1-validation assertion in schema.rs documents the failure mode explicitly. |
| Transient unversioned variant output if WU1 merges alone and a release is cut. | Medium, process | Explicit sequencing rule in §7: WU2 before any tag/release; noted in the delivery ask. |
| Three-path lockstep: `RelatedPartPlan` is consumed by text, structured, and tables. | Medium | Consumers destructure only `entry.index` on the flat/tables paths; the shared `expected.json` oracle pins all three paths; text/tables tests keep the two-tuple shape so any drift fails CI. |
| First multi-version schema layout: a missing v2 mirror or an unfixed helper fails CI. | Medium | Both are explicit WU2 tasks; `docs-schemas-check` covers v2 mirroring; the metadata test derives the version segment from the directory (v3-ready). |
| Snapshot churn beyond the two structured snapshots would signal variant leakage outside the structured path. | Low | The R4 plain-text/tables regression tests and the unmodified flat-text snapshot tests act as leakage tripwires; any unexpected snapshot diff is a stop-and-investigate signal, not a mechanical update. |
| Semantic drift of `variant` for shared/deduped parts. | Low | Positional-first rule is structural (dedup at first reference stores the first variant); pinned by the new plan unit test **and** end-to-end via the one-word fixture edit making section 2's shared reference `w:type="first"`; documented in the schema description and `docs/formats/docx.md`. |
| Estimate error recurrence (#177 lesson). | Medium | This design applies the 1.5–2× test/fixture multiplier up front, recommends three units sized to measured reality, and keeps ask-on-risk as the pressure valve rather than assuming the estimate holds. |
| 95% coverage gate: new branches in `label()`/plan entry must be exercised. | Low | `label()` is a total match over a 3-variant enum with a unit test; plan-entry variants are covered by plan unit tests + oracle tests; no new untested error path is introduced. |

## 9. Rollout

1. **WU1** (core + oracle): strict TDD per R1/R4/R6; gate = full `ci-rust` set; oracle and
   parser change in the same commit; plain-text/tables snapshot stability asserted by
   running the unmodified tests.
2. **WU2** (versioned contract): strict TDD per R3/R5/R7; gate includes
   `make docs-schemas-check`; snapshot review step (PPTX snapshot content checked against
   the §5 table before commit; DOCX snapshot diff shows only `schema_version`).
3. **Pause point (ask-on-risk)**: after WU2 GREEN, report measured sizes for WU1+WU2 and
   ask for the delivery decision on WU3/docs chaining (expected trivial, but the decision
   belongs to the orchestrator, not this design).
4. **WU3** (docs + CHANGELOG): gate = `make docs-check docs-links docs-schemas-check` +
   fmt/clippy (docs touch no Rust).
5. Archive composes the delta specs into `openspec/specs/docx-extraction/spec.md` and the
   new `structured-text-schema` domain spec.

## 10. Decision log

| Decision | Choice | Rationale |
| --- | --- | --- |
| Plan representation | `Vec<RelatedPartPlanEntry>` (index + `Option<RelatedRefVariant>`) | Variant is already computed and merely discarded (#177); a second collection pass could drift from the ordering pass. Flat/tables consumers name only `.index`, so their behavior is compile-time pinned. |
| Variant label source | `RelatedRefVariant::label()` → `&'static str` | No allocation on the hot path; mapping mirrors the ordering rank exactly; unknown/missing `w:type` → `default` silently (spec: no warning). |
| `TextBlock::variant` placement | Last field, `skip_serializing_if`, `Option<String>` | Preserves existing JSON field order and byte-stability; omitted, never `null`; `new()` unchanged keeps PPTX and all call sites compiling. |
| Orphan header/footer variant | Omitted (not `default`) | An orphan has no section-assigned variant; labeling it `default` would fabricate provenance. Matches the proposal's question-round assumption 2. |
| Shared-part variant | First reference wins | Falls out of the existing `emitted_paths` dedup for free; pinned by unit test + one-word fixture edit (`w:type="first"` on section 2's shared reference) with zero oracle churn. |
| `schema_version` placement | CLI `TextStructuredPayload`, not core `StructuredText` | Mirrors tables/rows-jsonl/audit convention; keeps the core public struct untouched; asymmetry documented. |
| Schema versioning layout | `schemas/v2/` holds only the changed structured-text schema; v1 frozen | Follows the stated policy; avoids copying nine unchanged schemas; tooling made version-driven so v3 is a one-line table edit. |
| PPTX snapshot source | Generated once from implementation output, then frozen and reviewed against the known block table | Locks existing behavior (AC 3/4) without inventing expectations; the §5 table is the review oracle. |
| Fixture strategy | Extend `docx/section-order` in place (one-word `document.xml` edit + oracle variants); no new fixture packages | The fixture already exercises every variant edge (first/even/default/title/orphan/dedup/warnings); a new fixture would duplicate coverage and repeat #177's fixture-weight blowout. |
| Work-unit split | Three units (plumbing / versioned contract / docs) instead of the proposal's two | The proposal's WU1 concentrates schema + parser + oracle + tests and lands ~430–510 honest lines (over budget); rebalancing keeps every unit under 400 with no exception and no silent chain. |
| Coverage of `validate_object` const checking | Local asserts in the structured tests | Extending the shared helper would change semantics for nine other tests for one field; local asserts mirror the rows-jsonl precedent. |
