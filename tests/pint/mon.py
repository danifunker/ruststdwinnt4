#!/usr/bin/env python3
"""Send commands to the QEMU monitor socket of tests/pint/qemu.sh.

  tests/pint/mon.py WORKDIR screendump out.ppm
  tests/pint/mon.py WORKDIR sendkey ret
  tests/pint/mon.py WORKDIR type "some text"      # one sendkey per character
  tests/pint/mon.py WORKDIR settle [--since PPM] [--graphics] [--quiet S] [--timeout S]

settle waits until the screen has stopped changing: until two screen dumps QUIET seconds
apart (default 3) differ in fewer than 64 pixels (a blinking caret), and exits 1 if that
takes over TIMEOUT seconds (default 120). --since first waits for the screen to differ
from a dump taken before a key was sent (by the time this starts, the guest may have
reacted already); --graphics for a graphics mode of at least 640x480, past the text
screens of the BIOS and the boot.
"""

import argparse

import os
import socket
import sys
import time

KEYS = {" ": "spc", "\n": "ret", "\\": "backslash", ".": "dot", ":": "shift-semicolon",
        "-": "minus", ">": "shift-dot", "<": "shift-comma", "_": "shift-minus",
        "/": "slash", "=": "equal", ",": "comma", ";": "semicolon", '"': "shift-apostrophe"}


def key(c):
    if c in KEYS:
        return KEYS[c]
    if c.isupper():
        return "shift-" + c.lower()
    return c


def send(sock_path, lines):
    s = socket.socket(socket.AF_UNIX)
    s.connect(sock_path)
    s.settimeout(2)
    out = b""

    def drain():
        nonlocal out
        try:
            while True:
                chunk = s.recv(65536)
                if not chunk:
                    break
                out += chunk
        except socket.timeout:
            pass

    drain()
    for line in lines:
        s.sendall(line.encode() + b"\n")
        time.sleep(0.15)
        drain()
    s.close()
    return out.decode(errors="replace")


def read_ppm(f):
    """(width, height, RGB bytes) of a screen dump, or None while it is incomplete."""
    try:
        with open(f, "rb") as fh:
            d = fh.read()
        magic, w, h, maxval, data = d.split(maxsplit=4)
        if magic == b"P6" and len(data) == int(w) * int(h) * 3:
            return int(w), int(h), data
    except (OSError, ValueError):
        pass
    return None


def screen(sock, n):
    """The screen as (width, height, RGB bytes), from a screen dump."""
    f = os.path.abspath(f"settle{n % 2}.ppm")  # QEMU may run elsewhere (-daemonize: /)
    try:
        os.unlink(f)  # not the last dump to this name
    except FileNotFoundError:
        pass
    send(sock, [f"screendump {f}"])
    for _ in range(50):
        s = read_ppm(f)
        if s:
            return s
        time.sleep(0.1)
    return 0, 0, b""


def differ(a, b, limit=64):
    """Whether two screens differ in at least LIMIT pixels."""
    if a[:2] != b[:2]:
        return True
    x, y, n = a[2], b[2], 0
    for i in range(0, len(x), 3 * 1024):  # compare runs of 1024 pixels, then pixels
        if x[i : i + 3072] != y[i : i + 3072]:
            for j in range(i, min(i + 3072, len(x)), 3):
                if x[j : j + 3] != y[j : j + 3]:
                    n += 1
                    if n >= limit:
                        return True
    return False


def settle(sock, args):
    ap = argparse.ArgumentParser(prog="mon.py WORKDIR settle")
    ap.add_argument("--since")
    ap.add_argument("--graphics", action="store_true")
    ap.add_argument("--quiet", type=float, default=3)
    ap.add_argument("--timeout", type=float, default=120)
    a = ap.parse_args(args)
    end, n = time.time() + a.timeout, 0
    prev = screen(sock, n)
    since = read_ppm(a.since) if a.since else None
    changed = since is None or differ(prev, since)
    still_since = time.time()
    while time.time() < end:
        time.sleep(min(1.0, a.quiet / 2))
        n += 1
        cur = screen(sock, n)
        if not changed and differ(cur, since):
            changed = True
        if differ(cur, prev) or not changed or (a.graphics and (cur[0] < 640 or cur[1] < 480)):
            still_since = time.time()
        elif time.time() - still_since >= a.quiet:
            return 0
        prev = cur
    print(f"settle: the screen did not settle in {a.timeout:.0f} s", file=sys.stderr)
    return 1


def main():
    w, cmd, *rest = sys.argv[1:]
    # A relative socket path: a Unix socket's path must be under 108 bytes. Screen dumps
    # name files relative to QEMU's directory, which is WORKDIR too.
    os.chdir(w)
    sock = "mon.sock"
    if cmd == "settle":
        sys.exit(settle(sock, rest))
    if cmd == "type":
        send(sock, [f"sendkey {key(c)}" for c in " ".join(rest)])
    else:
        print(send(sock, [" ".join([cmd, *rest])]))


if __name__ == "__main__":
    main()
