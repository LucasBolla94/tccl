#!/bin/sh
# Installs the tccl command-line tool on Linux (x86_64, aarch64).
#   curl -fsSL https://tccl.the-coin.cloud/install.sh | sh
# Options (environment): TCCL_VERSION=v0.3.0  TCCL_INSTALL_DIR=/usr/local/bin
# The archive is verified against SHA256SUMS published with the same release.
set -eu

REPO="LucasBolla94/tccl"
VERSION="${TCCL_VERSION:-latest}"

say() { printf '%s\n' "$*"; }
fail() { printf 'tccl install: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "'$1' is required"; }

need uname
need tar
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL --proto '=https' --tlsv1.2 -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
else
  fail "curl or wget is required"
fi
if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
  fail "sha256sum or shasum is required to verify the download"
fi

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux) ;;
  *) fail "unsupported system '$os' (use the Windows installer, or build from source: cargo install --git https://github.com/$REPO tccl-cli)" ;;
esac
case "$arch" in
  x86_64 | amd64) target="x86_64-unknown-linux-musl" ;;
  aarch64 | arm64) target="aarch64-unknown-linux-musl" ;;
  *) fail "unsupported architecture '$arch'" ;;
esac

if [ "$VERSION" = "latest" ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  base="https://github.com/$REPO/releases/download/$VERSION"
fi
archive="tccl-$target.tar.gz"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM
say "Downloading $archive ($VERSION)…"
fetch "$base/$archive" "$tmp/$archive" || fail "download failed: $base/$archive"
fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" || fail "download failed: $base/SHA256SUMS"
expected="$(grep " $archive\$" "$tmp/SHA256SUMS" | cut -d' ' -f1)"
[ -n "$expected" ] || fail "$archive is not listed in SHA256SUMS"
actual="$(sha256 "$tmp/$archive")"
[ "$expected" = "$actual" ] || fail "checksum mismatch for $archive (expected $expected, got $actual)"
tar -xzf "$tmp/$archive" -C "$tmp"

dir="${TCCL_INSTALL_DIR:-}"
if [ -z "$dir" ]; then
  if [ -w /usr/local/bin ]; then dir=/usr/local/bin; else dir="$HOME/.local/bin"; fi
fi
mkdir -p "$dir"
install -m 0755 "$tmp/tccl" "$dir/tccl" 2>/dev/null || { cp "$tmp/tccl" "$dir/tccl" && chmod 0755 "$dir/tccl"; }
say "Installed $("$dir/tccl" version) to $dir/tccl"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) say "Add $dir to your PATH, e.g.: echo 'export PATH=\"$dir:\$PATH\"' >> ~/.profile" ;;
esac
say "Next: tccl new my-first-contract && cd my-first-contract && tccl test"
