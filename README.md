# Dinghy

![rustc >= 1.70.0](https://img.shields.io/badge/rustc-%3E%3D1.70.0-brightgreen)
![MIT/Apache 2](https://img.shields.io/crates/l/dinghy)
![Build and test](https://github.com/snipsco/dinghy/workflows/Build%20and%20test/badge.svg)

## What?

Dinghy is a `cargo` extension to bring cargo workflow to cross-compilation situations.

Dinghy is specifically useful with "small" processor-based devices, like
Android and iOS phones, or small single board computers like the Raspberry Pi.
Situations where native compilation is not possible, or not practical.

Initially tests and benches were the primary objective of Dinghy, but now
at Snips we use it to cross-compile our entire platform. This includes setting
up the stage for `cc` and `pkg-config` crates in one single place.

If you are a Rust library author, **you can run your tests and benches on
your smartphone in minutes.** And you should, at least once in a while.

## Demo

Let's try how BurntSushi's byteorder handles f32 on a few arm devices, two
smartphones, and a Raspberry Pi.

![Demo](docs/demo.gif)

Phew. It works.

## How?

Once dinghy knows about your toolchains and devices, you will be able to run 
tests and benches from a simple cargo command **in any cargo project**, most of
the time without altering them.

Just add `dinghy -d some_device` between `cargo` and its subcommand:

```
cargo dinghy -d my_android test
cargo dinghy -d my_raspberry bench
```

By default, without `-d`, Dinghy will make a native build, just like `cargo` would do.

## Getting started

Depending on your targets and your workstation, the ease of setting
up Dinghy can vary. 

* [Android](docs/android.md) is relatively easy, specifically if you already are
a mobile developer.
* [iOS](docs/ios.md) setup has a lot of steps, but at least Apple provides everything
you will need. Once again, if you are an iOS developer, most of the heavy lifting has
been already done. And if you are not, be aware that you won't have to pay anything.
* [other remote ssh-accessible devices](docs/ssh.md) are the easiest from dinghy point of view,
but you will be on your own to obtain the toolchain for your device architecture and
operating system. If your device is a Raspberry Pi running raspbian, we can help. :)

## Advanced topics and features

* Some projects need [resources files](docs/files.md) for running their tests or benches. Dinghy
tries its best to make it work in as many project/target configurations as
possible but some projects need a bit of help.
* In some bigger projects, you may need to [filter](docs/filter.md) your project's members depending
on the platform you want to test.
* Passing [environment](docs/vars.md) variables to your executable may sometimes be useful.
* Dinghy offers an [overlay](docs/overlay.md) system to "add" stuff to your toolchain 
sysroot. This allows you to add "stuff" to your build dependencies, like static libraries or headers
without altering the sysroot toolchain.
* The [`dinghy-build` crate](docs/dinghy-build.md) offers some `build.rs` features that are useful in
the context of cross-compilation.

## Using dinghy as a runner only
If your project already build for the target platform without dinghy and you only want to use dinghy to run code on a 
device, you can use dinghy's bundled runner directly. You simply need to register the dinghy as a runner in `.cargo/config`.
Here's an example for all apple targets

```toml
[target.'cfg(all(any(target_arch="aarch64",target_arch="x86_64"),target_vendor="apple",any(target_os="ios",target_os="tvos",target_os="apple-watchos")))']
runner = "cargo dinghy runner --"
```

You can then run your tests directly with `cargo test --target aarch64-apple-ios-sim` for example. 

Please note that the recommended way to use dinghy is as a cargo subcommand as it will set up quite a few things 
automatically for your project to even build. 

The runner will try to auto-detect the platform if it is not passed (as in the above example)

# License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
