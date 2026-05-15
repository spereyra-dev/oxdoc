"""Subprocess-backed client for the oxdoc CLI."""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Literal, Sequence

PathLike = str | Path
TextFormat = Literal["json", "structured-json"]
ValueMode = Literal["raw", "formatted"]


class OxdocError(Exception):
    """Base exception for the Python wrapper."""


class OxdocNotFoundError(OxdocError):
    """Raised when the configured oxdoc binary cannot be executed."""

    def __init__(self, binary: str) -> None:
        super().__init__(f"oxdoc binary not found: {binary}")
        self.binary = binary


class OxdocProcessError(OxdocError):
    """Raised when oxdoc exits with a non-zero status."""

    def __init__(
        self,
        command: Sequence[str],
        returncode: int,
        stdout: str,
        stderr: str,
    ) -> None:
        message = stderr.strip() or f"oxdoc exited with status {returncode}"
        super().__init__(message)
        self.command = tuple(command)
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr


class OxdocJsonError(OxdocError):
    """Raised when oxdoc produced invalid JSON for a JSON-returning API."""

    def __init__(self, command: Sequence[str], stdout: str, source: json.JSONDecodeError) -> None:
        super().__init__(f"failed to parse oxdoc JSON output: {source}")
        self.command = tuple(command)
        self.stdout = stdout
        self.source = source


@dataclass(frozen=True)
class OxdocResult:
    """Successful oxdoc output plus warning lines emitted on stderr."""

    value: Any
    warnings: tuple[str, ...] = ()


class Oxdoc:
    """Small, typed wrapper around the oxdoc binary."""

    def __init__(self, binary: PathLike = "oxdoc") -> None:
        self.binary = str(binary)

    def extract_text(self, path: PathLike, *, structured: bool = False) -> OxdocResult:
        """Extract DOCX/PPTX text as a JSON dictionary."""

        fmt: TextFormat = "structured-json" if structured else "json"
        stdout, stderr, command = self._run(
            ["extract", "text", str(path), "--format", fmt]
        )
        return OxdocResult(self._json(stdout, command), warning_lines(stderr))

    def extract_text_records(self, paths: Iterable[PathLike]) -> OxdocResult:
        """Extract text as JSONL records, preserving per-file errors."""

        args = ["extract", "text", *[str(path) for path in paths], "--format", "jsonl"]
        stdout, stderr, command = self._run(args)
        records = []
        for line in stdout.splitlines():
            if line.strip():
                records.append(self._json(line, command))
        return OxdocResult(records, warning_lines(stderr))

    def extract_csv(
        self,
        path: PathLike,
        *,
        sheet: str | None = None,
        sheet_index: int | None = None,
        include_hidden: bool = False,
        delimiter: str = ",",
        value_mode: ValueMode = "raw",
    ) -> OxdocResult:
        """Extract one XLSX sheet as CSV text."""

        args = ["extract", "csv", str(path), "--delimiter", delimiter, "--value-mode", value_mode]
        if sheet is not None:
            args.extend(["--sheet", sheet])
        if sheet_index is not None:
            args.extend(["--sheet-index", str(sheet_index)])
        if include_hidden:
            args.append("--include-hidden")

        stdout, stderr, _ = self._run(args)
        return OxdocResult(stdout, warning_lines(stderr))

    def list_sheets(self, path: PathLike, *, include_hidden: bool = False) -> OxdocResult:
        """List XLSX sheets as dictionaries with index, name, and optional visibility."""

        args = ["extract", "csv", str(path), "--list-sheets"]
        if include_hidden:
            args.append("--include-hidden")
        stdout, stderr, _ = self._run(args)
        return OxdocResult(parse_sheet_list(stdout), warning_lines(stderr))

    def read_info(self, path: PathLike) -> OxdocResult:
        """Read metadata as a JSON dictionary."""

        stdout, stderr, command = self._run(["info", str(path), "--format", "json"])
        return OxdocResult(self._json(stdout, command), warning_lines(stderr))

    def audit(self, path: PathLike) -> OxdocResult:
        """Read audit signals as a JSON dictionary."""

        stdout, stderr, command = self._run(["audit", str(path), "--format", "json"])
        return OxdocResult(self._json(stdout, command), warning_lines(stderr))

    def _run(self, args: Sequence[str]) -> tuple[str, str, tuple[str, ...]]:
        command = (self.binary, *args)
        try:
            completed = subprocess.run(
                command,
                check=False,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
        except FileNotFoundError as exc:
            raise OxdocNotFoundError(self.binary) from exc

        if completed.returncode != 0:
            raise OxdocProcessError(
                command, completed.returncode, completed.stdout, completed.stderr
            )

        return completed.stdout, completed.stderr, command

    @staticmethod
    def _json(stdout: str, command: Sequence[str]) -> Any:
        try:
            return json.loads(stdout)
        except json.JSONDecodeError as exc:
            raise OxdocJsonError(command, stdout, exc) from exc


def warning_lines(stderr: str) -> tuple[str, ...]:
    """Return non-empty stderr lines as warning strings."""

    return tuple(line for line in stderr.splitlines() if line.strip())


def parse_sheet_list(stdout: str) -> list[dict[str, Any]]:
    """Parse the stable human sheet-list format into dictionaries."""

    sheets: list[dict[str, Any]] = []
    for line in stdout.splitlines():
        index_text, _, rest = line.partition(": ")
        if not index_text or not rest:
            continue
        sheet: dict[str, Any] = {"index": int(index_text)}
        if rest.endswith(")") and " (" in rest:
            name, visibility = rest.rsplit(" (", 1)
            sheet["name"] = name
            sheet["visibility"] = visibility[:-1]
        else:
            sheet["name"] = rest
        sheets.append(sheet)
    return sheets
