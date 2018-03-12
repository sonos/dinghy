## Build script helpers [WIP]

Dinghy also provides a dinghy-helper crate to help with the writing of build scripts that performs cross-compilation, and more specifically:

- CommandExt: a std::process::Command extension that can:
  - Setup pkg-config environment variables for a subprocess (`PKG_CONFIG_LIBDIR`, ... e.g. when running Automake `./configure`)
  - Setup toolchain environment variables for a subprocess (`TARGET_CC`, ...)
  - A few other useful methods (e.g. `configure_prefix()` to set the prefix to rust output dir when running Automake `./configure`)
- BindGenBuilderExt: a bindgen::Builder extension to help writing C to rust bindings that supports cross compilation properly (see `new_bindgen_with_cross_compilation_support()`).

*This is still a WIP. Beware of possible breaking changes*


