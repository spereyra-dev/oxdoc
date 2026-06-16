# oxdoc Python Wrapper

This package is a thin Python wrapper around the `oxdoc` CLI. It is intended for notebooks, Airflow, Dagster, pandas preprocessing, and other data workflows that want structured Office extraction without writing subprocess glue by hand.

The wrapper expects an `oxdoc` binary on `PATH`, or a binary path passed to `Oxdoc(binary=...)`.

```python
from oxdoc import Oxdoc

client = Oxdoc()
text = client.extract_text("contract.docx").value
rows = client.extract_csv("workbook.xlsx", sheet="Sales Q1").value
typed_rows = client.extract_rows(
    "workbook.xlsx",
    sheet="Sales Q1",
    value_mode="formatted",
).value
info = client.read_info("contract.docx").value
```

`extract_rows` parses the CLI JSONL stream into a list of dictionaries. It accepts
`sheet` or 1-based `sheet_index`, `include_hidden`, and `value_mode="raw"` or
`"formatted"`. Row and column indices in returned records are 0-based, and raw
numeric cell values remain strings. Recoverable CLI warnings are available on
the returned `OxdocResult.warnings` tuple.

The initial package is intentionally pure Python and subprocess-based. Native bindings can be evaluated later if users need lower call overhead or direct in-process APIs.
