# OOXML Producer Compatibility Corpus

`oxdoc` maintains a small, public regression corpus for OOXML files produced
by different applications. It is a compatibility signal, not a claim that a
format or producer is fully supported. Each checked-in fixture has a
repository-authored input, a provenance note, an expected output snapshot, and
a SHA-256 digest in
[`tests/fixtures/compatibility-matrix.json`](../tests/fixtures/compatibility-matrix.json).

## Current Matrix

| Producer family | Producer | Format | Exercised capabilities | Status |
| --- | --- | --- | --- | --- |
| Python OOXML library | python-docx 1.2.0 | DOCX | body, table, header, footer, metadata text | Covered |
| Python OOXML library | openpyxl 3.1.5 | XLSX | strings, numeric and boolean cells, CSV quoting, hidden sheets, metadata | Covered |
| Python OOXML library | python-pptx 1.0.2 | PPTX | slide, paragraph, bullet, and metadata text | Covered |
| Microsoft Office | — | DOCX/XLSX/PPTX | — | Planned; no Office sample is checked in |
| LibreOffice | — | DOCX/XLSX/PPTX | — | Planned; no LibreOffice sample is checked in |
| Google Workspace | — | DOCX/XLSX/PPTX exports | — | Planned; no Workspace export is checked in |

Planned rows are intentionally **not** compatibility claims. A producer moves
to Covered only when a redistributable fixture and an exercised regression
assertion are checked in.

## Adding a Fixture

1. Create the document specifically for this repository using synthetic,
   non-sensitive content. Do not sanitize a customer or personal document for
   submission; recreate the minimal behavior instead.
2. Minimize the package and remove macros, external links, embedded media,
   comments, tracked changes, custom XML, document properties, and any data not
   needed for the regression. Verify the archive before committing it.
3. Add the fixture, its expected snapshot, and a note under
   `tests/fixtures/provenance/` that states Source, Producer (including
   version), Redistribution, Purpose, Sanitization, and SHA-256.
4. Add a record to `tests/fixtures/compatibility-matrix.json`. Include the
   exact producer/version, capabilities covered, paths, and SHA-256 digest.
5. Add or extend a core or CLI integration test that consumes the fixture and
   asserts the snapshot. Then run:

   ```bash
   python3 scripts/check-compatibility-corpus.py
   cargo test --workspace --all-features --all-targets
   ```

## Licensing and Safety Policy

Only fixtures whose contents and redistribution rights are known may enter the
repository. Repository-authored synthetic documents are preferred. A
contributor must not submit customer, employer, confidential, personal,
licensed-template, or third-party documents, even after redaction. When a
third-party sample is essential, link to its license and written redistribution
permission in the provenance note before review; maintainers may decline it.

Never include credentials, names, email addresses, document history, revision
metadata, hidden worksheets/slides, comments, embedded objects, macros, or
external relationship targets. Fixture review includes inspecting the unpacked
OOXML package and metadata as well as the visible text.

## Automated Guardrails

The compatibility-corpus check runs in CI. It verifies manifest fields,
fixture/snapshot/provenance paths, provenance labels, and fixture SHA-256
digests. Parser integration tests consume the listed fixtures and compare their
outputs to versioned snapshots. The check is deliberately standard-library-only
so external contributors can run it without installing producer software.
