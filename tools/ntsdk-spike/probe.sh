#!/bin/sh
# probe.sh [CPU...]: compile each probes/*.c with Visual C++ 4.1 (ntsdk nt40 CPU cl -c) and
# print one line per probe: ok, warn (compiled with warnings) or FAIL, then the messages.
# Run it in the probes folder's parent, inside the ntsdk image (or with ntsdk on PATH).
cd "$(dirname "$0")" || exit 1
cpus=${*:-mips alpha ppc}
out=probe-out
mkdir -p $out
for f in probes/*.c; do
    n=$(basename "$f" .c)
    line=$(printf '%-22s' "$n")
    for c in $cpus; do
        log=$out/$n-$c.txt
        if ntsdk nt40 "$c" cl -nologo -c -W3 -Fo"$out/$n-$c.obj" "$f" > "$log" 2>&1; then
            if grep -q -i 'warning' "$log"; then r=warn; else r=ok; fi
        else
            r=FAIL
        fi
        line="$line $(printf '%-6s' "$c:$r")"
    done
    echo "$line"
done
echo
for f in $out/*.txt; do
    grep -i -E 'error|warning' "$f" | sed "s#^#$(basename "$f" .txt): #"
done
