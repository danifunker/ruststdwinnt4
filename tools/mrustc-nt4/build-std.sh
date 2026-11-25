#!/bin/sh
# build-std.sh CPU DIR: compile translate-std.sh's C in DIR (std, its dependencies, stdtest
# and stdsmoke) with Visual C++ 4.1 for CPU, link stdtest.exe and stdsmoke.exe and run them
# with ntrun: stdsmoke must pass; stdtest is run for its report (ntrun lacks functions it
# needs, see PLAN-mips-alpha-ppc.md). Runs inside
# the tenox7/ntsdk image. Like build.sh: the time and peak memory of each cl, diagnostics in
# DIR/*-CPU.log. Import libraries: the NT 4.0 SDK's, and ntdll's and userenv's (which the
# SDK lacks) made from imports/arm/*.def: NT 4.0's export lists, the same names on Alpha
# (ntdll's came from the Alpha CD's alphadef.src).
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
win() { printf 'Z:%s' "$(cd "$1" && pwd | tr / '\\')"; }
inc=$(win "$here/include")
cpu=$1 d=$2
# Alpha: IEEE floating point (NaN, infinities, division by zero) without traps, as Rust needs:
# without it, a comparison with a NaN raises c0000090 (probe: tools/ntsdk-spike/probes/nan.c)
case $cpu in alpha) cflags=-QAieee1 ;; *) cflags= ;; esac
peak() { cat /sys/fs/cgroup/memory.peak 2>/dev/null || echo "?"; }
rc=0 objs=
for c in "$d"/*.c; do
    n=$(basename "$c" .c)
    start=$(date +%s)
    ntsdk nt40 "$cpu" sh -c 'INCLUDE="$2;$INCLUDE" exec cl -nologo -c -W1 $3 -Fo"$0" "$1"' \
        "$d/$n-$cpu.obj" "$c" "$inc" "$cflags" > "$d/$n-cl-$cpu.log" 2>&1
    s=$?
    echo "cl $n.c ($cpu): exit $s, $(( $(date +%s) - start )) s, $(wc -c < "$c") bytes of C, container peak memory $(peak) bytes, object $(wc -c < "$d/$n-$cpu.obj" 2>/dev/null || echo none)"
    grep -E 'error|fatal' "$d/$n-cl-$cpu.log" | head -30
    [ $s = 0 ] || rc=1
    case $n in *.exe) ;; *) objs="$objs $n-$cpu.obj" ;; esac
done
[ $rc = 0 ] || exit 1
for dll in ntdll userenv; do
    cp "$repo/imports/arm/$dll.def" "$d/$dll-$cpu.def"
    ( cd "$d" && ntsdk nt40 "$cpu" lib -nologo -machine:$cpu -def:$dll-$cpu.def -out:$dll-$cpu.lib ) > "$d/lib-$cpu.log" 2>&1 \
        || { cat "$d/lib-$cpu.log"; exit 1; }
done
# ntrun has no sockets (no WS2_32.DLL) and no USERENV.DLL: stand-ins next to the programs,
# for these runs
# and ntrun-shims.c: functions std calls that ntrun lacks (NtWriteFile...), ahead of the
# import libraries, for these runs only
cp "$here/ntrun-stubs/ntrun-shims.c" "$d/ntrun-shims-$cpu.c"
( cd "$d" && ntsdk nt40 "$cpu" cl -nologo -c -W1 $cflags -Fontrun-shims-$cpu.obj ntrun-shims-$cpu.c ) > "$d/ntrun-shims-$cpu.log" 2>&1 \
    || { cat "$d/ntrun-shims-$cpu.log"; exit 1; }
# (made in a folder of their own: the stand-in's import library mustn't replace the SDK's at
# link; only the DLL goes next to the program)
mkdir -p "$d/ntrun-dlls"
dlldir=$(win "$d/ntrun-dlls")
link_run() { # link_run NAME
    start=$(date +%s)
    ( cd "$d" && ntsdk nt40 "$cpu" link -nologo -out:$1-$cpu.exe -subsystem:console $1.exe-$cpu.obj ntrun-shims-$cpu.obj $objs \
        kernel32.lib advapi32.lib ws2_32.lib userenv-$cpu.lib ntdll-$cpu.lib ) > "$d/link-$1-$cpu.log" 2>&1
    s=$?
    echo "link $1 ($cpu): exit $s, $(( $(date +%s) - start )) s, $(wc -c < "$d/$1-$cpu.exe" 2>/dev/null || echo no) bytes"
    head -80 "$d/link-$1-$cpu.log"
    [ $s = 0 ] || return 1
    for dll in ws2_32 userenv; do
        [ -f "$d/ntrun-dlls/$dll.dll" ] || "$here/ntrun-stubs/mkstub.sh" "$cpu" "$d/$1-$cpu.exe" "$d/ntrun-dlls" $dll || return 1
        cp "$d/ntrun-dlls/$dll.dll" "$d/"  # ntrun looks next to the program
    done
    ( cd "$d" && ntsdk nt40 "$cpu" sh -c 'NTRUN_DLLPATH="$1;$NTRUN_DLLPATH" exec ntrun $2-$3.exe' sh "$dlldir" "$1" "$cpu" ) 2>&1 \
        | tee "$d/run-$1-$cpu.log"
}
link_run stdsmoke || exit 1
grep -q '^stdsmoke: ok' "$d/run-stdsmoke-$cpu.log" || exit 1
link_run stdtest || exit 1
grep -q '^stdtest: ok' "$d/run-stdtest-$cpu.log" || echo "stdtest: does not pass under ntrun (see above; PLAN-mips-alpha-ppc.md)"
