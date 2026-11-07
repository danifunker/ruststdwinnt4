#!/usr/bin/env bash
# Boot a PINT SD image in QEMU raspi2b without a window, as PINT's build/run.sh does, with
# its serial console written to a file and the QEMU monitor on a Unix socket (for sendkey
# and screendump; tests/pint/mon.py talks to it).
#
#   tests/pint/qemu.sh IMAGE WORKDIR [QEMU ARGS...]
#
# WORKDIR gets arcfw.elf (ARCFW=path: that firmware; otherwise made from the image's
# arcfw.bin), serial.log, mon.sock and qemu.pid. The image is changed in place: NT writes
# to its disk. NT gets PINT's USB Ethernet adapter on QEMU's user-mode network (NONET=1:
# none); NT's first run expects it. QEMU=path: another qemu-system-arm, such as the one in
# PINT's portable package (bin/linux-x86_64; its lib/ is put on LD_LIBRARY_PATH), when
# the system's has no user-mode networking.
set -euo pipefail
IMG=$1 W=$2; shift 2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$W"
if [ -n "${ARCFW:-}" ]; then
    cp "$ARCFW" "$W/arcfw.elf"
elif [ ! -f "$W/arcfw.elf" ]; then
    mcopy -o -i "$IMG@@1M" ::/arcfw.bin "$W/arcfw.bin"
    python3 -I "$HERE/bin2elf.py" "$W/arcfw.bin" "$W/arcfw.elf"
fi
rm -f "$W/mon.sock"
IMG="$(realpath "$IMG")"
# A relative socket path: a Unix socket's path must be under 108 bytes.
QEMU=${QEMU:-qemu-system-arm}
if [ -d "$(dirname "$QEMU")/lib" ]; then
    export LD_LIBRARY_PATH="$(cd "$(dirname "$QEMU")/lib" && pwd)${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
case $QEMU in /*) ;; */*) QEMU="$(realpath "$QEMU")" ;; esac
cd "$W"
exec "$QEMU" -M raspi2b -m 1G -kernel arcfw.elf \
    -drive if=sd,format=raw,file="$IMG" \
    -device ds1338,bus=i2c-bus.1,address=0x68 -rtc base=localtime \
    -device usb-kbd -device usb-mouse \
    ${NONET:+-nic none} ${NONET:--netdev user,id=n0 -device usb-net,netdev=n0} \
    -global bcm2835-fb.xres=1024 -global bcm2835-fb.yres=768 \
    -display none -serial file:serial.log \
    -monitor unix:mon.sock,server,nowait -pidfile qemu.pid "$@"
