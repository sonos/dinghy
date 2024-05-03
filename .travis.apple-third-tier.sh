#!/bin/bash
set -e
set -x

SIM_TARGET=$1

title() {
    set +x
    echo -e '\n\033[1;33m '$@' \033[0m\n'
    set -x
}
if [ -z "$CARGO_DINGHY" ]
then
    title "••• build cargo-dinghy •••"
    cargo build -p cargo-dinghy
    CARGO_DINGHY="`pwd`/target/debug/cargo-dinghy -vv"
fi
echo RUST_VERSION: ${RUST_VERSION:=1.70.0}

rustup toolchain add $RUST_VERSION
export RUSTUP_TOOLCHAIN=$RUST_VERSION
rustup toolchain add nightly --component rust-src;

tests_sequence_unstable_target() {
    # There's something odd with using the .cargo/config runner attribute and
    # workspaces when the runner uses `cargo run --manifest-path ../Cargo.toml
    # --bin cargo-dinghy ...`
    title "testing from project directory for rust target $1 on device $2"
    title "testing from workspace directory"
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY   -d $1 -p $2 +nightly test -Zbuild-std pass \
        && ! $CARGO_DINGHY -d $1 -p $2 +nightly test -Zbuild-std fails \
        && ! $CARGO_DINGHY -d $1 -p $2 +nightly test -Zbuild-std \
    )

    title "testing from project directory"
    ( \
        cd test-ws/test-app \
        && cargo clean \
        && $CARGO_DINGHY   -d $1 -p $2 +nightly test -Zbuild-std pass \
        && ! $CARGO_DINGHY -d $1 -p $2 +nightly test -Zbuild-std fails \
        && ! $CARGO_DINGHY -d $1 -p $2 +nightly test -Zbuild-std \
    )

    title "test from workspace directory with project filter"
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY   -d $1 -p $2 +nightly test -p test-app -Zbuild-std pass \
        && ! $CARGO_DINGHY -d $1 -p $2 +nightly test -p test-app -Zbuild-std fails \
        && ! $CARGO_DINGHY -d $1 -p $2 +nightly test -p test-app -Zbuild-std \
    )
}

if [ $SIM_TARGET = tvOS ]
then
     title "••••• Darwin: tvos simulator tests •••••"
     title "boot a simulator"

     # *-apple-{tvos,watchos}[-sim] require `-Zbuild-std`
     TVOS_RUNTIME_ID=$(xcrun simctl list runtimes | grep tvOS | cut -d ' ' -f 7 | tail -1)
     export TV_SIM_ID=$(xcrun simctl create My-4ktv com.apple.CoreSimulator.SimDeviceType.Apple-TV-4K-3rd-generation-4K $TVOS_RUNTIME_ID)

     xcrun simctl boot $TV_SIM_ID
     if [ "$(uname -m)" = "arm64" ]; then
         tests_sequence_unstable_target ${TV_SIM_ID} auto-tvos-aarch64-sim
     else
         # The x86 tvOS simulator tripple does not end in -sim.
         tests_sequence_unstable_target ${TV_SIM_ID} auto-tvos-x86_64
     fi
     xcrun simctl delete $TV_SIM_ID
fi

if [ $SIM_TARGET = watchOS ]
then
     title "••••• Darwin: watchvos simulator tests •••••"
     title "boot a simulator"
     WATCHOS_RUNTIME_ID=$(xcrun simctl list runtimes | grep watchOS | cut -d ' ' -f 7 | tail -1)
     export WATCHOS_SIM_ID=$(xcrun simctl create My-apple-watch com.apple.CoreSimulator.SimDeviceType.Apple-Watch-SE-44mm-2nd-generation $WATCHOS_RUNTIME_ID)

     xcrun simctl boot $WATCHOS_SIM_ID
     if [ "$(uname -m)" = "arm64" ]; then
         tests_sequence_unstable_target ${WATCHOS_SIM_ID} auto-watchos-aarch64-sim
     else
         tests_sequence_unstable_target ${WATCHOS_SIM_ID} auto-watchos-x86_64-sim
     fi
     xcrun simctl delete $WATCHOS_SIM_ID

fi

# This depends on https://github.com/sonos/dinghy/pull/223
if [ $SIM_TARGET = visionOS ]
then
     if [ "$(uname -m)" = "arm64" ]; then
         title "••••• Darwin: visionOS simulator tests •••••"
         title "boot a simulator"
         xcrun simctl list devicetypes
         VISIONOS_DEVICE_TYPE=$(xcrun simctl list devicetypes vision -j | jq -r '.devicetypes[0].identifier')
         VISIONOS_RUNTIME_ID=$(xcrun simctl list runtimes | grep visionOS | cut -d ' ' -f 7 | tail -1)
         export VISIONOS_SIM_ID=$(xcrun simctl create My-apple-vision-pro $VISIONOS_DEVICE_TYPE $VISIONOS_RUNTIME_ID)

         xcrun simctl boot $VISIONOS_SIM_ID
         tests_sequence_unstable_target ${VISIONOS_SIM_ID} auto-visionos-aarch64-sim
         xcrun simctl delete $VISIONOS_SIM_ID
     fi
fi
rustup default stable
