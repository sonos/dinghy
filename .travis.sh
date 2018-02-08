#!/bin/sh
set -e
set -x
export CARGO_DINGHY="`pwd`/target/debug/cargo-dinghy"
export RUST_BACKTRACE=1
export TARGET_ARCH="x86_64-apple-ios"

curl https://sh.rustup.rs -sSf | sh -s -- -y

export PATH=$PATH:$HOME/.cargo/bin
RUST_VERSION=${RUST_VERSION:=stable}

rustup install $RUST_VERSION
rustup override set $RUST_VERSION

if [ `uname` = Darwin ]
then
    (xcrun simctl list devices | grep Booted) || xcrun simctl boot "iPhone 6"
    rustup target install $TARGET_ARCH
    pip install six
    export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
    export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib

    # Setup dinghy config
    echo "[platforms.ios]" >> .dinghy.toml
    echo "rustc_triple='$TARGET_ARCH'" >> .dinghy.toml
fi

cargo build --verbose
cargo test --verbose

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

if [ `uname` = Darwin ]
then
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY --platform 'ios' test pass \
        && ! $CARGO_DINGHY --platform 'ios' test fails \
    )
    echo "##"
    echo "## latest failure was expected"
    echo "##"
fi


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

if [ `uname` = Darwin ]
then
    ( \
        cd test-ws/test-app \
        && cargo clean \
        && $CARGO_DINGHY --platform 'ios' test pass \
        && ! $CARGO_DINGHY --platform 'ios' test fails \
    )
    echo "##"
    echo "## latest failure was expected"
    echo "##"
fi


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

if [ `uname` = Darwin ]
then
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY --platform 'ios' test -p test-app pass \
        && ! $CARGO_DINGHY --platform 'ios' test -p test-app fails \
    )
    echo "##"
    echo "## latest failure was expected ##"
    echo "##"
fi
