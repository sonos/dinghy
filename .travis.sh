#!/bin/bash
set -e
set -x

title() {
    set +x
    echo -e '\n\033[1;33m '$@' \033[0m\n'
    set -x
}

if [ ! cargo --version ]
then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    PATH=$PATH:$HOME/.cargo/bin
fi


if [ -z "$CARGO_DINGHY" ]
then
    title "••• build cargo-dinghy •••"
    cargo build -p cargo-dinghy
    CARGO_DINGHY="`pwd`/target/debug/cargo-dinghy -vv"
fi
echo RUST_VERSION: ${RUST_VERSION:=1.85.0}

rustup toolchain add $RUST_VERSION
export RUSTUP_TOOLCHAIN=$RUST_VERSION

title "••• test original cargo build •••"
cargo build
cargo test

( \
    cd test-ws/test-app \
    && export NOT_BUILT_WITH_DINGHY=1 \
    && cargo test pass \
    && ! NOT_BUILT_WITH_DINGHY=1 cargo test fails \
)

tests_sequence() {
    title "testing from workspace directory"
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY -d $1 test pass \
        && $CARGO_DINGHY -d $1 test --doc \
        && ! $CARGO_DINGHY -d $1 test fails \
        && ! $CARGO_DINGHY -d $1 test \
    )

    title "testing from project directory"
    ( \
        cd test-ws/test-app \
        && cargo clean \
        && $CARGO_DINGHY -d $1 test pass \
        && $CARGO_DINGHY -d $1 test --doc \
        && ! $CARGO_DINGHY -d $1 test fails \
        && ! $CARGO_DINGHY -d $1 test \
    )

    title "test from workspace directory with project filter"
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY -d $1 test -p test-app pass \
        && $CARGO_DINGHY -d $1 test -p test-app --doc \
        && ! $CARGO_DINGHY -d $1 test -p test-app fails \
        && ! $CARGO_DINGHY -d $1 test -p test-app \
    )
}

tests_sequence_aarch64_ios_sim() {
    title "testing from workspace directory"
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY   -d $1 -p auto-ios-aarch64-sim test pass \
        && $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test --doc \
        && ! $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test fails \
        && ! $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test \
    )

    title "testing from project directory"
    ( \
        cd test-ws/test-app \
        && cargo clean \
        && $CARGO_DINGHY   -d $1 -p auto-ios-aarch64-sim test pass \
        && $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test --doc \
        && ! $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test fails \
        && ! $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test \
    )

    title "test from workspace directory with project filter"
    ( \
        cd test-ws \
        && cargo clean \
        && $CARGO_DINGHY   -d $1 -p auto-ios-aarch64-sim test -p test-app pass \
        && $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test -p test-app --doc \
        && ! $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test -p test-app fails \
        && ! $CARGO_DINGHY -d $1 -p auto-ios-aarch64-sim test -p test-app \
    )
}


if [ `uname` = Darwin ]
then
     title "••••• Darwin: ios simulator tests •••••"
     title "boot a simulator"
     RUNTIME_ID=$(xcrun simctl list runtimes | grep iOS | cut -d ' ' -f 7 | tail -1)

     # Installed simulators on github runners differ depending on the version
     # of macos. When the simulator device type ID needs to be updated, select
     # a new one:
     # https://github.com/actions/runner-images/blob/main/images/macos/macos-12-Readme.md#installed-simulators
     # https://github.com/actions/runner-images/blob/main/images/macos/macos-13-Readme.md#installed-simulators
     # https://github.com/actions/runner-images/blob/main/images/macos/macos-14-arm64-Readme.md#installed-simulators
     export SIM_ID=$(xcrun simctl create My-iphone-se com.apple.CoreSimulator.SimDeviceType.iPhone-SE-3rd-generation $RUNTIME_ID)
     xcrun simctl boot $SIM_ID

     # The x86_64-apple-ios target seems to not work on an ARM64 host,
     # and the aarch64-apple-ios-sim target doesn't work on an x86-64 host.
     if [ "$(uname -m)" = "arm64" ]; then
         rustup target add aarch64-apple-ios-sim;
         tests_sequence_aarch64_ios_sim $SIM_ID
     else
         rustup target add x86_64-apple-ios;
         tests_sequence $SIM_ID
     fi

     xcrun simctl delete $SIM_ID

    if ios-deploy -c -t 1 > /tmp/ios_devices
    then
        device=$(grep "Found" /tmp/ios_devices | head -1 | cut -d " " -f 3)
        title "••••• Darwin: ios-deploy detected a device •••••"
        rustup target add aarch64-apple-ios
        tests_sequence $device
    fi
else
    echo $ANDROID_SDK_ROOT
    if [ -n "$ANDROID_SDK_ROOT" ]
    then
        title "••••• Linux: android tests •••••"
        title "setup simulator"
        rustup target add armv7-linux-androideabi

         ## BEGIN FIX-EMULATOR
         # Use emulator version 32.1.15 as latest version (36.3.10.0 as of writing)
	 # doesn't support arm images anymore

         EMULATOR="$(pwd)/target/emulator/emulator"
         [ -e $EMULATOR ] ||  ( \
           cd target/ \
           && wget -q https://redirector.gvt1.com/edgedl/android/repository/emulator-linux_x64-10696886.zip \
           && unzip emulator-linux_x64-10696886.zip \
           && rm emulator-linux_x64-10696886.zip
         )

         # to revert when the bundled emulator doesn't crash anymore use the following line
	 # EMULATOR="$ANDROID_SDK_ROOT/emulator/emulator"

        # END FIX-EMULATOR

        yes | $ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager --licenses > /dev/null
        $ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager --install "system-images;android-24;default;armeabi-v7a" "ndk;27.3.13750724" "emulator" "platform-tools" # "cmdline-tools;latest"
        echo no | $ANDROID_SDK_ROOT/cmdline-tools/latest/bin/avdmanager create avd -c 1000M --force -n testdinghy -k "system-images;android-24;default;armeabi-v7a"
        ANDROID_AVD_HOME=$HOME/.config/.android/avd $EMULATOR @testdinghy -partition-size 1024 -no-audio -no-boot-anim -no-window -accel on -gpu off &
        timeout 180 $ANDROID_SDK_ROOT/platform-tools/adb wait-for-device

        export ANDROID_NDK_HOME=$ANDROID_SDK_ROOT/ndk/27.3.13750724

        tests_sequence android
    fi

    title "••••• Linux: script tests (with qemu) •••••"
    title "setup qemu"

    rustup target add aarch64-unknown-linux-musl
    sudo apt-get update
    sudo apt-get -y install --no-install-recommends qemu-system-arm qemu-user binutils-aarch64-linux-gnu gcc-aarch64-linux-gnu
    echo -e "[platforms.qemu]\nrustc_triple='aarch64-unknown-linux-musl'\ndeb_multiarch='aarch64-linux-gnu'" > .dinghy.toml
    echo -e "[script_devices.qemu]\nplatform='qemu'\npath='/tmp/qemu'" >> .dinghy.toml
    echo -e "#!/bin/sh\nexe=\$1\nshift\n/usr/bin/qemu-aarch64 -L /usr/aarch64-linux-gnu/ \$exe --test-threads 1 \"\$@\"" > /tmp/qemu
    chmod +x /tmp/qemu

    tests_sequence qemu
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
