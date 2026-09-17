# docx/section-order

- Source: hand-authored minimal OOXML package tree and JSON oracle, maintained in this repository.
- Producer: no Microsoft Office, LibreOffice, Google Workspace, python-docx, or third-party document output was used.
- Redistribution: permitted; all content is repository-authored.
- Purpose: section-order oracle for related header and footer parts. The package's two sections and deliberately scrambled relationship file pin section document order, headers before footers, `first`/`even`/`default` variant ranking, dedup by resolved path at first reference, orphans appended in relationship order, footnotes and comments after all headers and footers, the `titlePg` no-op, and the missing/unknown `r:id` warnings. All three extraction paths consume the same `expected.json`.
- Archive generation: tests may ZIP the source tree deterministically; no `.docx` binary is checked in.
- Sanitization: not applicable; the fixture was created from synthetic text and contains no private document content.
