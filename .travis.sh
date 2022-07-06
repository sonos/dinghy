#!/bin/sh
set -e
set -x
export CARGO_DINGHY="cargo dinghy -vvv"
export RUST_BACKTRACE=1
echo RUST_VERSION: ${RUST_VERSION:=1.61.0}

rustup toolchain add $RUST_VERSION
export RUSTUP_TOOLCHAIN=$RUST_VERSION

if [ `uname` = Darwin ]
then
    export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
    export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib
fi

cargo build
cargo test

export PATH="`pwd`/target/debug/:$PATH"

# Test original cargo build
( \
    cd test-ws/test-app \
    && export NOT_BUILT_WITH_DINGHY=1 \
    && cargo test pass \
    && ! NOT_BUILT_WITH_DINGHY=1 cargo test fails \
)

# Test on the ios-simulator on macos and on an android emulator otherwise
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
        && ! $CARGO_DINGHY -d $SIM_ID test -p test-app \
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
        && ! $CARGO_DINGHY -d $SIM_ID test \
    )
    echo "##"
    echo "## latest failure was expected ##"
    echo "##"
else
   echo "##"
   echo "## ANDROID DEVICES (QEMU)"
   echo "##"

  rustup target add armv7-linux-androideabi

   $ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager --install "system-images;android-24;default;armeabi-v7a" "ndk;22.1.7171670"
   echo no | $ANDROID_SDK_ROOT/cmdline-tools/latest/bin/avdmanager create avd -n testdinghy -k "system-images;android-24;default;armeabi-v7a"
   $ANDROID_SDK_ROOT/emulator/emulator @testdinghy -no-audio -no-boot-anim -no-window -accel on -gpu off &
   timeout 180 $ANDROID_SDK_ROOT/platform-tools/adb wait-for-device
 
   export ANDROID_NDK_HOME=/usr/local/lib/android/sdk/ndk/22.1.7171670
 
   # Test cargo from workspace dir
   ( \
       cd test-ws \
       && cargo clean \
       && $CARGO_DINGHY -d android test pass \
       && ! $CARGO_DINGHY -d android test fails \
       && ! $CARGO_DINGHY -d android test \
   )
   echo "##"
   echo "## latest failure was expected ##"
   echo "##"
 
   # Test in project subdir
   ( \
       cd test-ws/test-app \
       && cargo clean \
       && $CARGO_DINGHY -d android test pass \
       && ! $CARGO_DINGHY -d android test fails \
       && ! $CARGO_DINGHY -d android test \
   )
   echo "##"
   echo "## latest failure was expected ##"
   echo "##"
 
   # Test from workspace root with project filter
   ( \
       cd test-ws \
       && cargo clean \
       && $CARGO_DINGHY -d android test -p test-app pass \
       && ! $CARGO_DINGHY -d android test -p test-app fails \
       && ! $CARGO_DINGHY -d android test -p test-app \
   )
   echo "##"
   echo "## latest failure was expected ##"
   echo "##"

   echo "##"
   echo "## SCRIPT DEVICES (QEMU)"
   echo "##"

  rustup target add aarch64-unknown-linux-musl
  sudo apt-get -y install --no-install-recommends qemu-system-arm qemu-user
  echo "[script_devices.qemu]\nplatform='qemu'\npath='/tmp/qemu'" >> .dinghy.toml
  echo "#!/bin/sh\nexe=\$1\nshift\n/usr/bin/qemu -L /usr/aarch64-linux-gnu/ \$exe --test-threads 1 \"\$@\"" > /tmp/qemu

   # Test cargo from workspace dir
   ( \
       cd test-ws \
       && cargo clean \
       && $CARGO_DINGHY -d aarch64 test pass \
       && ! $CARGO_DINGHY -d aarch64 test fails \
       && ! $CARGO_DINGHY -d aarch64 test \
   )
   echo "##"
   echo "## latest failure was expected ##"
   echo "##"
 
   # Test in project subdir
   ( \
       cd test-ws/test-app \
       && cargo clean \
       && $CARGO_DINGHY -d aarch64 test pass \
       && ! $CARGO_DINGHY -d aarch64 test fails \
       && ! $CARGO_DINGHY -d aarch64 test \
   )
   echo "##"
   echo "## latest failure was expected ##"
   echo "##"
 
   # Test from workspace root with project filter
   ( \
       cd test-ws \
       && cargo clean \
       && $CARGO_DINGHY -d aarch64 test -p test-app pass \
       && ! $CARGO_DINGHY -d aarch64 test -p test-app fails \
       && ! $CARGO_DINGHY -d aarch64 test -p test-app \
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
