#!/usr/bin/env python3
"""Measure the incremental cost of hashing files for a batch manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import tempfile
import time
from pathlib import Path


def build_corpus(root: Path, files: int, file_size: int) -> list[Path]:
    chunk = bytes((index % 251 for index in range(min(file_size, 1024 * 1024))))
    paths = []
    for index in range(files):
        path = root / f"fixture-{index:05}.bin"
        remaining = file_size
        with path.open("wb") as output:
            while remaining:
                block = chunk[:remaining]
                output.write(block)
                remaining -= len(block)
        paths.append(path)
    return paths


def inventory(paths: list[Path], hash_files: bool) -> tuple[int, str | None]:
    total_bytes = 0
    combined = hashlib.sha256() if hash_files else None
    for path in paths:
        total_bytes += path.stat().st_size
        if combined is not None:
            digest = hashlib.sha256()
            with path.open("rb") as source:
                for chunk in iter(lambda: source.read(1024 * 1024), b""):
                    digest.update(chunk)
            combined.update(digest.digest())
    return total_bytes, combined.hexdigest() if combined is not None else None


def measure(paths: list[Path], hash_files: bool, iterations: int) -> list[float]:
    samples = []
    for _ in range(iterations):
        started = time.perf_counter()
        inventory(paths, hash_files)
        samples.append(time.perf_counter() - started)
    return samples


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--files", type=int, default=256)
    parser.add_argument("--file-size", type=int, default=256 * 1024)
    parser.add_argument("--iterations", type=int, default=5)
    args = parser.parse_args()
    if args.files < 1 or args.file_size < 1 or args.iterations < 1:
        parser.error("--files, --file-size, and --iterations must be positive")

    with tempfile.TemporaryDirectory(prefix="oxdoc-batch-spike-") as temporary:
        paths = build_corpus(Path(temporary), args.files, args.file_size)
        stat_samples = measure(paths, False, args.iterations)
        hash_samples = measure(paths, True, args.iterations)
        total_bytes, combined_sha256 = inventory(paths, True)

    stat_seconds = statistics.median(stat_samples)
    hash_seconds = statistics.median(hash_samples)
    result = {
        "files": args.files,
        "file_size_bytes": args.file_size,
        "total_bytes": total_bytes,
        "iterations": args.iterations,
        "stat_median_seconds": round(stat_seconds, 6),
        "sha256_median_seconds": round(hash_seconds, 6),
        "sha256_overhead_seconds": round(hash_seconds - stat_seconds, 6),
        "sha256_throughput_mib_per_second": round(
            total_bytes / (1024 * 1024) / hash_seconds, 2
        ),
        "combined_sha256": combined_sha256,
    }
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
