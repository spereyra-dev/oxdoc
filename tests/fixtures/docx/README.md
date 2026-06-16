# DOCX Structural Fixtures

These fixtures are repository-authored inputs for a future structural DOCX
table API. They are kept as readable XML and JSON rather than checked-in Office
binaries.

- `table-semantics/` covers paragraphs, `gridSpan`, raw `vMerge` states,
  `gridBefore`, `gridAfter`, nested block order, and revisions.
- `related-parts/` is a complete minimal package tree with one table in each
  supported text-bearing part. Tests may ZIP the tree deterministically.
- `malformed-table/` fixes the partial-result boundary for malformed XML.

Each `expected.json` is a design oracle, not output from the current parser.
The corresponding redistribution and authorship records live in
`tests/fixtures/provenance/`.
