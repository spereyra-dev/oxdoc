# OOXML Compatibility Playground

Evaluate `oxdoc` input/output behavior using the small, checked-in fixtures that power its integration tests. This is a compatibility reference, not an upload service or a document renderer.

Every result below is generated from a versioned expected-output snapshot. Run `python3 scripts/compatibility-playground.py --check` to verify that this page has not drifted from its fixtures.
The hand-authored package trees are zipped by the integration-test helper; the commands below show the corresponding CLI invocation after that deterministic packaging step.

## Supported behavior

| Format | Feature | Fixture | Expected output |
| --- | --- | --- | --- |
| DOCX | Paragraph and table text | `tests/fixtures/corpus/docx/basic` | `tests/fixtures/snapshots/docx_basic_text.txt` |
| PPTX | Slide order, tabs, and speaker notes | `tests/fixtures/corpus/pptx/text` | `tests/fixtures/snapshots/pptx_text.txt` |
| XLSX | Shared strings and sparse rows | `tests/fixtures/corpus/xlsx/basic` | `tests/fixtures/snapshots/xlsx_basic_csv.txt` |

### DOCX: Paragraph and table text

**Supported:** Paragraph text is extracted; cells use tabs and rows use line breaks.

Fixture: `tests/fixtures/corpus/docx/basic`; provenance: `tests/fixtures/provenance/docx-basic.md`

```bash
oxdoc extract text fixture.docx
```

```text
Alpha	Beta
Gamma & Delta
```

### PPTX: Slide order, tabs, and speaker notes

**Supported:** Slide text remains in presentation order and linked speaker notes follow their slide.

Fixture: `tests/fixtures/corpus/pptx/text`; provenance: `tests/fixtures/provenance/pptx-text.md`

```bash
oxdoc extract text fixture.pptx
```

```text
First Slide
Alpha	Beta & Co
Gamma < Delta
Speaker note
Second Slide
```

### XLSX: Shared strings and sparse rows

**Supported:** Shared strings resolve and missing cells are emitted as empty CSV fields.

Fixture: `tests/fixtures/corpus/xlsx/basic`; provenance: `tests/fixtures/provenance/xlsx-basic.md`

```bash
oxdoc extract csv fixture.xlsx
```

```csv
id,Cliente A,monto
1,,5000
```

## Safety behavior and warnings

**Supported safety behavior:** external relationship targets are rejected as a hard error; `oxdoc` does not fetch them. The approved `corpus/docx/external-target` fixture produces:

```text
suspicious OOXML relationship target in _rels/.rels: https://example.invalid/document.xml: external relationship targets are not supported
```

Recoverable malformed XML can produce partial output plus a warning on stderr. See [Errors and Warnings](errors-and-warnings.md) for the stable error and warning contract.

## Non-goals

- Rendering Word pages or PowerPoint slides, including fonts, layout, and pagination.
- Calculating formulas or reproducing Excel's full display engine.
- Repairing malformed input, decrypting documents, or fetching external relationship targets.
- Accepting uploaded documents on this documentation site.

## Fixture and privacy policy

Examples are an explicit allowlist of hand-authored, minimal OOXML package trees. Their provenance notes state redistribution permission, and the [fixture corpus policy](https://github.com/spereyra-dev/oxdoc/blob/main/tests/fixtures/README.md) prohibits private, customer, and user documents. Do not add samples containing personal, proprietary, or sensitive data.

For complete format contracts, see [DOCX](formats/docx.md), [PPTX](formats/pptx.md), and [XLSX](formats/xlsx.md).
