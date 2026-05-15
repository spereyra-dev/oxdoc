#!/usr/bin/env python3
"""Generate synthetic OOXML fixtures and measure oxdoc peak RSS.

The script intentionally uses only Python's standard library so maintainers can
run the same workflow on macOS or Linux without extra tooling.
"""

from __future__ import annotations

import argparse
import datetime as dt
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BINARY = REPO_ROOT / "target" / "release" / "oxdoc"


@dataclass(frozen=True)
class Case:
    name: str
    description: str
    builder: str
    command: tuple[str, ...]


@dataclass(frozen=True)
class Measurement:
    case: Case
    fixture_bytes: int
    output_bytes: int
    peak_rss_mib: float
    samples_mib: list[float]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Measure peak RSS for representative oxdoc extraction cases."
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
        help="Number of peak RSS samples per case. Use 3+ for release baselines.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Optional Markdown file to write. When omitted, prints to stdout.",
    )
    parser.add_argument(
        "--keep-fixtures",
        action="store_true",
        help="Keep generated fixtures and print their directory.",
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

    with tempfile.TemporaryDirectory(prefix="oxdoc-memory-") as tmp:
        fixture_dir = Path(tmp)
        cases = build_fixtures(fixture_dir)
        measurements = [
            measure_case(binary, fixture_dir / f"{case.name}.ooxml", case, args.iterations)
            for case in cases
        ]
        report = render_markdown(binary, measurements, args.iterations)

        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(report, encoding="utf-8")
        else:
            print(report, end="")

        if args.keep_fixtures:
            kept = REPO_ROOT / "target" / "memory-fixtures"
            if kept.exists():
                shutil.rmtree(kept)
            shutil.copytree(fixture_dir, kept)
            print(f"fixtures: {kept}", file=sys.stderr)

    return 0


def build_fixtures(fixture_dir: Path) -> list[Case]:
    write_docx(fixture_dir / "docx-text-256kb.ooxml", target_text_bytes=256 * 1024)
    write_pptx(fixture_dir / "pptx-text-256kb.ooxml", slides=256, text_bytes_per_slide=1024)
    write_xlsx_shared_strings(
        fixture_dir / "xlsx-shared-strings-spill.ooxml",
        shared_count=4096,
        value_len=2048,
    )
    write_xlsx_wide_sparse(
        fixture_dir / "xlsx-wide-sparse.ooxml",
        rows=200,
        far_column="ZZ",
    )

    return [
        Case(
            "docx-text-256kb",
            "DOCX text extraction, 256 KiB extracted text",
            "synthetic DOCX",
            ("extract", "text", "{fixture}"),
        ),
        Case(
            "pptx-text-256kb",
            "PPTX slide text extraction, 256 slides",
            "synthetic PPTX",
            ("extract", "text", "{fixture}"),
        ),
        Case(
            "xlsx-shared-strings-spill",
            "XLSX shared-string-heavy CSV extraction past spill threshold",
            "synthetic XLSX",
            ("extract", "csv", "{fixture}", "--sheet", "Shared"),
        ),
        Case(
            "xlsx-wide-sparse",
            "XLSX wide/sparse row CSV extraction",
            "synthetic XLSX",
            ("extract", "csv", "{fixture}", "--sheet", "Sparse"),
        ),
    ]


def measure_case(binary: Path, fixture: Path, case: Case, iterations: int) -> Measurement:
    samples: list[float] = []
    output_bytes = 0
    for _ in range(iterations):
        command = [
            str(binary),
            *[part.format(fixture=str(fixture)) for part in case.command],
        ]
        output, peak_rss_mib = run_with_peak_rss(command)
        samples.append(peak_rss_mib)
        output_bytes = len(output)

    return Measurement(
        case=case,
        fixture_bytes=fixture.stat().st_size,
        output_bytes=output_bytes,
        peak_rss_mib=statistics.median(samples),
        samples_mib=samples,
    )


def run_with_peak_rss(command: list[str]) -> tuple[bytes, float]:
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

    completed = subprocess.run(
        timed_command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed with {completed.returncode}: {' '.join(command)}\n"
            f"{completed.stderr.decode(errors='replace')}"
        )

    stderr = completed.stderr.decode(errors="replace")
    match = pattern.search(stderr)
    if not match:
        raise RuntimeError(f"could not parse peak RSS from /usr/bin/time output:\n{stderr}")

    return completed.stdout, int(match.group(1)) / divisor


def render_markdown(
    binary: Path, measurements: list[Measurement], iterations: int
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
        "# Peak Memory Baselines",
        "",
        f"Generated: {generated_at}",
        "",
        f"- Platform: `{platform.platform()}`",
        f"- Machine: `{platform.machine()}`",
        f"- Python: `{platform.python_version()}`",
        f"- Rust: `{rustc}`",
        f"- Binary: `{display_path(binary)}`",
        f"- Samples per case: `{iterations}`",
        "",
        "| Case | Fixture | Output | Peak RSS MiB | Notes |",
        "| --- | ---: | ---: | ---: | --- |",
    ]

    for measurement in measurements:
        sample_note = ""
        if len(measurement.samples_mib) > 1:
            sample_note = " samples=" + ", ".join(
                f"{sample:.1f}" for sample in measurement.samples_mib
            )
        lines.append(
            "| "
            f"`{measurement.case.name}` | "
            f"{format_bytes(measurement.fixture_bytes)} | "
            f"{format_bytes(measurement.output_bytes)} | "
            f"{measurement.peak_rss_mib:.1f} | "
            f"{measurement.case.description}{sample_note} |"
        )

    lines.extend(
        [
            "",
            "Regenerate with:",
            "",
            "```bash",
            "python3 scripts/peak-memory-baselines.py --iterations 3 --output docs/performance-memory-baselines.md",
            "```",
            "",
        ]
    )
    return "\n".join(lines)


def format_bytes(size: int) -> str:
    if size >= 1024 * 1024:
        return f"{size / (1024 * 1024):.1f} MiB"
    if size >= 1024:
        return f"{size / 1024:.1f} KiB"
    return f"{size} B"


def display_path(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


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
        '<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        "<p:sldIdLst>"
        + "".join(slide_ids)
        + "</p:sldIdLst></p:presentation>"
    )
    entries["ppt/_rels/presentation.xml.rels"] = relationships(rels)
    write_zip(path, entries)


def write_xlsx_shared_strings(path: Path, shared_count: int, value_len: int) -> None:
    shared = []
    rows = []
    for index in range(shared_count):
        value = f"shared-{index:08d}-" + ("x" * max(0, value_len - 16))
        shared.append(f"<si><t>{value}</t></si>")
        row = index + 1
        rows.append(f'<row><c r="A{row}" t="s"><v>{index}</v></c></row>')
    write_xlsx(path, "Shared", "".join(rows), shared_strings="".join(shared))


def write_xlsx_wide_sparse(path: Path, rows: int, far_column: str) -> None:
    sheet_rows = []
    for row in range(1, rows + 1):
        sheet_rows.append(
            f'<row><c r="A{row}"><v>{row}</v></c>'
            f'<c r="{far_column}{row}"><v>{row * 2}</v></c></row>'
        )
    write_xlsx(path, "Sparse", "".join(sheet_rows))


def write_xlsx(
    path: Path, sheet_name: str, sheet_rows: str, shared_strings: str | None = None
) -> None:
    entries = {
        "[Content_Types].xml": content_types(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
        ),
        "_rels/.rels": office_document_rels("xl/workbook.xml"),
        "xl/workbook.xml": (
            '<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            f'<sheets><sheet name="{sheet_name}" sheetId="1" r:id="rId1"/></sheets></workbook>'
        ),
        "xl/_rels/workbook.xml.rels": relationships(
            [
                '<Relationship Id="rId1" '
                'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" '
                'Target="worksheets/sheet1.xml"/>'
            ]
        ),
        "xl/worksheets/sheet1.xml": f"<worksheet><sheetData>{sheet_rows}</sheetData></worksheet>",
    }
    if shared_strings is not None:
        entries["xl/sharedStrings.xml"] = f"<sst>{shared_strings}</sst>"
    write_zip(path, entries)


def content_types(main_content_type: str) -> str:
    return (
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        f'<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>'
        f'<Override PartName="/word/document.xml" ContentType="{main_content_type}"/>'
        f'<Override PartName="/ppt/presentation.xml" ContentType="{main_content_type}"/>'
        f'<Override PartName="/xl/workbook.xml" ContentType="{main_content_type}"/>'
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
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        + "".join(items)
        + "</Relationships>"
    )


def write_zip(path: Path, entries: dict[str, str]) -> None:
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, contents in entries.items():
            archive.writestr(name, contents)


if __name__ == "__main__":
    raise SystemExit(main())
