#!/bin/sh
set -e

export PATH=$PATH:$HOME/.cargo/bin

if [ `uname` = Darwin ]
then
    (xcrun simctl list devices | grep Booted) || xcrun simctl boot "iPhone 6"
    rustup target install aarch64-apple-ios
fi

cargo build --verbose
cargo test --verbose

cd test-app
cargo test -- works
! cargo test -- fails
echo "## last failure was expected ##"
if [ `uname` = Darwin ]
then
    ../target/debug/cargo-dinghy dinghy test -- works
    ! ../target/debug/cargo-dinghy dinghy test -- fails
    echo "## last failure was expected ##"
fi
