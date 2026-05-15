"""Python wrapper for the oxdoc CLI."""

from .client import (
    Oxdoc,
    OxdocError,
    OxdocJsonError,
    OxdocNotFoundError,
    OxdocProcessError,
    OxdocResult,
)

__all__ = [
    "Oxdoc",
    "OxdocError",
    "OxdocJsonError",
    "OxdocNotFoundError",
    "OxdocProcessError",
    "OxdocResult",
]
