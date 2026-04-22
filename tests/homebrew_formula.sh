#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT=$(mktemp "${TMPDIR:-/tmp}/oxdoc-formula.XXXXXX")
trap 'rm -f "$OUT"' EXIT INT TERM

sh "$ROOT/scripts/render-homebrew-formula.sh" v1.2.3 "$(printf '%064d' 1)" >"$OUT"

grep -Fq 'class Oxdoc < Formula' "$OUT"
grep -Fq 'desc "Fast OOXML text, CSV, and metadata extractor"' "$OUT"
grep -Fq 'url "https://github.com/spereyra-dev/oxdoc/archive/refs/tags/v1.2.3.tar.gz"' "$OUT"
grep -Fq 'sha256 "0000000000000000000000000000000000000000000000000000000000000001"' "$OUT"
grep -Fq 'cargo", "install", *std_cargo_args(path: "crates/oxdoc-cli")' "$OUT"
grep -Fq 'assert_match "1.2.3"' "$OUT"

printf 'homebrew formula rendering tests passed\n'
