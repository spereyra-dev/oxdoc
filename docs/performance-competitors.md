# Competitive Workbench

`oxdoc` includes a local workbench for comparing the release CLI with optional
OOXML extraction tools on safe synthetic fixtures. The goal is reproducibility,
not a universal leaderboard: filesystem cache state, CPU governor, Java/Python
startup cost, and tool versions all affect the numbers.

Run the default workflow with:

```bash
make competitor-workbench
```

For a quicker smoke run:

```bash
python3 scripts/competitor-workbench.py --iterations 1 --output target/competitor-workbench.md
```

The script always measures `oxdoc`. It also measures external tools when they
are available locally:

| Tool | Cases | Discovery |
| --- | --- | --- |
| `apache-tika` | DOCX and PPTX text extraction | Set `TIKA_APP_JAR=/path/to/tika-app.jar` or install a `tika` command. |
| `xlsx2csv` | XLSX CSV extraction | Install a local `xlsx2csv` command. |
| `mammoth` | DOCX extraction | Install a local `mammoth` command. |

Missing tools are reported under "Skipped Tools" instead of failing the run.

## Fixture Cases

The workbench generates temporary OOXML files with deterministic repository-owned
content:

| Case | Purpose |
| --- | --- |
| `docx-text-256kb` | DOCX plain text extraction over generated paragraphs. |
| `pptx-text-256slides` | PPTX slide text extraction over 256 generated slides. |
| `xlsx-dense-10000x8` | Dense XLSX first-sheet CSV extraction. |
| `xlsx-sparse-xfd-200rows` | Sparse XLSX rows with values in columns `A` and `XFD`. |
| `xlsx-shared-strings-4096x2048` | Shared-string-heavy XLSX extraction with long values. |

Each run records status, fixture size, output size, median wall time, and median
peak RSS. Use at least three iterations before interpreting a result.

## Raw Results

The Make target writes:

- Markdown report: `target/competitor-workbench/report.md`
- Raw CSV rows: `target/competitor-workbench/results.csv`

When publishing benchmark claims, include the generated report date, platform,
architecture, Rust version, competitor versions, and exact command used.

## Interpreting Results

Compare tools by scenario rather than as a single ranking:

- Apache Tika is a broad document extraction framework; it is the most useful
  general text-extraction comparison for DOCX/PPTX.
- `xlsx2csv` is the most direct XLSX-to-CSV comparison.
- Mammoth is DOCX-focused and conversion-oriented, so output shape differs from
  `oxdoc` even when both process the same fixture.
- `oxdoc` optimizes for small deployment footprint, stable CLI output, structured
  warnings, and predictable memory on supported OOXML extraction paths.

Do not compare these results with Criterion throughput numbers or peak-memory
baselines directly. Criterion measures library parser throughput in-process;
this workbench measures full command execution, including process startup.
