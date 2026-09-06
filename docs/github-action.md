# GitHub Action

Use the official `oxdoc` setup action to install a release binary in a GitHub
Actions job. The `version` input is required: pin it to an exact release tag.
The action downloads the platform archive from GitHub Releases and verifies it
against that release's `SHA256SUMS` file before adding `oxdoc` to `PATH`.

```yaml
- uses: spereyra-dev/oxdoc@<commit-sha>
  with:
    version: v0.1.0
- run: oxdoc --version
```

Replace `<commit-sha>` with the full commit SHA for the action release you
intend to trust. Do not use floating release tags or `latest` for the action
reference or the `version` input.

The action supports GitHub-hosted Linux, macOS, and Windows runners. Its
`version` and `path` outputs contain the installed release tag and the
directory added to `PATH`.

## Examples

Check out the files to inspect before invoking `oxdoc`:

```yaml
- uses: actions/checkout@v7
- uses: spereyra-dev/oxdoc@<commit-sha>
  with:
    version: v0.1.0
```

### Extract DOCX text

```yaml
- name: Extract text
  run: oxdoc extract text input/report.docx > output/report.txt

- uses: actions/upload-artifact@v7
  with:
    name: report-text
    path: output/report.txt
```

### Convert XLSX to CSV

```yaml
- name: Convert workbook
  run: oxdoc extract csv input/workbook.xlsx --sheet "Sales" > output/sales.csv

- uses: actions/upload-artifact@v7
  with:
    name: sales-csv
    path: output/sales.csv
```

### Save audit JSON

```yaml
- name: Audit document
  run: oxdoc audit input/report.docx --format json > output/audit.json

- uses: actions/upload-artifact@v7
  with:
    name: document-audit
    path: output/audit.json
```

`oxdoc` writes extracted text, CSV, and JSON to stdout, so redirect stdout to
the file you intend to upload. Warnings and errors remain on stderr and appear
in the workflow log. The setup action does not create or upload artifacts; use
`actions/upload-artifact` explicitly for each output you want to retain.
