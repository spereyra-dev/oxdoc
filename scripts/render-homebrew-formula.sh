#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  printf 'usage: %s <version> <source-tarball-sha256>\n' "$0" >&2
  printf 'example: %s 0.1.0 abc123...\n' "$0" >&2
  exit 2
fi

VERSION=$1
SHA256=$2

case "$VERSION" in
  v*) TAG=$VERSION; FORMULA_VERSION=${VERSION#v} ;;
  *) TAG="v$VERSION"; FORMULA_VERSION=$VERSION ;;
esac

cat <<EOF
class Oxdoc < Formula
  desc "Fast OOXML text, CSV, and metadata extractor"
  homepage "https://github.com/spereyra-dev/oxdoc"
  url "https://github.com/spereyra-dev/oxdoc/archive/refs/tags/$TAG.tar.gz"
  sha256 "$SHA256"
  license "MIT"
  head "https://github.com/spereyra-dev/oxdoc.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/oxdoc-cli")
  end

  test do
    assert_match "$FORMULA_VERSION", shell_output("#{bin}/oxdoc --version")
    (testpath/"sample.docx").write("not a zip")
    assert_match "error", shell_output("#{bin}/oxdoc extract text sample.docx 2>&1", 1)
  end
end
EOF
