# Integration Recipes

Ready-to-use workflows for wiring `oxdoc` into the automation you actually run:
shell pipelines, GitHub Actions, Python/pandas orchestration, backend ingestion
under resource limits, and audit triage.

How to read these recipes:

- Recipes are **workflows**, not flag references. [`cli.md`](cli.md) is the flag
  authority; every recipe links the page that owns the flags and fields it uses.
- Commands were verified against the current CLI surface at the time of writing.
  `oxdoc` is deterministic and text-safe: it never renders, paginates, or
  produces PDFs.
- Every expected-output block is labeled: either **example output**, transcribed
  from or consistent with the current docs, or **illustrative**, where the docs
  have no sample.
- Each recipe carries a short constraints blurb stating only guarantees already
  documented in the repository docs.

Every recipe uses the same five-part shape:

- **Goal** — one sentence, the workflow outcome.
- **Command** — a copy/paste-ready shell, YAML, or Python block.
- **Example output** — a fenced block labeled as example or illustrative output.
- **Constraints** — the applicable documented safety and non-rendering guarantees.
- **See also** — the reference page(s) that own the flags and fields.

## Recipe A - Shell pipelines

Batch-extract text, CSV, typed rows, and slides from Office files into JSON and
JSONL streams that a shell pipeline can consume.

**Command:**

```bash
oxdoc extract text inbox/*.docx --format jsonl > extracted-text.jsonl
```

```bash
cat contract.docx | oxdoc extract text - --format json > contract.json
```

```bash
oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter "," > sales.csv
```

```bash
oxdoc extract rows data.xlsx --sheet "Sales Q1" --value-mode formatted > rows.jsonl
```

```bash
oxdoc extract slides deck.pptx --format jsonl > slides.jsonl
```

Exit code `0` means success (recoverable warnings may still appear on stderr),
`1` is a hard runtime failure that prints `error[<code>]: <message>` to stderr,
and `2` is a CLI usage error from Clap:

```bash
if oxdoc --warnings json extract text inbox/*.docx --format jsonl > extracted-text.jsonl 2> warnings.jsonl; then
  echo "extraction finished; stderr warnings: $(wc -l < warnings.jsonl)"
else
  echo "hard failure; inspect warnings.jsonl and stderr" >&2
  exit 1
fi
```

Example output — one JSONL line per record, transcribed from
[`cli.md`](cli.md) and [`json-output.md`](json-output.md):

```jsonl
{"file":"contrato.docx","document_type":"docx","text":"Plain text..."}
{"schema_version":2,"file":"workbook.xlsx","sheet_name":"Sales Q1","row_index":2,"cells":[{"column_index":0,"kind":"string","raw":"Widget","value":"Widget","has_formula":false},{"column_index":2,"kind":"number","raw":"42.50","has_formula":true,"formula":"B2*2","formula_cached":true}]}
{"schema_version":1,"file":"deck.pptx","slide_id":256,"slide_ordinal":1,"slide_path":"ppt/slides/slide2.xml","text":"First Slide\n","notes":"Speaker note\n"}
```

Example output — a recoverable warning on stderr, transcribed from
[`json-output.md`](json-output.md):

```text
warning[parser/W001]: word/document.xml: stopped after malformed XML: ...
```

**Constraints:** warnings go to stderr as
`warning[<category>/<code>]: <path>: <message>` and never contaminate the JSONL
stdout stream (JSONL text records also embed them as a `warnings` field); JSONL
batch extraction continues after per-file failures; `--value-mode formatted` is
deterministic and locale-independent; `oxdoc` never renders, paginates, or
produces PDFs.

**See also:** [`cli.md`](cli.md), [`json-output.md`](json-output.md),
[`errors-and-warnings.md`](errors-and-warnings.md).

## Recipe B - GitHub Actions

Run `oxdoc` extraction and audit in a GitHub Actions job via the official setup
action, publishing outputs as artifacts and a job summary.

This generalizes the repository's own self-use pattern: its `action` workflow
(`.github/workflows/action.yml`) validates the action on ubuntu, macOS, and
windows runners with `uses: ./` and `version: v0.1.0`, asserting the `version`
and `path` outputs; `ci.yml` builds from source and does not invoke the action.

**Command:**

```yaml
jobs:
  extract-and-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: spereyra-dev/oxdoc@<commit-sha>
        id: oxdoc
        with:
          version: v0.1.0
      - name: Extract document text
        run: oxdoc extract text input/report.docx > output/report.txt
      - name: Audit document
        run: oxdoc audit input/report.docx --format json > output/audit.json
      - name: Publish job summary
        if: always()
        run: jq -r '.signals[] | "- \(.severity) \(.kind): \(.message)"' output/audit.json >> "$GITHUB_STEP_SUMMARY"
      - uses: actions/upload-artifact@v7
        if: always()
        with:
          name: oxdoc-outputs
          path: |
            output/report.txt
            output/audit.json
```

Replace `<commit-sha>` with the full commit SHA for the action release you
intend to trust, exactly as [`github-action.md`](github-action.md) documents.

Illustrative output — one job-summary line derived from the audit JSON signal
shape documented in [`audit.md`](audit.md):

```text
- warning hidden_sheet: worksheet 'Model Inputs' is hidden
```

**Constraints:** extraction and audit JSON go to stdout, so redirect stdout to
the file you keep; warnings and errors remain on stderr and appear in the
workflow log. `oxdoc` never renders or paginates. Pin the action by commit SHA
and `version` to an exact `vX.Y.Z` tag; the setup action checksum-verifies the
download before adding `oxdoc` to `PATH`. If `jq` is unavailable the summary
step fails, but `if: always()` keeps the artifacts publishing.

**See also:** [`github-action.md`](github-action.md), [`cli.md`](cli.md).

## Recipe C - Python pandas orchestration

Drive `oxdoc` from Python and turn XLSX content into pandas DataFrames inside
notebooks, Airflow, or Dagster jobs.

**Command:** the `oxdoc-python` wrapper and the `oxdoc` binary are installed
separately — the wrapper is pure Python and does **not** bundle the binary:

```bash
python -m pip install oxdoc-python
cargo install oxdoc-cli
```

Put `oxdoc` on `PATH`, or pass its location explicitly with
`Oxdoc(binary="./target/release/oxdoc")`. See
[`python-integration.md`](python-integration.md) for the other supported
install paths.

**Command:** build a `DataFrame` from typed rows, and CSV text via `StringIO`
(API verified against `python/src/oxdoc/client.py`):

```python
import io

import pandas as pd
from oxdoc import Oxdoc

client = Oxdoc()  # or Oxdoc(binary="./target/release/oxdoc")

rows = client.extract_rows("workbook.xlsx", sheet="Sales Q1").value

records = []
for row in rows:
    record = {"row_index": row["row_index"]}
    for cell in row["cells"]:
        record[str(cell["column_index"])] = cell.get("value", cell["raw"])
    records.append(record)

df = pd.DataFrame.from_records(records)

# Raw numbers arrive as JSON strings; convert cell columns explicitly.
for column in df.columns:
    if column != "row_index":
        numeric = pd.to_numeric(df[column], errors="coerce")
        df[column] = numeric.where(numeric.notna(), df[column])

csv_text = client.extract_csv("workbook.xlsx", sheet="Sales Q1").value
df_csv = pd.read_csv(io.StringIO(csv_text))
```

Illustrative output — minimal `df` repr sketch (two rows, `row_index` plus cell
columns with numbers converted from JSON strings):

```text
   row_index      0     2
0          0  Widget  42.5
1          1    Bolt  17.0
```

**Command:** JSONL batch extraction preserves per-file errors in the returned
records instead of raising, and `OxdocResult.warnings` carries the stderr lines:

```python
import sys

result = client.extract_text_records(["a.docx", "b.pptx"])

for record in result.value:
    if "error" in record:
        print(f"extraction failed: {record['file']}")
    else:
        print(f"extracted: {record['file']}")

for line in result.warnings:
    print(line, file=sys.stderr)
```

**Constraints:** the wrapper shells out to the CLI — no native bindings and no
bundled binary. Rows formula cells carry `formula` (the stored expression text,
never recalculated or rewritten) and `formula_cached`. Errors surface as
`OxdocNotFoundError`, `OxdocProcessError` (carrying `command`, `returncode`,
`stdout`, and `stderr`), or `OxdocJsonError`; warnings arrive as
`OxdocResult.warnings`, the tuple of non-empty stderr lines.

**See also:** [`python-integration.md`](python-integration.md),
[`json-output.md`](json-output.md).

## Recipe D - Backend ingestion

Accept untrusted uploads in a serverless or multi-tenant backend with hard
per-request memory limits and machine-readable failures.

**Command:** a single limited request. All four limits are global options with
the defaults shown; set `--max-input-size` to the request-body limit and the
package and part limits to one invocation's memory budget:

```bash
oxdoc extract text upload.docx --format json > record.json 2> warnings.jsonl \
  --max-input-size 67108864 \
  --max-package-uncompressed-size 268435456 \
  --max-part-size 67108864 \
  --max-compression-ratio 200 \
  --warnings json
```

**Command:** stdin intake with a 10 MiB body cap:

```bash
cat request-body | oxdoc extract text - --format json --max-input-size 10485760
```

**Command:** batch triage of an intake directory, plus a support bundle for
bug reports (`diagnostics` accepts no document path and reads no document
content):

```bash
oxdoc audit intake/*.docx --format jsonl > audit.jsonl
oxdoc diagnostics --format json
```

Example output — one machine-readable JSON warning record on stderr with
`--warnings json`, transcribed from [`cli.md`](cli.md):

```json
{"category":"parser","code":"W001","path":"word/document.xml","message":"stopped after malformed XML: ..."}
```

Example output — one failed-file record from a JSONL audit batch, shape
consistent with
[`schemas/v1/oxdoc-audit-jsonl.schema.json`](schemas/v1/oxdoc-audit-jsonl.schema.json);
failed files carry `document_type: "unknown"`:

```jsonl
{"schema_version":1,"file":"intake/broken.docx","document_type":"unknown","error":{"code":"E002","message":"not a readable ZIP/OOXML package"}}
```

Limit failures are typed hard failures: the CLI prints
`error[<code>]: <message>` to stderr and exits `1`. The limit codes are `E014`
(input-package size), `E011` (combined uncompressed package size), and the
existing `E005` (oversized part) and `E006` (suspicious ZIP entry) classes;
the full stable list `E001`–`E010` lives in
[`errors-and-warnings.md`](errors-and-warnings.md).

**Constraints:** stdin is deliberately buffered only up to
`--max-input-size`, because ZIP central-directory access requires a seekable
source; the CLI never creates temporary files or spills stdin to a temporary
directory (large XLSX workbooks keep their existing bounded library-internal
shared-string spill). Warnings go to stderr with `--warnings json` and never
contaminate the JSON/JSONL stdout stream. The defaults (64 MiB input / 256 MiB
uncompressed / 64 MiB part / ratio 200) are secure and preserve
ordinary-document compatibility. `oxdoc` never renders, paginates, or
produces PDFs.

**See also:** [`cli.md`](cli.md), [`json-output.md`](json-output.md),
[`security.md`](security.md), [`performance.md`](performance.md),
[`errors-and-warnings.md`](errors-and-warnings.md).

## Recipe E - Audit workflow

Triage a batch of untrusted documents with `oxdoc audit`, escalating
high-severity signals and routing failed files by stable error code.

**Command:** single-file JSON for one document, JSONL for a batch (JSONL
continues after per-file failures and exits `0`), and text for a quick human
read when `jq` is not available:

```bash
oxdoc audit report.docx --format json > audit.json
oxdoc audit intake/*.docx --format jsonl > audit.jsonl
oxdoc audit report.docx --format text
```

**Command:** triage step 1 — escalate high-severity signals from the
single-file audit JSON:

```bash
jq -r '.signals[] | select(.severity == "high") | [.kind, .path] | @tsv' audit.json
```

**Command:** triage step 2 — route failed files in the batch by their stable
`error.code` values (error records carry `document_type: "unknown"`):

```bash
jq -r 'select(.error) | [.file, .error.code] | @tsv' audit.jsonl
```

Example output — one audit JSON record, transcribed from the JSON Shape in
[`audit.md`](audit.md):

```json
{
  "oxdoc_version": "2.0.0",
  "file": "workbook.xlsx",
  "document_type": "xlsx",
  "metadata": {
    "file": "workbook.xlsx",
    "application": "Excel",
    "has_macros": false
  },
  "signals": [
    {
      "kind": "hidden_sheet",
      "severity": "warning",
      "path": "xl/workbook.xml",
      "message": "worksheet 'Model Inputs' is hidden"
    }
  ]
}
```

**Constraints:** audit output is factual signals only — no risk scoring, no
rendering, and no mutation of input files; severity buckets are `info`,
`warning`, and `high`. In JSONL records `audit` and `error` are mutually
exclusive (schema-enforced via `oneOf` in
[`schemas/v1/oxdoc-audit-jsonl.schema.json`](schemas/v1/oxdoc-audit-jsonl.schema.json));
failed files carry `document_type: "unknown"` and error codes are stable
(`E001`–`E010` classes, per [`errors-and-warnings.md`](errors-and-warnings.md)).
`oxdoc` never renders, paginates, or produces PDFs.

**See also:** [`audit.md`](audit.md),
[`errors-and-warnings.md`](errors-and-warnings.md),
[`json-output.md`](json-output.md),
[`schemas/v1/oxdoc-audit-jsonl.schema.json`](schemas/v1/oxdoc-audit-jsonl.schema.json).

## Reference pages

- [`cli.md`](cli.md) — flag authority for every command and global option.
- [`json-output.md`](json-output.md) — JSON/JSONL field contracts and schemas.
- [`audit.md`](audit.md) — audit signal kinds, severities, and shapes.
- [`errors-and-warnings.md`](errors-and-warnings.md) — stable `E001`–`E010`
  error classes and warning categories/codes.
- [`github-action.md`](github-action.md) — the setup action and its examples.
- [`python-integration.md`](python-integration.md) — the Python wrapper and
  install paths.
- Schema mirrors used by these recipes:
  [`schemas/v1/oxdoc-audit.schema.json`](schemas/v1/oxdoc-audit.schema.json),
  [`schemas/v1/oxdoc-audit-jsonl.schema.json`](schemas/v1/oxdoc-audit-jsonl.schema.json).
