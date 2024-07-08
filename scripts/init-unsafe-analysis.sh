#!/bin/bash

set -e

ROOT=$(git rev-parse --show-toplevel)
HERE=$ROOT/scripts
TOOL=$ROOT/third-party/cargo-geiger

$HERE/deps/rust.sh

# MIRI Setup
rustup component add --toolchain nightly-2024-04-21-x86_64-unknown-linux-gnu miri

# Cargo Geiger Setup
git submodule update --init $TOOL
cargo +stable install cargo-geiger --force --locked \
	--path $TOOL/cargo-geiger \
	--target x86_64-unknown-linux-gnu

# Unsafe Analyzer Setup
rustup component add rust-src rustc-dev llvm-tools-preview

cd $ROOT/lib/unsafe-analyzer
cargo build --release
mkdir -p bin
cp ../../out/x86_64-unknown-linux-gnu/release/unsafe_analyzer ./bin/rustc

mkdir -p lib
cp -r $(rustc --print sysroot)/lib/* ./lib/

rustup toolchain link unsafe-analyzer $ROOT/lib/unsafe-analyzer
