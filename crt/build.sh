#!/usr/bin/env bash
# Build nt4c99.lib (crt/src) with PINT's SDK: target/crt/<arch>/nt4c99.lib.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT=$ROOT/target/crt/arm
mkdir -p "$OUT"
if [ -f "$OUT/nt4c99.lib" ] && [ -z "$(find "$ROOT/crt" -newer "$OUT/nt4c99.lib" -print -quit)" ]; then
    echo "$OUT/nt4c99.lib"; exit 0
fi
cd "$OUT"
"$ROOT/tools/pint-sdk" sh -c "wineserver -p
    for f in $ROOT/crt/src/*.c; do
        NT4_C99_LIB=/nonexistent cl -nologo -MD -O2 -W3 -Oi- -c -I'Z:${ROOT//\//\\}\\crt\\include' \"Z:\$(echo \$f | tr / '\\\\')\" || exit 1
    done
    lib -nologo -out:nt4c99.lib *.obj && rm -f *.obj
    wineserver -k"
echo "$OUT/nt4c99.lib"
