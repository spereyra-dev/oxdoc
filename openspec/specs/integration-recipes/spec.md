# Integration Recipes Specification

## Purpose

Give users a single workflow surface — `docs/recipes.md` — that shows how to
wire oxdoc into the automation they actually run (shell pipelines, GitHub
Actions, Python/pandas orchestration, backend ingestion under resource limits,
audit triage), with copy/paste-ready commands, example output, and the
documented safety/non-rendering constraints that make that automation safe.
The page is a workflow surface, not a flag reference: it links the owning
reference pages instead of restating their content, and it is reachable from
the docs sidebar and the README documentation list. This is a documentation
domain only; it changes no CLI, library, schema, or output contract.

Audience (parent-resolved): every recipe is pitched at the same reader — a
competent engineer automating document processing who has never opened oxdoc.

## Requirements

### Requirement: Recipe coverage

`docs/recipes.md` MUST contain five recipe sections covering exactly these
workflow categories, in this order:

1. Shell pipelines (batch text/csv/rows/slides extraction, `--format jsonl`
   streaming, stdin `-`, exit codes, stderr warning channel).
2. GitHub Actions (the repository's self-use pattern
   `.github/workflows/action.yml` generalized into a job skeleton that adds
   pipeline value beyond the examples in `docs/github-action.md`).
3. Python / pandas orchestration (installing the `oxdoc-python` wrapper and
   the `oxdoc` binary separately, `Oxdoc(binary=...)`, `extract_rows` /
   `extract_csv` → `DataFrame`, per-file error tolerance, `OxdocResult.warnings`).
4. Backend ingestion under resource limits (`--max-input-size`,
   `--max-package-uncompressed-size`, `--max-part-size`,
   `--max-compression-ratio`, typed limit failures `E014`/`E011`/`E005`/`E006`,
   `--warnings json`, `oxdoc diagnostics --format json`).
5. Audit workflow (`audit --format json|jsonl|text`, signal kind/severity
   filtering for triage, and a basic E-code triage step: route failed files by
   the stable `error.code` values carried in audit error records).

Each recipe MUST open with a one-sentence goal stating the workflow outcome
and MUST contain at least one copy/paste-ready command, YAML, or Python block
that assembles the workflow end-to-end. Recipes MUST NOT be command-reference
shaped (no flag tables; wiring only). Recipe headings MUST use the documented
recipe names so the five workflows are skimmable.

#### Scenario: All five recipes present

- GIVEN a reader opens `docs/recipes.md`
- WHEN they scan the recipe headings
- THEN they find one section each for shell pipelines, GitHub Actions,
  Python/pandas, backend ingestion with resource limits, and audit workflow.

#### Scenario: Each recipe is copy/paste-ready

- GIVEN any of the five recipes on `docs/recipes.md`
- WHEN the reader copies its command, YAML, or Python block into the target
  environment described by the recipe
- THEN the block is a complete working step (no elided placeholders such as
  `...` inside the executable lines) and references only CLI flags, APIs, and
  codes documented on the linked reference pages.

#### Scenario: Actions recipe adds pipeline value

- GIVEN the GitHub Actions recipe
- WHEN the reader compares it with the examples in `docs/github-action.md`
- THEN the recipe presents the repository's self-use pattern
  (`.github/workflows/action.yml`, `uses: ./`) generalized into a job
  skeleton and adds at least one pipeline capability the existing examples do
  not show (for example stdout redirects, `actions/upload-artifact`, or a job
  summary derived from audit JSON), rather than repeating those examples.

### Requirement: Example-output labeling

Every recipe MUST include at least one fenced block explicitly labeled as
example output. Every such block MUST be either:

- transcribed from an existing documentation page (`docs/cli.md`,
  `docs/json-output.md`, `docs/audit.md`), consistent with the shapes those
  pages show, or
- explicitly labeled as illustrative, where the existing docs have no output
  sample (for example Actions logs or a pandas `DataFrame` repr), and kept
  minimal.

No output block MAY present output that contradicts a documented output shape,
and no block MAY reference an undocumented flag, field, or code.

#### Scenario: Output blocks are labeled

- GIVEN any of the five recipes
- WHEN the reader inspects its example-output fenced block(s)
- THEN the block is introduced or captioned as example output (or explicitly
  as illustrative output where no upstream sample exists), never presented as
  a guaranteed capture.

#### Scenario: No fabricated content

- GIVEN all example-output blocks in `docs/recipes.md`
- WHEN each block is compared against `docs/cli.md`, `docs/json-output.md`,
  and `docs/audit.md`
- THEN every flag, field name, error/warning code, and record shape in the
  block matches a documented shape, or the block is explicitly labeled
  illustrative and contains no undocumented claims.

### Requirement: Safety and non-rendering constraints

Each recipe MUST state at least one applicable documented constraint from
this set, in a short constraints blurb within the recipe:

- oxdoc never renders, paginates, or produces PDFs.
- Formulas are never recalculated (rows emit stored text plus cache presence).
- Warnings go to stderr and never contaminate JSON/JSONL stdout streams.
- Stdin input is buffered only up to `--max-input-size` and is never spilled
  to a temporary file (required by the backend-ingestion recipe).
- Limit failures are typed (`E014` input size, `E011` uncompressed size,
  `E005`/`E006` part size and suspicious entry).
- Audit output is factual signals only (no risk scoring, no mutation).

The blurb MUST state only guarantees already documented in the repository
docs; it MUST NOT introduce new behavior claims.

#### Scenario: Each recipe carries its applicable constraints

- GIVEN any of the five recipes
- WHEN the reader reads its constraints blurb
- THEN at least one documented guarantee relevant to that workflow is stated
  (for example the backend-ingestion recipe states the stdin no-spill and
  typed-limit guarantees; the shell-pipeline recipe states the
  warnings-on-stderr / stdout-stream-clean guarantee).

#### Scenario: No new behavior claims

- GIVEN every constraints blurb in `docs/recipes.md`
- WHEN each claim is checked against `README.md`, `docs/cli.md`,
  `docs/audit.md`, `docs/security.md`, and `docs/errors-and-warnings.md`
- THEN every claim traces to an existing documented guarantee.

### Requirement: Navigation links

`docs/recipes.md` MUST be linked from exactly one new entry in the Usage
section of `docs/_sidebar.md`, placed after Python Integration, and from
exactly one new entry in the README "Key documentation pages" list, placed
after Python Integration. No other existing documentation page MAY be
modified by this change.

#### Scenario: Sidebar entry under Usage

- GIVEN the Docsify sidebar `docs/_sidebar.md`
- WHEN the reader looks at the Usage section
- THEN an `Integration Recipes` entry linking `recipes.md` exists after the
  Python Integration entry, and no Start/Design/Project section was edited.

#### Scenario: README documentation list entry

- GIVEN the README "Key documentation pages" list
- WHEN the reader scans the list
- THEN an `Integration Recipes` entry linking `docs/recipes.md` exists after
  the Python Integration entry.

#### Scenario: Minimal diff outside the new page

- GIVEN the change's file diff
- WHEN compared against the previous state
- THEN only `docs/recipes.md` (new), one `docs/_sidebar.md` line, one
  `README.md` list line, and the change's own openspec artifacts differ.

### Requirement: Docsify-stable heading anchors

Recipe headings in `docs/recipes.md` MUST be Docsify-stable anchor targets:
each of the five recipes MUST have a heading whose Docsify anchor slug uses
hyphenated lowercase words derived from the heading text (for example
`#recipe-a-shell-pipelines`), so deep links remain valid without
punctuation-dependent or locale-dependent transformations. Recipe headings
MUST NOT rely on characters that Docsify slugging strips unpredictably
(accents, emoji, punctuation beyond hyphens) while this spec domain governs
the page.

#### Scenario: Deep links resolve

- GIVEN a deep link of the form `docs/recipes.md#<recipe-slug>` for each of
  the five recipes
- WHEN the Docsify site (`make docs-check`) serves the page
- THEN each slug matches the heading's hyphenated lowercase anchor and the
  reader lands on the intended recipe.

#### Scenario: Anchors survive heading edits

- GIVEN the heading text of a recipe
- WHEN the text is written
- THEN it contains only letters, digits, spaces, and hyphens so the anchor
  slug is a deterministic hyphenated lowercase form.

### Requirement: Link-validation gate

All links in `docs/recipes.md` MUST be relative internal links (Docsify
relative targets such as `cli.md`, `github-action.md`) or already-documented
external URLs. The change MUST keep `make docs-links` green: at most one
absolute `github.com/spereyra-dev/oxdoc` URL, no freshly minted external
URLs, and no absolute GitHub blob URLs (blob URLs are live-checked by
`markdown-link-check` because the configured ignore is a bare-prefix match).
Schema links MUST target the published `docs/schemas/v1/...` mirrors, never
the repo-root `schemas/...` paths, so `make docs-schemas-check` and the
published site both resolve. `make docs-check`, `make docs-links`,
`make docs-schemas-check`, and `make docs-playground-check` MUST pass after
the change.

#### Scenario: Docs links gate green

- GIVEN the change applied with `docs/recipes.md` and the two navigation
  insertions
- WHEN `make docs-links` runs over `README.md` and `docs/**/*.md`
- THEN every link validates and the command exits successfully.

#### Scenario: Schema links use published mirrors

- GIVEN any schema reference in `docs/recipes.md`
- WHEN the link target is inspected
- THEN it points under `docs/schemas/` (the published mirror), not the
  repo-root `schemas/` path.

#### Scenario: Other docs gates stay green

- GIVEN the applied change
- WHEN `make docs-check`, `make docs-schemas-check`, and
  `make docs-playground-check` run
- THEN all three pass with no new configuration.

### Requirement: No duplication of reference content

Each recipe MUST link the reference page that owns the flags, fields, or
examples it uses (`docs/cli.md`, `docs/json-output.md`, `docs/audit.md`,
`docs/errors-and-warnings.md`, `docs/github-action.md`,
`docs/python-integration.md`) instead of restating flag tables, full
output-field lists, or the existing `docs/github-action.md` examples.
Inline flag/field mentions are limited to what the pipeline wiring itself
needs; definitions live on the linked page.

#### Scenario: Flags resolve through links

- GIVEN a reader on `docs/recipes.md` who needs a flag definition used in a
  recipe command
- WHEN the recipe is read
- THEN it links the owning reference page rather than reproducing a flag
  table or an output-fields list.

#### Scenario: Actions examples are not duplicated

- GIVEN the GitHub Actions recipe
- WHEN it is compared with `docs/github-action.md`
- THEN the recipe links that page for its existing examples and does not
  copy them into `docs/recipes.md`.

#### Scenario: Closing reference list

- GIVEN the end of `docs/recipes.md`
- WHEN the reader needs the owning reference pages
- THEN a closing reference list links `cli.md`, `json-output.md`,
  `audit.md`, `errors-and-warnings.md`, `github-action.md`, and
  `python-integration.md` (plus the `docs/schemas/...` mirrors where used).
