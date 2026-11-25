#!/bin/sh
# compile-c.sh CPU FILE.c...: compile mrustc's C with Visual C++ 4.1 for CPU (mips, alpha, ppc),
# with inc/ first on INCLUDE (stdint.h and friends), and count the diagnostics by code.
# Run inside the tenox7/ntsdk image (ntsdk on PATH). Output: FILE-CPU.log and a summary.
# cl stops after 100 errors (C1003), so a file's count is of its first errors only.
here=$(cd "$(dirname "$0")" && pwd)
inc="Z:$(printf '%s' "$here/inc" | tr / '\\')"
cpu=$1; shift
for f; do
    log=${f%.c}-$cpu.log
    start=$(date +%s)
    ntsdk nt40 "$cpu" sh -c 'INCLUDE="$2;$INCLUDE" exec cl -nologo -c -W1 -Fo"$0" "$1"' \
        "${f%.c}-$cpu.obj" "$f" "$inc" > "$log" 2>&1
    rc=$?
    end=$(date +%s)
    echo "== $f ($cpu): exit $rc, $((end - start)) s, $(wc -c < "$f") bytes"
    grep -oE '(fatal error|error|warning) C[0-9]+' "$log" | sort | uniq -c | sort -rn
    # the first 25 distinct errors past mrustc's prelude, with the line they point at
    grep -E '\) : (fatal )?error' "$log" | while IFS= read -r m; do
        n=$(printf '%s' "$m" | sed -nE 's/^[^(]*\(([0-9]+)\).*/\1/p')
        [ -n "$n" ] && [ "$n" -gt 400 ] || continue
        printf '   %s\n      | %s\n' "$(printf '%s' "$m" | cut -c1-160)" "$(sed -n "${n}p" "$f" | cut -c1-200)"
    done | awk '!seen[$0]++' | head -50
done
