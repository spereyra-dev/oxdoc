#!/usr/bin/env python3
"""Validate the checked-in OOXML producer compatibility corpus.

The parser integration tests exercise the fixtures and their snapshots. This
script protects the companion manifest: each checked-in binary must have a
provenance note, a stable digest, and an explicit producer/capability record.
It uses only the Python standard library so it is suitable for CI.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"
MANIFEST = FIXTURES / "compatibility-matrix.json"
REQUIRED_FIELDS = {
    "id",
    "format",
    "path",
    "provenance",
    "producer",
    "producer_family",
    "producer_version",
    "capabilities",
    "snapshot",
    "sha256",
}
REQUIRED_PROVENANCE_LABELS = ("Source:", "Producer:", "Redistribution:", "Purpose:", "Sanitization:")
VALID_FORMATS = {"docx", "xlsx", "pptx"}
VALID_COVERAGE_STATUS = {"planned", "covered"}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(64 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def fixture_path(relative_path: str) -> Path:
    path = (FIXTURES / relative_path).resolve()
    if FIXTURES.resolve() not in path.parents:
        raise ValueError("path escapes tests/fixtures")
    return path


def main() -> int:
    errors: list[str] = []
    try:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"compatibility corpus: cannot read {MANIFEST}: {error}", file=sys.stderr)
        return 1

    if manifest.get("version") != 1:
        errors.append("manifest version must be 1")

    fixture_ids: set[str] = set()
    for entry in manifest.get("fixtures", []):
        label = entry.get("id", "<unnamed fixture>")
        missing = REQUIRED_FIELDS - entry.keys()
        if missing:
            errors.append(f"{label}: missing fields: {', '.join(sorted(missing))}")
            continue
        if entry["id"] in fixture_ids:
            errors.append(f"{label}: duplicate fixture id")
        fixture_ids.add(entry["id"])
        if entry["format"] not in VALID_FORMATS:
            errors.append(f"{label}: unsupported format {entry['format']!r}")
        if not entry["capabilities"] or not all(isinstance(item, str) and item for item in entry["capabilities"]):
            errors.append(f"{label}: capabilities must be a non-empty list of strings")

        try:
            fixture = fixture_path(entry["path"])
            provenance = fixture_path(entry["provenance"])
            snapshot = fixture_path(entry["snapshot"])
        except ValueError as error:
            errors.append(f"{label}: {error}")
            continue
        for kind, path in (("fixture", fixture), ("provenance", provenance), ("snapshot", snapshot)):
            if not path.is_file():
                errors.append(f"{label}: {kind} does not exist: {path.relative_to(FIXTURES)}")
        if fixture.is_file() and sha256(fixture) != entry["sha256"]:
            errors.append(f"{label}: SHA-256 does not match {entry['path']}")
        if provenance.is_file():
            note = provenance.read_text(encoding="utf-8")
            for heading in REQUIRED_PROVENANCE_LABELS:
                if heading not in note:
                    errors.append(f"{label}: provenance is missing {heading}")

    for entry in manifest.get("producer_coverage", []):
        family = entry.get("producer_family", "<unnamed producer family>")
        if entry.get("status") not in VALID_COVERAGE_STATUS:
            errors.append(f"{family}: coverage status must be planned or covered")
        if not entry.get("note"):
            errors.append(f"{family}: coverage note is required")

    if errors:
        print("compatibility corpus validation failed:", file=sys.stderr)
        print("\n".join(f"- {error}" for error in errors), file=sys.stderr)
        return 1
    print(f"compatibility corpus validation passed ({len(fixture_ids)} fixtures)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
