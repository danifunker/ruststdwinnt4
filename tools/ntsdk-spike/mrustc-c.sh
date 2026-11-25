#!/bin/sh
# mrustc-c.sh TARGET [MRUSTC]: translate core, alloc, nt4rt and the spike's hello to C with mrustc,
# without compiling it (-C emit-build-command). TARGET is a file in targets/ without .toml.
# Needs Rust 1.90's library sources in LIBRARY (rustup's rust-src component has them).
# Output: out-TARGET/*.c
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
T=$1
M=${2:-$HOME/repos/mrustc/bin/mrustc}
L=${LIBRARY:-$HOME/.rustup/toolchains/1.90.0-aarch64-apple-darwin/lib/rustlib/src/rust/library}
O=out-$T
mkdir -p "$O"
export MRUSTC_TARGET_VER=1.90
t="--target $here/targets/$T.toml -L $O"
$M "$L/core/src/lib.rs" --crate-name core --crate-type rlib --edition 2024 $t -o $O/libcore.rlib -C emit-build-command=$O/core.cmd > $O/core.log 2>&1
$M "$L/alloc/src/lib.rs" --crate-name alloc --crate-type rlib --edition 2024 $t -o $O/liballoc.rlib -C emit-build-command=$O/alloc.cmd > $O/alloc.log 2>&1
$M "$repo/crates/nt4rt/src/lib.rs" --crate-name nt4rt --crate-type rlib --edition 2021 \
    --cfg 'feature="rt"' --cfg 'feature="default"' $t -o $O/libnt4rt.rlib -C emit-build-command=$O/nt4rt.cmd > $O/nt4rt.log 2>&1
# mrustc has no #![no_main]: hello/main.rs is examples/hello with a start lang item instead of nt4rt::entry!
$M "$here/hello/main.rs" --crate-name hello --crate-type bin --edition 2021 $t \
    --extern nt4rt=$O/libnt4rt.rlib -o $O/hello.exe -C emit-build-command=$O/hello.cmd > $O/hello.log 2>&1
ls -l $O/*.c
