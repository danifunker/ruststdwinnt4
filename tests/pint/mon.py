#!/usr/bin/env python3
"""Send commands to the QEMU monitor socket of tests/pint/qemu.sh.

  tests/pint/mon.py WORKDIR screendump out.ppm
  tests/pint/mon.py WORKDIR sendkey ret
  tests/pint/mon.py WORKDIR type "some text"      # one sendkey per character
"""

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


def main():
    w, cmd, *rest = sys.argv[1:]
    # A relative socket path: a Unix socket's path must be under 108 bytes. Screen dumps
    # name files relative to QEMU's directory, which is WORKDIR too.
    os.chdir(w)
    sock = "mon.sock"
    if cmd == "type":
        send(sock, [f"sendkey {key(c)}" for c in " ".join(rest)])
    else:
        print(send(sock, [" ".join([cmd, *rest])]))


if __name__ == "__main__":
    main()
