## Getting started - Ssh device

Ssh setup is useful if you want to target small-ish devices that can host an
operating system but are too slow to iterate over Rust compilation/test cycles
comfortably. Think... something like a Raspberry, a NAS, or an old x86 box
in your garage.

### Dinghy setup

Assuming [rustup](http://rustup.rs) is already installed...

```
cargo install cargo-dinghy

# If it's already installed, add '--force'
cargo install cargo-dinghy --force
```

### Finding a toolchain

This is the hard part. Rust or dinghy will not solve anything magically here.
If you're lucky, your hardware vendor has provided you with one you can actually use.
Or somebody on the internet did the hard work.
Or else https://crosstool-ng.github.io/ will become your best friend.

Dinghy will assume the toolchain looks relatively "regular". That is, it expects to
find something that looks like a `sysroot`, a directory called bin with a compiler
and binutils.

Once you have this toolchain, that can compile and link a simple C helloworld
to something running on your device, you're ready to start playing with rust and dinghy.

### Install Rust target

```
rustup target install arm-unknown-linux-gnueabi
```

### Configure dinghy

The minimum configuration to be added on `~/.dinghy.toml` should look like that. You need
to define a platform by linking rustc target to your toolchain, and to link your device and
account to this platform. We recommend that you add your ssh key to the account you want to
link to, or the password prompts will drive you crazy.

```
[platforms.raspbian-stretch]
rustc_triple="arm-unknown-linux-gnueabihf"
toolchain="/path/to/a/toolchain/for/arm-unknown-linux-gnueabi"

[ssh_devices]
raspi = { hostname = "raspi.local", username="pi", platform="raspbian-stretch" }
```

### Try it

Let's try it with dinghy demo project. The project tests with "pass" in the
name are supposed to pass, the one with fail should break.

```
% git clone https://github.com/sonos/dinghy
% cd dinghy/test-ws
[...]
# these ones should pass
% cargo dinghy -d raspi test pass
[...]
# this one shall not pass
% cargo dinghy -d raspi test fail
[...]
```

That's it! Enjoy!
