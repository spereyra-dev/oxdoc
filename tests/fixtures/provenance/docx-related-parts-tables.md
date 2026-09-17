# docx/related-parts

- Source: hand-authored minimal OOXML package tree and JSON oracle, maintained in this repository.
- Producer: no Microsoft Office, LibreOffice, Google Workspace, or third-party document output was used.
- Redistribution: permitted; all content is repository-authored.
- Purpose: structural table API coverage for the main document, headers, footers, footnotes, endnotes, comments, and normalized part paths; the package's two sections order the footer before the header, proving section-order traversal.
- Archive generation: tests may ZIP the source tree deterministically; no `.docx` binary is checked in.
- Sanitization: not applicable; the fixture was created from synthetic text and contains no private document content.
