#!/bin/sh

set -ex

MUSL_TRIPLE=x86_64-linux-musl
RUST_TRIPLE=x86_64-unknown-linux-musl

rustup target add $RUST_TRIPLE

[ -e ${MUSL_TRIPLE}-cross ] || (curl -s https://musl.cc/${MUSL_TRIPLE}-cross.tgz | tar zx)

MUSL_BIN=`pwd`/${MUSL_TRIPLE}-cross/bin
export PATH=$MUSL_BIN:$PATH

export TARGET_CC=$MUSL_BIN/${MUSL_TRIPLE}-gcc

RUST_TRIPLE_ENV=$(echo $RUST_TRIPLE | tr 'a-z-' 'A-Z_')
export CARGO_TARGET_${RUST_TRIPLE_ENV}_CC=$TARGET_CC
export CARGO_TARGET_${RUST_TRIPLE_ENV}_LINKER=$TARGET_CC

cargo build --target $RUST_TRIPLE --release -p cargo-dinghy

mv target/${RUST_TRIPLE}/release/cargo-dinghy target/cargo-dinghy
