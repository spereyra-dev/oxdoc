#!/usr/bin/env python3
"""Generate the public compatibility playground from approved test fixtures.

The page deliberately has a small, explicit allowlist.  Adding a new example
requires a repository-authored fixture, a redistribution/provenance note, and
an expected output snapshot that is also exercised by integration tests.
"""

from __future__ import annotations

import argparse
import difflib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "tests" / "fixtures"
OUTPUT = ROOT / "docs" / "compatibility-playground.md"

EXAMPLES = (
    {
        "format": "DOCX",
        "feature": "Paragraph and table text",
        "fixture": "corpus/docx/basic",
        "provenance": "docx-basic.md",
        "snapshot": "docx_basic_text.txt",
        "command": "oxdoc extract text fixture.docx",
        "output_type": "text",
        "behavior": "Paragraph text is extracted; cells use tabs and rows use line breaks.",
    },
    {
        "format": "PPTX",
        "feature": "Slide order, tabs, and speaker notes",
        "fixture": "corpus/pptx/text",
        "provenance": "pptx-text.md",
        "snapshot": "pptx_text.txt",
        "command": "oxdoc extract text fixture.pptx",
        "output_type": "text",
        "behavior": "Slide text remains in presentation order and linked speaker notes follow their slide.",
    },
    {
        "format": "XLSX",
        "feature": "Shared strings and sparse rows",
        "fixture": "corpus/xlsx/basic",
        "provenance": "xlsx-basic.md",
        "snapshot": "xlsx_basic_csv.txt",
        "command": "oxdoc extract csv fixture.xlsx",
        "output_type": "csv",
        "behavior": "Shared strings resolve and missing cells are emitted as empty CSV fields.",
    },
)

WARNING = {
    "fixture": "corpus/docx/external-target",
    "provenance": "docx-external-target.md",
    "snapshot": "docx_external_target_error.txt",
}


def checked_text(path: Path) -> str:
    if not path.is_file():
        raise ValueError(f"approved playground input is missing: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8").rstrip("\n")


def validate_example(example: dict[str, str]) -> None:
    checked_text(FIXTURES / example["fixture"] / "[Content_Types].xml")
    provenance = checked_text(FIXTURES / "provenance" / example["provenance"])
    if "Redistribution: permitted" not in provenance:
        raise ValueError(f"fixture is not approved for redistribution: {example['fixture']}")
    checked_text(FIXTURES / "snapshots" / example["snapshot"])


def render() -> str:
    for example in EXAMPLES:
        validate_example(example)
    validate_example({
        "fixture": WARNING["fixture"],
        "provenance": WARNING["provenance"],
        "snapshot": WARNING["snapshot"],
    })

    lines = [
        "# OOXML Compatibility Playground",
        "",
        "Evaluate `oxdoc` input/output behavior using the small, checked-in fixtures that power its integration tests. This is a compatibility reference, not an upload service or a document renderer.",
        "",
        "Every result below is generated from a versioned expected-output snapshot. Run `python3 scripts/compatibility-playground.py --check` to verify that this page has not drifted from its fixtures.",
        "The hand-authored package trees are zipped by the integration-test helper; the commands below show the corresponding CLI invocation after that deterministic packaging step.",
        "",
        "## Supported behavior",
        "",
        "| Format | Feature | Fixture | Expected output |",
        "| --- | --- | --- | --- |",
    ]
    for example in EXAMPLES:
        lines.append(
            f"| {example['format']} | {example['feature']} | `tests/fixtures/{example['fixture']}` | `tests/fixtures/snapshots/{example['snapshot']}` |"
        )

    for example in EXAMPLES:
        snapshot = checked_text(FIXTURES / "snapshots" / example["snapshot"])
        lines.extend([
            "",
            f"### {example['format']}: {example['feature']}",
            "",
            f"**Supported:** {example['behavior']}",
            "",
            f"Fixture: `tests/fixtures/{example['fixture']}`; provenance: `tests/fixtures/provenance/{example['provenance']}`",
            "",
            "```bash",
            example["command"],
            "```",
            "",
            f"```{example['output_type']}",
            snapshot,
            "```",
        ])

    warning = checked_text(FIXTURES / "snapshots" / WARNING["snapshot"])
    lines.extend([
        "",
        "## Safety behavior and warnings",
        "",
        "**Supported safety behavior:** external relationship targets are rejected as a hard error; `oxdoc` does not fetch them. The approved `corpus/docx/external-target` fixture produces:",
        "",
        "```text",
        warning,
        "```",
        "",
        "Recoverable malformed XML can produce partial output plus a warning on stderr. See [Errors and Warnings](errors-and-warnings.md) for the stable error and warning contract.",
        "",
        "## Non-goals",
        "",
        "- Rendering Word pages or PowerPoint slides, including fonts, layout, and pagination.",
        "- Calculating formulas or reproducing Excel's full display engine.",
        "- Repairing malformed input, decrypting documents, or fetching external relationship targets.",
        "- Accepting uploaded documents on this documentation site.",
        "",
        "## Fixture and privacy policy",
        "",
        "Examples are an explicit allowlist of hand-authored, minimal OOXML package trees. Their provenance notes state redistribution permission, and the [fixture corpus policy](https://github.com/spereyra-dev/oxdoc/blob/main/tests/fixtures/README.md) prohibits private, customer, and user documents. Do not add samples containing personal, proprietary, or sensitive data.",
        "",
        "For complete format contracts, see [DOCX](formats/docx.md), [PPTX](formats/pptx.md), and [XLSX](formats/xlsx.md).",
        "",
    ])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the generated page is stale")
    args = parser.parse_args()
    expected = render()
    actual = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
    if args.check:
        if actual != expected:
            print("compatibility playground is stale; run python3 scripts/compatibility-playground.py")
            print("".join(difflib.unified_diff(actual.splitlines(True), expected.splitlines(True), fromfile=str(OUTPUT), tofile="generated")))
            return 1
        return 0
    OUTPUT.write_text(expected, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
