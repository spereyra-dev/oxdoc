# Python Integration

`oxdoc` includes a minimal pure-Python wrapper for data teams that prefer notebooks, Airflow, Dagster, pandas preprocessing, or other Python-first orchestration.

## Decision Record

The first Python integration is a thin subprocess wrapper around the stable `oxdoc` CLI, not native bindings.

Why this path:

- It works with the existing release binary and does not require Rust, PyO3, maturin, or platform-specific wheels.
- It preserves the CLI contract that already carries warnings on stderr, JSON output, JSONL batch records, and stable exit codes.
- It is easy to use in schedulers and notebooks where shelling out is acceptable but hand-written subprocess parsing is repetitive.
- It keeps native bindings available as a later option if users need lower per-call overhead or direct in-process APIs.

Native bindings should be reconsidered when there is clear demand for high-volume in-process extraction, zero-copy data access, or packaging through Python wheels that bundle the Rust implementation.

## Package Layout

The wrapper lives under `python/`:

```text
python/
  pyproject.toml
  src/oxdoc/
  tests/
```

Install it locally for development:

```bash
python3 -m pip install -e python
```

The package expects an `oxdoc` binary on `PATH`. You can also pass a binary path explicitly:

```python
from oxdoc import Oxdoc

client = Oxdoc(binary="./target/release/oxdoc")
```

## API

```python
from oxdoc import Oxdoc

client = Oxdoc()

text = client.extract_text("contract.docx").value
structured = client.extract_text("contract.docx", structured=True).value
records = client.extract_text_records(["a.docx", "b.xlsx"]).value
csv = client.extract_csv("workbook.xlsx", sheet="Sales Q1").value
sheets = client.list_sheets("workbook.xlsx", include_hidden=True).value
info = client.read_info("contract.docx").value
audit = client.audit("contract.docx").value
```

Each successful call returns `OxdocResult`:

```python
result.value
result.warnings
```

`value` is a Python dictionary/list for JSON-producing commands, or CSV text for `extract_csv`. `warnings` is a tuple of non-empty stderr lines.

## Errors

The wrapper exposes explicit exception types:

| Exception | Meaning |
| --- | --- |
| `OxdocNotFoundError` | The configured binary could not be executed. |
| `OxdocProcessError` | The CLI exited non-zero. The exception includes `command`, `returncode`, `stdout`, and `stderr`. |
| `OxdocJsonError` | A JSON-returning API received invalid JSON from the CLI. |

JSONL text extraction preserves per-file extraction errors in returned records instead of raising, matching the CLI contract.

## Testing

Run wrapper tests with:

```bash
make python-test
```

The tests use fake `oxdoc` executables so they can cover subprocess failures, JSON parsing failures, and missing binary behavior without requiring Office fixtures.
