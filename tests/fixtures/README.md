# Fixture Corpus

This directory holds the checked-in OOXML corpus used by the integration tests.

The corpus is intentionally minimal, redistributable, and split into two
fixture classes:

- Hand-authored package trees are authored in this repository and zipped by the
  test helper.
- Application-generated files are small checked-in `.docx`, `.xlsx`, and
  `.pptx` binaries generated from repository-authored content.
- No private, customer, or user documents belong in this directory.

Layout:

- `corpus/` contains the OOXML source trees that are zipped at test time.
- `files/` contains small application-generated OOXML binaries consumed as-is.
- `snapshots/` contains the versioned expected outputs.
- `provenance/` contains one note per fixture with source and redistribution status.
- `tools/` contains optional fixture generation scripts. CI does not require
  these tools.

Current fixtures:

- `corpus/docx/basic`
- `corpus/docx/external-target`
- `corpus/xlsx/basic`
- `corpus/xlsx/app-metadata`
- `corpus/pptx/basic`
- `corpus/pptx/text`
- `files/docx/python-docx-basic.docx`
- `files/xlsx/openpyxl-basic.xlsx`
- `files/pptx/python-pptx-basic.pptx`

Application-generated fixture matrix:

| Format | Fixture | Producer |
| --- | --- | --- |
| DOCX | `files/docx/python-docx-basic.docx` | python-docx 1.2.0 |
| XLSX | `files/xlsx/openpyxl-basic.xlsx` | openpyxl 3.1.5 |
| PPTX | `files/pptx/python-pptx-basic.pptx` | python-pptx 1.0.2 |

Additional Microsoft Office, LibreOffice, or Google Workspace exports can be
added when they are generated from repository-authored content and accompanied
by explicit provenance and redistribution notes.
