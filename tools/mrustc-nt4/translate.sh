#!/bin/sh
# translate.sh TARGET OUT [MRUSTC]: Rust to C with mrustc for core, alloc, nt4rt and
# examples/hello. TARGET is a file in targets/ without .toml. Rust 1.90's library sources
# come from LIBRARY (rustup's rust-src component). Writes OUT/*.c, and OUT/*.log on failure.
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
T=$1 O=$2
M=${3:-$HOME/repos/mrustc/bin/mrustc}
L=${LIBRARY:-$HOME/.rustup/toolchains/1.90.0-aarch64-apple-darwin/lib/rustlib/src/rust/library}
mkdir -p "$O"
export MRUSTC_TARGET_VER=1.90
t="--target $here/targets/$T.toml -L $O"
run() { # run NAME ARGS...: mrustc, its output in OUT/NAME.log, which is shown if it fails
    n=$1; shift
    if ! "$M" "$@" $t -C emit-build-command="$O/$n.cmd" > "$O/$n.log" 2>&1; then
        tail -20 "$O/$n.log"; echo "mrustc failed: $n" >&2; exit 1
    fi
}
run core "$L/core/src/lib.rs" --crate-name core --crate-type rlib --edition 2024 -o "$O/libcore.rlib"
run alloc "$L/alloc/src/lib.rs" --crate-name alloc --crate-type rlib --edition 2024 -o "$O/liballoc.rlib"
run nt4rt "$repo/crates/nt4rt/src/lib.rs" --crate-name nt4rt --crate-type rlib --edition 2021 \
    --cfg 'feature="rt"' --cfg 'feature="default"' -o "$O/libnt4rt.rlib"
run hello "$repo/examples/hello/src/main.rs" --crate-name hello --crate-type bin --edition 2021 \
    --extern nt4rt="$O/libnt4rt.rlib" -o "$O/hello.exe"
ls -l "$O"/*.c
