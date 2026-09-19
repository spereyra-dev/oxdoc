# Proposal — publish the Rust crates to crates.io (issue #173)

- Change id: `release-crates-io-publish`
- Status: proposed (SDD propose phase, artifact store: openspec)
- Inputs: `openspec/changes/release-crates-io-publish/exploration.md`, GitHub
  issue #173 acceptance criteria (as reflected in the exploration's §6 gap list —
  see "Acceptance-criteria mapping" for the sourcing caveat), parent-resolved
  product decisions (authoritative; traceability table below), `openspec/config.yaml`,
  the current `Cargo.toml` manifests, `CHANGELOG.md`, `.github/workflows/release.yml`,
  `.github/workflows/publish-python.yml` (approved-version gate precedent),
  `docs/release-process.md`, `docs/discoverability.md`, `docs/launch-publicity.md`,
  `docs/roadmap.md`, `ROADMAP.md`, `README.md`, and `.markdown-link-check.json`.
- Delivery: session preflight (`auto-chain` default, artifact store openspec,
  400-line review budget, publish is a human-controlled gate). The chain/exception
  decision is **not** made here; review-budget risk routes to the parent's
  `ask-on-risk` gate.
- This change is release engineering: it edits metadata, versions, changelog, and
  documentation, and it ends with a **maintainer-executed, irreversible**
  `cargo publish` sequence. No runtime behavior, CLI surface, or output contract
  changes.

## Intent / problem

`oxdoc` has never been published to crates.io, yet the repository already behaves
and documents as if it had:

1. **The channel is announced but dead.** `docs/discoverability.md` lists
   "crates.io CLI" under **Published Channels**, `README.md` carries a live
   `crates.io` badge for `oxdoc-cli`, `docs/installation.md` opens with "can be
   installed from GitHub Release binaries, crates.io, or source", and
   `crates/oxdoc-cli/README.md` and `python/README.md` tell users to run
   `cargo install oxdoc-cli`. The link-check configuration carries an
   `^https://crates.io/crates/oxdoc-cli/?$` ignore pattern precisely because the
   page 404s, and `docs/roadmap.md` / `ROADMAP.md` still list crates.io
   publication as pending work. Users following the docs today hit a dead end.
2. **The publishable metadata is incomplete.** `crates/oxdoc-tabular/Cargo.toml`
   has no `readme` key (its README exists but is never rendered on the crate
   page), no `include` list (the two sibling crates have one), and no
   `[package.metadata.docs.rs]` metadata, so the optional `parquet` module would
   not render on docs.rs under default features. One crate README also links
   outside the package with a relative path that cannot resolve on crates.io.
3. **There is no repeatable release flow.** `docs/release-process.md` states the
   dependency order and shows three `cargo publish --dry-run` commands, and it
   already documents the `cargo package -p oxdoc-tabular --no-verify` workaround,
   but there is no ordered, gate-checked checklist that ties version approval,
   packaging, dry-runs, tagging, the real publish, and receipts together.
   `docs/launch-publicity.md` still describes the 1.0 publish order and omits
   `oxdoc-tabular` entirely.
4. **The release version was undecided and is now decided.** `CHANGELOG.md` holds
   a large `Unreleased` section containing breaking changes (the
   `oxdoc-core::XlsxCell` source break and two versioned output-contract breaks),
   which cannot be published without a semver decision. The maintainer has now
   approved the release versions (below).
5. **Nobody has verified the packages.** `cargo package` and
   `cargo publish --dry-run` have not been run in this change's exploration
   (no shell access), and a stale version literal in a snapshot fixture means a
   version bump will break `cargo test --workspace` unless it is updated in the
   same commit.

The value: `cargo install oxdoc-cli` actually works, the crate pages render
READMEs and parquet docs, the three crates are published in the only order the
dependency graph allows, an irreversible registry operation is preceded by a
verified, reviewable checklist, and every public claim about crates.io is true
both before and after the publish.

## Approved release decision (parent-resolved, authoritative)

Maintainer decision (2026-09-18), treated as fixed by this proposal:

| Crate | From | To | Semver rationale |
| --- | --- | --- | --- |
| `oxdoc-core` | 1.2.0 | **2.0.0** | `XlsxCell` gains `formula: Option<XlsxFormula>` — a source break for external struct literals; after 1.0 the project uses strict semver, so this is a major bump. |
| `oxdoc-tabular` | 0.1.0 | **0.2.0** | Pre-1.0 crate whose public surface re-exports core types; a 0.x minor is the honest signal for that break (`^0.2` still isolates consumers from 0.3). |
| `oxdoc-cli` | 1.2.0 | **2.0.0** | Documented CLI output breaks: rows-JSONL v2 and structured-JSON v2 payloads. |

Consequences carried by this change:

- Every in-repo cross-reference moves with it: `oxdoc-tabular → oxdoc-core = "2.0.0"`,
  `oxdoc-cli → oxdoc-core = "2.0.0"`, `oxdoc-cli → oxdoc-tabular = "0.2.0"`
  (a 0.x requirement must name the exact minor, so `"0.2.0"` is required, not
  `"0.1.0"`), plus `Cargo.lock`, `fuzz/Cargo.lock`, the docs/library install
  snippets, and the crate README snippet.
- `CHANGELOG.md`: the `Unreleased` section is dated as the `2.0.0` release
  section with its documented breaking changes intact and a one-line note naming
  the three crate versions (so the tabular 0.2.0 difference is discoverable).
- Tags are cut **at publish time** (`v2.0.0`, matching the repository's
  `vMAJOR.MINOR.PATCH` convention and the CLI/core version), not in the
  version-bump commit.
- The real `cargo publish` runs **locally from the maintainer's machine** after
  the checklist and dry-run validation; the orchestrator confirms the version
  triple once more immediately before publishing. **No `CARGO_REGISTRY_TOKEN` is
  added to CI**; CI-based publishing is documented as a future prerequisite.

## Solution shape

### 1. Version bump with every in-repo reference and a required fixture fix

Exact edits (all in one commit, slice 1):

| File | Edit |
| --- | --- |
| `crates/oxdoc-core/Cargo.toml` | `version = "2.0.0"` |
| `crates/oxdoc-tabular/Cargo.toml` | `version = "0.2.0"`; `oxdoc-core` requirement `"2.0.0"` |
| `crates/oxdoc-cli/Cargo.toml` | `version = "2.0.0"`; `oxdoc-core` requirement `"2.0.0"`; `oxdoc-tabular` requirement `"0.2.0"` |
| `Cargo.lock` | regenerated by `cargo check --workspace --all-features` (three version lines) |
| `fuzz/Cargo.lock` | `oxdoc-core` path-dep entry `1.2.0` → `2.0.0` (regenerated via `cargo metadata --manifest-path fuzz/Cargo.toml` or a local `cargo fuzz build`) |
| `CHANGELOG.md` | `## Unreleased` → `## 2.0.0 - <release date>` + crate-version note |
| `tests/fixtures/snapshots/cli_info_json.json` | `"oxdoc_version": "1.2.0"` → `"2.0.0"` — **required**: `crates/oxdoc-cli/tests/cli.rs::prints_info_as_json_and_text` compares the full JSON value against this snapshot, so the bump breaks the suite without it |
| `crates/oxdoc-core/tests/schema.rs` | representative payload literal `"oxdoc_version": "1.2.0"` → `"2.0.0"` (schema-valid either way; kept current for consistency) |
| `docs/library-api.md`, `crates/oxdoc-tabular/README.md` | dependency snippet `version = "0.1.0"` → `"0.2.0"` |
| `docs/cli.md`, `docs/json-output.md`, `docs/audit.md`, `docs/recipes.md` | example payload `"oxdoc_version": "1.2.0"` → `"2.0.0"` (four one-line examples that would otherwise be stale) |
| `README.md` | `Status` line "stable 1.x CLI and Rust API contract" → 2.x wording; tabular description drops "unpublished" **after** publish (see §4) |

Deliberately **not** changed: `python/pyproject.toml` (0.1.0) and
`.github/workflows/python-package.yml`'s python version assertion — the Python
wrapper is unaffected by this change.

### 2. Crate metadata completion (tabular, aligned with its siblings)

`crates/oxdoc-tabular/Cargo.toml` gains, matching the `oxdoc-core`/`oxdoc-cli`
patterns:

```toml
readme = "README.md"
include = ["Cargo.toml", "README.md", "src/**", "examples/**"]

[package.metadata.docs.rs]
features = ["parquet"]
```

- `readme` — without it crates.io renders the package page with no README even
  though `crates/oxdoc-tabular/README.md` exists.
- `include` — the sibling pattern; today the directory is clean, so this closes
  a future drift risk rather than a current leak. It covers both examples
  (`examples/xlsx_to_parquet.rs`, `examples/tabular_gate.rs`), which are
  feature-gated by the manifest and must ship for the crate to build.
- `[package.metadata.docs.rs] features = ["parquet"]` — explicit, minimal, and
  deliberate: it renders the optional parquet module on docs.rs. `all-features = true`
  is rejected (§Alternatives) because it silently widens the docs build to
  every future optional feature without review.
- `crates/oxdoc-tabular/README.md` — the closing link
  ``[`docs/spikes/xlsx-arrow-parquet.md`](../../docs/spikes/xlsx-arrow-parquet.md)``
  escapes the package and cannot resolve on the crates.io page; it becomes an
  absolute repository link. This is the only relative link in any crate README.

Metadata verification (acceptance criterion, checked per crate, not just
per-field presence):

| Field | Source | Expected |
| --- | --- | --- |
| `description` | each crate manifest | crate-specific, non-empty |
| `license` | `workspace = true` | `MIT` (SPDX) |
| `repository`, `homepage` | `workspace = true` | `https://github.com/spereyra-dev/oxdoc` |
| `readme` | crate-relative | `README.md` for all three, file present and rendering |
| `keywords` | workspace | `["docx","xlsx","ooxml","parser","cli"]` — exactly crates.io's max of 5 and all lowercase |
| `categories` | workspace | `["command-line-utilities","parser-implementations"]` — valid crates.io slugs |
| `rust-version` | workspace | `1.88` |
| `include` | per crate | present for all three; `cargo package --list` shows no tests, fixtures, or workspace-only files |
| `publish` | per crate | not `false` (`oxdoc-tabular` currently sets `publish = true` explicitly) |
| docs.rs metadata | tabular only | `features = ["parquet"]` |
| README links | all three | no link escapes the package; the only external/relative link (tabular spike doc) is absolute |

### 3. One documented, ordered release checklist (source of truth)

The checklist lands in the **existing** `docs/release-process.md` §Crates.io
Publishing (that section already owns the dependency order, the dry-run commands,
and the `--no-verify` caveat), expanded into the exact ordered flow in the
"Exact ordered publish flow" section below. Content requirements:

- The approved-version gate, mirroring the *semantics* of
  `.github/workflows/publish-python.yml` (a maintainer-approved version asserted
  against the manifests before any build/publish) with local, read-only commands:

  ```bash
  cargo pkgid -p oxdoc-core     # expect …#oxdoc-core@2.0.0
  cargo pkgid -p oxdoc-tabular  # expect …#oxdoc-tabular@0.2.0
  cargo pkgid -p oxdoc-cli      # expect …#oxdoc-cli@2.0.0
  grep -n '^## 2.0.0' CHANGELOG.md
  ```

- Clean-tree precondition (`git status --porcelain` empty) before each real
  publish, because `cargo publish` refuses a dirty work tree by default.
- Package-then-dry-run per crate in dependency order, including the
  **resolution caveat**: `oxdoc-tabular` and `oxdoc-cli` cannot fully verify
  until their upstream crate exists on the registry, so pre-publish validation is
  `cargo package … --no-verify` plus a file-list inspection, and the full
  `cargo publish --dry-run` is repeated for each crate immediately after its
  upstream is live. Sparse-index propagation lag is called out with the retry
  instruction ("wait and re-run; never skip a dry-run").
- Tag + push, then the real publish per crate with a verification command per
  crate and receipts recorded in the change artifacts.
- A short recovery subsection: published versions can only be **yanked**, never
  replaced or deleted; a wrong release is fixed forward with a patch version
  (which re-cascades the requirement bumps), and `cargo yank` is the documented
  last resort.
- A short "CI publishing (future prerequisite)" subsection: crates.io has no
  OIDC trusted-publishing flow for cargo, so a CI job would need a
  `CARGO_REGISTRY_TOKEN` repository secret plus a `workflow_dispatch`
  approved-version input asserted against all three manifests (modeled on
  `publish-python.yml`) and dependency-ordered jobs, with token rotation and
  revocation documented. Explicitly deferred by this change.

A new `docs/releasing.md` is rejected (Alternatives 2) because
`docs/release-process.md` already owns the crates.io order and the `--no-verify`
caveat; splitting them would create two sources of truth that drift.

### 4. Documentation reconciliation with explicit pre/post-publish truth

crates.io state is described honestly at every commit, so the docs never claim a
channel that 404s:

| Surface | Before publish (slice 1) | After publish (slice 2) |
| --- | --- | --- |
| `docs/discoverability.md` | crates.io CLI moves out of **Published Channels** into a "Publishing with 2.0.0" entry under **Channels Not Yet Published**, with the activation steps (approved version, ordered dry-runs, receipts) and the note that the README badge returns with the release | crates.io CLI returns to **Published Channels** with the published versions and the install command; "Channels Not Yet Published" drops the entry |
| `README.md` | the `crates.io` badge is removed (the page's own policy: "Only add a README badge after its destination is public") | the badge is restored, verified against the live crate page |
| `docs/launch-publicity.md` | crates.io section rewritten to the real dependency order including `oxdoc-tabular`, with the 2.0.0 versions and a pointer to the release checklist; the outreach checklist item names all three crates | unchanged (already accurate) |
| `docs/roadmap.md` + `ROADMAP.md` | unchanged (still pending work, which is true) | Phase 5 bullet "Publish `oxdoc-core`, `oxdoc-tabular`, and `oxdoc-cli` to crates.io in dependency order" flips to implemented |
| `.markdown-link-check.json` | keep the `^https://crates.io/crates/oxdoc-cli/?$` ignore (the URL is still a not-yet-live destination) | remove the ignore so `make docs-links` validates the now-live crates.io URL (flagged as a decision in the question round because it trades a defense for a potential CI flake against crates.io) |
| `README.md` tabular description | unchanged | drops "unpublished" from the `oxdoc-tabular` description |
| `docs/installation.md` | unchanged | verified only: its crates.io sentence becomes true at publish; no edit needed |

`docs-links` runs against `README.md` and `docs/**/*.md`, so every reconciliation
edit must keep that gate green.

### 5. Verification: local gates plus publish receipts

- **Rust gates unchanged and enforced**: `make ci` (or at minimum
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features --all-targets`,
  `cargo clippy --workspace --all-features --all-targets -- -D warnings`,
  `cargo test --workspace --all-features --all-targets`, `cargo test --doc --workspace --all-features`,
  coverage ≥ 95) plus `scripts-test`, `docs-check`, `docs-links`,
  `docs-schemas-check`, `docs-playground-check`. The version bump must leave
  coverage unchanged (no Rust logic changes; test/literal edits only).
- **Packaging receipts** (per crate): `cargo package -p <crate> [--no-verify]`,
  `cargo package -p <crate> --list` (file-list evidence that nothing
  workspace-only ships), and `cargo publish -p <crate> --dry-run` where
  resolution allows.
- **Publish receipts** (per crate): the real publish output plus a registry check
  (`cargo search oxdoc-core --limit 1`, and/or the crates.io API JSON) showing
  2.0.0 / 0.2.0 / 2.0.0, and one end-to-end receipt:
  `cargo install oxdoc-cli --version 2.0.0` in a clean target directory.
- Receipts are recorded in `openspec/changes/release-crates-io-publish/apply-progress.md`
  (a "publish receipts" section) and cited by the verify report; the final
  publish is a human gate and its receipts are the only proof of the deliverable.

### 6. Spec artifact

A new spec domain is authored in the spec phase at
`openspec/changes/release-crates-io-publish/specs/crates-io-release/spec.md`
(no existing domain is touched; no behavior contract changes). The proposal fixes
its contract:

| Requirement | Content |
| --- | --- |
| Approved-version consistency | The three crate manifests, their mutual version requirements, `Cargo.lock`, `fuzz/Cargo.lock`, the CHANGELOG release section, and in-repo install snippets all agree on 2.0.0 / 0.2.0 / 2.0.0 |
| Publishable metadata | Each crate ships description, MIT license, repository, homepage, a crate-local README, a non-leaking `include` list, ≤5 lowercase keywords, valid categories, `rust-version`, and `cargo package --list` evidence; `oxdoc-tabular` additionally enables the parquet feature for docs.rs |
| Ordered release checklist | `docs/release-process.md` documents the exact ordered flow (version gate → package → dry-run → tag/push → publish in dependency order → receipts → docs reconciliation), including the tabular/CLI resolution caveat with the `--no-verify` workaround, the clean-tree precondition, and yank-based recovery |
| Local publish with approval gate | The change adds no CI token or secret; the real publish is maintainer-approved and locally executed in dependency order, with the approved version re-confirmed immediately before the first real publish |
| Docs truthfulness | Every public claim about crates.io state is true at the commit that carries it: docs and the README badge describe crates.io as upcoming before publish and as published (with versions) after, and the link/serve gates stay green |
| Publish receipts | Registry-verified evidence exists for all three published versions (2.0.0 / 0.2.0 / 2.0.0), plus a clean install of `oxdoc-cli` 2.0.0, recorded in the change artifacts |
| No behavior or CI change | No runtime/CLI/output behavior change, no schema change, no workflow change, no new publish automation |

## Exact ordered publish flow

The authoritative sequence, as it will be documented in
`docs/release-process.md`. Steps 1–6 are slice 1 (pre-publish), steps 7–9 are the
human-gated publish, step 10 is slice 2. "Gate" marks a step that must stop on
failure or missing human approval.

| # | Step | Command / evidence | Gate |
| --- | --- | --- | --- |
| 0 | Preconditions | clean `main`, CI green, local cargo credentials loaded, maintainer authorizes publishing | **human** |
| 1 | Version bump commit | the manifest/lock/CHANGELOG/README/doc edits in §1 | — |
| 2 | Approved-version gate | `cargo pkgid -p oxdoc-core/tabular/cli` and `grep -n '^## 2.0.0' CHANGELOG.md` match the approved triple | **stop on mismatch** |
| 3 | Metadata gate | tabular `readme`/`include`/docs.rs metadata present; per-crate metadata table in §2 checked | **stop on gap** |
| 4 | Full local gate | `make ci` (fmt, check, clippy, test, doctest, coverage ≥ 95, scripts-test, docs-check, docs-links, docs-schemas-check, docs-playground-check) | **stop on red** |
| 5 | Review + merge | release PR reviewed; merge to `main` | **human** |
| 6 | Package + dry-run | `cargo package -p oxdoc-core` (full verify) · `cargo package -p oxdoc-tabular --no-verify` · `cargo package -p oxdoc-cli --no-verify` · `--list` inspection per crate · `cargo publish -p oxdoc-core --dry-run` (full) | **stop on failure** |
| 7 | Tag + push | `git tag -a v2.0.0 -m "oxdoc 2.0.0"` on the merged commit · `git push origin v2.0.0` → `release.yml` builds/ships binaries (note: the GitHub Release can complete before crates.io) | — |
| 8 | Confirm + publish core | orchestrator/maintainer re-confirms `2.0.0 / 0.2.0 / 2.0.0` · `git status --porcelain` empty · `cargo publish -p oxdoc-core` · verify `cargo search oxdoc-core --limit 1` = 2.0.0 | **human** |
| 9 | Publish tabular then CLI | `cargo publish -p oxdoc-tabular --dry-run` (now resolves core 2.0.0; retry after index lag) → `cargo publish -p oxdoc-tabular` → verify 0.2.0 · `cargo publish -p oxdoc-cli --dry-run` → `cargo publish -p oxdoc-cli` → verify 2.0.0 · end-to-end `cargo install oxdoc-cli --version 2.0.0` in a clean target dir | **stop between crates** |
| 10 | Post-publish reconciliation | docs/README/badge/roadmap/link-check flip (§4) · `make docs-links` · receipts recorded in `apply-progress.md` | — |

Failure handling at any publish step: do not re-publish the same version
(permanent `crate version already exists`), resume from the failed crate after
fixing the cause, and fix forward with a patch version if the damage is already
live; `cargo yank --version <v> -p <crate>` is the documented last resort (it
hides the version from new resolution without deleting it).

## Acceptance-criteria mapping

**Sourcing caveat:** the parent context states issue #173 carries the acceptance
criteria, but they are not quoted verbatim anywhere in this change's inputs — the
exploration (§6, "Gaps vs acceptance criteria") enumerates five criteria in
prose. The mapping below uses that enumerated set; the apply/verify phase must
re-read the live issue text and confirm the set before claiming AC completion.

| Acceptance criterion (exploration §6) | Slice that satisfies it | How it is checked |
| --- | --- | --- |
| 1. Metadata verification (tabular `readme`; tabular include list; docs.rs metadata for parquet; README links resolve on crates.io) | Slice 1 (§2) | Per-crate metadata table; `cargo package --list`; crate README link audit (no escaping relative links) |
| 2. Repeatable release checklist / dry-run flow | Slice 1 (§3 + exact flow) | The ordered checklist exists in `docs/release-process.md` including the `--no-verify` caveat, version gate, tag step, and receipts |
| 3. Validate `cargo package` / `cargo publish --dry-run` for every crate in dependency order | Slice 1 step 6 + step 9 | Recorded package/dry-run receipts; `--no-verify` used only where registry resolution makes full verification impossible pre-publish |
| 4. Publishing gate (approved version + documented prerequisites; real publish deferred to the maintainer) | Slice 1 §3 + step 8 human gate | `cargo pkgid` gate + explicit re-confirmation immediately before `cargo publish`; no `CARGO_REGISTRY_TOKEN` in CI |
| 5. Docs consistency (`discoverability.md`, `launch-publicity.md`, README badge policy) | Slice 1 (pre-publish truth) + slice 2 (post-publish flip) | Docs tables in §4; `make docs-links` green; badge target verified live after publish |

## Scope / affected areas

Slice 1 — version, metadata, checklist, pre-publish docs (one PR):

- `crates/oxdoc-core/Cargo.toml`, `crates/oxdoc-tabular/Cargo.toml`, `crates/oxdoc-cli/Cargo.toml`
- `Cargo.lock`, `fuzz/Cargo.lock`
- `CHANGELOG.md`
- `tests/fixtures/snapshots/cli_info_json.json`, `crates/oxdoc-core/tests/schema.rs`
- `crates/oxdoc-tabular/README.md`
- `README.md`, `docs/library-api.md`, `docs/cli.md`, `docs/json-output.md`, `docs/audit.md`, `docs/recipes.md`
- `docs/release-process.md`, `docs/discoverability.md`, `docs/launch-publicity.md`
- `openspec/changes/release-crates-io-publish/specs/crates-io-release/spec.md` (spec phase)

Slice 2 — post-publish reconciliation (after the human-gated publish):

- `docs/discoverability.md`, `README.md`, `docs/roadmap.md`, `ROADMAP.md`, `.markdown-link-check.json`
- `openspec/changes/release-crates-io-publish/apply-progress.md` (receipts)

Not touched: `crates/**/src/**`, `schemas/**`, `docs/schemas/**`, `python/**`,
`.github/workflows/**`, `Makefile`, `tests/fixtures/**` other than the one
snapshot, `action.yml`, and `install.sh`.

## Non-goals (explicit)

1. **No CI token or secret additions** — no `CARGO_REGISTRY_TOKEN`, no publish
   workflow, no OIDC attempt; CI publishing is documented as a future
   prerequisite only (parent decision 2).
2. **No new automation for the publish itself** — no Makefile target, no shell
   script, no cron; the checklist is executable documentation.
3. **No Python package version change** — `python/pyproject.toml` stays 0.1.0 and
   `python-package.yml` stays untouched.
4. **No behavioral code changes** — no public API, CLI, output, schema, warning,
   or error change; schemas `v1`/`v2` are untouched.
5. **No workflow changes** — `release.yml`, `publish-python.yml`, `ci.yml`, and
   the rest stay byte-identical; the tag simply triggers the existing release
   workflow.
6. **No new documentation site page** (`docs/releasing.md`) and no docs
   restructuring.
7. **No crates.io ownership/team changes** and no new crate name; the three
   existing crate names are published as-is.
8. **No `docs.rs` badge, download badge, or new README badges**; only the
   existing crates.io badge's lifecycle is reconciled.
9. **Not touched: `.agents/product-marketing-context.md`**, whose
   "1.0 release and crates.io publication are next" line becomes stale — it is
   internal agent context, not a published surface, and is out of this change's
   scope.

## Alternatives considered (rejected)

1. **CI publish job with a `CARGO_REGISTRY_TOKEN` secret (the `publish-python.yml`
   shape) — rejected in favor of the local maintainer publish.** A Rust equivalent
   of `publish-python.yml` is genuinely attractive (repeatable, auditable, no
   local state), but: (a) crates.io has no OIDC trusted-publishing flow for
   cargo, unlike PyPI's `id-token: write` path that the house already uses, so it
   would require a long-lived, high-privilege registry token stored in the
   repository; (b) crates.io versions are immutable, so a CI job that publishes on
   a tag push converts any tag mistake into a permanent registry artifact with no
   human in the loop; (c) the three-crate dependency order plus index-propagation
   waits make a single CI job fragile, and a matrix job multiplies the failure
   surface; (d) the parent's approved-version gate already requires a maintainer
   decision immediately before publishing, which a CI job cannot express any more
   strongly than an input assertion. The change therefore publishes locally with
   credentials the maintainer already holds, and documents the CI path
   (token + approved-version input + dependency-ordered jobs + rotation policy)
   as a future prerequisite instead of implementing it.
2. **A new `docs/releasing.md` page — rejected.** `docs/release-process.md`
   already owns the crates.io order, the dry-run commands, the `--no-verify`
   caveat, and the per-crate keep-list; a parallel page would immediately become a
   second source of truth for the same irreversible procedure. Extending the
   existing section keeps one owner, one review diff, and no need to add a
   sidebar/README entry.
3. **A CI dry-run-only workflow (no token) that runs `cargo package` +
   `cargo publish --dry-run` on release tags — deferred, not rejected.** It would
   add real value later, but `oxdoc-tabular`/`oxdoc-cli` dry-runs cannot fully
   verify before their upstream is published, so the job would either fail
   confusingly or need `--no-verify` (already covered locally), and it violates
   non-goal 5 for this change. It is the natural follow-up once CI publishing is
   pursued.
4. **A `make release-crates` target or helper script — rejected.** The operation
   is irreversible and needs per-crate human confirmation with index-propagation
   waits; wrapping it in a single shell invocation hides exactly the checkpoints
   the release needs. Read-only gates stay as documented commands.
5. **Publishing all three crates as `2.0.0` for a uniform tag — rejected.**
   `oxdoc-tabular` is a pre-1.0 crate whose consumers read 0.x semantics; a
   gratuitous 2.0.0 would misstate its maturity and lock future minor breaks into
   a major-looking number. The mixed triple (2.0.0 / 0.2.0 / 2.0.0) is the
   semver-honest set, with `v2.0.0` as the repository tag.
6. **Bumping only `oxdoc-core` to 2.0.0 and leaving tabular/CLI at 1.2.0/0.1.0 —
   rejected.** `oxdoc-cli` publishes documented output-contract breaks (rows-JSONL
   v2, structured-JSON v2), so its version must move; `oxdoc-tabular`'s public
   surface re-exports core types, so its 0.x minor must move and the CLI's
   requirement must name `0.2.0`.
7. **Publishing straight from the release commit without `cargo package`
   pre-inspection — rejected.** Registry versions cannot be replaced; the
   `--list` file inspection and the full dry-run on `oxdoc-core` are the only
   cheap safety nets before an irreversible upload.
8. **Publishing crates before tagging (so the GitHub Release never advertises an
   unpublished crate) — noted, not adopted.** The parent-fixed order is
   tag+push then publish. The proposal documents the consequence (the GitHub
   Release can go public before crates.io resolves, and `release.yml`'s
   validation job acts as a final gate on the tagged commit) and raises it in the
   question round rather than silently reordering an authoritative decision.
9. **Removing the `.markdown-link-check.json` crates.io ignore immediately —
   rejected for slice 1.** The URL is still a not-yet-live destination while the
   crate is unpublished; the ignore is removed only in the post-publish slice,
   where the crates.io URL becomes a real, checkable link.
10. **Adding docs.rs metadata to all three crates — rejected.** `oxdoc-core` and
    `oxdoc-cli` have no optional features, so the metadata would be inert
    configuration; it is added only where it changes the published docs.

## Risks

- **Registry immutability (highest).** A published version can never be replaced
  or deleted, only yanked. Mitigation: the approved-version gate, full
  `cargo package` on `oxdoc-core`, `--list` inspection on all three, full dry-run
  before each real publish, a clean-tree requirement, and a per-crate human stop
  in the documented flow.
- **Version-reference miss (high, mechanical).** The bump touches manifests,
  three version requirements, two lockfiles, a snapshot fixture, and several doc
  examples; the snapshot would break `cargo test --workspace` if forgotten.
  Mitigation: the explicit file table in §1 and `make ci` as the gate before
  review.
- **Publish ordering.** Publishing out of dependency order fails
  (`oxdoc-tabular`/`oxdoc-cli` cannot resolve unpublished upstreams). Mitigation:
  the fixed core → tabular → cli order in the checklist, the `--no-verify`
  pre-check, and the index-lag retry note.
- **Tag-before-crates ordering.** Pushing `v2.0.0` starts `release.yml`, so the
  GitHub Release (which tells users to `cargo install oxdoc-cli`) can be public
  before the crate resolves. Mitigation: the crate sequence follows immediately
  the same session, and the checklist records the ordering consequence; the
  question round offers the maintainer a chance to invert it.
- **Split-brain docs state (medium).** Pre-publish "upcoming" wording plus a
  post-publish flip means two commits in one change; skipping slice 2 leaves the
  docs claiming crates.io is unpublished forever and the README without its
  badge. Mitigation: slice 2 is an explicit success criterion with receipts, and
  the publish is the change's terminal human gate.
- **Human gate dependency (medium).** The change cannot be verified complete
  until the maintainer actually publishes; if the gate is not authorized, the
  deliverable stops at slice 1 (accurate pre-publish docs + checklist) and
  success criteria 6–7 become blocked rather than silently skipped.
- **docs.rs build weight (medium).** Building `parquet` + Arrow 60 on docs.rs
  under `features = ["parquet"]` is heavier than the default build and could time
  out. Mitigation: an explicit feature list (no `all-features`), and a documented
  fallback (drop the metadata rather than widen it if docs.rs cannot build the
  feature set).
- **Link-gate flakiness (low).** Removing the crates.io ignore makes
  `make docs-links` hit crates.io on every docs run, which can be slow or
  rate-limited. Mitigation: flagged in the question round with the alternative of
  keeping the ignore deliberately.
- **Review budget (low–medium).** See the estimate; if the counted total
  approaches 400, the parent's `ask-on-risk` gate decides the delivery shape.

## Rollback

- **Before any real publish (slice 1 or an aborted release):** revert the version
  commit — manifests, lockfiles, CHANGELOG header, snapshot, docs — and the
  repository returns exactly to its previous state. Nothing was published, no tag
  needs to exist, and no consumer is affected. Nothing in this change touches
  runtime behavior, so there is no runtime rollback.
- **After the tag but before the crates are live:** delete the local and remote
  tag (`git tag -d v2.0.0 && git push origin :refs/tags/v2.0.0`) and delete the
  draft GitHub Release; the crates are still unpublished, so the revert is clean.
- **After a crates.io publish:** the registry is immutable. The documented
  recovery is fix-forward with a patch version (`2.0.1` / `0.2.1`), which
  re-cascades the internal requirement bumps and repeats the checklist; `cargo yank
  --version <v> -p <crate>` is the last resort for hiding a broken release from
  new resolution (existing lockfiles keep working, and the tarball stays public).
  A bad tag can be deleted independently of the crates.
- **After publish, if the docs flip is not applied:** the docs keep saying
  crates.io is upcoming and the README keeps no badge. That is a truthful but
  incomplete state; the fix is the slice 2 diff, which contains no crates-related
  side effects (docs and one JSON config only).

## Size estimate (vs 400-line review budget)

Realized shipped diff (excludes the change's own SDD artifacts), with the 1.5×
multiplier the repository has learned to apply to prose/config blocks:

| Slice | Content | Raw | Realistic |
| --- | --- | --- | --- |
| Slice 1 — version/reference bump | 3 manifests, 2 lockfiles, CHANGELOG, snapshot + core test literal, 2 dependency snippets, 4 doc examples, README status | 35–45 | **50–70** |
| Slice 1 — tabular metadata | `readme`, `include`, docs.rs metadata, README version + absolute link | 8–10 | **12–16** |
| Slice 1 — release checklist | `docs/release-process.md` ordered flow, caveat, recovery, CI-prerequisite subsections | 80–110 | **110–150** |
| Slice 1 — pre-publish docs truth | `discoverability.md` upcoming entry, `launch-publicity.md` order/versions | 30–40 | **40–55** |
| **Slice 1 total** | one PR | **~155–205** | **~215–290** |
| Slice 2 — post-publish flip | discoverability published row, README badge + tabular wording, roadmap + ROADMAP, link-check ignore | 20–30 | **30–45** |
| **Shipped change total** | | **~175–235** | **~245–335** |
| Change artifacts | `specs/crates-io-release/spec.md` (~60–90), `tasks.md` (~50–70), receipts sections | 110–160 | **140–200** |

Reading: the **shipped** diff (the parts a reviewer must judge) lands around
**245–335 realized lines** and fits the 400-line budget as a single PR. If the
budget counts the SDD artifacts as well, the total approaches or crosses 400, and
the delivery decision belongs to the parent's `ask-on-risk` gate — **no chain
strategy and no `size:exception` is invented here**. The natural boundary, if one
is chosen, is already visible: slice 1 (version + metadata + checklist +
pre-publish docs + spec) then slice 2 (the publish + docs flip + receipts).

## Success criteria

1. `oxdoc-core` 2.0.0, `oxdoc-tabular` 0.2.0, and `oxdoc-cli` 2.0.0 appear in the
   three manifests, in every mutual version requirement, in `Cargo.lock` and
   `fuzz/Cargo.lock`, and in the `CHANGELOG.md` release section; `cargo pkgid`
   confirms the triple.
2. `CHANGELOG.md`'s `## Unreleased` section is dated as the 2.0.0 release with
   its documented breaking changes preserved and the crate versions named.
3. `crates/oxdoc-tabular/Cargo.toml` wires `readme = "README.md"`, an `include`
   list covering `Cargo.toml`, `README.md`, `src/**`, and `examples/**`, and
   `[package.metadata.docs.rs] features = ["parquet"]`; the tabular README's
   crates.io-invalid relative link is absolute and its install snippet reads
   `0.2.0`.
4. Every crate passes the metadata verification table in §2, and
   `cargo package --list` for each crate shows no tests, fixtures, or
   workspace-only files.
5. `docs/release-process.md` contains the exact ordered flow (version gate,
   package per crate, dry-run per crate with the tabular/CLI resolution caveat and
   `--no-verify` workaround, tag + push, real publish in dependency order,
   receipts, docs reconciliation), the clean-tree precondition, the yank-based
   recovery subsection, and the deferred CI-token prerequisite.
6. **Publish receipts exist**: `cargo publish --dry-run` output is recorded for
   each crate (core before publish; tabular and CLI after their upstream is
   live), the registry reports `oxdoc-core` 2.0.0, `oxdoc-tabular` 0.2.0, and
   `oxdoc-cli` 2.0.0, and `cargo install oxdoc-cli --version 2.0.0` succeeds in a
   clean target directory; all receipts are recorded in
   `openspec/changes/release-crates-io-publish/apply-progress.md`.
7. `docs/discoverability.md`, `README.md`, `docs/launch-publicity.md`,
   `docs/roadmap.md`, and `ROADMAP.md` describe crates.io truthfully in every
   commit: upcoming (with the 2.0.0 plan) before publish and published (with
   versions and the restored badge) after; the README badge policy is honored at
   every point.
8. `make ci` is green, including the docs gates (`docs-check`, `docs-links`,
   `docs-schemas-check`, `docs-playground-check`) and coverage ≥ 95%, with no
   coverage delta from the version/metadata edits.
9. No file under `.github/workflows/`, `python/`, `schemas/`, `docs/schemas/`, or
   `crates/*/src/` is modified; no `CARGO_REGISTRY_TOKEN` or other CI credential
   is added.
10. The `v2.0.0` tag exists on the merged release commit and the release workflow
    has produced its GitHub Release/artifacts independently of the crates.
11. Realized shipped lines stay within the 400-line budget in one PR, **or** the
    parent's `ask-on-risk` gate records the delivery decision (slice boundary or
    explicit exception) — neither is assumed here.

## Parent decision traceability

| Parent decision | Where honored |
| --- | --- |
| 1. Approved versions core 2.0.0 / tabular 0.2.0 / cli 2.0.0, all in-repo cross-references updated, CHANGELOG dated as the 2.0.0 release, tags cut at publish time | "Approved release decision"; §1; §3 step 7; success criteria 1–2 |
| 2. Real `cargo publish` locally from the maintainer's machine after checklist + dry-run; no CI token, CI publishing documented as a future prerequisite | §3 (CI prerequisite subsection, local publish commands); exact flow steps 0/8/9; Alternatives 1; Non-goals 1; success criterion 9 |
| 3. Metadata fixes: tabular `readme` + `include` + docs.rs metadata for the optional parquet feature; verify all crate metadata fields | §2 (plus the tabular README link fix); success criteria 3–4 |
| 4. Documented repeatable release checklist with the exact ordered flow and the CLI/tabular dry-run caveat (`--no-verify` workaround) | §3; "Exact ordered publish flow"; Alternatives 2; success criterion 5 |
| 5. Docs reconciliation: `discoverability.md` crates.io state, `launch-publicity.md` publish order, README badge policy consistency | §4 (pre/post-publish tables); success criterion 7 |
| 6. Non-goals: no CI token/secret, no new publish automation, no Python version change, no behavioral code change | Non-goals 1–5, 9; success criteria 8–9 |

Known deviation from the parent's wording, flagged rather than silently resolved:
the decision text says the CHANGELOG should date the release "with both documented
breaks", while the current `Unreleased` section carries three breaking items (rows
JSONL v2, structured JSON v2, and the `XlsxCell.formula` source break). This
proposal dates the whole section as 2.0.0 with every documented break intact, and
raises the count in the question round.

## Proposal question round

The parent-resolved decisions above are authoritative and are not re-opened here.
These are the remaining product/operations choices the design, apply, and publish
steps should not make silently. Answer, correct, reframe, skip, or ask for a
second round:

1. **README badge window.** The proposal removes the crates.io badge in the
   pre-publish commit and restores it after the publish (policy-true at every
   commit, ~4 lines of churn). The alternative is to keep the existing badge so
   the change reads as "publish completes, badge becomes valid", accepting a
   window where `main` shows a badge pointing at a 404 if the human publish gate
   is delayed. Which is preferred?
2. **Where the checklist lives.** Extending `docs/release-process.md` §Crates.io
   Publishing (proposed) keeps one source of truth; a standalone
   `docs/releasing.md` was named in the parent context as the primary option. Is
   extending the existing page acceptable, or is a dedicated page wanted (with
   the sidebar/README entries that implies)?
3. **crates.io link gate.** Should `.markdown-link-check.json`'s crates.io ignore
   be removed after publish (the URL becomes a real check, at the cost of hitting
   crates.io on every docs run) or kept permanently as a deliberate, documented
   exclusion?
4. **Tag before publish.** The authoritative flow tags and pushes `v2.0.0`
   before the crates are published, so the GitHub Release (which advertises
   `cargo install oxdoc-cli`) can be public first. Confirm that ordering is
   intended, or should the crates publish before the tag? (The proposal follows
   the parent order and documents the consequence.)
5. **Adjacent truthfulness edits.** Beyond `discoverability.md`,
   `launch-publicity.md`, and the badge, this change also refreshes the README
   `Status` line ("stable 1.x"), the README `oxdoc-tabular` "unpublished"
   wording, both roadmap files' Phase 5 bullet, and four `oxdoc_version` doc
   examples. Is that the right boundary, or should some of those wait for a
   separate docs-accuracy change?
6. **CHANGELOG break count.** Should all three documented breaks in `Unreleased`
   land under the 2.0.0 heading (proposed), or was the intent to date only the two
   the decision text implies?
