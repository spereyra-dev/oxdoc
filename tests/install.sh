#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
INSTALL_SH="$ROOT/install.sh"

fail() {
  printf 'install test: %s\n' "$*" >&2
  exit 1
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

make_release() {
  release_dir=$1
  version=$2
  target=$3
  output_text=$4

  package="oxdoc-$version-$target"
  archive="$release_dir/$package.tar.gz"
  mkdir -p "$release_dir/$package"
  {
    printf '%s\n' '#!/bin/sh'
    printf "printf '%s\\n'\n" "$output_text"
  } >"$release_dir/$package/oxdoc"
  chmod 755 "$release_dir/$package/oxdoc"
  tar -czf "$archive" -C "$release_dir" "$package"
  printf '%s  %s\n' "$(sha256_file "$archive")" "$package.tar.gz" >"$release_dir/SHA256SUMS"
}

test_installs_from_file_release() (
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/oxdoc-install-test.XXXXXX")
  trap 'rm -rf "$tmp"' EXIT INT TERM

  release_dir="$tmp/release"
  install_dir="$tmp/bin"
  target="x86_64-unknown-linux-gnu"
  make_release "$release_dir" "v9.9.9" "$target" "oxdoc fake 9.9.9"

  env \
    OXDOC_VERSION="9.9.9" \
    OXDOC_TARGET="$target" \
    OXDOC_DOWNLOAD_BASE="file://$release_dir" \
    OXDOC_INSTALL_DIR="$install_dir" \
    sh "$INSTALL_SH" >"$tmp/install.out"

  [ -x "$install_dir/oxdoc" ] || fail "expected installed executable"
  output=$("$install_dir/oxdoc")
  [ "$output" = "oxdoc fake 9.9.9" ] || fail "unexpected installed binary output: $output"
  grep -q "oxdoc v9.9.9 installed" "$tmp/install.out" ||
    fail "install output did not mention normalized version"
)

test_rejects_checksum_mismatch() (
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/oxdoc-install-bad-checksum.XXXXXX")
  trap 'rm -rf "$tmp"' EXIT INT TERM

  release_dir="$tmp/release"
  install_dir="$tmp/bin"
  target="x86_64-unknown-linux-gnu"
  make_release "$release_dir" "v9.9.8" "$target" "oxdoc fake 9.9.8"
  printf '%064d  oxdoc-v9.9.8-%s.tar.gz\n' 0 "$target" >"$release_dir/SHA256SUMS"

  if env \
    OXDOC_VERSION="v9.9.8" \
    OXDOC_TARGET="$target" \
    OXDOC_DOWNLOAD_BASE="file://$release_dir" \
    OXDOC_INSTALL_DIR="$install_dir" \
    sh "$INSTALL_SH" >"$tmp/install.out" 2>"$tmp/install.err"; then
    fail "install unexpectedly succeeded with a bad checksum"
  fi

  grep -q "checksum mismatch" "$tmp/install.err" ||
    fail "checksum failure did not explain the mismatch"
  [ ! -e "$install_dir/oxdoc" ] || fail "bad checksum should not install oxdoc"
)

test_resolves_latest_version_from_api() (
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/oxdoc-install-latest.XXXXXX")
  trap 'rm -rf "$tmp"' EXIT INT TERM

  release_dir="$tmp/release"
  install_dir="$tmp/bin"
  api_dir="$tmp/api"
  target="x86_64-unknown-linux-gnu"
  make_release "$release_dir" "v9.10.0" "$target" "oxdoc fake latest"

  mkdir -p "$api_dir/repos/example/oxdoc/releases"
  printf '{"tag_name":"v9.10.0"}\n' >"$api_dir/repos/example/oxdoc/releases/latest"

  env \
    OXDOC_VERSION="latest" \
    OXDOC_REPO="example/oxdoc" \
    OXDOC_GITHUB_API="file://$api_dir" \
    OXDOC_TARGET="$target" \
    OXDOC_DOWNLOAD_BASE="file://$release_dir" \
    OXDOC_INSTALL_DIR="$install_dir" \
    sh "$INSTALL_SH" >"$tmp/install.out"

  output=$("$install_dir/oxdoc")
  [ "$output" = "oxdoc fake latest" ] || fail "latest install used wrong binary: $output"
  grep -q "oxdoc v9.10.0 installed" "$tmp/install.out" ||
    fail "latest install output did not mention resolved version"
)

test_installs_from_file_release
test_rejects_checksum_mismatch
test_resolves_latest_version_from_api
printf 'install.sh tests passed\n'
