## Getting started - OpenHarmony(HarmonyOS Next) phone

### Dinghy setup

Assuming [rustup](http://rustup.rs) is already installed...

```
cargo install cargo-dinghy

# If it's already installed, add '--force'
cargo install cargo-dinghy --force
```

### HarmonyOS SDK

First, download the [HarmonyOS SDK](https://developer.huawei.com/consumer/cn/download/). Make sure to set the environment variable `OHOS_SDK_HOME` to point to the extracted SDK folder, you can check if it's set correctly by running: `file $OHOS_SDK_HOME/default/openharmony/toolchains/hdc`.

If you did everything correctly, Dinghy should be able to recognize a large quantities of platforms. The following is an example of what you should see :

```
% cargo dinghy all-platforms
[...]
* auto-ohos-arm64-v8a aarch64-unknown-linux-ohos
* auto-ohos-armeabi-v7a armv7-unknown-linux-ohos
* auto-ohos-x86_64 x86_64-unknown-linux-ohos
[...]
```

If you get all the platforms, your SDK is set up. To finish your setup, you should [install the appropriate Rust target](#rust-target).

### See your OpenHarmony devices

`$OHOS_SDK_HOME` should be set to the path of HarmonyOS SDK, and your phone must have debugging enabled. If you don't have a phone, you can run an emulator(Download and run [DevEco Studio](https://developer.huawei.com/consumer/cn/download/), then create a new emulator by navigating to `Tools -> Device Manager -> New Emulator`).
See [hdc doc](https://gitee.com/openharmony/docs/blob/master/en/device-dev/subsystems/subsys-toolchain-hdc-guide.md).
`hdc list targets` must show your phone/emulator like this:

```
% $OHOS_SDK_HOME/default/openharmony/toolchains/hdc list targets
127.0.0.1:5555
```

Now dinghy should also "see" your phone:

```
% cargo dinghy all-devices
List of available devices for all platforms:
OpenHarmony/127.0.0.1:5555: [OhosPlatform { regular_platform: auto-ohos-arm64-v8a, arch: Aarch64, .. } ]
```

### Rust target

After you set up the SDK, you may need to ask rustup to install the relevant target:

```
rustup target install aarch64-unknown-linux-ohos
```

AArch64 is the most likely architecture for OpenHarmony devices. However, maybe yours is one of these (two last are very unlikely):

```
rustup target install armv7-unknown-linux-ohos
rustup target install x86_64-unknown-linux-ohos
```

### Try it

Let's try it with the Dinghy demo project. The project tests with "pass" in the name is supposed to pass, the one with "fail" should break.

(Worth noting that `-Zbuild-std` is required for old Rust toolchains)

```
% git clone https://github.com/sonos/dinghy
% cd dinghy/test-ws
[...]
# these ones should pass
% cargo dinghy -d ohos test pass -Zbuild-std
[...]
# this one shall not pass
% cargo dinghy -d ohos test fail -Zbuild-std
[...]
```

That's it! Enjoy!
