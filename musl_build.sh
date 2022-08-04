#!/bin/sh

set -ex

RUST_TRIPLE=x86_64-unknown-linux-musl

rustup target add $RUST_TRIPLE

cargo build --target $RUST_TRIPLE --release -p cargo-dinghy

mv target/${RUST_TRIPLE}/release/cargo-dinghy target/cargo-dinghy
