
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
