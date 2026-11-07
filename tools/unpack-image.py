#!/usr/bin/env python3
"""Unpack an OCI/Docker image's layers into a directory, as its root filesystem.

    unpack-image.py MANIFEST.json BLOBS_DIR ROOTFS

Layers are applied in order, with their whiteouts (.wh.NAME deletes NAME, .wh..wh..opq
empties the directory). Files belong to the user running this; device nodes are skipped
(the sandbox gives /dev); set-id bits are dropped.
"""
import json, os, shutil, stat, sys, tarfile

manifest, blobs, root = sys.argv[1:4]
root = os.path.abspath(root)


def target(name):
    p = os.path.normpath(os.path.join(root, name))
    if not (p == root or p.startswith(root + "/")):
        sys.exit(f"unpack-image: {name} leaves the root")
    return p


def remove(p):
    if os.path.isdir(p) and not os.path.islink(p):
        shutil.rmtree(p)
    elif os.path.lexists(p):
        os.remove(p)


for layer in json.load(open(manifest))["layers"]:
    with tarfile.open(os.path.join(blobs, layer["digest"][7:]), "r:*") as t:
        members = t.getmembers()
        for m in members:
            base, d = os.path.basename(m.name), os.path.dirname(m.name)
            if base == ".wh..wh..opq":
                p = target(d)
                if os.path.isdir(p):
                    for e in os.listdir(p):
                        remove(os.path.join(p, e))
            elif base.startswith(".wh."):
                remove(target(os.path.join(d, base[4:])))
        keep = []
        for m in members:
            if os.path.basename(m.name).startswith(".wh.") or m.ischr() or m.isblk() or m.isfifo():
                continue
            p = target(m.name)
            if os.path.lexists(p) and not (m.isdir() and os.path.isdir(p) and not os.path.islink(p)):
                remove(p)
            m.uid = m.gid = os.getuid()
            m.uname = m.gname = ""
            m.mode = (m.mode | stat.S_IRUSR | stat.S_IWUSR | (stat.S_IXUSR if m.isdir() else 0)) & ~(stat.S_ISUID | stat.S_ISGID)
            keep.append(m)
        t.extractall(root, members=keep, filter="fully_trusted")
