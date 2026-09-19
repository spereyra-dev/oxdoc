# Design — integration recipes documentation (issue #176)

- Change id: `docs-integration-recipes` · Status: designed (SDD design phase)
- Store: openspec · Delivery: `auto-chain` · Chain strategy: `stacked-to-main` (per tasks) · Review budget: 400 changed lines
- Docs-only change: the only repo-visible files are `docs/recipes.md` (new), one `docs/_sidebar.md` line, one `README.md` line. No code, schema, workflow, or existing-page edits.

Inputs read for this design: `proposal.md`, `specs/integration-recipes/spec.md`, `tasks.md`, `docs/cli.md`, `docs/json-output.md`, `docs/audit.md`, `docs/github-action.md`, `docs/python-integration.md`, `docs/errors-and-warnings.md`, `docs/schemas/v1/oxdoc-audit.schema.json`, `docs/schemas/v1/oxdoc-audit-jsonl.schema.json`, `docs/_sidebar.md`, `README.md`, `.github/workflows/action.yml`, `.github/workflows/ci.yml`, `python/src/oxdoc/client.py`, `.gitignore`, `openspec/config.yaml`. Every command, flag, field, and code below is transcribed from those verified surfaces; nothing is invented.

## Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | Headings use ASCII hyphens (`Recipe A - Shell pipelines`), never em dashes, slashes, or accents, so Docsify slugs are deterministic | Spec requirement "Docsify-stable heading anchors" mandates letters/digits/spaces/hyphens only |
| D2 | Sidebar entry is inserted **immediately after** Python Integration (before Compatibility Playground), not as the section's last entry | Spec scenario says "after the Python Integration entry"; proposal wording agrees. This **amends task 5**, whose "last entry of the Usage section" phrasing conflicts — spec is authoritative |
| D3 | Zero absolute GitHub URLs planned. `.github/workflows/action.yml` is named in inline code only; the recipe links `github-action.md` instead | `markdown-link-check` ignore is a bare-prefix match, so blob URLs would be live-checked (proposal Alternative 7) |
| D4 | Schema links from `docs/recipes.md` are written relative (`schemas/v1/oxdoc-audit-jsonl.schema.json`), matching the existing convention in `docs/cli.md`/`docs/json-output.md`; the resolved target is under `docs/schemas/` (the published mirror), keeping `docs-schemas-check` meaningful | `make docs-schemas-check` diffs `schemas/` vs `docs/schemas/`; from within `docs/*.md` the relative path *is* the mirror |
| D5 | Recipe B keeps the `<commit-sha>` placeholder with the documented "replace it" instruction, exactly as `docs/github-action.md` does | It is the documented action-reference convention, not an elision of command content; the spec's no-elision rule targets elided pipeline logic |
| D6 | Two illustrative output blocks total (Recipe B job-summary line, Recipe C `DataFrame` repr), both minimal and explicitly labeled; every other output block is transcribed or schema-shape-consistent | Spec "Example-output labeling"; proposal Q4 resolved: illustrative allowed where labeled |
| D7 | Recipe E triage uses `jq` on the **single-file JSON** for severity filtering (`.signals[]`) and on **JSONL** for E-code routing (`select(.error)`), with `--format text` shown as the no-jq human path | Verified field paths: signal objects always carry `kind`/`severity`/`path`/`message` (schema `$defs.auditSignal` requires all four); JSONL records carry top-level `file` and `error.code` |
| D8 | Measurement convention (session preflight): the budgetable diff excludes `.gitignore`-matched files and `openspec/**` bookkeeping; measure with `git diff --stat <base>...HEAD -- . ':(exclude)openspec' ':(exclude).gitignore'` | Parent-provided convention; supersedes the tasks forecast that counted spec/tasks artifacts toward the 400 budget (see Task reconciliation) |

## 1. Page structure

`docs/recipes.md`, single page, one coherent heading hierarchy:

```text
# Integration Recipes                       → #integration-recipes
  (intro: how to read these recipes, ~15–20 lines)
## Recipe A - Shell pipelines               → #recipe-a-shell-pipelines
## Recipe B - GitHub Actions                → #recipe-b-github-actions
## Recipe C - Python pandas orchestration   → #recipe-c-python-pandas-orchestration
## Recipe D - Backend ingestion             → #recipe-d-backend-ingestion
## Recipe E - Audit workflow                → #recipe-e-audit-workflow
## Reference pages                          → #reference-pages
```

Slug rules (locked): lowercase, spaces → single hyphens, no punctuation to strip. Recipe D's heading is shortened to "Backend ingestion" (not "…under resource limits") so the slug stays short and scannable; the limits are the recipe's content, not its name. These five slugs are the spec's deep-link targets and MUST NOT change between apply and archive (task 12 checks them).

**Intro premise** (the page's contract with the reader, stated up front):

1. Recipes are **workflows**, not flag references — `docs/cli.md` is the flag authority and every recipe links the page that owns its flags.
2. Commands were verified against the current CLI surface at the time of writing; oxdoc is deterministic and text-safe: it never renders, paginates, or produces PDFs.
3. Every expected-output block is labeled: either **example output transcribed from the current docs** or **illustrative** where the docs have no sample (Actions logs, a `DataFrame` repr).
4. Safety guarantees used across recipes (warnings on stderr, stdout stream-clean, formulas never recalculated, typed limit failures, no stdin temp-file spill) are each stated in the recipe where they apply, and only as documented.

**Per-recipe template** (all five recipes use the same five-part shape; each part is separately checkable):

```text
Goal            one sentence, the workflow outcome
Command         copy/paste-ready shell / YAML / Python block (no elided executable lines)
Example output  fenced block labeled "example output" or "illustrative output"
Constraints     short safety / non-rendering blurb from the documented guarantee set
See also        links to the reference page(s) that own the flags/fields
```

**Closing section** (`## Reference pages`): relative links to `cli.md`, `json-output.md`, `audit.md`, `errors-and-warnings.md`, `github-action.md`, `python-integration.md`, and the schema mirrors used (`schemas/v1/oxdoc-audit.schema.json`, `schemas/v1/oxdoc-audit-jsonl.schema.json`, `schemas/v2/oxdoc-xlsx-rows-jsonl.schema.json`, `schemas/v1/oxdoc-pptx-slides.schema.json` as referenced by recipes).

## 2. Per-recipe skeletons (exact transcribed content)

### Recipe A — Shell pipelines

- **Goal:** batch-extract text, CSV, typed rows, and slides from Office files into JSON/JSONL streams a shell pipeline can consume.
- **Commands (6 blocks, all flags verified in `docs/cli.md`):**
  1. `oxdoc extract text inbox/*.docx --format jsonl > extracted-text.jsonl`
  2. `cat contract.docx | oxdoc extract text - --format json > contract.json` (stdin `-`, single package)
  3. `oxdoc extract csv data.xlsx --sheet "Ventas Q1" --delimiter "," > sales.csv`
  4. `oxdoc extract rows data.xlsx --sheet "Sales Q1" --value-mode formatted > rows.jsonl`
  5. `oxdoc extract slides deck.pptx --format jsonl > slides.jsonl`
  6. Exit codes + warning channel:
     ```bash
     if oxdoc --warnings json extract text inbox/*.docx --format jsonl > extracted-text.jsonl 2> warnings.jsonl; then
       echo "extraction finished; stderr warnings: $(wc -l < warnings.jsonl)"
     else
       echo "hard failure; inspect warnings.jsonl and stderr" >&2
       exit 1
     fi
     ```
     with prose: `0` success, `1` hard runtime failure (`error[<code>]: <message>` on stderr), `2` CLI usage error (Clap).
- **Example output (labeled; sources):** slides JSONL line verbatim from `docs/cli.md` (`{"schema_version":1,"file":"deck.pptx","slide_id":256,...,"notes":"Speaker note\n"}`); rows JSONL line verbatim from `docs/json-output.md` (`{"schema_version":2,...,"formula":"B2*2","formula_cached":true}`); one text-JSONL record shape-consistent with `docs/cli.md` fields (`file`, `document_type`, `text`): `{"file":"contrato.docx","document_type":"docx","text":"Plain text..."}`; stderr warning line verbatim from `docs/json-output.md`: `warning[parser/W001]: word/document.xml: stopped after malformed XML: ...`.
- **Constraints blurb:** warnings go to stderr and never contaminate the JSONL stdout stream (JSONL text records also embed them as `warnings`); JSONL batch continues after per-file failures; `--value-mode formatted` is deterministic and locale-independent; oxdoc never renders, paginates, or produces PDFs.
- **See also:** `cli.md`, `json-output.md`, `errors-and-warnings.md`.

### Recipe B — GitHub Actions

- **Goal:** run oxdoc extraction and audit in a GitHub Actions job via the official setup action, publishing outputs as artifacts and a job summary.
- **Self-use framing (prose, no URLs):** the repository's own `action` workflow (`.github/workflows/action.yml`) validates the action on ubuntu/macos/windows by running `uses: ./` with `version: v0.1.0` and asserting `steps.oxdoc.outputs.version` and `path`; `ci.yml` builds from source and does not invoke the action.
- **Job skeleton (one YAML block; every step complete):**

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

  with the documented instruction to replace `<commit-sha>` (D5) and the `github-action.md` stdout rule restated in prose: redirect stdout to the file you keep; warnings and errors remain on stderr in the workflow log.
- **Example output (labeled illustrative):** one job-summary line reusing a message transcribed from `docs/audit.md`: `- warning hidden_sheet: worksheet 'Model Inputs' is hidden`.
- **Constraints blurb:** extraction/audit JSON goes to stdout (redirect it), warnings/errors stay on stderr; oxdoc never renders or paginates; pin the action by commit SHA and `version` to an exact `vX.Y.Z` tag; the setup action checksum-verifies the download before adding `oxdoc` to `PATH`.
- **Pipeline value added** (vs `docs/github-action.md`, which this recipe links and does not copy): job summary derived from audit JSON + combined artifact upload.
- **See also:** `github-action.md`, `cli.md`.

### Recipe C — Python pandas orchestration

- **Goal:** drive oxdoc from Python and turn XLSX content into pandas DataFrames inside notebooks, Airflow, or Dagster jobs.
- **Install block (two lines, wrapper and binary separately — verified in `docs/python-integration.md`):**

  ```bash
  python -m pip install oxdoc-python
  cargo install oxdoc-cli
  ```

  with prose: `oxdoc-python` is pure Python and does **not** bundle the binary; alternatively use the `install.sh` path; put `oxdoc` on `PATH` or pass `Oxdoc(binary="./target/release/oxdoc")`.
- **Python block 1 — DataFrame from rows, CSV via StringIO** (API verified against `python/src/oxdoc/client.py`):

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

- **Python block 2 — batch tolerance and warnings:**

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

- **Example output (labeled illustrative, minimal `df` repr):** a 2-row × 3-column frame sketch showing `row_index` plus cell columns with converted numbers — no undocumented fields.
- **Constraints blurb:** the wrapper shells out to the CLI — no native bindings and no bundled binary (documented decision record); rows formula cells carry `formula` (stored text, never recalculated or rewritten) and `formula_cached`; errors surface as `OxdocNotFoundError` / `OxdocProcessError` (carries `command`, `returncode`, `stdout`, `stderr`) / `OxdocJsonError`; warnings arrive as `OxdocResult.warnings` (non-empty stderr lines).
- **See also:** `python-integration.md`, `json-output.md`.

### Recipe D — Backend ingestion

- **Goal:** accept untrusted uploads in a serverless or multi-tenant backend with hard per-request memory limits and machine-readable failures.
- **Commands (flags and defaults verified in `docs/cli.md` Global Options):**
  1. Limited single request:
     ```bash
     oxdoc extract text upload.docx --format json > record.json 2> warnings.jsonl \
       --max-input-size 67108864 \
       --max-package-uncompressed-size 268435456 \
       --max-part-size 67108864 \
       --max-compression-ratio 200 \
       --warnings json
     ```
     (defaults shown; prose explains setting `--max-input-size` to the request-body limit and package/part limits to one invocation's memory budget)
  2. Stdin intake: `cat request-body | oxdoc extract text - --format json --max-input-size 10485760`
  3. Batch triage: `oxdoc audit intake/*.docx --format jsonl > audit.jsonl`
  4. Support bundle: `oxdoc diagnostics --format json` (fields `schema_version`, `oxdoc_version`, `platform`, `enabled_features`, `limits`; reads no document content).
- **Typed failures (prose block, no invented message text):** hard failures print `error[<code>]: <message>` to stderr and exit `1`. Typed limit codes: `E014` input-package size, `E011` uncompressed-package size, `E005` oversized part, `E006` suspicious ZIP entry (plus the other stable classes `E001`–`E010` in `errors-and-warnings.md`).
- **Example output (labeled; sources):** JSON warning record verbatim from `docs/cli.md`: `{"category":"parser","code":"W001","path":"word/document.xml","message":"stopped after malformed XML: ..."}`; one audit-JSONL error record shape-consistent with `docs/schemas/v1/oxdoc-audit-jsonl.schema.json` (`error` requires `code` + `message`; failed files use `document_type: "unknown"`): `{"schema_version":1,"file":"intake/broken.docx","document_type":"unknown","error":{"code":"E002","message":"not a readable ZIP/OOXML package"}}`.
- **Constraints blurb:** stdin is buffered only up to `--max-input-size` because ZIP central-directory access needs a seekable source; the CLI never writes temp files or spills stdin to disk (XLSX shared-string parsing keeps its own bounded library-internal spill); warnings never contaminate the JSON/JSONL stdout stream; defaults (64 MiB / 256 MiB / 64 MiB / 200) are secure and preserve ordinary-document compatibility.
- **See also:** `cli.md`, `json-output.md`, `security.md`, `performance.md`, `errors-and-warnings.md`.

### Recipe E — Audit workflow

- **Goal:** triage a batch of untrusted documents with `oxdoc audit`, escalating high-severity signals and routing failed files by stable error code.
- **Commands (verified in `docs/audit.md` / `docs/cli.md`):**
  1. `oxdoc audit report.docx --format json > audit.json`
  2. `oxdoc audit intake/*.docx --format jsonl > audit.jsonl` (batch; continues after per-file failures; exit `0`)
  3. `oxdoc audit report.docx --format text` (human path; shown as the no-`jq` fallback, per D7)
- **Triage blocks (field paths verified against `docs/schemas/v1/oxdoc-audit.schema.json` and `...-jsonl.schema.json`):**
  1. Severity filter over single-file JSON:
     ```bash
     jq -r '.signals[] | select(.severity == "high") | [.kind, .path] | @tsv' audit.json
     ```
  2. E-code routing over the batch (stable `error.code` on failed files):
     ```bash
     jq -r 'select(.error) | [.file, .error.code] | @tsv' audit.jsonl
     ```
- **Example output (labeled example output, transcribed verbatim from `docs/audit.md` JSON Shape):** the `workbook.xlsx` record with the `hidden_sheet` / `warning` signal on `xl/workbook.xml`.
- **Constraints blurb:** audit output is factual signals only — no risk scoring, no rendering, no mutation of input files; severity buckets are `info` / `warning` / `high`; in JSONL records `audit` and `error` are mutually exclusive (schema-enforced via `oneOf` + `not`); failed files carry `document_type: "unknown"`; error codes are stable (`E001`–`E010` classes).
- **See also:** `audit.md`, `errors-and-warnings.md`, `json-output.md`, `schemas/v1/oxdoc-audit-jsonl.schema.json`.

## 3. Navigation insertions (exact)

**`docs/_sidebar.md`** — one new line inside the **Usage** section, directly after the `- [Python Integration](python-integration.md)` line (before `- [Compatibility Playground](compatibility-playground.md)`):

```markdown
  - [Integration Recipes](recipes.md)
```

**`README.md`** — one new line in the "Key documentation pages" list, directly after the `- [Python Integration](docs/python-integration.md)` line (before `- [Architecture](docs/architecture.md)`):

```markdown
- [Integration Recipes](docs/recipes.md)
```

No other existing page, section, or list entry changes. `docs/_navbar.md`, `docs/index.html`, and Docsify config need no change.

## 4. Slice boundaries

Per tasks + proposal; the chain strategy is `stacked-to-main`, confirmed here as design-consistent (not invented at apply time):

| Slice | Content | Estimated realized lines | Gate state at slice end |
|-------|---------|--------------------------|-------------------------|
| **Slice 1 (PR 1)** | `docs/recipes.md` with intro + template + Recipes A/B/C; both nav insertions; the change's spec artifact committed alongside | ~200–290 | all four docs gates green; page reachable; anchors A–C resolve |
| **Slice 2 (PR 2)** | Recipes D + E appended; `## Reference pages` closing section; final gate run + whole-change guards | ~125–185 | all four gates green; all five anchors resolve; measured diff recorded |

If the measured budgetable diff (D8 convention) lands under 400 for the whole change, a single PR is acceptable; otherwise the two stacked PRs above are the boundary. No `size:exception` is claimed under any outcome.

## 5. Verification

Per slice and at final: `make docs-check`, `make docs-links`, `make docs-schemas-check`, `make docs-playground-check` — all green with no configuration changes (`.github/workflows/docs.yml` re-runs them on PR and `main`). Link discipline inside `docs/recipes.md`: relative internal links only, at most one absolute `github.com/spereyra-dev/oxdoc` URL (plan: zero), no blob URLs, schema links relative into `docs/schemas/v1/...` mirrors (D4). Anchor check per slice: deep links `#recipe-a-shell-pipelines` … `#recipe-e-audit-workflow` resolve in the served site. Command read-through of every block against `docs/cli.md` (and `oxdoc --help` where available) is recorded in `apply-progress.md` as the accuracy evidence trail — no recipe-execution harness is added (Non-goal 5; `strict_tdd` and the 95% coverage gate are satisfied vacuously since no Rust/Python source changes).

## 6. Task reconciliation (13 tasks, 2 slices)

Tasks were authored before this design; reconciliation result: **the 13-task structure stands**, with these amendments apply must honor:

| Task | Reconciliation |
|------|----------------|
| 5 | Insert the sidebar line **after Python Integration**, not as the Usage section's last entry (D2 — spec wins over the task's parenthetical) |
| 2–4, 8–10 | Recipe content follows §2 exactly: command lists, output-block sources, and see-also sets are now pinned; apply transcribes, never invents |
| 6, 11 | Anchor checks use the §1 slug table (Recipe D slug is `#recipe-d-backend-ingestion`, matching the shortened D1 heading) |
| 7, 12 | Use the D8 measurement command; the tasks' "count spec + tasks artifacts" forecast is superseded — those artifacts are openspec bookkeeping, excluded per session convention; the budgetable surface is `docs/recipes.md` + `docs/_sidebar.md` + `README.md` (~355–510 realized lines), which still straddles 400, so the two-slice plan remains the default |
| 13 | Verify phase checks the six spec requirements statically over the final page plus the four gates, as tasks already state |

## 7. Risks (design-level)

1. **Page length vs budget:** the doc page alone estimates ~355–510 realized lines — the primary reason the two-slice chain is pre-planned. Mitigation enforced by design: thin-on-flags (link, don't restate), shared five-part template, only two illustrative blocks.
2. **Drift:** transcribed output and inline flags stale when the CLI changes. Mitigation: intro scope note, labeled blocks, per-recipe owner links; an execution checker remains a recorded deferred follow-up (proposal Alternative 8).
3. **jq dependency in Recipes B/E:** `jq` may be absent in some environments; Recipe E shows `--format text` as the no-`jq` path and Recipe B's summary step degrades to a failed step (`if: always()` keeps artifacts publishing).
4. **Nested-cell flattening in Recipe C:** the pandas block is the page's most opinionated code; its correctness rests only on verified `client.py` field names (`row_index`, `cells`, `column_index`, `raw`, `value`), and the read-through evidence trail must walk it line by line.

## Review checklist

- [ ] Five recipe headings present, slugs locked per §1, ASCII-only heading text
- [ ] Every recipe: goal + complete command block + labeled output block + constraints blurb + see-also links
- [ ] No flag tables, no restated output-field lists, no copied `github-action.md` examples
- [ ] Sidebar + README insertions exactly one line each, placed per §3
- [ ] All links relative; zero blob URLs; schema links into `docs/schemas/` mirrors
- [ ] All four docs gates green per slice; measured diff recorded with the D8 command
