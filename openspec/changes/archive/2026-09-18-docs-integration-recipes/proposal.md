# Proposal — integration recipes documentation (issue #176)

- Change id: `docs-integration-recipes`
- Status: proposed (SDD propose phase, artifact store: openspec)
- Inputs: `openspec/changes/docs-integration-recipes/exploration.md`, GitHub
  issue #176 acceptance criteria (as reflected in the exploration's
  acceptance-criteria table and the parent-provided decision list),
  parent-resolved product decisions (authoritative; traceability table below),
  `openspec/config.yaml`, and the docs tooling verified in the Makefile
  (`docs-check`, `docs-links`, `docs-schemas-check`, `docs-playground-check`)
  plus `.github/workflows/docs.yml`.
- Delivery: `auto-chain` (session preflight) · review budget 400 changed lines ·
  artifact store openspec · chain strategy deferred (not assumed here).
- **This is the first docs-only change in this repository.** Previous changes
  carried CLI/API/schema contracts (docx, pptx, xlsx). This one modifies
  documentation and one minimal requirements artifact, and no production code.

## Intent / problem

oxdoc has a complete reference surface — `docs/cli.md`, `docs/json-output.md`,
`docs/audit.md`, `docs/github-action.md`, `docs/python-integration.md` — but no
**workflow** surface. Every one of those pages answers "what does this flag
do?"; none answers "how do I wire this into the automation I actually run?".
The gap is provable from the repo, not from user anecdote:

1. **No recipe content exists.** `grep -ri recipe docs/` returns nothing. The
   closest precedents are single-command snippets: `docs/github-action.md` has
   three short YAML examples, `docs/getting-started.md` is a cargo-run
   walkthrough, and `README.md` has a flag-rich CLI Usage section. All are
   reference-shaped, not pipeline-shaped.
2. **The five real automation workflows are undocumented end-to-end.** Shell
   batch pipelines over `--format jsonl`, the repository's own self-use Actions
   pattern, Python/pandas orchestration, backend ingestion under resource
   limits, and audit triage each require assembling facts from three or four
   different pages. A user cannot copy a working pipeline out of the docs today.
3. **The safety guarantees that make automation safe live far from the
   workflows they protect.** "Never renders, never recalculates, warnings never
   contaminate stdout, stdin is never spilled to a temporary file, limit
   failures are typed (E014/E011)" are spread across `README.md`,
   `docs/cli.md`, `docs/audit.md`, and `docs/security.md`. A pipeline author who
   reads only `docs/cli.md` will not know that JSONL stdout is stream-safe by
   construction.
4. **Discoverability.** The Docsify sidebar and the README "Key documentation
   pages" list contain no entry that promises integration guidance, so a user
   looking for "how do I use this in CI / in a backend / in pandas" has no
   navigational landing point.

Users and situations: data engineers piping extractions into warehouses,
platform engineers wiring extraction into GitHub Actions or a serverless
intake endpoint, Python/pandas and orchestration (Airflow/Dagster) users, and
audit/security reviewers triaging documents. The moment of need is *before*
the first working pipeline, when the user has documents and an automation
target but no assembled example; urgency is low per user but repeated for every
new integrator.

The value is a single, copy/paste-ready page where each of the five workflow
categories is demonstrated with a working command or workflow, an expected
output block, and the safety/non-rendering constraint that applies to it —
reachable from the sidebar and README, with every Markdown link validated by the
existing docs gate.

## Solution shape

### 1. One page: `docs/recipes.md`

A single new page, not a `recipes/` directory. Contents:

- **Intro / how to read these recipes** (~15–20 lines): recipes are workflows,
  not flag references — `docs/cli.md` is the flag authority and each recipe
  links to it; every expected-output block is *example output transcribed from
  the current docs* and labeled as such; commands are verified against the
  current CLI surface at the time of writing.
- **Recipe A — Shell pipelines** (~60–75 lines): batch text/csv/rows/slides
  extraction, `--format jsonl` streaming, stdin `-`, exit codes 0/1/2, and the
  stderr warning channel (`warning[<category>/<code>]: <path>: <message>`,
  `--warnings json`).
- **Recipe B — GitHub Actions** (~40–55 lines): the repository's self-use
  pattern (`.github/workflows/action.yml` runs `uses: ./` and asserts the
  action's `version`/`path` outputs), generalized into a job skeleton that adds
  pipeline value on top of the examples already in `docs/github-action.md`
  (stdout redirects, `actions/upload-artifact@v7`, a job summary derived from
  audit JSON). Links `docs/github-action.md` for the action's examples instead
  of duplicating them.
- **Recipe C — Python / pandas orchestration** (~50–65 lines): installing both
  the `oxdoc-python` wrapper and the `oxdoc` binary (the wrapper does not bundle
  it), `Oxdoc(binary=...)`, `extract_rows` → `DataFrame` construction with the
  numeric-as-string caveat and the schema-v2 `formula`/`formula_cached` keys,
  `extract_csv` → `io.StringIO`, `extract_text_records` per-file error
  tolerance, and `OxdocResult.warnings`.
- **Recipe D — Backend ingestion under resource limits** (~50–65 lines):
  `--max-input-size` / `--max-package-uncompressed-size` / `--max-part-size` /
  `--max-compression-ratio` as the per-request memory contract, typed limit
  failures (`E014` input size, `E011` uncompressed size, `E005`/`E006` part size
  and suspicious entry), `--warnings json` for machine-readable stderr, batch
  JSONL error records with `document_type: "unknown"`, `oxdoc diagnostics
  --format json`, and the no-stdin-spill guarantee (stdin is buffered only up to
  `--max-input-size` because ZIP central-directory access needs a seekable
  source; the CLI never writes a temp file).
- **Recipe E — Audit workflow** (~45–60 lines): `audit` in `json`/`jsonl`/`text`,
  filtering signals by kind/severity for triage, stable error codes in error
  records, `document_type: "unknown"` on failed files, and the schema-enforced
  `audit`/`error` mutual exclusion in the JSONL contract
  (`docs/schemas/v1/oxdoc-audit-jsonl.schema.json`).
- **Closing reference list** (~5 lines): links to `cli.md`, `json-output.md`,
  `audit.md`, `errors-and-warnings.md`, `github-action.md`,
  `python-integration.md`, and the `docs/schemas/...` mirrors.

Every recipe uses the same four-part shape so the page is skimmable and so each
part is separately checkable:

```text
Goal            one sentence, the workflow outcome
Command         copy/paste-ready shell / YAML / Python block
Example output  fenced block explicitly labeled as example output
Constraints     short safety / non-rendering blurb (where relevant)
See also        links to the reference page that owns the flags/fields
```

### 2. Navigation: exactly two insertions

- `docs/_sidebar.md` — append one entry to the **Usage** section after Python
  Integration:

  ```markdown
  - [Integration Recipes](recipes.md)
  ```

- `README.md` — one entry in the "Key documentation pages" list after Python
  Integration:

  ```markdown
  - [Integration Recipes](docs/recipes.md)
  ```

No other existing page is edited. `docs/_navbar.md`, `docs/index.html`, and the
Docsify configuration need no change (`docs/index.html` is boilerplate and the
sidebar is the only per-page registry).

### 3. One minimal spec so the archive has a truthful artifact

Docs-only does not mean requirement-free. The change carries **one** new spec
domain at `openspec/changes/docs-integration-recipes/specs/integration-recipes/spec.md`
(the spec phase authors the file; this proposal fixes its contract):

| Requirement | Content |
| --- | --- |
| Recipe coverage | `docs/recipes.md` contains the five numbered recipe categories (shell pipelines, GitHub Actions, Python/pandas, backend ingestion, audit) |
| Expected output | Every recipe includes at least one fenced block explicitly labeled as example output, transcribed from an existing docs page (or, where the existing docs have no output sample, marked as illustrative) |
| Safety / non-rendering constraints | Every recipe states at least one applicable constraint from the documented guarantee set (no rendering/PDF, never recalculates formulas, warnings stay on stderr, no stdin temp-file spill, typed limit failures) |
| Navigation links | `docs/recipes.md` is linked from the `docs/_sidebar.md` Usage section and the README "Key documentation pages" list |
| Link validation | All links in the new page are relative internal links or existing documented URLs; `make docs-links` passes; at most one absolute `github.com/spereyra-dev/oxdoc` link and no freshly minted external URL |
| No duplication drift | Each recipe links the owning reference page (`cli.md`, `json-output.md`, `audit.md`, `github-action.md`, `python-integration.md`) rather than restating flag tables or re-listing the examples already in `github-action.md` |

Each requirement carries scenarios in the repository's GIVEN/WHEN/THEN style
(for example: GIVEN a reader on `docs/recipes.md`, WHEN a recipe's command needs
a flag definition, THEN the recipe links the owning reference page instead of
restating the flag table). The archive composes this new
`openspec/specs/integration-recipes/spec.md` domain; **no existing spec domain
is touched** and no delta spec lands in `docx-extraction`,
`structured-text-schema`, `pptx-slide-extraction`, or `xlsx-formula-provenance`.

### 4. Accuracy rules (drift control)

- **Thin on flags, heavy on wiring.** No recipe reproduces a flag table or a full
  output-fields list; each links the owning page. This is the main defense
  against a future flag change silently staling the recipes.
- **Transcribe, don't invent.** Expected-output blocks reuse shapes that already
  exist in `docs/cli.md`, `docs/json-output.md`, and `docs/audit.md`. Where no
  output sample exists upstream (Actions logs, a pandas `DataFrame` repr), the
  block is explicitly labeled illustrative and kept minimal.
- **No behavior claims beyond the docs.** The page states only guarantees that
  are already documented (never renders, never recalculates, warnings on stderr,
  no stdin temp-file spill, typed limit codes).
- **Link targets that keep the gate green.** Prefer relative links plus the
  already-published `docs/schemas/v1/...` mirrors. The Actions recipe names
  `.github/workflows/action.yml` in prose/code and links the docs page rather
  than emitting an absolute blob URL, because `markdown-link-check` ignores only
  the bare `^https://github.com/spereyra-dev/oxdoc` prefix and would validate
  blob URLs live.

### 5. Verification (docs-only, no test harness change)

- `make docs-check` — Docsify serves with the new page and sidebar entry.
- `make docs-links` — every link in `README.md` and `docs/**/*.md` validates
  (this is AC 4's gate; `docs.yml` already runs it on PRs and `main`).
- `make docs-schemas-check` — unaffected, but must stay green (recipes link the
  `docs/schemas/...` copies, never `schemas/...`).
- `make docs-playground-check` — unaffected, must stay green.
- Manual read-through of each command against `docs/cli.md` and, where
  available, `oxdoc --help`, recorded in the apply progress as the evidence
  trail.
- **No** `cargo` gate, coverage, or fixture change: no Rust/Python source is
  touched, so `strict_tdd` and the 95% coverage gate are satisfied vacuously.
  A recipe-execution harness is an explicit non-goal (see Non-goals 5).

## Acceptance-criteria mapping (issue #176)

| Acceptance criterion (as reflected in the exploration) | Slice that satisfies it | How it is checked |
| --- | --- | --- |
| Cover shell pipelines, GitHub Actions, Python/pandas, backend ingestion, and audit workflows | S1 (page + Recipes A/B/C), S2 (Recipes D/E) — same page, one artifact | Spec requirement "Recipe coverage"; all five headings present in `docs/recipes.md` |
| Each recipe shows expected output and the relevant safety / non-rendering constraints | S1 (A/B/C), S2 (D/E) | Spec requirements "Expected output" and "Safety / non-rendering constraints"; each recipe has an example-output fenced block and a constraints blurb |
| Link recipes from the docs navigation and README | S1 (nav + README insertion, so the page is reachable as soon as it exists) | Spec requirement "Navigation links"; `docs/_sidebar.md` Usage entry + README list entry, both diffed in review |
| Validate all Markdown links | S1 and S2 (each slice must leave the gate green) | Spec requirement "Link validation"; `make docs-links` in `docs.yml` and locally |

Supporting, non-AC success criteria are listed under Success criteria below.

## Backwards compatibility

Docs-only, so nothing is versioned and nothing is deprecated.

- **No CLI, library, schema, or output change.** No file under `crates/`,
  `python/`, `schemas/`, or `docs/schemas/` is modified; no snapshot changes.
- **No existing documentation content is rewritten.** The only edits outside the
  new page are one sidebar line and one README list line. `docs/github-action.md`
  examples stay byte-identical — recipes link to them.
- **No URLs are removed or renamed**, so existing inbound links and the published
  Docsify site continue to resolve; `docs/recipes.md` is purely additive.
- **If the change is reverted before archive**, the repository returns exactly to
  its previous docs state (see Rollback). If reverted after archive, the
  composed `openspec/specs/integration-recipes/spec.md` section is removed
  alongside the page.

## Scope / affected areas

Documentation and change artifacts only:

- `docs/recipes.md` — **new** page (the whole change).
- `docs/_sidebar.md` — one Usage entry.
- `README.md` — one "Key documentation pages" entry.
- `openspec/changes/docs-integration-recipes/specs/integration-recipes/spec.md`
  — new minimal spec domain (spec phase).
- `openspec/changes/docs-integration-recipes/{proposal,design?,tasks,apply-progress,verify-report,archive-report}.md`
  — SDD artifacts.
- `openspec/specs/integration-recipes/spec.md` — archive composition (archive
  phase only).

Not touched: `crates/**`, `python/**`, `schemas/**`, `docs/schemas/**`,
`.github/workflows/**`, `Makefile`, `.markdown-link-check.json`,
`tests/**`, `docs/github-action.md`, and every other existing docs page.

## Non-goals (explicit)

1. **No CLI, library, wrapper, or schema change**; no new flag, command, or
   output field, and no change to any documented behavior contract.
2. **No new workflows files** and no change to `.github/workflows/**`; the
   Actions recipe documents the existing self-use pattern, it does not add CI.
3. **No renaming, moving, or restructuring of existing docs** (`docs/recipes.md`
   vs `docs/integration.md` naming is settled as `recipes`, matching the issue
   title), and no edits beyond the two navigation insertions.
4. **No `docs/recipes/` directory** and no per-recipe page split in this change.
5. **No recipe-execution test harness.** Running recipes in CI (a
   playground-style checker) is not added; the gates remain serve/link/schema.
6. **No new external links or badge changes**, and no `size:exception`; the
   change assumes no chain strategy beyond what the parent confirms.
7. **No changes to the compatibility playground** or its versioned CSV snapshot.

## Alternatives considered (rejected)

1. **A `docs/recipes/` directory with an index plus five pages.** Rejected —
   five new files multiply sidebar edits, splitting a ~300-line page into five
   ~60-line pages with duplicated intro/constraint boilerplate, and the diff
   grows toward and past the 400-line budget without adding review value. A
   single file keeps one coherent diff, uses Docsify anchors (`recipes.md#recipe-a-shell-pipelines`)
   for deep links, and can be split later if the page ever outgrows roughly
   double its current size. The parent resolved this decision: single file.
2. **Recipes embedded in existing pages** (pipeline examples in `cli.md`, the
   backend recipe in `security.md`, the pandas recipe in
   `python-integration.md`). Rejected — it scatters a cross-cutting workflow
   surface across reference pages, inflates pages that are already long
   (`docs/cli.md`), creates overlapping examples with `docs/github-action.md`,
   and leaves no single navigable "recipes" landing point for the sidebar/README
   link the acceptance criteria require.
3. **No spec artifact at all** (docs-only change carries no delta specs).
   Rejected by the parent decision: the archive would then report a
   user-visible documentation deliverable with no requirement artifact, which is
   not a truthful record of what was promised. The one minimal
   `integration-recipes` domain above is the smallest honest artifact.
4. **A delta spec on an existing domain** (for example adding a "documentation"
   requirement to `docx-extraction`). Rejected — no existing domain governs
   documentation, and polluting a behavior contract with docs requirements would
   make that contract misleading.
5. **Self-contained recipes with full flag tables and complete output field
   lists.** Rejected — guarantees immediate duplication drift against
   `docs/cli.md`/`docs/json-output.md`/`docs/audit.md`, which is the single
   largest maintenance risk for this page. Recipes stay thin and link.
6. **Duplicating the `docs/github-action.md` examples inside the Actions recipe
   for "completeness".** Rejected — the recipe must add pipeline value (stdout
   redirects, artifacts, a derived job summary) and link the existing examples,
   consistent with parent decision 4.
7. **Absolute `github.com/spereyra-dev/oxdoc/blob/main/.github/workflows/action.yml`
   links.** Rejected — the `markdown-link-check` ignore is a bare-prefix match,
   so blob URLs are validated live against the default branch; a rename or
   branch change would break the gate for a documentation page. The workflow is
   named in prose/code and the docs page is linked instead.
8. **A recipe-execution checker (`make docs-recipes-check`) so recipes cannot
   stale.** Deferred, not rejected — it is a genuinely useful follow-up, but it
   needs fixtures, a runner, and a new CI gate; the compatibility playground
   (`scripts/compatibility-playground.py --check`) is the existing pattern to
   imitate. Out of scope for a docs-only change (Non-goal 5).
9. **Naming the page `docs/integration.md`.** Rejected — "recipes" matches the
   issue title and the copy/paste intent; inventing a second term for the same
   surface adds vocabulary without benefit.

## Risks

- **Review budget (primary).** The estimate below lands the change at or just
  above 400 realized changed lines once the change's own spec artifact is
  counted. Mitigation: the parent's single-file decision removes five pages of
  nav churn; the page is deliberately thin on flags; recipes share one four-part
  template instead of restating schemas. The slices in the estimate give a
  ready chain boundary, and the session delivery is `auto-chain` — but the chain
  strategy itself is deferred, so if the realized diff exceeds 400 the apply
  phase must confirm the slice boundary rather than invent one. No
  `size:exception` is assumed.
- **Content drift / staleness (primary content risk).** Transcribed example
  output and any flag mentioned inline go stale when the CLI changes.
  Mitigation: the thin-on-flags rule, explicit "example output, transcribed"
  labeling, the intro scope note, and the explicit deferral of an execution
  harness (Alternative 8) recorded so the gap is known rather than silent.
- **Duplication with `docs/github-action.md`.** Mitigation: the recipe is
  required by spec to link rather than restate the existing examples, and it
  must add pipeline value (redirects, artifacts, derived summary).
- **Link-gate liveness.** Any external URL added to the page is checked for
  real by `make docs-links` and `docs.yml`. Mitigation: keep links relative
  (Docsify serves `docs/`), use the published `docs/schemas/...` mirrors, and add
  at most one already-documented GitHub URL.
- **Docsify navigation behaviour.** A wrong relative link resolves in the
  repository but 404s on the published site. Mitigation: `make docs-check`
  serves the site locally with the new page and sidebar entry in every slice.
- **Spec/artifact truthfulness.** A spec that over-promises (for example
  asserting recipe execution) would make the archive report unverifiable
  requirements. Mitigation: the six requirements above are all statically
  checkable by reading the page and running the docs gates.
- **Unverifiable command correctness.** No automated runner means a typo could
  ship. Mitigation: commands are copied from existing docs or the current
  `docs/cli.md` flag surface, cross-checked in the apply evidence trail, and
  every recipe links the owning reference page so a reader can self-verify.

## Rollback

Revert `docs/recipes.md` (delete), the one `docs/_sidebar.md` Usage line, the
one `README.md` list line, and — before archive — the change directory's spec
artifact. Nothing else in the repository changed: no code, no schema, no
snapshot, no workflow, no link-check configuration, no published URL. There is no
runtime state, no data migration, and no consumer contract to unwind, so the
revert is a pure file-level undo and both the Docsify site and the docs gates
return to their previous state immediately. Post-archive, the same revert also
removes the composed `integration-recipes` section from
`openspec/specs/integration-recipes/spec.md`.

## Size estimate (vs 400-line review budget)

Raw line counts for the prose/command skeleton, then an honest 1.5× multiplier on
example and output blocks (the post-#177/#180 lesson: transcribed output blocks,
YAML/Python snippets, and fenced examples land well above their first estimate).

| Slice | Content | Raw | Realistic |
| --- | --- | --- | --- |
| Recipe A — shell pipelines | goal, 5–6 command blocks, JSONL/JSON output blocks, exit-code + warning-channel blurb, links | 55–75 | **~75–105** |
| Recipe B — GitHub Actions | goal, job skeleton YAML, output/artifact notes, self-use pattern, link to `github-action.md` | 35–50 | **~50–75** |
| Recipe C — Python / pandas | goal, install block, `Oxdoc` + pandas snippet, expected frame/record, wrapper constraints, links | 45–65 | **~65–95** |
| Recipe D — backend ingestion | goal, limited-batch command, typed-error and warning-JSON blocks, stdin/no-spill constraint, diagnostics, links | 45–65 | **~65–95** |
| Recipe E — audit workflow | goal, `json`/`jsonl`/`text` commands, triage filter, output blocks, mutual-exclusion constraint, links | 40–60 | **~60–90** |
| Page intro + template + closing reference list | scope note, how-to-read, reference links | 20–25 | **~20–30** |
| `docs/_sidebar.md` + `README.md` | two link insertions | 8 | **~10** |
| **Documentation total** | | **~250–340** | **~345–500** |
| Change artifacts (`spec.md`, `design.md` if the spec phase adds one, plus this proposal and `tasks.md`) | requirements + scenarios + task list | 70–110 | **~90–150** |
| **Change total** | | | **~435–650** |

Honest reading: the documentation page alone is designed to land around
**350–420 realized lines**, with the two nav insertions adding ~10. Counting the
change's own SDD artifacts, the change sits at or above the 400-line budget, so
**the apply phase should measure the realized diff and, if it exceeds 400, chain
along this boundary** (chain strategy to be confirmed at that point, not
invented here):

1. **Slice 1 — page + Recipes A/B/C + navigation links + spec artifact.** The
   page exists, is reachable from the sidebar and README, and covers three of the
   five workflows; every gate is green. ~200–290 realized lines.
2. **Slice 2 — Recipes D/E appended to the same page.** Completes coverage; the
   page diff is additive and independently reviewable. ~125–185 realized lines.

Both slices are documentation-only and leave `make docs-check`, `make docs-links`,
`make docs-schemas-check`, and `make docs-playground-check` green on their own.
If the realized total lands under 400, the change ships as a single PR.

## Success criteria

1. `docs/recipes.md` exists and contains all five recipe categories (shell
   pipelines, GitHub Actions, Python/pandas, backend ingestion, audit), each
   using the same goal / command / example-output / constraints / see-also shape
   (AC 1).
2. Every recipe contains at least one fenced block explicitly labeled as example
   output, and every block is either transcribed from `docs/cli.md`,
   `docs/json-output.md`, or `docs/audit.md`, or labeled illustrative (AC 2).
3. Every recipe states at least one applicable documented constraint: no
   rendering/pagination/PDF, formulas never recalculated, warnings on stderr
   with stdout stream-clean, no stdin temp-file spill, typed limit failures
   `E014`/`E011`/`E005`/`E006`, and audit output is factual signals only (no
   scoring) (AC 2).
4. The Actions recipe links `docs/github-action.md` and does **not** reproduce
   its three examples; the Python recipe documents that the wrapper shells out
   and does not bundle the binary; the backend recipe documents that stdin is
   buffered only up to `--max-input-size` and never spilled to a temp file.
5. `docs/_sidebar.md` has exactly one new Usage entry pointing at `recipes.md`,
   and `README.md` has exactly one new "Key documentation pages" entry pointing
   at `docs/recipes.md` (AC 3); no other existing docs page is modified.
6. `make docs-links` passes for `README.md` and all of `docs/**/*.md`, including
   the new page (AC 4), and `make docs-check`, `make docs-schemas-check`, and
   `make docs-playground-check` remain green.
7. No file under `crates/`, `python/`, `schemas/`, `docs/schemas/`,
   `.github/workflows/`, or `tests/` is modified; `git diff --stat` touches only
   `docs/recipes.md`, `docs/_sidebar.md`, `README.md`, and the change's openspec
   directory.
8. `openspec/changes/docs-integration-recipes/specs/integration-recipes/spec.md`
   contains the six requirements above with scenarios, and archive composes
   exactly one new `openspec/specs/integration-recipes/spec.md` domain without
   touching the four existing spec domains.
9. Every recipe's flags/fields are reachable through a link to the owning
   reference page, with no restated flag table or duplicated output-field list.
10. Realized changed lines stay within 400 in one PR, **or** the apply phase
    measured the diff and the work landed as the two slices above under the
    session's `auto-chain` delivery (with the chain boundary confirmed, not
    assumed) — no `size:exception` is claimed.

## Parent decision traceability

| Parent decision | Where honored |
| --- | --- |
| 1. Single file `docs/recipes.md`; link from `docs/_sidebar.md` (Usage) and README docs list | Solution shape §1–§2; Alternatives 1; Success criterion 5 |
| 2. Docs-only, no delta specs on behavior contracts; one minimal spec under `openspec/changes/docs-integration-recipes/specs/` (domain `integration-recipes`) stating the documentation requirements so the archive is truthful | Solution shape §3; Alternatives 3–4; Success criterion 8 |
| 3. Five recipes, each with goal, copy/paste-ready command/workflow, example output transcribed from existing docs, and a safety/non-rendering blurb; the exact five surfaces (A–E) with the named flags, APIs, and codes | Solution shape §1 (Recipes A–E); Acceptance-criteria mapping; Success criteria 1–4 |
| 4. Recipes stay thin on flags and link `cli.md`/`json-output.md`; no duplication of existing doc examples | Solution shape §4; Alternatives 5–6, 9; Success criterion 9 |
| 5. Non-goals: no CLI/code changes, no schema changes, no new workflows files, no renaming of existing docs | Non-goals 1–3, 6–7; Backwards compatibility; Success criterion 7 |

## Proposal question round

The five product decisions above are parent-resolved and treated as
authoritative; this section does not re-open them. These are the remaining
product-facing refinements that the design/apply phases should not silently bake
in. Answer, correct, frame differently, skip, or ask for a second round:

1. **Audience weighting across the five recipes.** Recipe D (backend ingestion
   with typed limit failures) and Recipe E (audit triage) assume an
   operations/security audience, while Recipes A/C assume a data-engineering
   audience. Should all five be pitched at the same reader level (a competent
   engineer who has never opened oxdoc), or should D/E explicitly address the
   platform/security owner deciding limits and review policy?
2. **The one "beyond reference" artifact per recipe.** Recipe B is specified to
   add a derived job summary, and Recipe E an audit triage filter, so no recipe
   merely repeats an existing example. Is a derived job summary (or a PR-comment
   pattern) the right pipeline value for the Actions recipe, or would the
   maintainers rather the Actions recipe stay minimal and only point at
   `docs/github-action.md` plus artifact upload?
3. **Triage filter choice for the audit recipe.** Recipe E needs one concrete
   "which documents do I escalate" step. Is a `jq`/severity filter acceptable as
   the canonical example (it may not be installed in every environment), and
   should the recipe show the `--format text` human path as the primary and the
   JSON filter as secondary, or the reverse?
4. **Illustrative output honesty.** Where upstream docs have no output sample
   (Actions logs, pandas `DataFrame` repr, a job summary), the proposal labels
   the block illustrative and keeps it minimal. Is that acceptable, or would the
   maintainers prefer such blocks to be omitted entirely so the page only ever
   shows verifiable output?
5. **Where the page sits in reading order.** The sidebar entry is appended to
   Usage after Python Integration; the README entry after Python Integration.
   Should Integration Recipes instead appear in the **Start** section right after
   Getting Started (as a primary onboarding path), or in Usage as proposed?
6. **Anchor stability.** Recipe headings will be the Docsify deep-link targets
   (`#recipe-a-shell-pipelines`, ...). Should the headings be locked as stable
   anchors in the spec (adding an explicit requirement), or is heading text free
   to change between now and archive?
