#!/usr/bin/env bash
# Run programs on x86 Windows NT 4.0 in QEMU and print what they wrote.
#
#   tests/x86/run-exe.sh BASE_IMAGE PROGRAM.EXE...
#
# BASE_IMAGE is an x86 NT 4.0 disk (qcow2 or raw) that boots to "Press Ctrl+Alt+Delete"
# and logs on to an administrator account (one with the shutdown privilege) by
# typing; it is not changed (the run writes to an overlay). The programs go on a CD-ROM
# with RUNTESTS.CMD, which runs each one with its standard output and error to
# C:\TMP\RUST\NAME.OUT and its exit code to NAME.RC, then restarts NT with REBOOT.EXE
# (examples/reboot: NtShutdownSystem, which flushes the disk). QEMU runs with -no-reboot,
# so that restart ends it; the results are then read off the overlay. The run logs on and
# starts RUNTESTS.CMD by typing into QEMU's monitor (sendkey), each step once the screen
# has reacted to the last and settled (tests/pint/mon.py settle). NT 4.0's disk is NTFS,
# which Linux can read here but shouldn't write, hence the CD and the typing.
#
# Environment:
#   NT_USER       the account to log on to (default: the one the logon dialog shows, which
#                 is the last to log on)
#   NT_PASSWORD   its password (default: empty)
#   TIMEOUT       seconds RUNTESTS.CMD gets once started (default 600)
#   ARGS_<NAME>   a program's command line
#   OUTDIR=dir    copy every file the programs left, and screen dumps, there
#   KEEP=1        keep the work directory (screen dumps of each step: NN-step.ppm)
#   QEMU          qemu-system-i386 with user networking (default: the first on PATH, then
#                 /usr/bin's, that has it); the guest reaches the host as 10.0.2.2
#   REBOOT        reboot.exe for x86 (default: the release build of examples/reboot)
# Exit status: 0 when every program exited with 0.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MON="$HERE/../pint/mon.py"
[ $# -gt 1 ] || { echo "usage: $0 BASE_IMAGE PROGRAM.EXE..." >&2; exit 2; }
BASE=$(realpath "$1"); shift
TIMEOUT=${TIMEOUT:-600}
REBOOT=${REBOOT:-$HERE/../../target/i686-nt4-windows-msvc/release/reboot.exe}
[ -f "$REBOOT" ] || {
    echo "run-exe: no $REBOOT (NT4_TARGET=i686-nt4-windows-msvc tools/cargo-nt4 build --release -p reboot)" >&2
    exit 2
}

has_user_net() { "$1" -netdev help 2>/dev/null | grep -qx user; }
if [ -z "${QEMU:-}" ]; then
    for q in $(type -ap qemu-system-i386) /usr/bin/qemu-system-i386; do
        if has_user_net "$q"; then QEMU=$q; break; fi
    done
    [ -n "${QEMU:-}" ] || { echo "run-exe: no qemu-system-i386 with user networking (-netdev user); set QEMU" >&2; exit 2; }
elif ! has_user_net "$QEMU"; then
    echo "run-exe: $QEMU has no user networking (-netdev user)" >&2; exit 2
fi

W=$(mktemp -d "${TMPDIR:-/tmp}/x86-run.XXXXXX")
cleanup() {
    [ -f "$W/qemu.pid" ] && kill "$(cat "$W/qemu.pid")" 2>/dev/null || true
    [ -n "${KEEP:-}" ] && echo "work directory: $W" || rm -rf "$W"
}
trap cleanup EXIT

mkdir "$W/cd"
cp "$REBOOT" "$W/cd/REBOOT.EXE"
names=()
{
    printf '@echo off\r\nmkdir C:\\TMP\r\nmkdir C:\\TMP\\RUST\r\ncd /d C:\\TMP\\RUST\r\n'
    for exe in "$@"; do
        name=$(basename "$exe" .exe | tr a-z A-Z | cut -c1-8)
        names+=("$name")
        cp "$exe" "$W/cd/$name.EXE"
        args_var=ARGS_$name
        printf 'D:\\%s.EXE %s > %s.OUT 2>&1\r\necho %%ERRORLEVEL%% > %s.RC\r\n' \
            "$name" "${!args_var:-}" "$name" "$name"
    done
    printf 'echo done > DONE.TXT\r\nD:\\REBOOT.EXE > REBOOT.OUT 2>&1\r\n'
} > "$W/cd/RUNTESTS.CMD"
genisoimage -quiet -J -r -V NT4TEST -o "$W/cd.iso" "$W/cd"

fmt=$(qemu-img info --output=json "$BASE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["format"])')
qemu-img create -q -f qcow2 -b "$BASE" -F "$fmt" "$W/disk.qcow2"
"$QEMU" -cpu pentium -vga cirrus -m 256 -hda "$W/disk.qcow2" -cdrom "$W/cd.iso" \
    -netdev user,id=n0 -device pcnet,netdev=n0 -display none -no-reboot \
    -monitor "unix:$W/mon.sock,server,nowait" -daemonize -pidfile "$W/qemu.pid"

running() { [ -f "$W/qemu.pid" ] && kill -0 "$(cat "$W/qemu.pid")" 2>/dev/null; }
mon() { python3 "$MON" "$W" "$@" >/dev/null; }
step=0 last=
# Wait for the screen to settle (mon.py settle ARGS), keeping a dump of it as NN-WHAT.ppm.
# A step after the first waits for the screen to change from the last step's dump first.
settle() {
    local what=$1; shift
    step=$((step + 1))
    python3 "$MON" "$W" settle ${last:+--since "$last"} "$@" || echo "run-exe: carrying on after: $what" >&2
    last=$W/$(printf '%02d' $step)-$what.ppm
    mon screendump "$last"
    sleep 0.5                                     # QEMU writes it
}

settle boot --graphics --quiet 10 --timeout 300   # "Press Ctrl+Alt+Delete"
mon sendkey ret                                   # any boot-time message box
mon sendkey ctrl-alt-delete
settle logon-dialog --timeout 60
if [ -n "${NT_USER:-}" ]; then
    mon sendkey shift-tab                         # the user name, selected
    mon type "$NT_USER"
    mon sendkey tab
fi
[ -z "${NT_PASSWORD:-}" ] || mon type "$NT_PASSWORD"
mon sendkey ret
settle desktop --quiet 8 --timeout 180   # the desktop, then the Welcome screen
mon sendkey esc                                   # the Welcome screen
mon sendkey ctrl-esc                              # Start
settle start-menu --timeout 30
mon sendkey r                                     # Run...
settle run-dialog --timeout 30
mon type 'D:\RUNTESTS.CMD'
mon sendkey ret

# RUNTESTS.CMD ends by restarting NT, which ends QEMU.
end=$((SECONDS + TIMEOUT))
while running && [ $SECONDS -lt $end ]; do sleep 2; done
if running; then
    echo "run-exe: still running after $TIMEOUT s; stopping QEMU (results may be incomplete)" >&2
    mon screendump "$W/timeout.ppm"
    sleep 15                                      # NT's lazy writer
    kill "$(cat "$W/qemu.pid")"
    sleep 2
fi
rm -f "$W/qemu.pid"

qemu-img convert -O raw "$W/disk.qcow2" "$W/disk.raw"
mkdir "$W/out"
7z e -y -o"$W/out" "$W/disk.raw" 'TMP/RUST/*' >/dev/null 2>&1 || true
rm -f "$W/disk.raw"
rc=0
[ -f "$W/out/DONE.TXT" ] || { echo "run-exe: RUNTESTS.CMD did not finish (screen dumps with KEEP=1)" >&2; rc=1; }
for name in "${names[@]}"; do
    code=$(tr -d '\r\n ' < "$W/out/$name.RC" 2>/dev/null || echo "?")
    echo "=== $name (exit $code)"
    tr -d '\r' < "$W/out/$name.OUT" 2>/dev/null || echo "(no output)"
    [ "$code" = 0 ] || rc=1
done
[ -s "$W/out/REBOOT.OUT" ] && { echo "=== REBOOT"; tr -d '\r' < "$W/out/REBOOT.OUT"; }
if [ -n "${OUTDIR:-}" ]; then
    mkdir -p "$OUTDIR"; cp "$W/out/"* "$W/"*.ppm "$OUTDIR/" 2>/dev/null || true
    echo "files: $OUTDIR"
fi
exit $rc
