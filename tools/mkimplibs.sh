#!/usr/bin/env bash
# Build the import libraries of an architecture from its .def files with the rust-lld of
# the pinned toolchain (its llvm-lib mode), so no Microsoft tool is needed.
#
#   tools/mkimplibs.sh [ARCH]      # default arm: imports/arm/*.def -> target/implib/arm/*.lib
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ARCH=${1:-arm}
case $ARCH in
    arm) MACHINE=arm ;;
    *) echo "mkimplibs: unknown architecture $ARCH" >&2; exit 1 ;;
esac
LLD=$(find "$(rustc --print sysroot)" -name rust-lld -type f | head -1)
[ -x "$LLD" ] || { echo "mkimplibs: no rust-lld in the toolchain" >&2; exit 1; }
OUT=target/implib/$ARCH
mkdir -p "$OUT"
for def in imports/$ARCH/*.def; do
    lib=$OUT/$(basename "${def%.def}").lib
    if [ ! -f "$lib" ] || [ "$def" -nt "$lib" ]; then
        "$LLD" -flavor link /lib /nologo /machine:$MACHINE /def:"$def" /out:"$lib"
    fi
done
