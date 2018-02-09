# Dinghy

[![Build Status](https://travis-ci.org/snipsco/dinghy.svg?branch=master)](https://travis-ci.org/snipsco/dinghy)

**Table of Contents**  *generated with [DocToc](http://doctoc.herokuapp.com/)*

- [Dinghy](#)
	- [What ?](#)
	- [Getting started](#)
	- [Android setup](#)
	- [iOS setup](#)
		- [Creating a signing id](#)
		- [Creating a certificate](#)
		- [Trust the certificate on your phone](#)
		- [Debugging tips](#)
			- [security find-identity -p codesigning](#)
			- [security find-certificate -a -c "name" -p | openssl x509 -text](#)
			- [Look for provisioning certificates](#)
	- [Ssh setup](#)
	- [Sending sources files to the devices](#)
	- [Sending more files to the devices](#)
	- [Advanced configuration](#)
		- [Package filtering](#)
		- [Environment variables](#)
		- [Overlays](#)
			- [Overlay configuration](#)
			- [Overlay directory](#)
			- [Overlay pkg-config](#)
	- [Build script helpers](#)
	- [Enjoy](#)
- [License](#)
	- [Contribution](#)

## What ?

Send `cargo test` or `cargo bench` to your phone. Painlessly (or at least with a bit less pain).

It is not a way to build portable apps, merely a way to run simple piece of
code and grab their output. On iOS, it also allows to run lldb and debug
interactively the executable.

The purpose here is to make it easier for teams to unit-test and bench their
libraries on more platforms. We want to get Rust everywhere right ?

Dinghy also supports compilation for, and execution on remote devices accessible
through ssh.

## Getting started

Depending on what is your target (iOS or Android) and your workstation, setting
up Dinghy can be more or less easy. See the setup sections for that.

First, let's install dinghy...

```
cargo install dinghy

# If it's already installed, add '--force'
cargo install dinghy --force
```

If you've never used cross compiling before, you'll probaby want to pick 
a toolchain or two...

```
# check available cross-compilation targets
rustup target list

# a few ios devices
rustup target install aarch64-apple-ios
rustup target install armv7-apple-ios
rustup target install armv7s-apple-ios
# a suitable ios simulator
rustup target install i386-apple-ios
rustup target install x86_64-apple-ios

# some android devices
rustup target install arm-linux-androideabi
rustup target install armv7-linux-androideabi
rustup target install aarch64-linux-android
rustup target install i686-linux-android
rustup target install x86_64-linux-android

# a Raspberry Pi
rustup target install armv7-unknown-linux-gnueabihf
```

We will need some configuration bits here. So let's create a `.dinghy/dinghy.toml` in
your home directory to specify available platforms with their rustc triple:
```toml
# For ios
[platforms.ios-armv7]
rustc_triple="armv7-apple-ios"

[platforms.ios-armv7s]
rustc_triple="armv7s-apple-ios"

[platforms.ios-arm64]
rustc_triple="aarch64-apple-ios"

[platforms.ios-i386]
rustc_triple="i386-apple-ios"

[platforms.ios-x86_64]
rustc_triple="x86_64-apple-ios"


# For android
[platforms.android-arm]
rustc_triple="arm-linux-androideabi"
toolchain="<path to your arm-linux-androideabi standalone toolchain>"

[platforms.android-armv7]
rustc_triple="armv7-linux-androideabi"
toolchain="<path to your armv7-linux-androideabi standalone toolchain>"

[platforms.android-arm64]
rustc_triple="aarch64-linux-android"
toolchain="<path to your aarch64-linux-android standalone toolchain>"

[platforms.android-x86]
rustc_triple="i686-linux-android"
toolchain="<path to your i686-linux-android standalone toolchain>"

[platforms.android-x86-64]
rustc_triple="x86_64-linux-android"
toolchain="<path to your x86_64-linux-android standalone toolchain>"


# A raspbian platform
[platforms.raspbian]
rustc_triple="arm-unknown-linux-gnueabihf"
toolchain="<path to your raspbian toolchain>"
```

And... stop here. Unfortunately, there is a bit of stuff to do by hand before
the fun stuff come.

## Android setup

You'll need to make a stand-alone toolchain as explained in the [official Android documentation](https://developer.android.com/ndk/guides/standalone_toolchain.html) (Dinghy cannot use the NDK directly), and `adb` somewhere in your path.
`fb-adb` will be used if its available on the path. It allows dinghy to forward correctly binaries/benches/tests exit codes from the conneced device to the host platform.
Also, your phone must have developer options enabled.

You will also probably have to set up a toolchain: rustc needs a working linker to be able to generate executable. On my case, to use android devices and emulators, I added the following lines in `~/.cargo/config`:

```
[target.arm-linux-androideabi]
linker = "/Users/kali/.dinghy/toolchain/android-arm-latest_linux/bin/arm-linux-androideabi-clang"

[target.aarch64-linux-android]
linker = "/Users/kali/.dinghy/toolchain/android-arm64-latest_linux/bin/aarch64-linux-android-clang"

[target.i686-linux-android]
linker = "/Users/kali/.dinghy/toolchain/android-x86-latest_linux/bin/i686-linux-android-clang"

[target.x86_64-linux-android]
linker = "/Users/kali/.dinghy/toolchain/android-x86_64-latest_linux/bin/x86_64-linux-android-clang"
```

## iOS setup

If you are on a mac, just start a simulator instance. If you're a mac/iOS developper, it's nice to try that before diving
into the more convoluted iOS device setup that follows.

On iOS, things are made complicated by the fact that there is no way to run
a naked executable: you need to make it an app, and sign it before sending
it to your phone. Setting up app signature requires some manipulations that
we could not find a way to dispense with through XCode. The good news is, this
is a constant time thing: *you'll have do it once in a while, but it will
cover all your Rust projects*.

Ho, and we don't need a paying account.

### Creating a signing id

You may skip most of this section if you're already setup to ship code to
your phone. 

* You'll need an Apple ID. Chances are you already have one, but you can
    an account there: https://appleid.apple.com/account .
* Go to XCode > Preferences > Accounts, and make sure your Apple ID is listed,
    or add it (`+` bottom well-hidden bottom left).
* View Details (bottom right this time)
* Click the "Create" button in front of "iOS Development" in the top half of
    this windows.
* We're done here.

### Creating a certificate

* Plug-in your iPhone (or the device you want to run test on).
* Fire up XCode, and /create a new XCode project/.
* Pick /iOS/, /Single view application/.
* Options box:
    * Make the bundle identifier `some.unique.domainame.Dinghy`.
    * So Product Name is `Dinghy`, and `Organization identifier` is something
        that will be unique and looks like a domain name in reverse.
    * Pick your team.
    * Leave the rest alone
* Save it somewhere.
* You should get the dreadful projects settings. The only thing that is
    relevant is the "Signing" bit.
    * If you see your Team there, and nothing red or yellows shows up, then
        you're OK. In some cases, you have to push the "repair" button to
        actually obtain a certificate from Apple.

If you're using a Apple ID free account, you will need to come back once a
week to refresh the certificate (aka "repair"). Paying account generate
longer living certificates, so you need this less often.

### Trust the certificate on your phone

On top of project window, make the target "Dinghy>Your Device", and run the
project (play button). Xcode should ask you to go to your phone settings
and Trust the certificate. It's in the Preferences > General > 
[your dev account name]. 

At this point, we're ready to roll.

### Debugging tips

If you got lost somewhere, here are a few hints to help you make sense of 
what is happening. This is more or less what Dinghy use when fishing for
your signing identity.

#### `security find-identity -p codesigning`

Shows you the codesigning identities available where you are. You should see 
one or more identities line, made of a long capitalize hex identifier, followed
by a "name". The name is very structured: For iOS development , its starts
with the string "iPhone Developer: ", followed by an email (for an Apple Id
account) or the name of your team. Then comes the developer identifier in
parenthesis.

#### `security find-certificate -a -c "name" -p | openssl x509 -text`

"name" is the identity name (the string between double quotes) from the command
before.

Shows you a certificate that makes the developer a part of a Team.
The certificate is signed and issued by Apple (Issuer:), but the interesting
part is the Subject line: CN= is the same developer name string, and OU= is
the actual Team identifier.

#### Look for provisioning certificates

Last but not least, we need one more certificate that proves that you and
your team has the right to deploy an App (identified by the bundle id we have
chosen while creating the project) on one (or more) devices.

These certificates are in `Library/MobileDevice/Provisioning\ Profiles`.

To read them, you'll need to do 

```
security cms -D -i  ~/Library/MobileDevice/Provisioning\ Profiles\....mobileprovision
```

The more interesting keys here are `TeamIdentifier`, `ProvisionedDevices` which
are relatively explicit, and the `Entitlements` dictionary. These entitlements
specify what the certificate is valid for, that is, signing an app identified
by a name.

Dinghy will actually scan this directory to find one that it can use (this is
where the app name being "Dinghy" plays a role).

Phew.

## Ssh setup

Ssh setup is useful if you want to target small-ish devices that can host an
operating system but are too slow to iterate over Rust compilation/test cycles
comfortably. Think... something like a Raspberry Pi or a NAS, or an old x86 box
in your garage.

We will need some configuration bits here. So let's append the specification of your ssh device to `.dinghy/dinghy.toml` in
your home directory:
```toml
[ssh_devices]
"raspbian" = { hostname = "10.1.2.3", username="pi", platform="raspbian" }
```

You must make sure that the ssh connection works without asking for a password
for the user/host combination. 

You will also probably have to set up a toolchain: rustc needs a working linker
to be able to generate executable. On my case, for using on a RaspberryPi, I
added these two lines in `~/.cargo/config`:

```toml
[target.armv7-unknown-linux-gnueabihf]
linker = "/Users/kali/.dinghy/toolchain/armv7-rpi2-linux-gnueabihf/bin/armv7-rpi2-linux-gnueabihf-gcc"
```

But once this is done, well, it works the same.

## Sending project files to the devices

Some tests are relying on the presence of files at relative paths to be able
to proceed. But we can not always control where we will be executing from (we
can not always do `cd someplace` before running the tests).

So, the tests are "bundled" in the following way:

* root dinghy test directory
    * test_executable
    * recursive copy of the not-ignorable files and directories from your projects
    * test_data is contains configurable data to be sent to the device
        * some_file
        * some_dir

Anything in .gitignore or .dinghyignore will not be bundled.

To open your test file easily, you can use the dinghy-test crate in your tests which contains a helper function to access your project directory:

```rust
#[cfg(test)]
extern crate dinghy_test;


#[cfg(test)]
mod tests {
    #[test]
    fn my_test() {
        let my_file_path = dinghy_test::test_project_path().join(("tests/data_1.txt");
        // ...
    }
}
```

## Sending more files to the devices

Now let's assume you have out-of-repository files to send over. You can do that
by adding it in `.dinghy.toml` (you'll probably want this one in the project
directory, or just above it if the data is shared among several cargo projects).

```toml
[test_data]
the_data = "../data-2017-02-05"
conf_file = "/etc/some/file"
```

The keys are the name under which to look for files below "test_data" in the
bundles, and the values are what to be copied (from your development workstation).

By default anything in `.gitignore` or `.dinghyignore` is not copied, however if
you need .gitignore'd files to be copied it can be excluded by adding
`copy_git_ignored = true`:

```toml
[test_data]
the_data = { source = "../data-2017-02-05", copy_git_ignored = true }
conf_file = "/etc/some/file"
```

Then you can use again the dinghy-test crate to access your specific test data directory:

```rust
#[cfg(test)]
extern crate dinghy_test;


#[cfg(test)]
mod tests {
    #[test]
    fn my_test() {
        let my_test_data_path = dinghy_test::test_file_path("the_data");
        let my_test_file_path = dinghy_test::test_file_path("conf_file");
        // ...
    }
}
```

## Advanced configuration

### Package filtering

Rust workspace builds all contained packages by default. However, when a worspace project builds multiple platforms, some packages might not be buildable on all targets. To help with that situation, Dinghy has a filter option which allows benching, building or testing only the packages compatible with the current target platform.

To make it works, modify the package toml to include some Dinghy metadata specifying the allowed rustc triples. For example, here we allow android only:
```toml
[package.metadata.dinghy]
allowed_rustc_triples = [ "armv7-linux-androideabi", "aarch64-linux-android", "i686-linux-android", "x86_64-linux-android" ]
```

Or specifying the rustc triples to ignore. For example, here we disallow ios:
```toml
[package.metadata.dinghy]
ignored_rustc_triples = ["aarch64-apple-ios", "armv7-apple-ios", "armv7s-apple-ios", "i386-apple-ios", "x86_64-apple-ios"]
```

### Environment variables

Dinghy allows defining environment variables per-platform. These variables will be set during the whole build process (including build.rs scripts targeting either the host or target platforms).

```toml
[platforms.my-platform]
env={ MY_ENV="my-value" }
```

Here is an example environment variable that helps running openssl build script for the host platform (when another dependency build script uses openssl to download some stuff) during an android cross compilation build:
```toml
[platforms.android-arm64]
env={ X86_64_UNKNOWN_LINUX_GNU_OPENSSL_DIR = "/usr" } 
```

It's possible to setup environment variables for a (non cross-compilation) build running for the host platform too:
```toml
[platforms.host]
env={ MY_ENV="my-value" }
```

### Overlays

A toolchain might not contains all the required dependencies for your project. To help with situation, Dinghy offers overlays.

#### Overlay configuration

By default, Dinghy will look for overlays in the `dinghy/overlay/<platform>` directory next to your configuration file.

Overlay directories can also be specified using the overlays section of your platform configuration:
```toml
[platforms.android-arm64]
overlays={ mydep={ path="/mypath" } } 
```

#### Overlay directory

An overlay is a directory which contains the required *.so*, *.h* and *.pc* files for a dependency. For example:
```
my-overlay
|- libmylib.so
|- bmylib.h
|- bmylib.pc
```

An overlay is like an additional sysroot. So you can also create directories and subdirectories. For example, we use a tensorflow overlay on our android projects with the following structure:
```
android-arm64
|- overlay
    |- tensorflow
        |- usr
            |- include
                |- pkgconfig
                    |- tensorflow.pc
            |- lib
                |- tensorflow.so
```

Dinghy looks for:
- *.pc* files either in the overlay root or in all sub-directories named `pkgconfig`.
- *.so* libraries in the overlay directory and all of its subdirectories.

#### Overlay pkg-config

Dinghy uses pkg-config to append dependencies during the compilation process (technically speaking using `PKG_CONFIG_LIBDIR`).

By default, if no pkgconfig *.pc* file is found, Dinghy will generate one before the build. In such a case, the overlay directory itself is appended as include and linking path in the pkgconfig files along all the `.so` files founds in its root. For example:
```
prefix=/

Name: mylib
Description: mylib
Requires:
Version: unspecified

Libs: -L${prefix} -lmylib
Cflags: -I${prefix}
```

Ideally, you should create a *.pc* file to make sure all compilation flags are set-up correctly. For example, our tensorflow overlay includes the following *.pc*:
```
prefix=/
exec_prefix=${prefix}
libdir=${exec_prefix}/usr/lib
includedir=${prefix}/usr/include

Name: Tensorflow
Description: Tensorflow for Android
Requires:
Version: 1.5

Cflags: -I${includedir}
Libs: -L${libdir} -ltensorflow -lstdc++ -landroid -lz
```

Overlays are usually outside the toolchain sysroot. As a consequence, Dinghy must overrides the `prefix` pkg-config variable to provide a correct overlay path relative to the toolchain sysroot, despite being outside of it.
Hence, when writing a *.pc* file, it's very *important* to:
- Define a `prefix` variable that Dinghy can override
- Consider that this `prefix` points to the root of the overlay directory

#### Overlay runtime

To make sure overlays are available at runtime, during benches, run or tests, Dinghy will copied all the `.so` files linked by the linker script during a build on the target device before running the appropriate executable.


## Build script helpers [WIP]

Dinghy also provides a dinghy-helper crate to help with the writing of build scripts that performs cross-compilation, and more specifically:

- CommandExt: a std::process::Command extension that can:
  - Setup pkg-config environment variables for a subprocess (`PKG_CONFIG_LIBDIR`, ... e.g. when running Automake `./configure`)
  - Setup toolchain environment variables for a subprocess (`TARGET_CC`, ...)
  - A few other useful methods (e.g. `configure_prefix()` to set the prefix to rust output dir when running Automake `./configure`)
- BindGenBuilderExt: a bindgen::Builder extension to help writing C to rust bindings that supports cross compilation properly (see `new_bindgen_with_cross_compilation_support()`).

*This is still a WIP. Beware of possible breaking changes*

## Enjoy

Now, you can go to your favourite Rust project. For a first try, I suggest you
consider something without too many exotic external dependencies.

Dinghy uses almost the same interface as cargo. So to build a rust project, simply do:
- For your host platform, `cargo dinghy build`  
- For your target platform, `cargo dinghy --platform <my-platform> build` or `cargo dinghy --device <my-device-id> build`  

The same goes for benches and tests. When performing benches and tests on a target platform, Dinghy looks for the first compatible device if none is specified explicitely. 

# License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
