#!/usr/bin/env bash
# Run NT programs on PINT in QEMU and print what they wrote.
#
#   tests/pint/run-exe.sh BASE_IMAGE PROGRAM.EXE...
#
# BASE_IMAGE is a PINT SD image past NT's first run (it logs on as Administrator by
# itself); it is not changed. A copy gets the programs in C:\TMP\RUST and a batch file in
# the Startup folder that runs each one, its standard output and error to NAME.OUT and its
# exit code to NAME.RC, then restarts NT with REBOOT.EXE (examples/reboot: NtShutdownSystem,
# which flushes the disk; the SDK's SHUTDOWN.EXE does not restart from there). The
# firmware's banner on the serial console at that restart ends the run: QEMU is stopped
# and the results are read off the copy.
#
# Environment: TIMEOUT (seconds, default 300), KEEP=1 keeps the work directory,
# REBOOT=path of reboot.exe (default: the release build of examples/reboot).
# Exit status: 0 when every program exited with 0.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE=$1; shift
[ $# -gt 0 ] || { echo "usage: $0 BASE_IMAGE PROGRAM.EXE..." >&2; exit 2; }
TIMEOUT=${TIMEOUT:-300}
W=$(mktemp -d "${TMPDIR:-/tmp}/pint-run.XXXXXX")
IMG=$W/sd.img
I="$IMG@@1M"
STARTUP="::/WINNT/Profiles/All Users/Start Menu/Programs/Startup"

cleanup() {
    [ -f "$W/qemu.pid" ] && kill "$(cat "$W/qemu.pid")" 2>/dev/null || true
    [ -n "${KEEP:-}" ] && echo "work directory: $W" || rm -rf "$W"
}
trap cleanup EXIT

REBOOT=${REBOOT:-$HERE/../../target/thumbv7a-nt4-windows-msvc/release/reboot.exe}
[ -f "$REBOOT" ] || { echo "run-exe: no $REBOOT (tools/cargo-nt4 build --release -p reboot)" >&2; exit 2; }

cp "$BASE" "$IMG"
mmd -i "$I" ::/TMP/RUST 2>/dev/null || true
mcopy -o -i "$I" "$REBOOT" ::/TMP/RUST/REBOOT.EXE
{
    printf '@echo off\r\ncd /d C:\\TMP\\RUST\r\n'
    for exe in "$@"; do
        name=$(basename "$exe" .exe | tr a-z A-Z | cut -c1-8)
        mcopy -o -i "$I" "$exe" "::/TMP/RUST/$name.EXE"
        printf '%s.EXE > %s.OUT 2>&1\r\necho %%ERRORLEVEL%% > %s.RC\r\n' "$name" "$name" "$name"
    done
    printf 'REBOOT.EXE > REBOOT.OUT 2>&1\r\n'
} > "$W/RUNTESTS.CMD"
mcopy -o -i "$I" "$W/RUNTESTS.CMD" "$STARTUP/RUNTESTS.CMD"

"$HERE/qemu.sh" "$IMG" "$W" -daemonize
start=$(date +%s)
# The firmware banner: once at the first boot, again at the restart.
until [ "$(grep -ac 'ARC Firmware' "$W/serial.log" 2>/dev/null)" -ge 2 ]; do
    if [ $(( $(date +%s) - start )) -ge "$TIMEOUT" ]; then
        echo "run-exe: no restart within ${TIMEOUT}s" >&2
        "$HERE/mon.py" "$W" screendump "$W/timeout.ppm" >/dev/null || true
        KEEP=1
        exit 1
    fi
    sleep 2
done
kill "$(cat "$W/qemu.pid")"; rm -f "$W/qemu.pid"
sleep 1

status=0
for exe in "$@"; do
    name=$(basename "$exe" .exe | tr a-z A-Z | cut -c1-8)
    rc=$(mtype -i "$I" "::/TMP/RUST/$name.RC" 2>/dev/null | tr -d '\r\n ' || true)
    echo "=== $name (exit ${rc:-none})"
    mtype -i "$I" "::/TMP/RUST/$name.OUT" 2>/dev/null | tr -d '\r' || echo "(no output)"
    [ "$rc" = 0 ] || status=1
done
exit $status
