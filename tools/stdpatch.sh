#!/usr/bin/env bash
# Edit the std patch (std/<version>/nt4.patch) as files:
#
#   tools/stdpatch.sh open    # target/stdwork/a: rust-src as released; b: with the patch
#   ...edit target/stdwork/b/library...
#   tools/stdpatch.sh save    # the patch again, from the difference of a and b
#
# crates/nt4compat is not part of the patch: tools/mkstd.sh adds it to the source.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
VERSION=$(rustc --version | awk '{print $2}')
PATCH=std/$VERSION/nt4.patch
W=target/stdwork
case ${1:-} in
    open)
        SRC=$(rustc --print sysroot)/lib/rustlib/src/rust/library
        rm -rf "$W"; mkdir -p "$W/a" "$W/b"
        cp -r "$SRC" "$W/a/library"; cp -r "$SRC" "$W/b/library"; chmod -R u+w "$W"
        [ -f "$PATCH" ] && patch -s -d "$W/b" -p1 < "$PATCH"
        echo "edit $W/b/library, then: $0 save" ;;
    save)
        mkdir -p "$(dirname "$PATCH")"
        # Without the files' times in the headers, so the same edit gives the same patch.
        { (cd "$W" && diff -ruN a/library b/library) || [ $? -eq 1 ]; } \
            | sed -E 's/^(---|\+\+\+) ([^\t]+)\t.*/\1 \2/' > "$PATCH"
        echo "$PATCH: $(grep -c '^+++' "$PATCH") files" ;;
    *) echo "usage: $0 open|save" >&2; exit 2 ;;
esac
