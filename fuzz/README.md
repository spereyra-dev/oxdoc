# Fuzzing

This directory contains `cargo-fuzz` harnesses for the XML parser entry points in
`oxdoc-core`. These targets exercise untrusted OOXML XML parts after the ZIP
container has been opened.

## Targets

| Target | Parser path |
| --- | --- |
| `docx_text` | DOCX document text XML |
| `pptx_text` | PPTX slide text XML |
| `xlsx_shared_strings` | XLSX shared-string XML |
| `xlsx_sheet` | XLSX worksheet XML |
| `relationships` | OOXML relationship XML |
| `metadata` | OOXML document-property XML |

## Local use

Install `cargo-fuzz` and use a nightly Rust toolchain:

```bash
cargo +nightly install cargo-fuzz --locked
cd fuzz
```

Build one target:

```bash
cargo +nightly fuzz build docx_text
```

Run one target:

```bash
cargo +nightly fuzz run xlsx_sheet -- -max_total_time=300 -timeout=10 -rss_limit_mb=1024
```

To reproduce a retained regression, pass its target directory as an additional
corpus:

```bash
cargo +nightly fuzz run xlsx_sheet regressions/xlsx_sheet -- -runs=0
```

To minimize a crash artifact before investigating it:

```bash
cargo +nightly fuzz tmin xlsx_sheet artifacts/xlsx_sheet/crash-* \
  -artifact=regressions/xlsx_sheet/<sha256>
```

Inspect and sanitize the minimized input before committing it. See
[`regressions/README.md`](regressions/README.md) for the retention policy.

## Continuous fuzzing

GitHub Actions builds all harnesses for pull requests that affect the fuzzing
surface. A separate Monday schedule runs every target for five minutes; it can
also be started manually with a per-target duration between 1 and 900 seconds.
Runs are bounded by wall time, input size, per-input timeout, and a 1 GiB RSS
limit. Failed runs upload crash artifacts for 14 days; artifacts are not
committed. The workflow has read-only repository permissions and does not run
for untrusted pull-request code beyond compiling the harnesses.
