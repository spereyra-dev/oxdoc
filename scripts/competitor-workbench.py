#!/usr/bin/env python3
"""Compare oxdoc with optional document extraction CLIs.

The workbench intentionally treats competitors as optional. A maintainer can run
it on a fresh checkout and still get oxdoc baseline rows; installing tools such
as Apache Tika, xlsx2csv, or Mammoth adds comparison rows automatically.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
import zipfile
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BINARY = REPO_ROOT / "target" / "release" / "oxdoc"


@dataclass(frozen=True)
class Case:
    name: str
    description: str
    extension: str
    oxdoc_command: tuple[str, ...]


@dataclass(frozen=True)
class Tool:
    name: str
    description: str
    cases: frozenset[str]
    command: tuple[str, ...]


@dataclass(frozen=True)
class Measurement:
    tool: str
    case: Case
    status: str
    command: tuple[str, ...]
    fixture_bytes: int
    output_bytes: int
    median_seconds: float | None
    median_peak_rss_mib: float | None
    seconds_samples: list[float]
    peak_rss_samples_mib: list[float]
    note: str


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run a reproducible competitive workbench for OOXML extraction tools."
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=DEFAULT_BINARY,
        help="Path to the oxdoc binary to measure.",
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="Use --binary as-is instead of building target/release/oxdoc first.",
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=1,
        help="Number of samples per tool/case. Use 3+ before drawing conclusions.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Optional Markdown report path. When omitted, prints Markdown to stdout.",
    )
    parser.add_argument(
        "--csv-output",
        type=Path,
        help="Optional CSV file for raw measurement rows.",
    )
    parser.add_argument(
        "--tika-jar",
        type=Path,
        default=os.environ.get("TIKA_APP_JAR"),
        help="Path to tika-app.jar. Can also be provided with TIKA_APP_JAR.",
    )
    parser.add_argument(
        "--keep-fixtures",
        action="store_true",
        help="Keep generated fixtures under target/competitor-fixtures.",
    )
    args = parser.parse_args()

    if args.iterations < 1:
        parser.error("--iterations must be at least 1")

    if not args.no_build:
        subprocess.run(
            ["cargo", "build", "-p", "oxdoc-cli", "--release"],
            cwd=REPO_ROOT,
            check=True,
        )

    binary = args.binary.resolve()
    if not binary.exists():
        raise SystemExit(f"oxdoc binary not found: {binary}")

    with tempfile.TemporaryDirectory(prefix="oxdoc-competitors-") as tmp:
        fixture_dir = Path(tmp)
        cases = build_fixtures(fixture_dir)
        tools, skipped = discover_tools(binary, args.tika_jar)
        measurements = run_workbench(cases, tools, fixture_dir, args.iterations)
        report = render_markdown(measurements, skipped, args.iterations)

        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(report, encoding="utf-8")
        else:
            print(report, end="")

        if args.csv_output:
            args.csv_output.parent.mkdir(parents=True, exist_ok=True)
            write_csv(args.csv_output, measurements)

        if args.keep_fixtures:
            kept = REPO_ROOT / "target" / "competitor-fixtures"
            if kept.exists():
                shutil.rmtree(kept)
            shutil.copytree(fixture_dir, kept)
            print(f"fixtures: {kept}", file=sys.stderr)

    return 0


def build_fixtures(fixture_dir: Path) -> list[Case]:
    fixture_dir.mkdir(parents=True, exist_ok=True)
    write_docx(fixture_dir / "docx-text-256kb.docx", target_text_bytes=256 * 1024)
    write_pptx(fixture_dir / "pptx-text-256slides.pptx", slides=256, text_bytes_per_slide=1024)
    write_xlsx_dense(fixture_dir / "xlsx-dense-10000x8.xlsx", rows=10_000, columns=8)
    write_xlsx_sparse(
        fixture_dir / "xlsx-sparse-xfd-200rows.xlsx",
        rows=200,
        far_column="XFD",
    )
    write_xlsx_shared_strings(
        fixture_dir / "xlsx-shared-strings-4096x2048.xlsx",
        shared_count=4096,
        value_len=2048,
    )

    return [
        Case(
            "docx-text-256kb",
            "DOCX plain text extraction, 256 KiB extracted text",
            ".docx",
            ("extract", "text", "{fixture}"),
        ),
        Case(
            "pptx-text-256slides",
            "PPTX slide text extraction, 256 synthetic slides",
            ".pptx",
            ("extract", "text", "{fixture}"),
        ),
        Case(
            "xlsx-dense-10000x8",
            "XLSX dense first-sheet CSV extraction, 10k rows x 8 columns",
            ".xlsx",
            ("extract", "csv", "{fixture}", "--sheet", "Data"),
        ),
        Case(
            "xlsx-sparse-xfd-200rows",
            "XLSX sparse CSV extraction with values in A and XFD",
            ".xlsx",
            ("extract", "csv", "{fixture}", "--sheet", "Sparse"),
        ),
        Case(
            "xlsx-shared-strings-4096x2048",
            "XLSX shared-string CSV extraction with 4096 long strings",
            ".xlsx",
            ("extract", "csv", "{fixture}", "--sheet", "Shared"),
        ),
    ]


def discover_tools(binary: Path, tika_jar: Path | None) -> tuple[list[Tool], list[str]]:
    tools = [
        Tool(
            "oxdoc",
            "This repository's release CLI.",
            frozenset(
                {
                    "docx-text-256kb",
                    "pptx-text-256slides",
                    "xlsx-dense-10000x8",
                    "xlsx-sparse-xfd-200rows",
                    "xlsx-shared-strings-4096x2048",
                }
            ),
            (str(binary),),
        )
    ]
    skipped: list[str] = []

    if tika_jar:
        jar = Path(tika_jar).expanduser().resolve()
        if jar.exists():
            tools.append(
                Tool(
                    "apache-tika",
                    "Apache Tika app JAR text extraction.",
                    frozenset({"docx-text-256kb", "pptx-text-256slides"}),
                    ("java", "-jar", str(jar), "--text"),
                )
            )
        else:
            skipped.append(f"apache-tika: tika JAR not found at {jar}")
    elif shutil.which("tika"):
        tools.append(
            Tool(
                "apache-tika",
                "Apache Tika CLI text extraction.",
                frozenset({"docx-text-256kb", "pptx-text-256slides"}),
                ("tika", "--text"),
            )
        )
    else:
        skipped.append("apache-tika: set TIKA_APP_JAR or install a `tika` command")

    if shutil.which("xlsx2csv"):
        tools.append(
            Tool(
                "xlsx2csv",
                "Python xlsx2csv first-sheet CSV extraction.",
                frozenset(
                    {
                        "xlsx-dense-10000x8",
                        "xlsx-sparse-xfd-200rows",
                        "xlsx-shared-strings-4096x2048",
                    }
                ),
                ("xlsx2csv",),
            )
        )
    else:
        skipped.append("xlsx2csv: install with `python3 -m pip install xlsx2csv`")

    if shutil.which("mammoth"):
        tools.append(
            Tool(
                "mammoth",
                "Mammoth DOCX to Markdown extraction.",
                frozenset({"docx-text-256kb"}),
                ("mammoth", "--output-format=markdown"),
            )
        )
    else:
        skipped.append("mammoth: install with `python3 -m pip install mammoth`")

    return tools, skipped


def run_workbench(
    cases: list[Case],
    tools: list[Tool],
    fixture_dir: Path,
    iterations: int,
) -> list[Measurement]:
    measurements: list[Measurement] = []
    for case in cases:
        fixture = fixture_dir / f"{case.name}{case.extension}"
        for tool in tools:
            if case.name not in tool.cases:
                continue
            command = command_for(tool, case, fixture)
            measurements.append(measure_tool_case(tool, case, fixture, command, iterations))
    return measurements


def command_for(tool: Tool, case: Case, fixture: Path) -> tuple[str, ...]:
    if tool.name == "oxdoc":
        return (*tool.command, *[part.format(fixture=str(fixture)) for part in case.oxdoc_command])
    return (*tool.command, str(fixture))


def measure_tool_case(
    tool: Tool,
    case: Case,
    fixture: Path,
    command: tuple[str, ...],
    iterations: int,
) -> Measurement:
    seconds_samples: list[float] = []
    peak_samples: list[float] = []
    output_bytes = 0
    note = tool.description

    for _ in range(iterations):
        result = run_with_peak_rss(list(command))
        if result.returncode != 0:
            return Measurement(
                tool=tool.name,
                case=case,
                status="failed",
                command=command,
                fixture_bytes=fixture.stat().st_size,
                output_bytes=output_bytes,
                median_seconds=None,
                median_peak_rss_mib=None,
                seconds_samples=seconds_samples,
                peak_rss_samples_mib=peak_samples,
                note=result.stderr_tail,
            )
        output_bytes = len(result.stdout)
        seconds_samples.append(result.seconds)
        peak_samples.append(result.peak_rss_mib)

    return Measurement(
        tool=tool.name,
        case=case,
        status="ok",
        command=command,
        fixture_bytes=fixture.stat().st_size,
        output_bytes=output_bytes,
        median_seconds=statistics.median(seconds_samples),
        median_peak_rss_mib=statistics.median(peak_samples),
        seconds_samples=seconds_samples,
        peak_rss_samples_mib=peak_samples,
        note=note,
    )


@dataclass(frozen=True)
class TimedResult:
    returncode: int
    stdout: bytes
    seconds: float
    peak_rss_mib: float
    stderr_tail: str


def run_with_peak_rss(command: list[str]) -> TimedResult:
    if sys.platform == "darwin":
        timed_command = ["/usr/bin/time", "-l", *command]
        divisor = 1024 * 1024
        pattern = re.compile(r"^\s*(\d+)\s+maximum resident set size", re.MULTILINE)
    else:
        timed_command = ["/usr/bin/time", "-v", *command]
        divisor = 1024
        pattern = re.compile(
            r"Maximum resident set size \(kbytes\):\s*(\d+)", re.MULTILINE
        )

    started = time.perf_counter()
    completed = subprocess.run(
        timed_command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    seconds = time.perf_counter() - started
    stderr = completed.stderr.decode(errors="replace")
    match = pattern.search(stderr)
    peak_rss_mib = int(match.group(1)) / divisor if match else 0.0

    return TimedResult(
        returncode=completed.returncode,
        stdout=completed.stdout,
        seconds=seconds,
        peak_rss_mib=peak_rss_mib,
        stderr_tail="\n".join(stderr.strip().splitlines()[-6:]),
    )


def render_markdown(
    measurements: list[Measurement],
    skipped: list[str],
    iterations: int,
) -> str:
    generated_at = dt.datetime.now(dt.UTC).strftime("%Y-%m-%d %H:%M:%S UTC")
    rustc = subprocess.run(
        ["rustc", "--version"],
        cwd=REPO_ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()

    lines = [
        "# Competitive Workbench",
        "",
        "This report compares `oxdoc` with optional local OOXML extraction tools on safe synthetic fixtures.",
        "Results are environment-specific and should be treated as local evidence, not universal claims.",
        "",
        f"Generated: {generated_at}",
        "",
        f"- Platform: `{platform.platform()}`",
        f"- Machine: `{platform.machine()}`",
        f"- Python: `{platform.python_version()}`",
        f"- Rust: `{rustc}`",
        f"- Samples per tool/case: `{iterations}`",
        "",
        "| Case | Tool | Status | Fixture | Output | Median Time | Median Peak RSS | Notes |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ]

    for measurement in measurements:
        lines.append(
            "| "
            f"`{measurement.case.name}` | "
            f"`{measurement.tool}` | "
            f"{measurement.status} | "
            f"{format_bytes(measurement.fixture_bytes)} | "
            f"{format_bytes(measurement.output_bytes)} | "
            f"{format_seconds(measurement.median_seconds)} | "
            f"{format_rss(measurement.median_peak_rss_mib)} | "
            f"{escape_table(measurement.note)} |"
        )

    if skipped:
        lines.extend(["", "## Skipped Tools", ""])
        for item in skipped:
            lines.append(f"- {item}")

    lines.extend(
        [
            "",
            "Regenerate with:",
            "",
            "```bash",
            "python3 scripts/competitor-workbench.py --iterations 3 --output target/competitor-workbench/report.md --csv-output target/competitor-workbench/results.csv",
            "```",
            "",
        ]
    )
    return "\n".join(lines)


def write_csv(path: Path, measurements: list[Measurement]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "case",
                "tool",
                "status",
                "fixture_bytes",
                "output_bytes",
                "median_seconds",
                "median_peak_rss_mib",
                "command",
                "note",
            ]
        )
        for measurement in measurements:
            writer.writerow(
                [
                    measurement.case.name,
                    measurement.tool,
                    measurement.status,
                    measurement.fixture_bytes,
                    measurement.output_bytes,
                    measurement.median_seconds,
                    measurement.median_peak_rss_mib,
                    " ".join(measurement.command),
                    measurement.note,
                ]
            )


def format_bytes(size: int) -> str:
    if size >= 1024 * 1024:
        return f"{size / (1024 * 1024):.1f} MiB"
    if size >= 1024:
        return f"{size / 1024:.1f} KiB"
    return f"{size} B"


def format_seconds(value: float | None) -> str:
    if value is None:
        return ""
    return f"{value:.3f}s"


def format_rss(value: float | None) -> str:
    if value is None:
        return ""
    return f"{value:.1f} MiB"


def escape_table(value: str) -> str:
    return value.replace("|", "\\|").replace("\n", "<br>")


def write_docx(path: Path, target_text_bytes: int) -> None:
    paragraphs: list[str] = []
    expected = 0
    index = 0
    while expected < target_text_bytes:
        text = f"Benchmark paragraph {index:05d} with stable ASCII text."
        paragraphs.append(f"<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
        expected += len(text) + 1
        index += 1

    document = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        "<w:body>"
        + "".join(paragraphs)
        + "</w:body></w:document>"
    )
    write_zip(
        path,
        {
            "[Content_Types].xml": content_types(
                "word/document.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
            ),
            "_rels/.rels": office_document_rels("word/document.xml"),
            "word/document.xml": document,
        },
    )


def write_pptx(path: Path, slides: int, text_bytes_per_slide: int) -> None:
    slide_ids: list[str] = []
    entries = {
        "[Content_Types].xml": content_types(
            "ppt/presentation.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"
        ),
        "_rels/.rels": office_document_rels("ppt/presentation.xml"),
    }
    rels: list[str] = []
    for slide in range(1, slides + 1):
        rel_id = f"rId{slide}"
        slide_ids.append(f'<p:sldId id="{255 + slide}" r:id="{rel_id}"/>')
        rels.append(
            f'<Relationship Id="{rel_id}" '
            'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" '
            f'Target="slides/slide{slide}.xml"/>'
        )
        text = ("Slide %04d " % slide) + ("x" * max(0, text_bytes_per_slide - 11))
        entries[f"ppt/slides/slide{slide}.xml"] = (
            '<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
            'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">'
            "<p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>"
            f"{text}"
            "</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"
        )

    entries["ppt/presentation.xml"] = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        "<p:sldIdLst>"
        + "".join(slide_ids)
        + "</p:sldIdLst></p:presentation>"
    )
    entries["ppt/_rels/presentation.xml.rels"] = relationships(rels)
    write_zip(path, entries)


def write_xlsx_dense(path: Path, rows: int, columns: int) -> None:
    sheet = [
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>',
    ]
    for row in range(1, rows + 1):
        sheet.append("<row>")
        for column in range(1, columns + 1):
            value = row * column
            sheet.append(f'<c r="{column_name(column)}{row}"><v>{value}</v></c>')
        sheet.append("</row>")
    sheet.append("</sheetData></worksheet>")
    write_xlsx(path, "Data", "sheet1.xml", "".join(sheet))


def write_xlsx_sparse(path: Path, rows: int, far_column: str) -> None:
    sheet = [
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>',
    ]
    for row in range(1, rows + 1):
        sheet.append("<row>")
        sheet.append(f'<c r="A{row}"><v>{row}</v></c>')
        sheet.append(f'<c r="{far_column}{row}"><v>{row * 10}</v></c>')
        sheet.append("</row>")
    sheet.append("</sheetData></worksheet>")
    write_xlsx(path, "Sparse", "sheet1.xml", "".join(sheet))


def write_xlsx_shared_strings(path: Path, shared_count: int, value_len: int) -> None:
    shared_strings = [
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
        '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">',
    ]
    sheet = [
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>',
    ]
    for index in range(shared_count):
        value = shared_string_value(index, value_len)
        shared_strings.append(f"<si><t>{value}</t></si>")
        row = index + 1
        sheet.append(f'<row><c r="A{row}" t="s"><v>{index}</v></c></row>')
    shared_strings.append("</sst>")
    sheet.append("</sheetData></worksheet>")
    write_xlsx(
        path,
        "Shared",
        "sheet1.xml",
        "".join(sheet),
        shared_strings="".join(shared_strings),
    )


def write_xlsx(
    path: Path,
    sheet_name: str,
    sheet_file: str,
    sheet_xml: str,
    shared_strings: str | None = None,
) -> None:
    entries = {
        "[Content_Types].xml": content_types(
            "xl/workbook.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
        ),
        "_rels/.rels": office_document_rels("xl/workbook.xml"),
        "xl/workbook.xml": (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            f'<sheets><sheet name="{sheet_name}" sheetId="1" r:id="rId1"/></sheets></workbook>'
        ),
        "xl/_rels/workbook.xml.rels": relationships(
            [
                '<Relationship Id="rId1" '
                'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" '
                f'Target="worksheets/{sheet_file}"/>'
            ]
        ),
        f"xl/worksheets/{sheet_file}": sheet_xml,
    }
    if shared_strings is not None:
        entries["xl/sharedStrings.xml"] = shared_strings
    write_zip(path, entries)


def content_types(main_part: str, main_content_type: str) -> str:
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        f'<Override PartName="/{main_part}" ContentType="{main_content_type}"/>'
        "</Types>"
    )


def office_document_rels(target: str) -> str:
    return relationships(
        [
            '<Relationship Id="rId1" '
            'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" '
            f'Target="{target}"/>'
        ]
    )


def relationships(items: list[str]) -> str:
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        + "".join(items)
        + "</Relationships>"
    )


def write_zip(path: Path, entries: dict[str, str]) -> None:
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, contents in entries.items():
            archive.writestr(name, contents.encode("utf-8"))


def shared_string_value(index: int, value_len: int) -> str:
    prefix = f"shared-{index:08}-"
    if len(prefix) >= value_len:
        return prefix[:value_len]
    return prefix + ("x" * (value_len - len(prefix)))


def column_name(index: int) -> str:
    name: list[str] = []
    while index > 0:
        index -= 1
        name.append(chr(ord("A") + (index % 26)))
        index //= 26
    return "".join(reversed(name))


if __name__ == "__main__":
    raise SystemExit(main())
