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


