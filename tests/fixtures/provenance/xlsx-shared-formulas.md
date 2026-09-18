# xlsx/shared-formulas

- Source: hand-authored OOXML shared-formula corpus, maintained in this repository.
- Producer: no Microsoft Office, LibreOffice, or Google Workspace output was used.
- Redistribution: permitted; all parts are repository-authored text fixtures.
- Purpose: XLSX shared-formula coverage — cached master plus cached and uncached slaves, a dangling `si` slave, slave-before-master ordering, first-registration-wins duplicate masters, an array master with a region cell holding only a cached value, and a namespace-prefixed worksheet — for the typed rows API.
- Sanitization: hand-authored; no third-party, private, or user data is embedded.
- Archive generation: tests zip the source tree at runtime; no `.xlsx` binary is checked in.
