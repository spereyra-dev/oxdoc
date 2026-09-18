# Design — slide-scoped PPTX JSON/JSONL with stable slide identity (#180)

- Change: `pptx-slide-scoped-json-jsonl`
- Phase: SDD design (artifact store: openspec)
- Authoritative spec: `specs/pptx-slide-extraction/spec.md` (11 requirements, 28 scenarios)
- Design grounded against: `crates/oxdoc-core/src/parsers/pptx.rs`,
  `crates/oxdoc-core/src/models.rs`, `crates/oxdoc-core/src/lib.rs`,
  `crates/oxdoc-cli/src/main.rs`, `crates/oxdoc-core/tests/schema.rs`,
  `tests/fixtures/mod.rs`, `schemas/v1/oxdoc-xlsx-rows-jsonl.schema.json`,
  `schemas/v1/oxdoc-docx-tables.schema.json`, `Makefile`,
  `scripts/check-compatibility-corpus.py`, `openspec/config.yaml`.

## 0. Executive summary

The change adds a per-slide PPTX contract alongside (never inside) the frozen
structured-text v2 output: a new unversioned core model `PptxSlideText`, three
new `extract_pptx_slides*` lib entry points, an output-neutral enrichment of the
shared slide-reference parse (`p:sldId/@id` + `sldIdLst` position), a per-slide
skip-with-warning loop with three spec-locked warning texts, a new
`oxdoc extract slides --format json|jsonl` subcommand, a new
`schemas/v1/oxdoc-pptx-slides.schema.json` contract (+ `docs/schemas/v1`
mirror), two new runtime-zipped corpus fixtures with provenance notes, and docs.
Every existing output, snapshot, signature, schema file, and fixture digest stays
byte-identical; the only shared-code change (the slide-reference parse) is
consumed by the existing text/structured paths through the same `r:id` field
they use today.

The parent-resolved decisions this design implements are already locked in the
spec (subcommand shape, stderr-only JSONL warnings, JSON embedded+stderr
warnings, skip-with-warning granularity, own `schema_version: 1` contract,
fixture reuse). The design answers the remaining engineering questions:
struct/serde shapes, the `@id` parse that cannot perturb `r:id` behavior,
oneOf-schema mechanics under the existing `schema.rs` harness, the stdin file
label, fixture XML layout, and how to keep four PRs near the 400-line budget.

## 1. Core API

### 1.1 Model — `crates/oxdoc-core/src/models.rs`

Add next to `StructuredText` / `DocxTables` (unversioned; the version marker is
a CLI emission concern, exactly like `StructuredText`):

```rust
/// One extracted PPTX slide with stable identity and its speaker notes.
///
/// `slide_id` is the `p:sldId/@id` attribute (OOXML `xsd:unsignedInt`) and is
/// omitted from serialization when the element had no parsable `@id`.
/// `slide_ordinal` is the 1-based position of the `p:sldId` element within
/// `p:sldIdLst` (gaps after skipped slides are intentional). `notes` is
/// omitted when the slide links no readable notes part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PptxSlideText {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slide_id: Option<u32>,
    pub slide_ordinal: usize,
    pub slide_path: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}
```

Design notes:

- `slide_id: Option<u32>` implements the spec's "key omitted when absent"
  through `skip_serializing_if`, matching the existing `TextBlock.variant` /
  `DocumentInfo` precedent. It is `u32` (not `usize`) because OOXML types
  `@id` as `xsd:unsignedInt`; the schema declares it `integer`.
- `text` is a plain `String` — a textless slide serializes `"text": ""`
  (spec: record set stays aligned with the presentation).
- `notes: Option<String>` is `Some("")` for a readable-but-empty notes part,
  `None` for no relationship / missing rels file / missing notes part.
- No changes to `TextBlock`, `StructuredText`, `DocxTables`, `DocxTextOptions`,
  or `Extraction<T>` (non-goals).
- No `Deserialize` derive: nothing in the repo deserializes models, and strict
  project rules keep derives minimal. `Serialize` + `PartialEq, Eq` are enough
  for tests and CLI emission.

### 1.2 Library entry points — `crates/oxdoc-core/src/lib.rs`

Mirror the existing PPTX family (path / reader / reader-with-limits), matching
the `extract_pptx_structured_text*` signature pattern exactly:

```rust
pub fn extract_pptx_slides(
    path: impl AsRef<Path>,
) -> Result<Extraction<Vec<PptxSlideText>>>;

pub fn extract_pptx_slides_from_reader<R: Read + Seek>(
    reader: R,
) -> Result<Extraction<Vec<PptxSlideText>>>;

pub fn extract_pptx_slides_from_reader_with_limits<R: Read + Seek>(
    reader: R,
    limits: OoxmlLimits,
) -> Result<Extraction<Vec<PptxSlideText>>>;
```

- The `_with_limits` variant is the real implementation
  (`OoxmlPackage::with_limits(reader, limits)?` then
  `pptx::extract_slides(&mut package)`); `path`/`from_reader` delegate with
  `OoxmlLimits::default()`. This matches the PPTX family (unlike the DOCX
  family there is no options struct — none is needed).
- `PptxSlideText` is added to the existing `pub use models::{…}` list
  (alphabetical position after `OutputWarning`).
- The CLI `Input` enum gains an `extract_pptx_slides` method modeled on
  `Input::extract_pptx_structured_text` (`File::open` for `Path`,
  `Cursor::new(bytes)` for `Stdin`, always passing `cli_limits().ooxml`), so
  stdin and the global `--max-*` limit flags work identically.

### 1.3 Shared slide-reference parse — `crates/oxdoc-core/src/parsers/pptx.rs`

Today `parse_slide_relation_ids(xml, path) -> Result<Extraction<Vec<String>>>`
walks `p:sldIdLst` in order, returns only namespaced `r:id` strings, warns
`ignored presentation slide without relationship id` for `p:sldId` elements
without one, and stops with `W001 malformed_xml` on malformed XML (partial
list). `relationship_id_value` matches only attributes whose key contains `:`
with local name `id` — the unprefixed `@id` is deliberately not touched.

Design: replace the internal return type with a richer struct; keep the
function's warning behavior and walk order byte-identical.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlideReference {
    /// 1-based position of the `p:sldId` element within `p:sldIdLst`,
    /// counting every `p:sldId` element (including ones without `r:id`,
    /// which are warned about and not returned).
    pub position: usize,
    /// Namespaced `r:id` attribute value.
    pub relation_id: String,
    /// Unprefixed `p:sldId/@id` attribute, parsed as `u32`; `None` when the
    /// attribute is absent or not parsable as an unsigned integer.
    pub slide_id: Option<u32>,
}

fn parse_slide_references(
    xml: &str,
    path: &str,
) -> Result<Extraction<Vec<SlideReference>>>
```

- The `p:sldId` element handler becomes: increment a `position` counter for
  every `p:sldId` start/empty element; if `relationship_id_value` yields an
  `r:id`, push `SlideReference { position, relation_id, slide_id:
  slide_id_value(&element) }`; else emit the existing
  `ignored presentation slide without relationship id` warning (same wording,
  same path) and continue.
- New helper, deliberately separate from `relationship_id_value` so `r:id`
  matching cannot regress:

```rust
fn slide_id_value(element: &BytesStart<'_>) -> Option<u32> {
    element
        .attributes()
        .with_checks(false)
        .flatten()
        .find(|attr| {
            let key = attr.key.as_ref();
            !key.contains(':') && key == "id"
        })
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
        .and_then(|value| value.trim().parse::<u32>().ok())
}
```

  Absent `@id` and unparsable `@id` both yield `None`; the slides contract
  omits `slide_id` and emits **no** warning (spec: "No warning is introduced
  for a `p:sldId` without `@id` on any path"). Existing paths never read
  `slide_id`, so they are unaffected by construction.

- Existing call sites adapt mechanically — output-neutral:

```rust
for slide_reference in slide_references.value {
    let relationship = presentation_rels
        .get(&slide_reference.relation_id)
        .ok_or_else(|| OxdocError::MissingPart(slide_reference.relation_id.clone()))?;
    // … unchanged …
}
```

  `extract_text` and `extract_structured_text` use only `relation_id`; their
  output, warnings, and error behavior are pinned by their existing tests and
  snapshots. Two dedicated regression tests (see §6 R8) pin the two refactor
  hazards: "`r:id` without `@id` still extracts in text mode" and "ordinals
  are never exposed to existing paths".

- The existing `parse_slide_relation_ids` unit test
  (`parses_slide_relationship_ids_in_presentation_order`) is updated to the new
  return type (asserting `position`/`relation_id`/`slide_id` for the two
  elements); this is the only edit to an existing test in the parser module.

## 2. Per-slide extraction loop — `pptx::extract_slides`

New `pub(crate) fn extract_slides<R: Read + Seek>(package: &mut
OoxmlPackage<R>) -> Result<Extraction<Vec<PptxSlideText>>>`:

1. **Main part.** `find_office_document_path(package, "ppt/presentation.xml")?`
   — a missing or unreadable `ppt/presentation.xml` stays a hard error (spec:
   nothing to enumerate). `parse_slide_references` on its XML keeps the
   existing recoverable behavior: malformed presentation XML → `W001` plus the
   partial reference list; extraction continues with whatever was parsed.
2. **Presentation rels.** `package.read_to_string(&presentation_rels_path)` is
   matched: `Ok(xml)` parses the map; `Err(OxdocError::MissingPart(_))` becomes
   an **empty relationship map with no warning** (spec; mirrors
   `read_notes_for_slide`'s existing silent-empty rule for slide rels); other
   errors propagate. Note this is deliberately *different* from
   `extract_text`/`extract_structured_text`, which propagate the missing rels
   as a hard error — the leniency is scoped to the slides loop.
3. **Per-reference loop** over `slide_references.value` in `p:sldIdLst` order:

   - `slide_ordinal = slide_reference.position` (1-based over all `p:sldId`
     elements; skipped slides leave gaps — asserted by a test).
   - `presentation_rels.get(&relation_id)` is `None` → warning
     `skipped PPTX slide {rid}: unknown relationship id` with warning path
     `ppt/presentation.xml` (`OutputWarning::new("ppt/presentation.xml",
     format!("skipped PPTX slide {rid}: unknown relationship id"))`); `continue`.
   - `resolve_relationship_target(parent_dir(&presentation_path),
     relationship, &presentation_rels_path)?` — a suspicious target
     (external target mode, escaping target, NUL) propagates
     `SuspiciousRelationshipTarget` as a **hard error** (never downgraded).
   - `read_text_part(package, &slide_path)`:
     - `Ok(slide)` → record text is `slide.value` (possibly `""` for a
       textless slide — the record is still emitted), slide warnings (incl.
       `W001 malformed_xml` for truncated-but-readable parts) merged.
     - `Err(OxdocError::MissingPart(_))` → warning
       `skipped related PPTX slide part {path}: missing part` with warning
       path = the absent part path; `continue` (slide skipped entirely).
     - Other errors (limits violations, IO) propagate as hard errors, parity
       with the text path.
   - **Notes resolution** via a new
     `read_notes_text_for_slides(package, &slide_path) ->
     Result<Extraction<Option<String>>>`:
     - slide `.rels` missing (`MissingPart`) → `None`, no warning (existing
       silent-empty rule);
     - no relationship with type ending `/notesSlide` → `None`, no warning;
     - notes relationships sorted by rel id (existing rule) and joined with
       `append_part_text`, so multiple notes parts concatenate deterministically;
     - `resolve_relationship_target` suspicious target → hard error (propagates);
     - `read_text_part` returns `Err(MissingPart(_))` → warning
       `skipped related PPTX notes part {path}: missing part` (path = absent
       notes part), result `None` (record still emitted with `notes` omitted);
     - readable part → `Some(joined)` (`Some("")` when the part has no text).
   - Push `PptxSlideText { slide_id: slide_reference.slide_id, slide_ordinal,
     slide_path: slide_path.clone(), text, notes }` — always, even when the
     slide is textless.
4. **Return** `Extraction::with_warnings(records, warnings)`. Ordering is
   deterministic: `p:sldIdLst` position; notes keep rel-id sort order.

The three spec-locked warning texts are produced exactly once each per
occurrence, with `{rid}`/`{path}` substituted and no extra text; malformed XML
inside readable parts continues to use the existing
`stopped after malformed XML: …` message via `OutputWarning::malformed_xml`
(W001), never the skip wording. A helper
`OutputWarning::skipped_missing_part(path, kind_label)`-style constructor is
**not** added; the three texts are built inline in `extract_slides` with
`format!` so the wording is visible at the single site that emits them
(keeps `models.rs` warning-code classification untouched — these messages fall
into the existing `Custom`/W999 bucket like the DOCX skip warnings do).

Resource-limit parity: the loop runs inside the same `OoxmlPackage<R>` the
lib wrapper built with caller limits; every part read goes through
`read_text_part` → `package.with_entry`, so `OoxmlLimits` behavior is
identical to the text path. No limits or security policy change (non-goal).

## 3. Schema contract

### 3.1 `schemas/v1/oxdoc-pptx-slides.schema.json` (new) + mirror

Draft 2020-12; `$id`
`https://github.com/spereyra-dev/oxdoc/schemas/v1/oxdoc-pptx-slides.schema.json`;
exact byte-identical copy at `docs/schemas/v1/oxdoc-pptx-slides.schema.json`
(`make docs-schemas-check` runs `diff -ru schemas/v1 docs/schemas/v1`, so the
mirror is automatically enforced). Structure:

```jsonc
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/spereyra-dev/oxdoc/schemas/v1/oxdoc-pptx-slides.schema.json",
  "title": "oxdoc PPTX slides JSON / JSONL output",
  "description": "Schema for `oxdoc extract slides --format json` payloads and `--format jsonl` records in oxdoc schema version 1. slide_ordinal is the 1-based position of the p:sldId element within p:sldIdLst; skipped slides leave gaps in the emitted ordinal sequence.",
  "type": "object",
  "oneOf": [
    { "$ref": "#/$defs/documentPayload" },
    { "$ref": "#/$defs/jsonlRecord" }
  ],
  "$defs": {
    "documentPayload": {
      "type": "object",
      "additionalProperties": false,
      "required": ["schema_version", "file", "document_type", "slides", "warnings"],
      "properties": {
        "schema_version": { "type": "integer", "const": 1 },
        "file": { "type": "string" },
        "document_type": { "type": "string", "const": "pptx" },
        "slides": { "type": "array", "items": { "$ref": "#/$defs/slide" } },
        "warnings": { "type": "array", "items": { "$ref": "#/$defs/warning" } }
      }
    },
    "jsonlRecord": {
      "type": "object",
      "additionalProperties": false,
      "required": ["schema_version", "file", "slide_ordinal", "slide_path", "text"],
      "properties": {
        "schema_version": { "type": "integer", "const": 1 },
        "file": { "type": "string" },
        "slide_id":     { "$ref": "#/$defs/slide/properties/slide_id" },
        "slide_ordinal":{ "$ref": "#/$defs/slide/properties/slide_ordinal" },
        "slide_path":   { "$ref": "#/$defs/slide/properties/slide_path" },
        "text":         { "$ref": "#/$defs/slide/properties/text" },
        "notes":        { "$ref": "#/$defs/slide/properties/notes" }
      }
    },
    "slide": {
      "type": "object",
      "additionalProperties": false,
      "required": ["slide_ordinal", "slide_path", "text"],
      "description": "One slide record. slide_id is omitted when p:sldId/@id is absent or unparsable; notes is omitted when the slide links no readable notes part; slide_ordinal keeps its p:sldIdLst position so skipped slides leave gaps.",
      "properties": {
        "slide_id":      { "type": "integer", "description": "p:sldId/@id (xsd:unsignedInt)." },
        "slide_ordinal": { "type": "integer", "minimum": 1 },
        "slide_path":    { "type": "string" },
        "text":          { "type": "string" },
        "notes":         { "type": "string" }
      }
    },
    "warning": { /* same shape as oxdoc-docx-tables.schema.json $defs/warning:
                    object, additionalProperties false, required
                    [category, code, path, message], all strings */ }
  }
}
```

Design decisions inside the schema:

- **`$defs.slide` sharing mechanics.** A blanket `"$ref": "#/$defs/slide"`
  cannot be used inside `jsonlRecord`: `slide` sets
  `additionalProperties: false` and does not declare `schema_version`/`file`,
  and `additionalProperties` is evaluated against properties declared in the
  *same* schema object, so an `allOf`-composed record would reject its own
  envelope fields. The record branch therefore `$ref`s the individual property
  schemas (`#/$defs/slide/properties/…`) — a single source of truth for field
  types, so JSON and JSONL cannot drift, while each branch keeps its own
  strict property set. Both branches and `$defs.slide` set
  `additionalProperties: false`.
- **`oneOf` discrimination.** A payload requires `slides`+`warnings` (absent
  from records); a record requires `slide_ordinal` (absent from payloads), so
  every valid instance matches exactly one branch.
- **Top-level `"type": "object"`** is declared (both branches are objects);
  top-level `additionalProperties` is intentionally *not* set (it would reject
  everything next to a bare `oneOf`).

### 3.2 Harness updates — `crates/oxdoc-core/tests/schema.rs`

- `SCHEMA_VERSIONS`: add `"oxdoc-pptx-slides.schema.json"` to the `v1` list
  (one line; keeps `assert_schema_metadata` coverage).
- `assert_schema_metadata` currently asserts a top-level
  `additionalProperties: false`. For this schema the strictness lives on the
  branches, so the assertion is made oneOf-aware: when the schema has a
  top-level `oneOf`, assert top-level `"type": "object"` and assert
  `additionalProperties == false` **on every branch** (and keep the `$schema`
  / `$id` checks unchanged). This is the only edit to the shared harness and
  does not weaken any existing schema's checks.
- `validate_object` reads top-level `required`/`properties`, which a oneOf
  schema does not have. Extract its body into
  `validate_against(schema_branch, object)` and make `validate_object`
  delegate; add `validate_slides_object(schema, output)` that picks the branch
  whose `required` fields are all present and validates against it. The
  harness still only enforces declared fields/types, so the tests add local
  assertions for `schema_version == 1` and `document_type == "pptx"` (same
  pattern as the structured-text tests).

### 3.3 Validation tests

- Representative JSON payload: validate the committed
  `cli_pptx_slides_json.json` snapshot (see §6 R11) against the new schema.
- Representative JSONL record: validate the first line of
  `cli_pptx_slides_jsonl.jsonl` (parsed per line), plus an inline record with
  `slide_id` omitted to pin optionality.
- Negative test (spec-mandated): the slides JSON snapshot must FAIL
  `schemas/v2/oxdoc-structured-text.schema.json` and
  `schemas/v1/oxdoc-structured-text.schema.json`, using the existing
  `catch_unwind` pattern from `v2_payload_fails_frozen_v1_validation`
  (`validate_object` panics on the undeclared `slides` field).
- No other schema file under `schemas/v1/**`, `schemas/v2/**`, or their
  mirrors is touched; `docs/json-output.md` gains the table row
  `| oxdoc extract slides --format json | schemas/v1/oxdoc-pptx-slides.schema.json |`
  and a JSONL line, plus the "new contract, not a structured-text widen"
  version-policy note.

## 4. CLI — `oxdoc extract slides`

### 4.1 Arg parsing — `crates/oxdoc-cli/src/main.rs`

```rust
/// Extract PPTX slides as slide-scoped JSON or JSONL records
Slides {
    file: PathBuf,
    #[arg(long, value_enum, default_value_t = SlidesFormat::Json)]
    format: SlidesFormat,
},
```

```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
enum SlidesFormat { Json, Jsonl }
```

Shape matches `ExtractCommand::Tables`/`Rows`: single positional file, no
`--output`/`--output-dir`, no multi-input (deferred per non-goals). `json` is
the default (spec scenario). Dispatch in `run()`:
`ExtractCommand::Slides { file, format } => extract_slides_command(&file,
format, warning_format)?`.

### 4.2 `extract_slides_command`

```rust
fn extract_slides_command(
    file: &Path,
    format: SlidesFormat,
    warning_format: WarningFormat,
) -> Result<(), CliError>
```

1. `let input = read_input(file)?;` — reuses the existing stdin (`-`)
   buffering, `--max-input-size` enforcement, and `InputTooLarge` handling.
2. **Document-type gate (pptx-only):**

```rust
match document_type_for_input(&input, file)? {
    DocumentType::Pptx => {}
    DocumentType::Docx | DocumentType::Unknown => {
        return Err(CliError::InvalidArgument(
            "cannot extract slides from a DOCX document".to_owned(),
        ));
    }
    DocumentType::Xlsx => {
        return Err(CliError::InvalidArgument(
            "cannot extract slides from an XLSX workbook".to_owned(),
        ));
    }
}
```

   Same style as `extract_docx_tables`' PPTX/XLSX rejections
   (`CliError::InvalidArgument`, code `E010`, exit 1).
3. `let extraction = input.extract_pptx_slides().map_err(CliError::Core)?;`
   (new `Input` method; hard errors — missing presentation part,
   suspicious target — propagate to `main`'s `error[…]` + exit 1 path).
4. **File label.** The spec pins the label as `-` for stdin ("the input file
   label, `-` for stdin"; JSONL scenario repeats it). This differs from the
   repo-wide `display_file_name` precedent (`<stdin>`), so the command uses a
   slides-specific helper:

```rust
fn slides_file_label(file: &Path) -> String {
    if file == Path::new("-") {
        "-".to_owned()
    } else {
        display_file_name(file)
    }
}
```

   ⚠ **Callout for parent/apply:** this is a conscious, spec-mandated
   divergence from the `<stdin>` label every other command emits; it is
   snapshot-locked by the stdin tests. If the parent prefers `<stdin>`
   consistency, the spec scenario (not the code) is what must change — surface
   it before apply rather than silently choosing.
5. **JSON branch.** Emit stderr warnings first (subject to global
   `--warnings`/`--quiet`; suppression affects only the stderr mirror, never
   the embedded array), then serialize:

```rust
#[derive(Debug, serde::Serialize)]
struct SlidesPayload<'a> {
    schema_version: u8,
    file: &'a str,
    document_type: &'static str, // always "pptx"
    slides: &'a [PptxSlideText],
    warnings: Vec<OwnedWarningPayload>, // reused; [] when empty, never omitted
}
```

   `serde_json::to_writer_pretty(io::stdout().lock(), &payload)?` + trailing
   newline (`println!`), exactly like `extract_tables_command`. `PptxSlideText`'s
   serde attributes produce `slide_id`/`notes` omission and `"text": ""` for
   textless slides with no extra CLI mapping. All-slides-skipped decks emit
   `"slides": []` + warnings and return `Ok(())` (exit 0) — no
   "no input files were processed successfully" error, because the document
   was processed.
6. **JSONL branch.** Stream one compact record per slide, in the same
   `p:sldIdLst` order as the JSON `slides` array:

```rust
#[derive(Debug, serde::Serialize)]
struct SlidesJsonlRecord<'a> {
    schema_version: u8,
    file: &'a str,
    #[serde(flatten)]
    slide: &'a PptxSlideText,
}
```

   `serde_json::to_writer(&mut stdout.lock(), &record)?` + `\n` per slide
   (rows-jsonl precedent: write, then flush, then `emit_warnings` **after**
   the stream — mirroring `extract_rows_command`, which emits warnings only
   after `writer.flush()` so stdout stays a pure record stream; warnings go to
   stderr only and there is no per-line error contract). Skipped slides are
   simply absent; consumers detect them via stderr.
7. **Exit codes:** success (including all-skipped) `0`; hard errors and
   invalid-argument rejections exit 1 through the existing `main` handler. No
   new `CliError` variant.

Reused helpers: `read_input`, `document_type_for_input`,
`emit_warnings`, `OwnedWarningPayload::from_output_warning`,
`display_file_name`. No new dependency; no changes to any existing subcommand.

## 5. Fixtures, provenance, snapshots

### 5.1 Reused corpora (unmodified)

- `tests/fixtures/corpus/pptx/basic/` — one slide, no notes → no-notes case.
- `tests/fixtures/corpus/pptx/text/` — `sldIdLst` order `rId2` (`@id 256`,
  slide2.xml, notes via `slide2.xml.rels` → `notesSlide2.xml`), then `rId1`
  (`@id 257`, slide1.xml) → identity/order case **and** notes case. Verified:
  presentation.xml, rels, slide2.xml, and notesSlide2.xml carry exactly the
  `@id` values the spec scenarios quote.

### 5.2 New corpus: `tests/fixtures/corpus/pptx/missing-target/`

Four slides so all three spec skip scenarios plus an intact control are
fixture-driven (spec requires dangling-rel, missing-slide-part, missing-notes
each exercised "through the fixtures, not only synthetic inline XML"):

```
[Content_Types].xml                 # overrides for presentation, slide1, slide3
_rels/.rels
ppt/presentation.xml                # sldIdLst: id=256 r:id=rId101
                                    #          id=257 r:id=rId999  (dangling)
                                    #          id=258 r:id=rId102  (absent part)
                                    #          id=259 r:id=rId103
ppt/_rels/presentation.xml.rels     # rId101 -> slides/slide1.xml
                                    # rId102 -> slides/absent.xml
                                    # rId103 -> slides/slide3.xml
                                    # (rId999 intentionally absent)
ppt/slides/slide1.xml               # intact, text "Intact Slide"
ppt/slides/slide3.xml               # intact, text "Notes Slide"
ppt/slides/_rels/slide3.xml.rels    # notesSlide rel -> ../notesSlides/notesSlide3.xml
```

Expected slides-contract behavior: emitted ordinals `1` (slide1) and `4`
(slide3, `notes` omitted) — gap `2,3`; warnings `skipped PPTX slide rId999:
unknown relationship id` (path `ppt/presentation.xml`),
`skipped related PPTX slide part ppt/slides/absent.xml: missing part` (path
= absent part), `skipped related PPTX notes part
ppt/notesSlides/notesSlide3.xml: missing part`. Expected old-path behavior:
`extract_pptx_text` / `extract_pptx_structured_text` hard-fail with
`MissingPart` (on `rId999`) — the same package pins both contracts.
Content types declare overrides only for parts that exist.

### 5.3 New corpus: `tests/fixtures/corpus/pptx/malformed-xml/`

```
[Content_Types].xml
_rels/.rels
ppt/presentation.xml                # sldIdLst: id=256 r:id=rId201, id=257 r:id=rId202
ppt/_rels/presentation.xml.rels     # rId201 -> slides/slide1.xml
                                    # rId202 -> slides/slide2.xml
ppt/slides/slide1.xml               # well-formed, text "Before Truncation"
ppt/slides/slide2.xml               # truncated mid-XML: opens p:sld/cSld/spTree/
                                    # sp/txBody/a:p/a:r/a:t with "Partial Text"
                                    # then cuts off mid-tag (no closing tags)
```

Expected: record 1 intact; record 2 emitted with recovered `"Partial Text\n"`
plus the existing `W001` (`stopped after malformed XML: …`, path =
`ppt/slides/slide2.xml`); ordinals `1,2` (no skip, no gap). This promotes the
inline `create_ooxml`-style malformed fixture into a durable corpus package;
the existing inline unit test stays as-is.

Both trees are hand-authored XML zipped at runtime by
`fixtures::build_package` (`tests/fixtures/mod.rs`); no binary checked in.

### 5.4 Provenance notes

`tests/fixtures/provenance/pptx-missing-target.md` and
`pptx-malformed-xml.md`, following `pptx-text.md`'s label set (`Source:`,
`Producer:`, `Redistribution:`, `Purpose:`, `Archive generation:`; no
`Sanitization:` needed — nothing was sanitized, stated like the existing
notes). Both names are added to `fixture_provenance_notes_are_present` in
`crates/oxdoc-core/tests/api.rs` **and** `crates/oxdoc-cli/tests/cli.rs`.

### 5.5 Compatibility corpus gate

Verified against `scripts/check-compatibility-corpus.py`: the manifest only
tracks checked-in binaries under `tests/fixtures/files/` with `sha256`
digests. Runtime-zipped corpus trees carry no manifest entry, so
`make compatibility-corpus-check` is untouched — no new entry, no digest
change, no existing tree edited.

### 5.6 Snapshots

- `tests/fixtures/snapshots/cli_pptx_slides_json.json` — full pretty payload
  from `corpus/pptx/text` (slide2 first with notes, slide1 second without).
- `tests/fixtures/snapshots/cli_pptx_slides_jsonl.jsonl` — same two records,
  compact, one per line. Both byte-compared by CLI tests and validated by
  `schema.rs`.

Inline scenarios that would otherwise need extra corpora are built with the
zip dev-dependency (already available to both test crates through
`tests/fixtures/mod.rs`): `p:sldId` without `@id` (R1), textless slide (R4),
missing `ppt/_rels/presentation.xml.rels` (R5), all-slides-skipped deck (R9),
suspicious target, and missing presentation part (R7). No third corpus tree.

## 6. Test plan (strict TDD, per requirement)

Every requirement is driven test-first with `cargo test`; production code for
each requirement lands in the same work unit as its tests (coverage-gate
discipline). Existing tests are edited only where §1.3/§3.2 state.

| Req | Tests (new unless noted) |
| --- | --- |
| R1 slide identity | api: `corpus/pptx/text` → `slide_id` `256` then `257` in `sldIdLst` order. Inline package with an `@id`-less `p:sldId` (valid `r:id`) → record with `slide_id` key absent, no warning, and **text/structured extraction of the same package unchanged** (assert both old outputs). parser unit: `slide_id_value` absent/unparsable/valid; updated `parse_slide_references` order test. |
| R2 ordinal | api: text corpus → ordinals `1,2` with `slide_path` `ppt/slides/slide2.xml` first (inverted file order); missing-target → emitted ordinals `1,4` (gap asserted, not renumbered). Schema description + docs assert the wording (R11/R14). |
| R3 slide_path | api: for the text corpus, each record's `slide_path` equals the `part_path` the structured path reports for that slide's text. |
| R4 record shape | api: text corpus → notes-bearing record has `notes` string, basic corpus record has no `notes` key, both non-empty `text`. Inline textless slide → record with `"text": ""`, not dropped. serde unit: serialization omits `slide_id`/`notes` keys (never `null`). |
| R5 skip-with-warning | api (missing-target fixture): exact warning strings `skipped PPTX slide rId999: unknown relationship id`, `skipped related PPTX slide part ppt/slides/absent.xml: missing part`, `skipped related PPTX notes part ppt/notesSlides/notesSlide3.xml: missing part`; intact slides extracted; extraction returns `Ok`. api (inline): missing presentation rels → every `r:id` produces only the unknown-relationship-id warning, no missing-rels warning, empty record set, `Ok`. |
| R6 malformed XML | api (malformed-xml fixture): truncated slide emitted with partial text + one `W001`; first slide unaffected; ordinals `1,2`. Presentation-malformed `W001` + partial list already unit-covered; slides loop test via inline malformed presentation asserting continuation. |
| R7 security / hard errors | api (inline, mirrors existing pptx suspicious-target test): slide rel with escaping target → slides path returns `SuspiciousRelationshipTarget` (hard), no records. api (inline): package without `ppt/presentation.xml` → hard error. |
| R8 byte-identity | api: `extract_pptx_text` **and** `extract_pptx_structured_text` on the missing-target fixture both return `Err(MissingPart)` (leniency scoping regression). api: `r:id` without `@id` still extracts via text mode (inline package). Full existing snapshot set must pass untouched (`pptx_text.txt`, `cli_structured_text_pptx_json.json`, tables/rows/info/audit snapshots) — CI-wide guard, no new snapshots for old paths. |
| R9 JSON contract | cli: default format is JSON (no `--format`); payload keys `schema_version=1, file, document_type="pptx", slides, warnings=[]` on text corpus; byte snapshot compare. cli (inline all-skipped package): exit code 0, `"slides": []`, warnings embedded; embedded warnings survive `--quiet` while the stderr mirror is silenced (assert stderr empty + payload warnings present). |
| R10 JSONL contract | cli: one compact record per line, same order/values as JSON `slides`; missing-target via `--format jsonl` → stdout parses line-by-line with no warning text on stdout, warnings on stderr, ordinal gap visible; stdin `-` → records emitted, `file` label `-`; `--warnings json` renders stderr lines as JSON warning payloads. |
| R11 schema | schema.rs: `SCHEMA_VERSIONS` registration (metadata incl. oneOf-aware branch strictness); JSON snapshot validates; JSONL first line + `slide_id`-less record validate; negative tests vs structured-text v1+v2; mirror identity enforced by `make docs-schemas-check` (CI). |
| R12 fixtures/provenance | api + cli `fixture_provenance_notes_are_present` extended; `make compatibility-corpus-check` green (no manifest change). |
| R13 warning texts | Covered exactly by the R5/R6 assertions (byte-stable strings, warning paths as specified). |
| R14 docs | `docs/formats/pptx.md` "Slide-scoped JSON / JSONL" section; `docs/json-output.md` table rows + semantics + version-policy note; `docs/cli.md` usage + ordinal/skip semantics; `README.md` example; `CHANGELOG.md`. `make docs-links` guards link rot. |

Strict-TDD sequencing within each work unit: write the failing test(s) for the
requirement slice, then the production code, `cargo test -p <crate>` green,
then the next slice; `make ci` before each PR.

## 7. Work-unit split (4 chained PRs, stacked to main)

Honest sizing applies the 1.5–2× multiplier to test/fixture weight (post-#177
lesson). Realized lines are re-measured at each PR; two units sit at the
budget edge and the apply phase pauses (`ask-on-risk`) before exceeding 400
realized lines rather than silently shipping an oversized diff.

**WU1 — fixtures, provenance, and output-neutral `@id` parse** (~240–300 lines)
- Content: the two corpus trees (§5.2–5.3), two provenance notes, provenance
  list updates (api.rs + cli.rs), `SlideReference`/`slide_id_value` parser
  refactor with the two existing call sites adapted (§1.3), updated parser
  unit test, and the two refactor-hazard regression tests (`r:id` without
  `@id` still extracts in text mode; text/structured outputs unchanged on a
  package with `@id`-less slides). No user-visible feature yet — every test
  green, all existing snapshots untouched.
- Commits: ① corpora + provenance + list tests; ② parser refactor +
  regression tests.
- Rationale for pulling the refactor into WU1: it keeps WU2 under the budget
  and gives the riskiest shared-code change its own review slice pinned by
  byte-identity tests before any new feature exists.

**WU2 — core per-slide API** (~360–400 lines, at the edge)
- Content: `PptxSlideText` model + serde unit test; `pptx::extract_slides`
  loop + `read_notes_text_for_slides` (§2); three lib entry points +
  re-export; api tests for R1–R8 (order/ids, notes/no-notes, textless,
  skip-with-warning ×3, gaps, malformed partial, suspicious hard error,
  missing presentation, stdin/reader wrappers, old-paths-`MissingPart`
  regression).
- Commits: ① model + lib wrappers + happy-path tests (text corpus + basic
  corpus); ② skip-with-warning loop + missing-target/malformed-xml tests;
  ③ security/hard-error + wrapper tests.
- Fallback if realized > 400: pause and ask — recommended split is moving the
  ③ tests into a follow-up stacked PR (5th unit) rather than relaxing
  coverage.

**WU3 — schema contract** (~330–380 lines)
- Content: `schemas/v1/oxdoc-pptx-slides.schema.json` + docs mirror,
  `schema.rs` registration + oneOf-aware metadata assertion +
  `validate_slides_object` + positive/negative validation tests, the two
  snapshots (generated from WU2 output through the real CLI? No — the CLI
  lands in WU4; snapshots in WU3 are generated by invoking the payload
  serialization path in a temporary test that is replaced/kept minimal, or
  more honestly: **snapshots move to WU4** if the maintainer prefers
  real-CLI provenance).
- Decision recorded here: keep the two snapshot *files* and their schema
  validation in WU3 by generating them from a temporary test that serializes
  `SlidesPayload`-equivalent output via `serde_json` against WU2's core
  output; WU4's CLI tests then byte-compare against these same files. If the
  maintainer rejects "snapshot before CLI", move both files to WU4 (~40 lines)
  and WU3 drops to ~300.
- Commits: ① schema + mirror + registration; ② validation tests +
  `docs/json-output.md` table row + version-policy note.

**WU4 — CLI subcommand + docs** (~400–460 lines, at the edge)
- Content: `ExtractCommand::Slides`, `SlidesFormat`, `extract_slides_command`,
  payload/record structs, `Input::extract_pptx_slides`, `slides_file_label`
  (§4); cli tests for R9/R10 (default format, payload shape, all-skipped,
  quiet interplay, JSONL stream purity, stdin, type rejections, snapshot byte
  compares); docs (`docs/cli.md`, `docs/formats/pptx.md`, `README.md`,
  `CHANGELOG.md`; `docs/json-output.md` already partly done in WU3).
- Commits: ① subcommand + CLI tests; ② docs + changelog.
- Fallback if realized > 400: pause and ask — recommended split is docs into
  a 5th unit (docs-only PRs review quickly) or accepting that CLI code +
  tests stay together and only the four doc files move.

Dependency order: WU1 → WU2 → WU3 → WU4; each is independently reviewable and
`make ci` green. Coverage stays ≥ 95% at every stack point because tests ride
with their production code in WU1/WU2 and WU3/WU4 add only declarative/emission
surface already exercised by tests in the same unit.

## 8. Rollback and risks

### Rollback

Each unit reverts independently: WU1 restores the `r:id`-only helper and
deletes the two corpus trees/notes (no existing fixture or digest touched);
WU2 deletes the model, loop, and lib wrappers (no existing signature
changed); WU3 deletes the schema, mirror, and harness additions (nothing
validated against it in the wild yet); WU4 deletes the subcommand and doc
edits. No data migration, no persisted state, no captured output becomes
invalid at any point.

### Risks

1. **Shared parser refactor leaking into existing outputs** (highest).
   Mitigated by construction (existing paths read only `relation_id`), pinned
   by the two WU1 regression tests, and guarded by the untouched existing
   snapshot suite. Residual risk is low but the WU1 review should focus on
   `relationship_id_value` remaining untouched.
2. **Review budget.** WU2/WU4 estimate 360–460 realistic lines. Mitigated by
   the intra-unit commit boundaries, the pre-agreed fallback splits, and the
   `ask-on-risk` pause before any oversized diff. No `size:exception` assumed.
3. **Skip-with-warning masking data loss.** Mitigated: JSON embeds `warnings`
   (never suppressed from the payload), JSONL documents the stderr channel,
   warning texts are spec-locked and byte-asserted, and the missing-target
   fixture asserts both skip paths end-to-end.
4. **Schema harness friction.** The oneOf layout requires the small
   `assert_schema_metadata`/`validate_object` refactor (§3.2); the negative
   tests use the established `catch_unwind` pattern. Risk is contained to
   `schema.rs` and covered by the existing nine schemas' tests still passing.
5. **`@id`-less slides and ordinal semantics ambiguity.** Ordinals count every
   `p:sldId` element; `@id`-less slides are emitted with `slide_id` omitted
   (not skipped — the spec overrode the proposal's earlier skip idea, and this
   design follows the spec). Docs and schema description state the gap rule.
6. **Stdin file label divergence** (`-` vs `<stdin>`): spec-mandated `-`;
   flagged in §4.2 for explicit parent awareness before apply so it is not
   discovered at review.
7. **Coverage gate.** All new parser branches (three skip paths, notes-missing,
   rels-missing, malformed continuation) are exercised through integration
   fixtures, not just unit tests, keeping `make coverage` ≥ 95%.

## 9. Explicitly out of scope (restated from proposal non-goals)

`--format structured-json` and the v2 schema; `TextBlock`/`StructuredText`/
DOCX model changes; plain-text/existing JSON byte changes; DOCX/XLSX paths and
`oxdoc-tabular`; Python wrapper; recalculation/rendering/thumbnails/media;
per-paragraph or per-shape granularity; `--output`/`--output-dir`/multi-input/
per-line error contract for `extract slides`; new dependencies; limits or
security-policy changes.
