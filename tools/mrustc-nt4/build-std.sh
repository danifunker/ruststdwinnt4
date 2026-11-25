#!/bin/sh
# build-std.sh CPU DIR: compile translate-std.sh's C in DIR (std, its dependencies and
# stdtest) with Visual C++ 4.1 for CPU, link stdtest.exe and run it with ntrun. Runs inside
# the tenox7/ntsdk image. Like build.sh: the time and peak memory of each cl, diagnostics in
# DIR/*-CPU.log. Import libraries: the NT 4.0 SDK's, and ntdll's and userenv's (which the
# SDK lacks) made from imports/arm/*.def: NT 4.0's export lists, the same names on Alpha
# (ntdll's came from the Alpha CD's alphadef.src).
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
win() { printf 'Z:%s' "$(cd "$1" && pwd | tr / '\\')"; }
inc=$(win "$here/include")
cpu=$1 d=$2
peak() { cat /sys/fs/cgroup/memory.peak 2>/dev/null || echo "?"; }
rc=0 objs=
for c in "$d"/*.c; do
    n=$(basename "$c" .c)
    start=$(date +%s)
    ntsdk nt40 "$cpu" sh -c 'INCLUDE="$2;$INCLUDE" exec cl -nologo -c -W1 -Fo"$0" "$1"' \
        "$d/$n-$cpu.obj" "$c" "$inc" > "$d/$n-cl-$cpu.log" 2>&1
    s=$?
    echo "cl $n.c ($cpu): exit $s, $(( $(date +%s) - start )) s, $(wc -c < "$c") bytes of C, container peak memory $(peak) bytes, object $(wc -c < "$d/$n-$cpu.obj" 2>/dev/null || echo none)"
    grep -E 'error|fatal' "$d/$n-cl-$cpu.log" | head -30
    [ $s = 0 ] || rc=1
    objs="$objs $n-$cpu.obj"
done
[ $rc = 0 ] || exit 1
for dll in ntdll userenv; do
    cp "$repo/imports/arm/$dll.def" "$d/$dll-$cpu.def"
    ( cd "$d" && ntsdk nt40 "$cpu" lib -nologo -machine:$cpu -def:$dll-$cpu.def -out:$dll-$cpu.lib ) > "$d/lib-$cpu.log" 2>&1 \
        || { cat "$d/lib-$cpu.log"; exit 1; }
done
start=$(date +%s)
( cd "$d" && ntsdk nt40 "$cpu" link -nologo -out:stdtest-$cpu.exe -subsystem:console $objs \
    kernel32.lib advapi32.lib ws2_32.lib userenv-$cpu.lib ntdll-$cpu.lib ) > "$d/link-$cpu.log" 2>&1
s=$?
echo "link ($cpu): exit $s, $(( $(date +%s) - start )) s, $(wc -c < "$d/stdtest-$cpu.exe" 2>/dev/null || echo no) bytes"
head -80 "$d/link-$cpu.log"
[ $s = 0 ] || exit 1
# ntrun has no sockets and no WS2_32.DLL: a stand-in next to the program, for this run
"$here/ntrun-ws2_32/mkstub.sh" "$cpu" "$d/stdtest-$cpu.exe" "$d" || exit 1
dll=$(win "$d")
( cd "$d" && ntsdk nt40 "$cpu" sh -c 'NTRUN_DLLPATH="$1;$NTRUN_DLLPATH" exec ntrun stdtest-$2.exe' sh "$dll" "$cpu" ) 2>&1 \
    | tee "$d/run-$cpu.log"
grep -q '^stdtest: ok' "$d/run-$cpu.log"
