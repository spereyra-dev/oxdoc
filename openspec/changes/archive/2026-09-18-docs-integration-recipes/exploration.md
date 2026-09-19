# Exploration — docs-integration-recipes (#176)

Explored against: `main`; OpenSpec store; canonical specs `docx-extraction`,
`structured-text-schema`, `pptx-slide-extraction`, `xlsx-formula-provenance`
(none govern documentation; this change is docs-only, so no delta specs into
those domains are expected — a `recipes` docs domain may be added or the
change may carry no spec deltas, per precedent from prior docs-adjacent
changes). No existing change dir; this is the first exploration iteration.

## 1. Goal restated

Add copy/paste-ready integration recipes demonstrating real automation
workflows (shell pipelines, GitHub Actions, Python/pandas orchestration,
backend ingestion, audit workflows), each with expected output and the
relevant safety/non-rendering constraints, linked from the docs navigation
and README, with all Markdown links validating.

## 2. Docs navigation structure (verified)

- `docs/_sidebar.md` is the Docsify nav. Sections: **Start** (Overview,
  Getting Started, Installation, GitHub Action, CLI Reference), **Usage**
  (formats/*, audit, json-output, library-api, python-integration,
  compatibility-playground), **Design**, **Project**. Recipes fit naturally
  as a new entry under **Usage** — either one page (`recipes.md`) or a
  `recipes/` subdirectory (Docsify serves subpaths fine; existing precedent:
  `formats/`, `schemas/`, `spikes/` subdirs already work and are linked from
  the sidebar... note: `schemas/` and `spikes/` are NOT in the sidebar, so
  only `formats/` proves the subdir-in-sidebar pattern).
- `docs/_navbar.md` has only GitHub/Roadmap/Contributing/Security — not a
  recipe target.
- `README.md` has a "Key documentation pages" bulleted list (absolute
  `docs/...` paths) — second required link location. AC says "where
  appropriate", so at minimum: sidebar entry + README list entry.
- `docs/index.html` is Docsify boilerplate (`$docsify` config); no per-page
  registration needed.

## 3. Existing recipe-like content (verified: none)

`grep -ri recipe docs/` → no matches. Closest precedents by style:
- `docs/github-action.md` § Examples — three short YAML snippets
  (extract text / CSV / audit JSON) with an artifact-upload note and the
  stdout-redirect rule. This is the pattern to extend, not duplicate.
- `docs/getting-started.md` — cargo-run flavored command walkthrough.
- `README.md` § CLI Usage — flag-rich examples with output blocks.
- `docs/spikes/*.md` — experiment reports, not user recipes.
Risk: overlap. Recipes must reference (`docs/cli.md`, `docs/audit.md`,
`docs/json-output.md`, `docs/python-integration.md`, `docs/github-action.md`)
rather than re-explain flags; each recipe should be workflow-shaped
(multi-step pipelines), not command-reference-shaped.

## 4. Exact CLI surfaces to feature per recipe (verified against docs/cli.md, README, main.rs)

### Recipe A — shell pipelines
- `extract text FILES... --format jsonl` (per-file `file`, `document_type`,
  `text`/`error` records; batch continues after failures), `--format json`,
  `--format structured-json` (schema v2, DOCX variants `header`/`footer`/
  `footnotes`/`endnotes`/`comments` + `variant` first/even/default).
- `extract csv` with `--sheet`/`--sheet-index`/`--list-sheets`
  (`--include-hidden`), `--value-mode raw|formatted`, `--delimiter`.
- `extract slides deck.pptx --format jsonl` (slide_id, slide_ordinal,
  slide_path, text, notes; ordinal gaps when slides skipped).
- `extract rows data.xlsx --format jsonl` (schema v2, `formula` +
  `formula_cached`, never recalculated; sparse cells omitted; warnings on
  stderr, stdout stays valid JSONL).
- stdin: `-` for a single package (seekable-buffer, see §6 limits).
- Exit codes 0/1/2 contract; warnings format
  `warning[<category>/<code>]: <path>: <message>`; `--warnings json` for
  machine-readable stderr records.
- Expected-output blocks should mirror the shapes already in docs/cli.md /
  docs/json-output.md (no invented output).

### Recipe B — GitHub Actions
- Repo **self-uses the action in CI**: `.github/workflows/action.yml`
  (workflow `action`) runs `uses: ./` with `version: v0.1.0` on
  ubuntu/macos/windows, asserts `steps.oxdoc.outputs.version` and `path`,
  and runs `oxdoc --version`. **`ci.yml` does not use the action** (it builds
  from source with `dtolnay/rust-toolchain`); so the recipe should say "the
  repo validates the action itself in the `action` workflow" and link
  `.github/workflows/action.yml` (absolute GitHub URL, since Docsify
  serves docs/ only; use the `https://github.com/spereyra-dev/oxdoc/blob/...`
  form — but note markdown-link-check ignores the bare repo URL pattern, so
  blob URLs will be checked live; they must exist on the default branch or
  the change can link the docs page instead).
- Action facts: composite (`action.yml`), required `version` input pinned to
  an exact `vX.Y.Z` tag, checksum-verified download via `install.sh`
  (unix) / pwsh SHA256 verification (windows), outputs `version` + `path`.
- Recipe content: checkout (`actions/checkout@v7`), `uses:
  spereyra-dev/oxdoc@<commit-sha>`, extraction steps with explicit
  `> output/...` stdout redirects, `actions/upload-artifact@v7`.
- Already covered in docs/github-action.md — recipe must add pipeline value
  (e.g. PR diff of extracted text, job summary from audit JSON) not flag
  duplication.

### Recipe C — Python/pandas
- Package: `oxdoc-python` on PyPI; pure Python, **does not bundle the
  oxdoc binary** (recipe must show install of both, and `Oxdoc(binary=...)`).
- Verified API in `python/src/oxdoc/client.py`:
  - `Oxdoc(binary="oxdoc")`
  - `extract_text(path, structured=False)` → JSON dict; `structured=True`
    → schema-v2 blocks.
  - `extract_text_records(paths)` → list of JSONL records (per-file errors
    preserved as records, not raised).
  - `extract_csv(path, sheet=, sheet_index=, include_hidden=, delimiter=,
    value_mode=)` → **CSV text** (`value` is str).
  - `extract_rows(path, ...)` → list of row dicts (schema v2; raw numbers
    remain strings — pandas recipe must `pd.to_numeric`; formula cells carry
    `formula`/`formula_cached`).
  - `list_sheets(path, include_hidden=False)` → dicts with index/name/
    visibility.
  - `read_info(path)` → metadata dict; `audit(path)` → audit dict.
  - `OxdocResult.value` / `.warnings` (tuple of stderr lines).
  - Exceptions: `OxdocNotFoundError`, `OxdocProcessError` (with command/
    returncode/stdout/stderr), `OxdocJsonError`.
- The pandas recipe maps naturally to `extract_rows` → `pd.json_normalize`/
  `DataFrame.from_records` with the string-numbers caveat, or
  `extract_csv` → `io.StringIO(csv_text)`. Airflow/Dagster naming in
  docs/python-integration.md gives the orchestration framing.
- Constraint to state: wrapper shells out; no wheels bundling the binary;
  native bindings intentionally deferred (decision record).

### Recipe D — backend ingestion
- Resource-limit flags (verified in docs/cli.md Global Options):
  `--max-input-size` (64 MiB default; also bounds the stdin buffer),
  `--max-package-uncompressed-size` (256 MiB), `--max-part-size` (64 MiB),
  `--max-compression-ratio` (200). Typed limit failures: E014 (input size),
  E011 (uncompressed size), E005/E006 (part size / suspicious entry).
- `--warnings json` for machine-readable stderr; `--quiet`/`--warnings none`.
- Batch: `extract text *.docx --format jsonl` and `audit intake/*.docx
  --format jsonl` (per-record `error` with stable codes, `document_type:
  "unknown"` on failure; process continues; exit 0). Schema:
  `docs/schemas/v1/oxdoc-audit-jsonl.schema.json`.
- stdin caveat for serverless: input buffered only up to `--max-input-size`;
  no temp-file spill (the CLI never writes stdin temp files; XLSX
  shared-strings has its own bounded spill-to-disk inside the library).
- Use `oxdoc diagnostics --format json` for support bundles.

### Recipe E — audit workflow
- `audit FILES... --format json|jsonl|text` (verified flags; batch jsonl).
- Signal kinds/severity table already in docs/audit.md (`macros` high,
  `hidden_sheet`/`workbook_protection`/`hyperlink`/`external_link`/
  `attached_template`/`ole_object`/`embedded_package`/`relationship_target`/
  `parser_warning` warning, `custom_properties` info). Schemas:
  `docs/schemas/v1/oxdoc-audit.schema.json` (single) +
  `oxdoc-audit-jsonl.schema.json` (per-line; mutually exclusive
  `audit`/`error` via oneOf+not).
- E-codes: error records carry `error.code` (E001–E010 classes; E010 =
  InvalidArgument per docs/errors-and-warnings.md). Recipes must present
  audit output as factual signals — **no risk scores, no rendering, no
  mutation** (already a documented guarantee; recipe must not imply scoring).

## 5. Safety / non-rendering constraints to feature (source of truth)

From README (Design Principles, Security), docs/cli.md (limits), docs/audit.md:
- Does not render, paginate, or produce PDFs; tolerant input / strict output.
- Never recalculates formulas (rows emits stored text + cache flag only).
- No locale-dependent formatting from `--value-mode formatted` (invariant
  output; W005/W006 fallback warnings).
- Hard-error guards: encrypted parts (E004), oversized parts (E005),
  zip-bomb-like ratios (E006), external/escaping relationship targets (E007).
- Warnings recoverable, on stderr, never contaminating JSON/JSONL stdout
  streams (except `extract slides --format json`, which embeds and mirrors).

## 6. Link validation infrastructure (verified)

- `make docs-links`: `find README.md docs -name '*.md' | xargs
  markdown-link-check@3 --config .markdown-link-check.json`.
- `.markdown-link-check.json` ignores: `^https://github.com/spereyra-dev/oxdoc`
  (bare repo), `^https://spereyra-dev.github.io/oxdoc/?$`, crates.io CLI URL.
  Note: the ignore is a prefix pattern, so blob URLs under the repo are
  **not** ignored and will be checked live — GitHub blob links validate,
  anchors do not (markdown-link-check does not verify anchors).
- `make docs-check` (Docsify serves), `make docs-schemas-check`
  (diffs `schemas/` vs `docs/schemas/` — recipes must link `docs/schemas/...`
  copies so the site works), `make docs-playground-check`.
- `.github/workflows/docs.yml` runs docs-check + docs-links +
  docs-schemas-check + docs-playground-check on PRs and main — new pages and
  their links are automatically gated. AC 4 is satisfied by keeping
  `make docs-links` green; no new tooling needed.

## 7. Gaps vs acceptance criteria

| AC | Status |
| --- | --- |
| Cover shell pipelines, GitHub Actions, Python/pandas, backend ingestion, audit workflows | No recipes exist; all five workflows have verified, current CLI/API surfaces documented above |
| Expected output + safety/non-rendering constraints | Output shapes available verbatim from docs/cli.md, docs/json-output.md, docs/audit.md snapshots; constraints enumerated in §5 |
| Link recipes from docs navigation and README | `_sidebar.md` (Usage section) and README "Key documentation pages" both need edits |
| Validate all Markdown links | Existing `make docs-links` gate covers new pages automatically |

## 8. What must NOT change

- Any CLI flag semantics, output shapes, schemas, or code — docs-only change.
- Existing doc pages beyond the two required link insertions (minimal-diff
  rule); github-action.md examples stay as-is (recipes link, don't duplicate).
- Schemas copies under `schemas/` vs `docs/schemas/` (docs-schemas-check).
- The Docsify sidebar ordering conventions (recipes appended to Usage).

## 9. Risks / decisions to surface in the proposal

1. **One page vs five pages**: a single `docs/recipes.md` (~250–350 lines)
   keeps nav simple and fits the 400-line review budget; five
   `docs/recipes/*.md` pages are more navigable but multiply sidebar edits
   and push the diff past the budget. Recommend single page with
   per-recipe anchors, or a small directory if the user prefers growth
   room. Decide at proposal time.
2. **Review budget**: 5 recipes × (commands + expected output + YAML/Python
   snippets) ≈ 300–450 lines plus nav/README links → at or near the 400-line
   `ask-on-risk` threshold; expect a size conversation.
3. **Duplication drift**: commands/output must be transcribed from current
   docs; a future flag change silently stales recipes. Mitigation: keep each
   recipe thin on flags (link cli.md) and heavy on pipeline wiring;
   optionally note the convention in the recipe intro. A doc-check that
   executes recipes is out of scope for this change (playground-style
   execution is an existing separate pattern).
4. **Absolute GitHub blob links** in the Actions recipe will be
   link-checked live (prefix ignore doesn't cover blob paths); prefer
   relative doc links (`github-action.md`, `cli.md`) and only one
   repo-file link, chosen to be stable on main.
5. **Naming**: `docs/recipes.md` vs `docs/integration.md` — "recipes"
   matches the issue title; do not invent a new term.
6. **No spec deltas expected**: docs-only; if the SDD flow requires a spec
   delta, add a minimal docs/recipes requirement or explicitly state
   none (precedent: prior changes all carried code contracts — this one
   is the first docs-only change; the proposal should say so explicitly).

## 10. Recommended next step

Proceed to proposal with the §9 decisions: page layout (single file
recommended), link targets (`docs/schemas/...` mirrors), and the
docs-only/no-spec-delta statement. Then design + tasks; keep total diff
under ~400 lines or trigger `ask-on-risk`.
