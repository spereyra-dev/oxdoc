from __future__ import annotations

import os
import stat
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
    script = directory / "oxdoc-fake"
    script.write_text(
        "#!/usr/bin/env python3\n" + textwrap.dedent(source).strip() + "\n",
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return os.fspath(script)


if __name__ == "__main__":
    unittest.main()
