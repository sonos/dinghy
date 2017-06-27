# Dinghy

[![Build Status](https://travis-ci.org/snipsco/dinghy.svg?branch=master)](https://travis-ci.org/snipsco/dinghy)

## What ?

Send `cargo test` or `cargo bench` to your phone. Painlessly.

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

If you've never used cross compiling before, you'll probaby want to pick 
a toolchain or two...

```
# a few ios devices
rustup target install aarch64-apple-ios
rustup target install armv7-apple-ios
rustup target install arm-linux-androideabi

# a suitable ios simulator
rustup target install x86_64-apple-ios

# android arm devices
rustup target install arm-linux-androideabi

# for a Raspberry Pi 2
rustup target install armv7-unknown-linux-gnueabihf
```

Let's install dinghy...

```
cargo install dinghy
```

And... stop here. Unfortunately, there is a bit of stuff to do by hand before
the fun stuff come.

## Android setup

You'll need the usual ANDROID_NDK_HOME, and `adb` somewhere in your path.
`fb-adb` will be used if its available on the path, giving better error
handling.
Also, your phone must have developer options enabled.

## iOS simulator setup

If you are on a mac, just start a simulator instance. Dinghy will detect it and
pick it. If you're a mac/iOS developper, it's nice to try that before diving
into the more convoluted iOS device setup that follows.

## iOS setup

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

We will need some configuration bits here. So let's create a `.dinghy.toml` in
your home directory:

```
[ssh_devices]
"raspi2" = { hostname = "10.1.2.3", username="pi", target="armv7-unknown-linux-gnueabihf" }
```

You must make sure that the ssh connection works without asking for a password
for the user/host combination. 

You will also probably have to set up a toolchain: rustc needs a working linker
to be able to generate executable. On my case, for using on a RaspberryPi, I
added these two lines in `~/.cargo/config`:

```
[target.armv7-unknown-linux-gnueabihf]
linker = "/Users/kali/dev/armv7-rpi2-linux-gnueabihf/bin/armv7-rpi2-linux-gnueabihf-gcc"
```

The reason why we did not have to do that in the iOS and Android case is... there
exists more or less standards ways to find out where the linker is. Most people
targetting iOS will use XCode, and most people targetting Android will have a
standard toolchain with a matching ANDROID_NDK_HOME. Here, we are on our own to
get and setup the toolchain.

But once this is done, well, it works the same.

## Sending sources files to the devices

Some tests are relying on the presence of files at relative places to be able
to proceed. But we can not always control where we will be executing from (we
can not always do `cd someplace` before running the tests).

So, the tests are "bundled" in the following way:

* root dinghy test directory
    * test_executable
    * src/ mirrors the not-ignorable files from your sources
    * test_data is contains configurable data to be sent to the device
        * some_file
        * some_dir

So let's say your integration test is in `tests/test_1.rs` and uses `tests/data_1.txt`.
It will be copied into `src/tests/data_1.txt`.

Anything in .gitignore or .dinghyignore will not be bundled.

To open your test file easily, you can cut and paste the following helper
function:

```rust
pub fn src_path() -> path::PathBuf {
    if cfg!(any(target_os = "ios", target_os = "android")) ||
       ::std::env::var("DINGHY").is_ok() {
        ::std::env::current_exe().unwrap().parent().unwrap().join("src")
    } else {
        path::PathBuf::from(".")
    }
}
```

Then in your test, use it accordingly.


```rust
    let data = src_path().join("tests/data_1.txt");
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

Once again, you'll need to go through an helper in your code. Something like
that should do:

```rust
pub fn test_data_path() -> Option<path::PathBuf> {
    if cfg!(any(target_os = "ios", target_os = "android")) ||
       ::std::env::var("DINGHY").is_ok() {
        Some(::std::env::current_exe().unwrap().parent().unwrap().join("test_data"))
    } else {
        None
    }
}
```

And use it like that:

```rust
let the_data = match test_data_path() {
    Some(p) => p.join("the_data"),
    None => path::PathBuf::from("../data-2017-02-05"),
}

let the_conf_file = match test_data_path() {
    Some(p) => p.join("conf_file"),
    None => path::PathBuf::from("/etc/some/file"),
```

We are playing with the idea of making some kind of `dinghy-rt`lib that you
could use as a `dev-dependency` to help with these.

## Enjoy

Now, you can go to your favourite Rust project. For a first try, I suggest you
consider something without too many exotic external dependencies. 

Then... instead of doing `cargo test`, try `cargo dinghy test`.

# License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
