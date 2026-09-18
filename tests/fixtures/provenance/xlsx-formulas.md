# xlsx/formulas

- Source: hand-authored OOXML formula-provenance corpus, maintained in this repository.
- Producer: no Microsoft Office, LibreOffice, or Google Workspace output was used.
- Redistribution: permitted; all parts are repository-authored text fixtures.
- Purpose: XLSX formula provenance coverage — cached and uncached formulas, empty-but-cached values, error formulas, string-result formulas, shared-string formula cells, entity/CDATA/numeric-reference formula text, and a non-formula error control for the typed rows API.
- Sanitization: hand-authored; no third-party, private, or user data is embedded.
- Archive generation: tests zip the source tree at runtime; no `.xlsx` binary is checked in.
