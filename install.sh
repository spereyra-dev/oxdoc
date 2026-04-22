#!/bin/sh
set -eu

REPO=${OXDOC_REPO:-spereyra-dev/oxdoc}
VERSION=${OXDOC_VERSION:-latest}
INSTALL_DIR=${OXDOC_INSTALL_DIR:-"$HOME/.local/bin"}
TARGET=${OXDOC_TARGET:-}
DOWNLOAD_BASE=${OXDOC_DOWNLOAD_BASE:-}
GITHUB_API=${OXDOC_GITHUB_API:-https://api.github.com}
GITHUB_URL=${OXDOC_GITHUB_URL:-https://github.com}

fail() {
  printf 'oxdoc install: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

detect_target() {
  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  arch=$(uname -m)

  case "$os:$arch" in
    linux:x86_64 | linux:amd64)
      printf 'x86_64-unknown-linux-gnu'
      ;;
    darwin:x86_64)
      printf 'x86_64-apple-darwin'
      ;;
    darwin:arm64 | darwin:aarch64)
      printf 'aarch64-apple-darwin'
      ;;
    *)
      fail "unsupported platform: $(uname -s) $(uname -m). Set OXDOC_TARGET to override."
      ;;
  esac
}

fetch_latest_version() {
  latest_json=$(curl -fsSL "$GITHUB_API/repos/$REPO/releases/latest") ||
    fail "could not resolve latest release for $REPO"
  latest_tag=$(printf '%s\n' "$latest_json" |
    sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
    head -n 1)
  [ -n "$latest_tag" ] || fail "latest release response did not include tag_name"
  printf '%s' "$latest_tag"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

download() {
  url=$1
  output=$2
  curl -fsSL --retry 3 --connect-timeout 10 "$url" -o "$output" ||
    fail "download failed: $url"
}

verify_checksum() {
  archive=$1
  archive_name=$2
  checksums=$3

  expected_line=$(grep "  $archive_name\$" "$checksums" || true)
  [ -n "$expected_line" ] || fail "SHA256SUMS does not contain $archive_name"

  expected=$(printf '%s\n' "$expected_line" | awk '{print $1}')
  actual=$(sha256_file "$archive")

  [ "$expected" = "$actual" ] ||
    fail "checksum mismatch for $archive_name: expected $expected, got $actual"
}

need_cmd awk
need_cmd curl
need_cmd grep
need_cmd sed
need_cmd tar

if ! command -v sha256sum >/dev/null 2>&1; then
  need_cmd shasum
fi

if [ "$VERSION" = "latest" ]; then
  VERSION=$(fetch_latest_version)
fi

case "$VERSION" in
  v*) ;;
  *) VERSION="v$VERSION" ;;
esac

if [ -z "$TARGET" ]; then
  TARGET=$(detect_target)
fi

case "$TARGET" in
  *windows* | *pc-windows*)
    fail "install.sh supports Unix tar.gz releases only. Download the Windows zip from GitHub Releases."
    ;;
esac

ARCHIVE_NAME="oxdoc-$VERSION-$TARGET.tar.gz"

if [ -n "$DOWNLOAD_BASE" ]; then
  BASE_URL=${DOWNLOAD_BASE%/}
else
  BASE_URL="$GITHUB_URL/$REPO/releases/download/$VERSION"
fi

TMPDIR_ROOT=${TMPDIR:-/tmp}
TMPDIR=$(mktemp -d "$TMPDIR_ROOT/oxdoc-install.XXXXXX")
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT INT TERM

ARCHIVE="$TMPDIR/$ARCHIVE_NAME"
CHECKSUMS="$TMPDIR/SHA256SUMS"

download "$BASE_URL/$ARCHIVE_NAME" "$ARCHIVE"
download "$BASE_URL/SHA256SUMS" "$CHECKSUMS"
verify_checksum "$ARCHIVE" "$ARCHIVE_NAME" "$CHECKSUMS"

tar -xzf "$ARCHIVE" -C "$TMPDIR"
BINARY="$TMPDIR/oxdoc-$VERSION-$TARGET/oxdoc"
[ -f "$BINARY" ] || fail "archive did not contain oxdoc binary"

mkdir -p "$INSTALL_DIR"
cp "$BINARY" "$INSTALL_DIR/oxdoc"
chmod 755 "$INSTALL_DIR/oxdoc"

printf 'oxdoc %s installed to %s/oxdoc\n' "$VERSION" "$INSTALL_DIR"
printf 'Run `%s/oxdoc --version` to verify the install.\n' "$INSTALL_DIR"
