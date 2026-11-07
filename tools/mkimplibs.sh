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
changed=
for def in imports/$ARCH/*.def; do
    lib=$OUT/$(basename "${def%.def}").lib
    if [ ! -f "$lib" ] || [ "$def" -nt "$lib" ]; then
        "$LLD" -flavor link /lib /nologo /machine:$MACHINE /def:"$def" /out:"$lib"
        changed=1
    fi
done

# The windows-targets crate (behind windows-sys up to 0.60) links one import library for
# all of Windows, named for its version, which Microsoft ships for x86, x64 and ARM64
# only. Ours is every library above in one: a function NT 4.0 lacks is then a link
# error, unless nt4compat gives it.
ALL=$OUT/windows.lib
if [ -n "$changed" ] || [ ! -f "$ALL" ]; then
    libs=$(ls "$OUT"/*.lib | grep -v -e '/windows\.' )
    "$LLD" -flavor link /lib /nologo /machine:$MACHINE /out:"$ALL" $libs
    for v in 0.48.5 0.52.0 0.53.0; do cp "$ALL" "$OUT/windows.$v.lib"; done
fi
