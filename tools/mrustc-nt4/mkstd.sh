#!/bin/sh
# mkstd.sh OUT: the std source mrustc builds for NT 4.0, in OUT/library: Rust 1.90's
# library/ from LIBRARY (rustup's rust-src), std/1.90.0/nt4.patch, crates/nt4compat as
# library/nt4compat, then std-edits.py; and std's crates.io dependencies in OUT/vendor
# (cargo vendor, so it needs the network and a cargo).
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
O=$1
L=${LIBRARY:-$HOME/.rustup/toolchains/1.90.0-aarch64-apple-darwin/lib/rustlib/src/rust/library}
rm -rf "$O"; mkdir -p "$O"
cp -r "$L" "$O/library"
chmod -R u+w "$O/library"
patch -s -d "$O" -p1 < "$repo/std/1.90.0/nt4.patch"
mkdir -p "$O/library/nt4compat"
cp -r "$repo/crates/nt4compat/Cargo.toml" "$repo/crates/nt4compat/src" "$O/library/nt4compat/"
python3 "$here/std-edits.py" "$O/library"
( cd "$O/library" && RUSTC_BOOTSTRAP=1 cargo vendor --manifest-path Cargo.toml ../vendor > ../vendor.log 2>&1 ) \
    || { tail -20 "$O/vendor.log"; exit 1; }
