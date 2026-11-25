#!/bin/sh
# translate-std.sh TARGET STD OUT [MRUSTC [MINICARGO]]: Rust to C for std and its dependencies
# (minicargo, from STD = mkstd.sh's output), examples/stdtest and stdsmoke.rs, for TARGET (a file in
# targets/ without .toml). Writes OUT/*.c; OUT/std-build.log has minicargo's output.
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
T=$1 S=$2 O=$3
M=${4:-$HOME/repos/mrustc/bin/mrustc}
MC=${5:-$(dirname "$M")/minicargo}
mkdir -p "$O"
O=$(cd "$O" && pwd)
export MRUSTC_TARGET_VER=1.90 MRUSTC_REAL="$M" MRUSTC_PATH="$here/wrap/mrustc"
# std's build script output (as mrustc's own overrides for Linux give it); STD_ENV_ARCH is
# what std's env::consts::ARCH reads
export STD_ENV_ARCH=$(sed -n 's/^name = "\(.*\)"/\1/p' "$here/targets/$T.toml")
ovr=$(cd "$(dirname "$M")/.." && pwd)/script-overrides/stable-1.90.0-linux
if ! "$MC" --vendor-dir "$S/vendor" --script-overrides "$ovr" --output-dir "$O" \
        --target "$here/targets/$T.toml" "$S/library/std" > "$O/std-build.log" 2>&1; then
    grep -E "error|BUG|MIR ERROR|MIR TODO" "$O/std-build.log" | head -20; echo "minicargo failed: std" >&2; exit 1
fi
for p in stdtest:"$repo/examples/stdtest/src/main.rs" stdsmoke:"$here/stdsmoke.rs"; do
    n=${p%%:*} src=${p#*:}
    if ! "$M" "$src" --crate-name $n --crate-type bin --edition 2021 \
            -L "$O" --target "$here/targets/$T.toml" -o "$O/$n.exe" -C emit-build-command="$O/$n.cmd" \
            > "$O/$n.log" 2>&1; then
        tail -20 "$O/$n.log"; echo "mrustc failed: $n" >&2; exit 1
    fi
done
ls -l "$O"/*.c
