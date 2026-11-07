#!/usr/bin/env bash
# Make the std source the NT 4.0 targets build with: the pinned toolchain's rust-src, with
# std/<version>/nt4.patch applied and crates/nt4compat added as library/nt4compat.
#
#   tools/mkstd.sh        # -> target/nt4-std/<version>/library (prints that directory)
#
# tools/cargo-nt4 runs it and points cargo's -Zbuild-std at the result
# (__CARGO_TESTS_ONLY_SRC_ROOT). The copy is remade when the patch or nt4compat changes.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
VERSION=$(rustc --version | awk '{print $2}')
PATCH=std/$VERSION/nt4.patch
[ -f "$PATCH" ] || { echo "mkstd: no $PATCH for Rust $VERSION" >&2; exit 1; }
SRC=$(rustc --print sysroot)/lib/rustlib/src/rust/library
[ -d "$SRC" ] || { echo "mkstd: no rust-src in the toolchain (rustup component add rust-src)" >&2; exit 1; }
OUT=target/nt4-std/$VERSION
STAMP=$OUT/.stamp
if [ -f "$STAMP" ] && [ -z "$(find "$PATCH" crates/nt4compat -newer "$STAMP" -print -quit)" ]; then
    echo "$PWD/$OUT/library"; exit 0
fi
rm -rf "$OUT"; mkdir -p "$OUT"
cp -r "$SRC" "$OUT/library"
chmod -R u+w "$OUT/library"
patch -s -d "$OUT" -p1 < "$PATCH"
mkdir -p "$OUT/library/nt4compat"
cp -r crates/nt4compat/Cargo.toml crates/nt4compat/src "$OUT/library/nt4compat/"
touch "$STAMP"
echo "$PWD/$OUT/library"
