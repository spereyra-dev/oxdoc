# oxdoc Python Wrapper

This package is a thin Python wrapper around the `oxdoc` CLI. It is intended for notebooks, Airflow, Dagster, pandas preprocessing, and other data workflows that want structured Office extraction without writing subprocess glue by hand.

The wrapper expects an `oxdoc` binary on `PATH`, or a binary path passed to `Oxdoc(binary=...)`.

```python
from oxdoc import Oxdoc

client = Oxdoc()
text = client.extract_text("contract.docx").value
rows = client.extract_csv("workbook.xlsx", sheet="Sales Q1").value
info = client.read_info("contract.docx").value
```

The initial package is intentionally pure Python and subprocess-based. Native bindings can be evaluated later if users need lower call overhead or direct in-process APIs.
