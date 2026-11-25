#!/bin/sh
# compile-c.sh CPU FILE.c...: compile mrustc's C with Visual C++ 4.1 for CPU (mips, alpha, ppc),
# with inc/ first on INCLUDE (stdint.h and friends), and count the diagnostics by code.
# Run where ntsdk is on PATH. Output: FILE-CPU.log and a summary on stdout.
here=$(cd "$(dirname "$0")" && pwd)
cpu=$1; shift
for f; do
    log=${f%.c}-$cpu.log
    start=$(date +%s)
    ntsdk nt40 "$cpu" sh -c 'INCLUDE="Z:'"$here"'/inc;$INCLUDE" exec cl -nologo -c -W1 -Fo"$0" "$1"' \
        "${f%.c}-$cpu.obj" "$f" > "$log" 2>&1
    rc=$?
    end=$(date +%s)
    echo "== $f ($cpu): exit $rc, $((end - start)) s, $(wc -c < "$f") bytes"
    grep -oE '(fatal error|error|warning) C[0-9]+' "$log" | sort | uniq -c | sort -rn
    for code in $(grep -oE '(fatal error|error) C[0-9]+' "$log" | grep -oE 'C[0-9]+' | sort -u); do
        grep -m1 "$code" "$log" | sed 's/^/   e.g. /' | cut -c1-200
    done
done
