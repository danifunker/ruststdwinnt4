#!/bin/sh
# build.sh CPU DIR: compile mrustc's C in DIR (translate.sh's output) with Visual C++ 4.1 for
# CPU (alpha; mips and ppc later), link hello.exe and run it with ntrun. Runs inside the
# tenox7/ntsdk image (ntsdk on PATH). Prints the time and peak memory of each cl, and
# each step's diagnostics are in DIR/*-CPU.log. The C run-time (LIBC.LIB) gives the start-up
# code, which calls mrustc's main, and memcpy, the maths and the compiler's helpers.
here=$(cd "$(dirname "$0")" && pwd)
win() { printf 'Z:%s' "$(cd "$1" && pwd | tr / '\\')"; }
inc=$(win "$here/include")
cpu=$1 d=$2
# Alpha: IEEE floating point (NaN, infinities, division by zero) without traps, as Rust needs:
# without it, a comparison with a NaN raises c0000090 (probe: tools/ntsdk-spike/probes/nan.c)
case $cpu in alpha) cflags=-QAieee1 ;; *) cflags= ;; esac
peak() { cat /sys/fs/cgroup/memory.peak 2>/dev/null || echo "?"; }
rc=0
for n in libcore.rlib liballoc.rlib libnt4rt.rlib hello.exe; do
    start=$(date +%s)
    ntsdk nt40 "$cpu" sh -c 'INCLUDE="$2;$INCLUDE" exec cl -nologo -c -W1 $3 -Fo"$0" "$1"' \
        "$d/$n-$cpu.obj" "$d/$n.c" "$inc" "$cflags" > "$d/$n-cl-$cpu.log" 2>&1
    s=$?
    echo "cl $n.c ($cpu): exit $s, $(( $(date +%s) - start )) s, $(wc -c < "$d/$n.c") bytes of C, container peak memory $(peak) bytes, object $(wc -c < "$d/$n-$cpu.obj" 2>/dev/null || echo none)"
    grep -E 'error|fatal' "$d/$n-cl-$cpu.log" | head -40
    grep -oE 'warning C[0-9]+' "$d/$n-cl-$cpu.log" | sort | uniq -c | sort -rn | head
    [ $s = 0 ] || rc=1
done
[ $rc = 0 ] || exit 1
start=$(date +%s)
( cd "$d" && ntsdk nt40 "$cpu" link -nologo -out:hello-$cpu.exe -subsystem:console \
    hello.exe-$cpu.obj libnt4rt.rlib-$cpu.obj liballoc.rlib-$cpu.obj libcore.rlib-$cpu.obj kernel32.lib ) \
    > "$d/link-$cpu.log" 2>&1
s=$?
echo "link ($cpu): exit $s, $(( $(date +%s) - start )) s, $(wc -c < "$d/hello-$cpu.exe" 2>/dev/null || echo no) bytes"
head -60 "$d/link-$cpu.log"
[ $s = 0 ] || exit 1
( cd "$d" && ntsdk nt40 "$cpu" ntrun hello-$cpu.exe ) 2>&1 | tee "$d/run-$cpu.log"
grep -q '^hello: ok' "$d/run-$cpu.log"
