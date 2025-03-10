#!/bin/bash

# This assumes that I live on x86-64 linux. Which I do, not sure about you.

mkdir artifacts

cargo build --release
mkdir artifacts/linux-x86-64 || continue
cp target/release/libtypst_shared.so artifacts/linux-x86-64/libtypst_shared.so || continue

#rm -rf target

# pamac install mingw-w64
# rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu --release
mkdir artifacts/windows-x86-64 || continue
cp target/x86_64-pc-windows-gnu/release/typst_shared.dll artifacts/windows-x86-64/typst_shared.dll || continue

#rm -rf target
