#!/usr/bin/env bash
# Build the C99 layer (crt/src) as target/crt/<arch>/nt4c99.lib:
#   crt/build.sh [arm]   with PINT's SDK (Microsoft C for ARM, tools/pint-sdk)
#   crt/build.sh x86     with clang (tools/nt4-clang-cl), and crt/x86/crt0.c as crt0.obj,
#                        the entry point tools/x86/nt4-gcc gives C programs
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH=${1:-arm}
OUT=$ROOT/target/crt/$ARCH
mkdir -p "$OUT"
if [ -f "$OUT/nt4c99.lib" ] && [ -z "$(find "$ROOT/crt" "$0" -newer "$OUT/nt4c99.lib" -print -quit)" ]; then
    echo "$OUT/nt4c99.lib"; exit 0
fi
cd "$OUT"
case $ARCH in
arm)
    "$ROOT/tools/pint-sdk" sh -c "wineserver -p
        for f in $ROOT/crt/src/*.c; do
            NT4_C99_LIB=/nonexistent cl -nologo -MD -O2 -W3 -Oi- -c -I'Z:${ROOT//\//\\}\\crt\\include' \"Z:\$(echo \$f | tr / '\\\\')\" || exit 1
        done
        lib -nologo -out:nt4c99.lib *.obj && rm -f *.obj
        wineserver -k" ;;
x86)
    # The layer's own sources do not take nt4c99.h (they define what it renames).
    cc() { NT4_CLANG_NO_FI=1 "$ROOT/tools/nt4-clang-cl" -nologo -MD -O2 -W3 -c "$@"; }
    rm -f ./*.obj
    for f in "$ROOT"/crt/src/*.c; do cc "-Fo$(basename "${f%.c}").obj" "$f"; done
    "$ROOT/tools/nt4-lib" /nologo /machine:x86 /out:nt4c99.lib ./*.obj && rm -f ./*.obj
    cc -Focrt0.obj "$ROOT/crt/x86/crt0.c" ;;
*) echo "crt/build.sh: unknown architecture $ARCH" >&2; exit 2 ;;
esac
echo "$OUT/nt4c99.lib"
