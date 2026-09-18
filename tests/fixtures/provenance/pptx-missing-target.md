# pptx/missing-target

- Source: hand-authored OOXML presentation fixture, maintained in this repository.
- Producer: no Microsoft Office, LibreOffice, or Google Workspace output was used.
- Redistribution: permitted; all parts are repository-authored text fixtures.
- Purpose: slide extraction skip-with-warning coverage for a dangling slide
  relationship id, an absent slide part target, and an absent notes part
  target, while keeping two intact slides for control assertions.
- Archive generation: tests zip the source tree at runtime; no `.pptx` binary is checked in.
