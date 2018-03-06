
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

