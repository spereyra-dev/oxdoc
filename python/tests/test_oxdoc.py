from __future__ import annotations

import json
import os
import stat
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

from oxdoc import Oxdoc, OxdocJsonError, OxdocNotFoundError, OxdocProcessError


class OxdocPythonWrapperTests(unittest.TestCase):
    def test_extract_text_parses_json_and_warnings(self) -> None:
        binary = fake_oxdoc(
            """
            import sys
            sys.stderr.write("warning[W001]: recovered\\n")
            print('{"file":"demo.docx","text":"hello"}')
            """
        )

        result = Oxdoc(binary).extract_text("demo.docx")

        self.assertEqual(result.value["text"], "hello")
        self.assertEqual(result.warnings, ("warning[W001]: recovered",))

    def test_extract_text_records_parses_jsonl(self) -> None:
        binary = fake_oxdoc(
            """
            print('{"file":"a.docx","document_type":"docx","text":"a"}')
            print('{"file":"b.xlsx","document_type":"xlsx","error":{"code":"E010","message":"bad"}}')
            """
        )

        result = Oxdoc(binary).extract_text_records(["a.docx", "b.xlsx"])

        self.assertEqual(len(result.value), 2)
        self.assertEqual(result.value[1]["error"]["code"], "E010")

    def test_extract_csv_and_sheet_list(self) -> None:
        binary = fake_oxdoc(
            """
            import sys
            args = sys.argv[1:]
            if "--list-sheets" in args:
                print("1: Visible (visible)")
                print("2: Hidden (hidden)")
            else:
                print("a,b")
                print("1,2")
            """
        )
        client = Oxdoc(binary)

        csv = client.extract_csv("book.xlsx", sheet="Visible").value
        sheets = client.list_sheets("book.xlsx", include_hidden=True).value

        self.assertEqual(csv, "a,b\n1,2\n")
        self.assertEqual(sheets[1], {"index": 2, "name": "Hidden", "visibility": "hidden"})

    def test_extract_csv_strips_bom_and_passes_bom_flag(self) -> None:
        binary = fake_oxdoc(
            """
            import sys
            args = sys.argv[1:]
            if "--bom" in args:
                sys.stdout.write("\\ufeff")
            print("a,b")
            """
        )
        client = Oxdoc(binary)

        with_bom = client.extract_csv("book.xlsx", bom=True).value
        without_bom = client.extract_csv("book.xlsx").value

        self.assertEqual(with_bom, "a,b\n")
        self.assertEqual(without_bom, "a,b\n")

    def test_extract_csv_passes_bom_flag_to_command(self) -> None:
        binary = fake_oxdoc(
            """
            import json
            import sys
            print(json.dumps({"args": sys.argv[1:]}))
            """
        )

        result = Oxdoc(binary).extract_csv("book.xlsx", bom=True)

        self.assertIn("--bom", json.loads(result.value)["args"])

    def test_extract_csv_passes_crlf_flag_to_command(self) -> None:
        binary = fake_oxdoc(
            """
            import json
            import sys
            print(json.dumps({"args": sys.argv[1:]}))
            """
        )

        result = Oxdoc(binary).extract_csv("book.xlsx", crlf=True)

        self.assertIn("--crlf", json.loads(result.value)["args"])

    def test_extract_csv_passes_quote_mode_to_command(self) -> None:
        binary = fake_oxdoc(
            """
            import json
            import sys
            print(json.dumps({"args": sys.argv[1:]}))
            """
        )

        result = Oxdoc(binary).extract_csv("book.xlsx", quote_mode="all")

        args = json.loads(result.value)["args"]
        self.assertIn("--quote-mode", args)
        self.assertEqual(args[args.index("--quote-mode") + 1], "all")

    def test_extract_rows_parses_typed_jsonl_and_warnings(self) -> None:
        binary = fake_oxdoc(
            """
            import sys
            sys.stderr.write("warning[parser/W001]: recovered row\\n")
            print('{"schema_version":1,"file":"book.xlsx","sheet_name":"Data","row_index":0,"cells":[{"column_index":0,"kind":"number","raw":"42.50","formatted":"42.50","has_formula":false}]}')
            print('{"schema_version":1,"file":"book.xlsx","sheet_name":"Data","row_index":2,"cells":[{"column_index":1,"kind":"boolean","raw":"1","value":true,"has_formula":false}]}')
            """
        )

        result = Oxdoc(binary).extract_rows(
            "book.xlsx", sheet="Data", value_mode="formatted"
        )

        self.assertEqual(len(result.value), 2)
        self.assertEqual(result.value[0]["cells"][0]["raw"], "42.50")
        self.assertIsInstance(result.value[0]["cells"][0]["raw"], str)
        self.assertIs(result.value[1]["cells"][0]["value"], True)
        self.assertEqual(result.warnings, ("warning[parser/W001]: recovered row",))

    def test_extract_rows_passes_v2_formula_fields_through(self) -> None:
        binary = fake_oxdoc(
            """
            import sys
            print('{"schema_version":2,"file":"book.xlsx","sheet_name":"Data","row_index":0,"cells":[{"column_index":0,"kind":"number","raw":"2","has_formula":true,"formula":"SUM(B1:B1)","formula_cached":true}]}')
            print('{"schema_version":2,"file":"book.xlsx","sheet_name":"Data","row_index":1,"cells":[{"column_index":1,"kind":"blank","has_formula":true,"formula":"SUM(C1:C1)","formula_cached":false}]}')
            """
        )

        result = Oxdoc(binary).extract_rows("book.xlsx", sheet="Data")

        self.assertEqual(len(result.value), 2)
        self.assertEqual(result.value[0]["schema_version"], 2)
        self.assertEqual(result.value[0]["cells"][0]["formula"], "SUM(B1:B1)")
        self.assertIs(result.value[0]["cells"][0]["formula_cached"], True)
        self.assertEqual(result.value[1]["cells"][0]["formula"], "SUM(C1:C1)")
        self.assertIs(result.value[1]["cells"][0]["formula_cached"], False)

    def test_extract_rows_passes_index_hidden_and_value_mode_options(self) -> None:
        binary = fake_oxdoc(
            """
            import json
            import sys
            print(json.dumps({"args": sys.argv[1:]}))
            """
        )

        result = Oxdoc(binary).extract_rows(
            "book.xlsx",
            sheet_index=2,
            include_hidden=True,
            value_mode="raw",
        )

        self.assertEqual(
            result.value[0]["args"],
            [
                "extract",
                "rows",
                "book.xlsx",
                "--format",
                "jsonl",
                "--value-mode",
                "raw",
                "--sheet-index",
                "2",
                "--include-hidden",
            ],
        )

    def test_process_errors_expose_status_and_stderr(self) -> None:
        binary = fake_oxdoc(
            """
            import sys
            sys.stderr.write("error[E010]: invalid argument\\n")
            raise SystemExit(2)
            """
        )

        with self.assertRaises(OxdocProcessError) as raised:
            Oxdoc(binary).read_info("bad.docx")

        self.assertEqual(raised.exception.returncode, 2)
        self.assertIn("error[E010]", raised.exception.stderr)

    def test_invalid_json_raises_json_error(self) -> None:
        binary = fake_oxdoc("print('not-json')")

        with self.assertRaises(OxdocJsonError):
            Oxdoc(binary).read_info("bad.docx")

    def test_missing_binary_raises_clear_error(self) -> None:
        with self.assertRaises(OxdocNotFoundError):
            Oxdoc("__missing_oxdoc_binary__").read_info("missing.docx")


def fake_oxdoc(source: str) -> str:
    directory = Path(tempfile.mkdtemp(prefix="oxdoc-python-test-"))
    script = directory / "oxdoc-fake.py"
    script.write_text(
        "#!/usr/bin/env python3\n" + textwrap.dedent(source).strip() + "\n",
        encoding="utf-8",
    )
    if os.name == "nt":
        launcher = directory / "oxdoc-fake.cmd"
        launcher.write_text(
            f'@echo off\r\n"{sys.executable}" "{script}" %*\r\n',
            encoding="utf-8",
        )
        return os.fspath(launcher)

    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return os.fspath(script)


if __name__ == "__main__":
    unittest.main()
