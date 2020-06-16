# Dinghy

![rustc >= 1.40.0](https://img.shields.io/badge/rustc-%3E%3D1.40.0-brightgreen)
![MIT/Apache 2](https://img.shields.io/crates/l/dinghy)
![Build and test](https://github.com/snipsco/dinghy/workflows/Build%20and%20test/badge.svg)

## What ?

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

## How 

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

Depending on what is your targets and your workstation, setting
up Dinghy can be more or less easy. 

* [Android](docs/android.md) is relatively easy, specifically if you already are
a mobile developper
* [iOS](docs/ios.md) setup has a lot of steps, but at least Apple provides everything
you will need. Once again, if you are an iOS developper, most of the heavy lifting has
been already done. And if you are not, be aware that you won't have anything to pay.
* [other remore ssh-accessible devices](docs/ssh.md) easiest from dinghy point of view,
but you willbe on your own to obtain the toolchain for your device architecture and
operating system. If your device is a RaspberryPi running raspbian, we can help :)

## Advanced topics and features

* Some projects need [resources files](docs/files.md) for running their tests or benches. Dinghy
tries its best to make it work in as many project/target configuration as
possible but some projects needs a bit of help.
* In some bigger projects, you may need to [filter](docs/filter.md) you projects members depending
on the platform you want to test.
* Passing [environment](docs/vars.md) variables to your executable may sometimes be useful.
* Dinghy offers an [overlay](docs/overlay.md) system to "add" stuff to your toolchain 
sysroot. This allows you to add "stuff" to your build dependencies, like static libraries or headers
without altering the sysroot toolchain.
* The [`dinghy-build` crate](docs/dinghy-build.md) offers some `build.rs` features that are useful in
the context of cross-compilation.

# License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
