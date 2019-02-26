## Getting started - Android phone

### Dinghy setup

Assuming [rustup](http://rustup.rs) is already installed...

```
cargo install cargo-dinghy

# If it's already installed, add '--force'
cargo install cargo-dinghy --force
```

### ADB

`adb` must be in your $PATH and your phone must have debugging enabled.
enabled. See [adb doc](https://developer.android.com/studio/command-line/adb.html) .
`adb devices -l` must show your phone like that when you connect it.

```
% adb devices -l
List of devices attached
3100b123456789       device usb:341966848X product:a3xeltexx model:SM_A310F device:a3xelte
```

Now dinghy should also "see" your phone:

```
% cargo dinghy all-devices
List of available devices for all platforms:
Host { }
Android { "id": "3100b123456789", "supported_targets": ["armv7-linux-androideabi", "arm-linux-androideabi"] }
```

### Android NDK

Starting NDK version r19, it is possible to use the NDK directly without building a standalone Android toolchain. It is the new recommended way to build Android binaries using Dinghy.

First, download the [Android NDK](https://developer.android.com/ndk/downloads) or install the Android SDK.

If you downloaded the NDK, make sure to set the environment variable `ANDROID_NDK_HOME` to point to the extracted NDK folder. If you installed the SDK, Dinghy should detect it automatically.

If you did everything correctly, Dinghy should be able to recognize a large quantities of platforms. The following is an example of what you should see :

```
% cargo dinghy all-platforms
* auto-android-aarch64 (aarch64-linux-android)
* auto-android-aarch64-api21 (aarch64-linux-android)
[...]
* auto-android-aarch64-api28 (aarch64-linux-android)
* auto-android-aarch64-latest (aarch64-linux-android)
* auto-android-aarch64-min (aarch64-linux-android)
* auto-android-armv7 (armv7-linux-androideabi)
* auto-android-armv7-api16 (armv7-linux-androideabi)
[...]
* auto-android-armv7-api28 (armv7-linux-androideabi)
* auto-android-armv7-latest (armv7-linux-androideabi)
* auto-android-armv7-min (armv7-linux-androideabi)
* auto-android-i686 (i686-linux-android)
* auto-android-i686-api16 (i686-linux-android)
[...]
* auto-android-i686-api28 (i686-linux-android)
* auto-android-i686-latest (i686-linux-android)
* auto-android-i686-min (i686-linux-android)
* auto-android-x86_64 (x86_64-linux-android)
* auto-android-x86_64-api21 (x86_64-linux-android)
[...]
* auto-android-x86_64-api28 (x86_64-linux-android)
* auto-android-x86_64-latest (x86_64-linux-android)
* auto-android-x86_64-min (x86_64-linux-android)
* host
```

As you can notice, there is a huge amount of new platforms that have appeared as `auto-android-[ARCHITECTURE]`. You can use those new platforms to build your binaries using the architecture you want but also the Android API level you want to use. You can explicitly build by providing the API level you want or you can ask either the earliest API level available or the latest API level using the aliases `min` and `latest`. You can also use the default API level which will default for the API level 21, which is the first common API level with 64 bits based architecture and 32 bits based architecture.

If you get all the platforms, your NDK is set up. To finish your setup, you should [install the appropriate Rust target](#rust-target).

### Android standalone toolchain

Before Android NDK version r19, Dinghy couldn't use the NDK directly, so you had to set up a standalone toolchain for your phone architecture: dinghy gave you the possible ones, (`arm` or `armv7` here). It is still possible to use that procedure, but the recommended procedure is now to use the Android NDK directly. Here are the steps to achieve:

* [download and install Android NDK](https://developer.android.com/ndk/downloads/index.html) (please note that only r18 and previous are working to generate a standalone toolchain)
* [build a stand-alone Android toolchain](https://developer.android.com/ndk/guides/standalone_toolchain.html#creating_the_toolchain) matching your phone architecture

Note that `arm64` and `aarch64` are two names to the same architecture.

After you generated your toolchain, you'll need to declare the platform in `~/.dinghy.toml`. It links everything together: the rustc target and your android toolchain.

```
[platforms.android-armv7]
rustc_triple="armv7-linux-androideabi"
toolchain="<path to your armv7-linux-androideabi standalone toolchain>"
```

### Rust target

After you set up the NDK or the standalone toolchain, you may need to ask rustup to install the relevant target.

```
rustup target install armv7-linux-androideabi
```

ARMv7 is the most likely architecture for Android devices. However, maybe yours is one of these (two last are very unlikely):

```
rustup target install arm-linux-androideabi
rustup target install aarch64-linux-android
rustup target install i686-linux-android
rustup target install x86_64-linux-android
```

### Try it

Let's try it with the Dinghy demo project. The project tests with "pass" in the name is supposed to pass, the one with fail should break.

```
% git clone https://github.com/snipsco/dinghy
% cd dinghy/test-ws
[...]
# these ones should pass
% cargo dinghy -d android test pass
[...]
# this one shall not pass
% cargo dinghy -d android test fail
[...]
```

That's it! Enjoy!
