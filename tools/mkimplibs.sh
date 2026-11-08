#!/usr/bin/env bash
# Build the import libraries of an architecture from its .def files with the rust-lld of
# the pinned toolchain (its llvm-lib mode), so no Microsoft tool is needed.
#
#   tools/mkimplibs.sh [ARCH]      # default arm: imports/arm/*.def -> target/implib/arm/*.lib
#                                  # x86: also imports/x86/oldnames.txt -> oldnames.lib
#
# x86 import libraries are written by tools/implib.py instead: their names are decorated
# (_CreateFileW@28) and must be imported plain, which rust-lld's /def cannot express.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ARCH=${1:-arm}
case $ARCH in
    arm) MACHINE=arm ;;
    x86) MACHINE=x86 ;;
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
        if [ "$ARCH" = x86 ]; then
            python3 tools/implib.py --def "$def" --out "$lib"
        else
            "$LLD" -flavor link /lib /nologo /machine:$MACHINE /def:"$def" /out:"$lib"
        fi
        changed=1
    fi
done
# x86: oldnames.lib, the old names of msvcrt.dll's functions (open for _open), for C code.
# On ARM, PINT's SDK has one.
if [ "$ARCH" = x86 ] && [ ! "$OUT/oldnames.lib" -nt imports/x86/oldnames.txt ]; then
    python3 tools/implib.py --aliases imports/x86/oldnames.txt --out "$OUT/oldnames.lib"
fi

# The windows-targets crate (behind windows-sys up to 0.60) links one import library for
# all of Windows, named for its version, which Microsoft ships for x86, x64 and ARM64
# only. Ours is every library above in one: a function NT 4.0 lacks is then a link
# error, unless nt4compat gives it.
ALL=$OUT/windows.lib
if [ -n "$changed" ] || [ ! -f "$ALL" ]; then
    libs=$(ls "$OUT"/*.lib | grep -v -e '/windows\.' -e '/oldnames\.lib$')
    "$LLD" -flavor link /lib /nologo /machine:$MACHINE /out:"$ALL" $libs
    for v in 0.48.5 0.52.0 0.53.0; do cp "$ALL" "$OUT/windows.$v.lib"; done
fi
