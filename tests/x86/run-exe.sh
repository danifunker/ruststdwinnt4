#!/usr/bin/env bash
# Run programs on x86 Windows NT 4.0 in QEMU and print what they wrote.
#
#   tests/x86/run-exe.sh BASE_IMAGE PROGRAM.EXE...
#
# BASE_IMAGE is an x86 NT 4.0 disk (qcow2 or raw) whose Administrator has an empty
# password; it is not changed (the run writes to an overlay). The programs go on a
# CD-ROM with RUNTESTS.CMD, which runs each one with its standard output and error to
# C:\TMP\RUST\NAME.OUT and its exit code to NAME.RC. The run logs on and starts
# RUNTESTS.CMD by typing into QEMU's monitor (sendkey), waits, stops QEMU and reads
# C:\TMP\RUST off the overlay. NT 4.0's disk is NTFS, which Linux can read here but
# shouldn't write, hence the CD and the typing.
#
# Environment: WAIT (seconds the programs get, default 120), ARGS_<NAME> (a program's
# command line), OUTDIR=dir (copy every file the programs left there), KEEP=1 (keep the
# work directory), QEMU (default qemu-system-i386 with user networking: the guest
# reaches the host as 10.0.2.2 and the host's networks through NAT).
# Exit status: 0 when every program exited with 0.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MON="$HERE/../pint/mon.py"
BASE=$(realpath "$1"); shift
[ $# -gt 0 ] || { echo "usage: $0 BASE_IMAGE PROGRAM.EXE..." >&2; exit 2; }
WAIT=${WAIT:-120}
QEMU=${QEMU:-qemu-system-i386}
W=$(mktemp -d "${TMPDIR:-/tmp}/x86-run.XXXXXX")

cleanup() {
    [ -f "$W/qemu.pid" ] && kill "$(cat "$W/qemu.pid")" 2>/dev/null || true
    [ -n "${KEEP:-}" ] && echo "work directory: $W" || rm -rf "$W"
}
trap cleanup EXIT

mkdir "$W/cd"
names=()
{
    printf '@echo off\r\nmkdir C:\\TMP\r\nmkdir C:\\TMP\\RUST\r\ncd /d C:\\TMP\\RUST\r\n'
    for exe in "$@"; do
        name=$(basename "$exe" .exe | tr a-z A-Z | cut -c1-8)
        names+=("$name")
        cp "$exe" "$W/cd/$name.EXE"
        args_var=ARGS_$name
        printf '%%CD%%\\%s.EXE %s > %s.OUT 2>&1\r\necho %%ERRORLEVEL%% > %s.RC\r\n' \
            "$name" "${!args_var:-}" "$name" "$name"
    done
    printf 'echo done > DONE.TXT\r\n'
} > "$W/cd/RUNTESTS.CMD"
# RUNTESTS.CMD runs from C:\TMP\RUST; %CD% there would be C:, so name the CD-ROM.
sed -i 's/%CD%/D:/' "$W/cd/RUNTESTS.CMD"
genisoimage -quiet -J -r -V NT4TEST -o "$W/cd.iso" "$W/cd"

qemu-img create -q -f qcow2 -b "$BASE" -F "$(qemu-img info --output=json "$BASE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["format"])')" "$W/disk.qcow2"
"$QEMU" -cpu pentium -vga cirrus -m 256 -hda "$W/disk.qcow2" -cdrom "$W/cd.iso" \
    -netdev user,id=n0 -device pcnet,netdev=n0 -display none \
    -monitor "unix:$W/mon.sock,server,nowait" -daemonize -pidfile "$W/qemu.pid"

mon() { python3 "$MON" "$W" "$@" >/dev/null; }
sleep 90                        # boot to "Press Ctrl+Alt+Delete"
mon sendkey ret                 # any boot-time message box
sleep 2
mon sendkey ctrl-alt-delete
sleep 5
mon sendkey ret                 # Administrator, empty password
sleep 25
mon sendkey esc                 # the Welcome screen
sleep 2
mon sendkey ctrl-esc            # Start
sleep 2
mon sendkey r                   # Run...
sleep 3
mon type 'D:\RUNTESTS.CMD'
mon sendkey ret
sleep "$WAIT"
mon screendump "$W/screen.ppm"
sleep 15                        # NT's lazy writer
kill "$(cat "$W/qemu.pid")"; rm -f "$W/qemu.pid"
sleep 2

qemu-img convert -O raw "$W/disk.qcow2" "$W/disk.raw"
mkdir "$W/out"
7z e -y -o"$W/out" "$W/disk.raw" 'TMP/RUST/*' >/dev/null 2>&1 || true
rc=0
[ -f "$W/out/DONE.TXT" ] || { echo "run-exe: RUNTESTS.CMD did not finish (see screen.ppm with KEEP=1)" >&2; rc=1; }
for name in "${names[@]}"; do
    code=$(tr -d '\r\n ' < "$W/out/$name.RC" 2>/dev/null || echo "?")
    echo "=== $name (exit $code)"
    cat "$W/out/$name.OUT" 2>/dev/null | tr -d '\r' || echo "(no output)"
    [ "$code" = 0 ] || rc=1
done
if [ -n "${OUTDIR:-}" ]; then
    mkdir -p "$OUTDIR"; cp "$W/out/"* "$W/screen.ppm" "$OUTDIR/" 2>/dev/null || true
    echo "files: $OUTDIR"
fi
exit $rc
