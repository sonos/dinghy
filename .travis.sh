#!/bin/sh
set -e
set -x
export CARGO_DINGHY="`pwd`/target/debug/cargo-dinghy"
export RUST_BACKTRACE=1
echo RUST_VERSION: ${RUST_VERSION:=stable}

rustup toolchain add $RUST_VERSION
export RUSTUP_TOOLCHAIN=$RUST_VERSION

if [ `uname` = Darwin ]
then
    export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
    export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib
fi

cargo build
cargo test

# Test original cargo build
( \
    cd test-ws/test-app \
    && export NOT_BUILT_WITH_DINGHY=1 \
    && cargo test pass \
    && ! NOT_BUILT_WITH_DINGHY=1 cargo test fails \
)

# Test cargo from workspace dir
( \
    cd test-ws \
    && cargo clean \
    && $CARGO_DINGHY test pass \
    && ! $CARGO_DINGHY test fails \
)
echo "##"
echo "## latest failure was expected ##"
echo "##"

# Test in project subdir
( \
    cd test-ws/test-app \
    && cargo clean \
    && $CARGO_DINGHY test pass \
    && ! $CARGO_DINGHY test fails \
)
echo "##"
echo "## latest failure was expected ##"
echo "##"

# Test from workspace root with project filter
( \
    cd test-ws \
    && cargo clean \
    && $CARGO_DINGHY test -p test-app pass \
    && ! $CARGO_DINGHY test -p test-app fails \
)
echo "##"
echo "## latest failure was expected ##"
echo "##"

# Test on the ios-simulator.
if [ `uname` = Darwin ]
then
    rustup target add x86_64-apple-ios;
    RUNTIME_ID=$(xcrun simctl list runtimes | grep iOS | cut -d ' ' -f 7 | tail -1)
    export SIM_ID=$(xcrun simctl create My-iphone7 com.apple.CoreSimulator.SimDeviceType.iPhone-7 $RUNTIME_ID)
    xcrun simctl boot $SIM_ID
    # Test from workspace root with project filter
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY -d $SIM_ID test -p test-app pass \
        && ! $CARGO_DINGHY -d $SIM_ID test -p test-app fails \
    )
    echo "##"
    echo "## latest failure was expected ##"
    echo "##"

    # Test in project subdir
    ( \
        cd test-ws/test-app \
        && cargo clean \
        && $CARGO_DINGHY -d $SIM_ID test pass \
        && ! $CARGO_DINGHY -d $SIM_ID test fails \
    )
    echo "##"
    echo "## latest failure was expected ##"
    echo "##"
fi

if [ -n "$DEPLOY" ]
then
    if [ `uname` = Linux ]
    then
        export OPENSSL_STATIC=yes
        export OPENSSL_INCLUDE_DIR=/usr/include
        export OPENSSL_LIB_DIR=$(dirname `find /usr -name libssl.a`)
        cargo clean
    fi
    cargo build --release -p cargo-dinghy
    mkdir -p cargo-dinghy-$DEPLOY
    cp target/release/cargo-dinghy cargo-dinghy-$DEPLOY
    tar vczf cargo-dinghy-$DEPLOY.tgz cargo-dinghy-$DEPLOY
fi
