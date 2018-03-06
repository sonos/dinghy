# Dinghy

[![Build Status](https://travis-ci.org/snipsco/dinghy.svg?branch=master)](https://travis-ci.org/snipsco/dinghy)

## What ?

Send `cargo test` or `cargo bench` to your phone. Painlessly (or at least with a bit less pain).

It is not a way to build portable apps, merely a way to run simple piece of
code and grab their output. On iOS, it also allows to run lldb and debug
interactively the executable.

The purpose here is to make it easier for teams to unit-test and bench their
libraries on more platforms. We want to get Rust everywhere right ?

Dinghy also supports compilation for, and execution on remote devices accessible
through ssh.

## How 

Once your dinghy setup is done, you will be able to run 
tests and benches from a simple cargo command **in any cargo project** without
altering them.

Just add `dinghy -d some_device` between `cargo` and its subcommand:

```
cargo dinghy -d my_android test
cargo dinghy -d my_raspberry bench
```

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
* The [`dinghy-helper` crate](docs/helper.md) offers some `build.rs` features that are useful in
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
