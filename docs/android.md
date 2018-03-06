## Getting started - Android phone

### Dinghy setup

Assuming [rustup](http://rustup.rs) is already installed...

```
cargo install dinghy

# If it's already installed, add '--force'
cargo install dinghy --force
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

### Android standalone toolchain

Dinghy cannot use the NDK directly, so you will need a standalone toolchain for your phone architecture: dinghy gave you the possible ones, (`arm` or `armv7` here).

* [download and install Android NDK](https://developer.android.com/ndk/downloads/index.html)
* [build a stand-alone Android toolchain](https://developer.android.com/ndk/guides/standalone_toolchain.html#creating_the_toolchain) mathing your phone architecture

Note that `arm64` and `aarch64` are two names to the same architecture.

### Rust target

Next, you may need to ask rustup to install the relevant target.

```
rustup target install armv7-linux-androideabi
```

Or maybe yours is one of these (two last are very unlikely):

```
rustup target install arm-linux-androideabi
rustup target install aarch64-linux-android
rustup target install i686-linux-android
rustup target install x86_64-linux-android
```

### Dinghy configuration

Finally declare the platform in `~/.dinghy.toml`. It links everything together: the rustc target, and your android toolchain.

```
[platforms.android-armv7]
rustc_triple="armv7-linux-androideabi"
toolchain="<path to your armv7-linux-androideabi standalone toolchain>"
```

### Try it

Let's try it with dinghy demo project. The project tests with "pass" in the
name are supposed to pass, the one with fail should break.

```
% git clone https://github.com/snipsco/dinghy
[...]
# these ones should pass
% cargo dinghy -d android test pass
[...]
# this one shall not pass
% cargo dinghy -d android test fail
[...]
```

That's it! Enjoy!
