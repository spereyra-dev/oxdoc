#!/usr/bin/env python3
"""Generate the tabular corpus, enforce throughput gates, and validate Parquet."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


def worksheet(rows: int, *, shared: bool = False, sparse: bool = False, long: bool = False) -> str:
    body = []
    for index in range(1, rows + 1):
        label = "x" * 8192 if long else f"name-{index}"
        name = f'<c r="B{index}" t="s"><v>0</v></c>' if shared else (
            f'<c r="B{index}" t="inlineStr"><is><t>{label}</t></is></c>'
        )
        optional = "" if sparse and index % 2 == 0 else name
        body.append(
            f'<row r="{index}"><c r="A{index}"><v>{index}</v></c>{optional}'
            f'<c r="C{index}" t="b"><v>{index % 2}</v></c></row>'
        )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        f'<sheetData>{"".join(body)}</sheetData></worksheet>'
    )


def write_xlsx(path: Path, sheet_xml: str, shared_string: str | None = None) -> None:
    overrides = (
        '<Override PartName="/xl/sharedStrings.xml" '
        'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>'
        if shared_string is not None else ""
    )
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", '<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' + overrides + '</Types>')
        archive.writestr("_rels/.rels", '<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>')
        archive.writestr("xl/workbook.xml", '<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>')
        archive.writestr("xl/_rels/workbook.xml.rels", '<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>')
        archive.writestr("xl/worksheets/sheet1.xml", sheet_xml)
        if shared_string is not None:
            archive.writestr("xl/sharedStrings.xml", f'<?xml version="1.0"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1" uniqueCount="1"><si><t>{shared_string}</t></si></sst>')


def run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=REPO_ROOT, check=True, text=True, **kwargs)


def infer(binary: Path, fixture: Path) -> dict[str, object]:
    completed = run(
        [str(binary), "infer", "schema", str(fixture)], stdout=subprocess.PIPE
    )
    return json.loads(completed.stdout)


def validate_parquet(path: Path, expected_rows: int) -> None:
    import duckdb
    import pyarrow.parquet as pq

    table = pq.read_table(path, columns=["id", "name", "active"])
    assert table.num_rows == expected_rows
    assert table.column("id")[0].as_py() == 1
    assert table.column("name")[expected_rows - 1].as_py() == f"name-{expected_rows}"
    connection = duckdb.connect()
    quoted = str(path).replace("'", "''")
    result = connection.execute(
        f"SELECT count(*), min(id), max(id), count(*) FILTER (WHERE active) FROM read_parquet('{quoted}') WHERE id >= 1"
    ).fetchone()
    assert result == (expected_rows, 1, expected_rows, expected_rows // 2)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rows", type=int, default=50_000)
    parser.add_argument("--keep", type=Path)
    args = parser.parse_args()
    if args.rows < 2:
        parser.error("--rows must be at least 2")

    context = tempfile.TemporaryDirectory(prefix="oxdoc-tabular-ci-") if args.keep is None else None
    root = args.keep or Path(context.name)
    root.mkdir(parents=True, exist_ok=True)
    fixtures = {
        "dense": worksheet(args.rows),
        "sparse": worksheet(128, sparse=True),
        "long-string": worksheet(8, long=True),
        "late-conflict": worksheet(16).replace('<c r="A16"><v>16</v></c>', '<c r="A16" t="inlineStr"><is><t>late</t></is></c>'),
        "over-limit": worksheet(1).replace("name-1", "x" * (65 * 1024 * 1024)),
    }
    for name, xml in fixtures.items():
        write_xlsx(root / f"{name}.xlsx", xml)
    write_xlsx(root / "shared-string.xlsx", worksheet(128, shared=True), "shared-value")
    write_xlsx(root / "mixed.xlsx", worksheet(128).replace('<c r="A128"><v>128</v></c>', '<c r="A128"><v>128.5</v></c>'))

    run(["cargo", "build", "--release", "-p", "oxdoc-cli"])
    binary = REPO_ROOT / "target" / "release" / "oxdoc"
    dense_schema = infer(binary, root / "dense.xlsx")
    sparse_schema = infer(binary, root / "sparse.xlsx")
    late_schema = infer(binary, root / "late-conflict.xlsx")
    mixed_schema = infer(binary, root / "mixed.xlsx")
    shared_schema = infer(binary, root / "shared-string.xlsx")
    long_schema = infer(binary, root / "long-string.xlsx")
    assert [column["logical_type"] for column in dense_schema["columns"]] == ["int64", "utf8", "bool"]
    assert sparse_schema["columns"][1]["nullable"] is True
    assert late_schema["columns"][0]["logical_type"] == "utf8"
    assert mixed_schema["columns"][0]["logical_type"] == "float64"
    assert shared_schema["columns"][1]["logical_type"] == "utf8"
    assert long_schema["columns"][1]["logical_type"] == "utf8"
    over_limit = subprocess.run(
        [str(binary), "infer", "schema", str(root / "over-limit.xlsx")],
        cwd=REPO_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    assert over_limit.returncode != 0

    parquet = root / "dense.parquet"
    completed = run(
        ["cargo", "run", "--release", "-p", "oxdoc-tabular", "--example", "tabular_gate", "--", str(root / "dense.xlsx"), str(parquet), str(args.rows)],
        stdout=subprocess.PIPE,
    )
    report = json.loads(completed.stdout.strip().splitlines()[-1])
    assert report["gates_passed"]
    validate_parquet(parquet, args.rows)
    report["fixtures"] = sorted(path.name for path in root.glob("*.xlsx"))
    report["parquet_bytes"] = parquet.stat().st_size
    print(json.dumps(report, sort_keys=True))
    if context is not None:
        context.cleanup()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
