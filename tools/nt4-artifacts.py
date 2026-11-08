#!/usr/bin/env python3
"""Read cargo's JSON messages on stdin and list the PE images it made, for tools/cargo-nt4.

    cargo build --message-format=json-render-diagnostics ... | tools/nt4-artifacts.py OUT [--quiet]

Writes the path of every .exe and .dll of a compiler-artifact message (fresh or not) to
OUT, one per line. Without --quiet, every line is passed on to stdout, for a caller that
asked cargo for its messages; with it, only the lines that are not JSON (a build that
printed something of its own).
"""

import json
import sys


def main():
    out, quiet = sys.argv[1], "--quiet" in sys.argv[2:]
    images = []
    for line in sys.stdin:
        msg = None
        if line.startswith("{"):
            try:
                msg = json.loads(line)
            except ValueError:
                pass
        if not quiet or msg is None:
            sys.stdout.write(line)
            sys.stdout.flush()
        if msg and msg.get("reason") == "compiler-artifact":
            for f in msg.get("filenames") or []:
                if f.lower().endswith((".exe", ".dll")) and f not in images:
                    images.append(f)
    with open(out, "w") as f:
        f.writelines(i + "\n" for i in images)


if __name__ == "__main__":
    main()
