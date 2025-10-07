# Changelog

## [0.8.2](https://github.com/sonos/dinghy/tree/0.8.2) (2025-10-07)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.8.1...0.8.2)

**Closed issues:**

- Doctests freeze on Rust 1.89 [\#255](https://github.com/sonos/dinghy/issues/255)

**Merged pull requests:**

- 1.85.0 [\#258](https://github.com/sonos/dinghy/pull/258) ([kali](https://github.com/kali))
- ignore devices with no udid [\#257](https://github.com/sonos/dinghy/pull/257) ([kali](https://github.com/kali))
- 1.89 doctests support [\#256](https://github.com/sonos/dinghy/pull/256) ([fredszaq](https://github.com/fredszaq))
- fix ci by holding back some deps [\#254](https://github.com/sonos/dinghy/pull/254) ([kali](https://github.com/kali))
- Fix display of android platforms in cargo dinghy all-devices [\#250](https://github.com/sonos/dinghy/pull/250) ([fredszaq](https://github.com/fredszaq))

## [0.8.1](https://github.com/sonos/dinghy/tree/0.8.1) (2025-06-30)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.8.0...0.8.1)

**Closed issues:**

- No devices found for name hint `my\_android' [\#252](https://github.com/sonos/dinghy/issues/252)

**Merged pull requests:**

- Use VersionReq api to support X.Y versions \(needed for iOS 18\) [\#253](https://github.com/sonos/dinghy/pull/253) ([hdlj](https://github.com/hdlj))
- fix workspace resolver warning in test-ws [\#249](https://github.com/sonos/dinghy/pull/249) ([fredszaq](https://github.com/fredszaq))
- search ndk in default install location in linux [\#248](https://github.com/sonos/dinghy/pull/248) ([fredszaq](https://github.com/fredszaq))
- debug ci [\#247](https://github.com/sonos/dinghy/pull/247) ([kali](https://github.com/kali))
- Fix provisioning profile path detection [\#246](https://github.com/sonos/dinghy/pull/246) ([emricksinisonos](https://github.com/emricksinisonos))
- Revert "Disable proc macro tests for tvOS and watchOS" [\#244](https://github.com/sonos/dinghy/pull/244) ([fredszaq](https://github.com/fredszaq))

## [0.8.0](https://github.com/sonos/dinghy/tree/0.8.0) (2024-11-18)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.7.3...0.8.0)

**Closed issues:**

- WathOS CI broken [\#238](https://github.com/sonos/dinghy/issues/238)

**Merged pull requests:**

- Disable proc macro tests for tvOS and watchOS [\#243](https://github.com/sonos/dinghy/pull/243) ([fredszaq](https://github.com/fredszaq))
- Introduce plugin platform [\#242](https://github.com/sonos/dinghy/pull/242) ([fredszaq](https://github.com/fredszaq))
- add `skip_source_copy` flag [\#241](https://github.com/sonos/dinghy/pull/241) ([fredszaq](https://github.com/fredszaq))

## [0.7.3](https://github.com/sonos/dinghy/tree/0.7.3) (2024-10-16)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.7.2...0.7.3)

**Merged pull requests:**

- XCode 16: profiles location has changed [\#239](https://github.com/sonos/dinghy/pull/239) ([kali](https://github.com/kali))
- accept licenses [\#236](https://github.com/sonos/dinghy/pull/236) ([kali](https://github.com/kali))

## [0.7.2](https://github.com/sonos/dinghy/tree/0.7.2) (2024-06-17)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.7.1...0.7.2)

**Closed issues:**

- About the build performance [\#232](https://github.com/sonos/dinghy/issues/232)
- xcrun: error: unable to find utility "devicectl", not a developer tool or in PATH [\#225](https://github.com/sonos/dinghy/issues/225)

**Merged pull requests:**

- remove deprecated atty and tempdir dependencies [\#235](https://github.com/sonos/dinghy/pull/235) ([fredszaq](https://github.com/fredszaq))
- cargo fmt [\#234](https://github.com/sonos/dinghy/pull/234) ([ThibautLorrainSonos](https://github.com/ThibautLorrainSonos))
- libclang path is in lib and not lib64 in ndk 26+ [\#233](https://github.com/sonos/dinghy/pull/233) ([ThibautLorrainSonos](https://github.com/ThibautLorrainSonos))
- Update github org [\#231](https://github.com/sonos/dinghy/pull/231) ([jayvdb](https://github.com/jayvdb))
- show id of ssh devices \(in addtion to their ip\) in all-devices command [\#230](https://github.com/sonos/dinghy/pull/230) ([fredszaq](https://github.com/fredszaq))
- Use matrix in CI for third tier apple simulators [\#228](https://github.com/sonos/dinghy/pull/228) ([simlay](https://github.com/simlay))

## [0.7.1](https://github.com/sonos/dinghy/tree/0.7.1) (2024-04-11)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.7.0...0.7.1)

**Merged pull requests:**

- fix macos binary upload [\#227](https://github.com/sonos/dinghy/pull/227) ([kali](https://github.com/kali))
- only warn if xcrun is missing [\#226](https://github.com/sonos/dinghy/pull/226) ([kali](https://github.com/kali))
- Bump msrv to 1.74 [\#224](https://github.com/sonos/dinghy/pull/224) ([simlay](https://github.com/simlay))

## [0.7.0](https://github.com/sonos/dinghy/tree/0.7.0) (2024-04-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.8...0.7.0)

**Closed issues:**

- Run test on iOS failed: No such file or directory \(os error 2\) [\#216](https://github.com/sonos/dinghy/issues/216)
- Support cargo-dinghy as a cargo test runner [\#210](https://github.com/sonos/dinghy/issues/210)
- Dinghy hangs on macOS-14 runner [\#207](https://github.com/sonos/dinghy/issues/207)
- Use `xcrun devicectl` to deploy to iOS devices. [\#204](https://github.com/sonos/dinghy/issues/204)
- Faiiled to build what use ndk\_context project [\#201](https://github.com/sonos/dinghy/issues/201)
- Add macOS M1 binary to release CI [\#200](https://github.com/sonos/dinghy/issues/200)
- Getting files generated by tests back? [\#199](https://github.com/sonos/dinghy/issues/199)
- `undefined symbol: ANativeWindow_setBuffersGeometry` with visual tests using `wgpu` [\#198](https://github.com/sonos/dinghy/issues/198)
- watchOS simulator support [\#194](https://github.com/sonos/dinghy/issues/194)
- tvOS simulator support [\#193](https://github.com/sonos/dinghy/issues/193)
- How to build with non-default crate features? [\#129](https://github.com/sonos/dinghy/issues/129)

**Merged pull requests:**

- use fs\_err when it make sense in dinghy\_lib [\#222](https://github.com/sonos/dinghy/pull/222) ([kali](https://github.com/kali))
- follow symlinks while walking dirs \(and improve error messages\) [\#221](https://github.com/sonos/dinghy/pull/221) ([kali](https://github.com/kali))
- fix when multiple Dinghy app are installed [\#220](https://github.com/sonos/dinghy/pull/220) ([kali](https://github.com/kali))
- add changelog generation script in ./pre-release.sh [\#219](https://github.com/sonos/dinghy/pull/219) ([fredszaq](https://github.com/fredszaq))
- better error message on pymobiledevice3 failure [\#218](https://github.com/sonos/dinghy/pull/218) ([kali](https://github.com/kali))
- android: fix SDK detection on mac. [\#217](https://github.com/sonos/dinghy/pull/217) ([ashdnazg](https://github.com/ashdnazg))
- Support for ios17 [\#215](https://github.com/sonos/dinghy/pull/215) ([kali](https://github.com/kali))
- Infer the platform when runner is called in standalone mode [\#213](https://github.com/sonos/dinghy/pull/213) ([fredszaq](https://github.com/fredszaq))
- Universal macos binary in CI [\#209](https://github.com/sonos/dinghy/pull/209) ([simlay](https://github.com/simlay))
- Test on macOS-13 \(x86\) and macOS-14 \(arm\) [\#206](https://github.com/sonos/dinghy/pull/206) ([simlay](https://github.com/simlay))
- android emulator needs to be installed for avdmanager to work [\#205](https://github.com/sonos/dinghy/pull/205) ([ThibautLorrainSonos](https://github.com/ThibautLorrainSonos))
- Support tvOS and watchOS simulators [\#203](https://github.com/sonos/dinghy/pull/203) ([simlay](https://github.com/simlay))
- use android emulator 32.1.15 in ci as 33.1.23 segfaults [\#202](https://github.com/sonos/dinghy/pull/202) ([fredszaq](https://github.com/fredszaq))
- Bump rustix from 0.38.14 to 0.38.19 [\#196](https://github.com/sonos/dinghy/pull/196) ([dependabot[bot]](https://github.com/apps/dependabot))
- Fix preferences path after Ventura [\#195](https://github.com/sonos/dinghy/pull/195) ([ldm0](https://github.com/ldm0))

## [0.6.8](https://github.com/sonos/dinghy/tree/0.6.8) (2023-09-29)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.7...0.6.8)

**Merged pull requests:**

- search android ndk also in ANDROID\_NDK env var and pass ANDROID\_NDK var to build if not set \(cmake compat\) [\#192](https://github.com/sonos/dinghy/pull/192) ([fredszaq](https://github.com/fredszaq))

## [0.6.7](https://github.com/sonos/dinghy/tree/0.6.7) (2023-09-14)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.6...0.6.7)

**Closed issues:**

- Maintain CHANGELOG [\#187](https://github.com/sonos/dinghy/issues/187)

**Merged pull requests:**

- Do not crash dinghy\_bindgen macro if DINGHY\_BUILD\_LIBCLANG\_PATH is not present [\#191](https://github.com/sonos/dinghy/pull/191) ([fredszaq](https://github.com/fredszaq))

## [0.6.6](https://github.com/sonos/dinghy/tree/0.6.6) (2023-09-14)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.5...0.6.6)

**Merged pull requests:**

- msrv 1.70 [\#190](https://github.com/sonos/dinghy/pull/190) ([fredszaq](https://github.com/fredszaq))
- fix bindgen config when using android ndk [\#189](https://github.com/sonos/dinghy/pull/189) ([fredszaq](https://github.com/fredszaq))

## [0.6.5](https://github.com/sonos/dinghy/tree/0.6.5) (2023-09-11)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.4...0.6.5)

**Closed issues:**

- `catch_unwind` broken on `armv7-apple-ios` [\#156](https://github.com/sonos/dinghy/issues/156)

**Merged pull requests:**

- properly setup clang++ as CXX in android auto platforms [\#188](https://github.com/sonos/dinghy/pull/188) ([fredszaq](https://github.com/fredszaq))
- cargo update \(procmacro2 failing to compile on nightly\) [\#186](https://github.com/sonos/dinghy/pull/186) ([fredszaq](https://github.com/fredszaq))
- make RUST\_BACKTRACE overwritable by --env [\#185](https://github.com/sonos/dinghy/pull/185) ([bestouff](https://github.com/bestouff))
- add legacy dinghy crate [\#184](https://github.com/sonos/dinghy/pull/184) ([fredszaq](https://github.com/fredszaq))
- bump msrv [\#183](https://github.com/sonos/dinghy/pull/183) ([kali](https://github.com/kali))

## [0.6.4](https://github.com/sonos/dinghy/tree/0.6.4) (2023-04-13)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.3...0.6.4)

**Merged pull requests:**

- use llvm-ar instead of the binutils one for android ndk 23+ [\#182](https://github.com/sonos/dinghy/pull/182) ([fredszaq](https://github.com/fredszaq))

## [0.6.3](https://github.com/sonos/dinghy/tree/0.6.3) (2022-11-18)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.2...0.6.3)

**Closed issues:**

- On the use `std::fs::ReadDir.next()` [\#177](https://github.com/sonos/dinghy/issues/177)
- failed to compile cargo-dinghy v0.3.2 - multiple packages link to native library git2 [\#66](https://github.com/sonos/dinghy/issues/66)

**Merged pull requests:**

- add all shared libs as args in run-with [\#181](https://github.com/sonos/dinghy/pull/181) ([fredszaq](https://github.com/fredszaq))
- bump iphone min version [\#180](https://github.com/sonos/dinghy/pull/180) ([kali](https://github.com/kali))
- add apt update [\#179](https://github.com/sonos/dinghy/pull/179) ([kali](https://github.com/kali))

## [0.6.2](https://github.com/sonos/dinghy/tree/0.6.2) (2022-08-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.1...0.6.2)

**Closed issues:**

- No more working on iOS devices 14.x [\#135](https://github.com/sonos/dinghy/issues/135)
- Cannot run tests on iOS 14 device [\#131](https://github.com/sonos/dinghy/issues/131)
- test --no-fail-fast is not recognized [\#127](https://github.com/sonos/dinghy/issues/127)
- Old android device using ro.product.cpu.abi rather than ro.product.cpu.abilist [\#109](https://github.com/sonos/dinghy/issues/109)
- ImportError: No module named six [\#6](https://github.com/sonos/dinghy/issues/6)

**Merged pull requests:**

- fix android toolchain discovery on macOS [\#178](https://github.com/sonos/dinghy/pull/178) ([fredszaq](https://github.com/fredszaq))
- Mention ios-deploy in docu [\#176](https://github.com/sonos/dinghy/pull/176) ([umgefahren](https://github.com/umgefahren))

## [0.6.1](https://github.com/sonos/dinghy/tree/0.6.1) (2022-08-04)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.6.0...0.6.1)

**Closed issues:**

- deploy to iOS devices with ios-deploy [\#166](https://github.com/sonos/dinghy/issues/166)
- No device support directory for iOS version 12.5 [\#165](https://github.com/sonos/dinghy/issues/165)
- lldb output is variable [\#158](https://github.com/sonos/dinghy/issues/158)

**Merged pull requests:**

- musl build fix [\#175](https://github.com/sonos/dinghy/pull/175) ([fredszaq](https://github.com/fredszaq))
- display compile errors and check cargo exit status in run-with [\#174](https://github.com/sonos/dinghy/pull/174) ([fredszaq](https://github.com/fredszaq))
- Dependency cleanup [\#172](https://github.com/sonos/dinghy/pull/172) ([madsmtm](https://github.com/madsmtm))
- Support newer versions of the OpenSSL x509 text format [\#171](https://github.com/sonos/dinghy/pull/171) ([madsmtm](https://github.com/madsmtm))

## [0.6.0](https://github.com/sonos/dinghy/tree/0.6.0) (2022-07-27)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.5.1...0.6.0)

**Closed issues:**

- Share resource files between tests [\#161](https://github.com/sonos/dinghy/issues/161)

**Merged pull requests:**

- some ios signing logging adjusts [\#170](https://github.com/sonos/dinghy/pull/170) ([kali](https://github.com/kali))
- Logging overhaul [\#169](https://github.com/sonos/dinghy/pull/169) ([fredszaq](https://github.com/fredszaq))
- use ios-deploy, remove in-house rust partial port [\#168](https://github.com/sonos/dinghy/pull/168) ([kali](https://github.com/kali))
- introduce run-with subcommand and transparent copy of files in runner args [\#167](https://github.com/sonos/dinghy/pull/167) ([fredszaq](https://github.com/fredszaq))
- do not copy ad-hoc rsync on device if file exists [\#164](https://github.com/sonos/dinghy/pull/164) ([fredszaq](https://github.com/fredszaq))
- Use package name instead of runnable id for dir on target [\#163](https://github.com/sonos/dinghy/pull/163) ([fredszaq](https://github.com/fredszaq))
- support android ndk 23 and up [\#162](https://github.com/sonos/dinghy/pull/162) ([fredszaq](https://github.com/fredszaq))
- Broken implicit wp dep [\#160](https://github.com/sonos/dinghy/pull/160) ([kali](https://github.com/kali))

## [0.5.1](https://github.com/sonos/dinghy/tree/0.5.1) (2022-07-08)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.5.0...0.5.1)

**Merged pull requests:**

- try to make the excluded bug appear in CI [\#159](https://github.com/sonos/dinghy/pull/159) ([kali](https://github.com/kali))

## [0.5.0](https://github.com/sonos/dinghy/tree/0.5.0) (2022-07-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.71...0.5.0)

**Closed issues:**

- cargo install dinghy takes too long [\#39](https://github.com/sonos/dinghy/issues/39)

**Merged pull requests:**

- Remove bundled cargo [\#157](https://github.com/sonos/dinghy/pull/157) ([fredszaq](https://github.com/fredszaq))
- Update cargo to v0.62 [\#154](https://github.com/sonos/dinghy/pull/154) ([madsmtm](https://github.com/madsmtm))

## [0.4.71](https://github.com/sonos/dinghy/tree/0.4.71) (2022-03-21)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.70...0.4.71)

**Closed issues:**

- Support for aarch64-apple-ios-sim in dinghy [\#147](https://github.com/sonos/dinghy/issues/147)

**Merged pull requests:**

- Fix iOS bundle to make app use the full screen [\#152](https://github.com/sonos/dinghy/pull/152) ([simlay](https://github.com/simlay))
- Initial stuff for aarch64 ios simulator support [\#151](https://github.com/sonos/dinghy/pull/151) ([simlay](https://github.com/simlay))

## [0.4.70](https://github.com/sonos/dinghy/tree/0.4.70) (2022-03-18)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.69...0.4.70)

**Merged pull requests:**

- bump cargo [\#150](https://github.com/sonos/dinghy/pull/150) ([kali](https://github.com/kali))

## [0.4.69](https://github.com/sonos/dinghy/tree/0.4.69) (2022-03-17)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.68...0.4.69)

**Closed issues:**

- Rust 2021 Edition [\#148](https://github.com/sonos/dinghy/issues/148)

**Merged pull requests:**

- Fix iOS simulator tests with lldb 13 [\#149](https://github.com/sonos/dinghy/pull/149) ([simlay](https://github.com/simlay))
- tests are actually runnable on host platform [\#146](https://github.com/sonos/dinghy/pull/146) ([kali](https://github.com/kali))
- Fix ci weirdness [\#145](https://github.com/sonos/dinghy/pull/145) ([kali](https://github.com/kali))
- Fix iOS 9.3 [\#143](https://github.com/sonos/dinghy/pull/143) ([madsmtm](https://github.com/madsmtm))
- Fix "Developer" typo [\#142](https://github.com/sonos/dinghy/pull/142) ([madsmtm](https://github.com/madsmtm))

## [0.4.68](https://github.com/sonos/dinghy/tree/0.4.68) (2022-01-19)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.67...0.4.68)

**Merged pull requests:**

- Dont run tests for `proc-macro` crates [\#144](https://github.com/sonos/dinghy/pull/144) ([madsmtm](https://github.com/madsmtm))

## [0.4.67](https://github.com/sonos/dinghy/tree/0.4.67) (2021-12-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.66...0.4.67)

**Merged pull requests:**

- add support for custom profiles [\#140](https://github.com/sonos/dinghy/pull/140) ([fredszaq](https://github.com/fredszaq))

## [0.4.66](https://github.com/sonos/dinghy/tree/0.4.66) (2021-11-29)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.63...0.4.66)

**Closed issues:**

- test-app build fails with unable to find library -lgcc [\#138](https://github.com/sonos/dinghy/issues/138)

**Merged pull requests:**

- manual import from anyhow [\#139](https://github.com/sonos/dinghy/pull/139) ([kali](https://github.com/kali))

## [0.4.63](https://github.com/sonos/dinghy/tree/0.4.63) (2021-11-04)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.62...0.4.63)

**Closed issues:**

- Support building crates that use resolver 2 [\#133](https://github.com/sonos/dinghy/issues/133)

**Merged pull requests:**

- update cargo and other dependencies [\#137](https://github.com/sonos/dinghy/pull/137) ([fredszaq](https://github.com/fredszaq))

## [0.4.62](https://github.com/sonos/dinghy/tree/0.4.62) (2021-07-28)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.61...0.4.62)

**Merged pull requests:**

- Bump cargo [\#134](https://github.com/sonos/dinghy/pull/134) ([kali](https://github.com/kali))
- Fixes the iOS test command [\#132](https://github.com/sonos/dinghy/pull/132) ([Robert-Steiner](https://github.com/Robert-Steiner))

## [0.4.61](https://github.com/sonos/dinghy/tree/0.4.61) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.60...0.4.61)

## [0.4.60](https://github.com/sonos/dinghy/tree/0.4.60) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.59...0.4.60)

## [0.4.59](https://github.com/sonos/dinghy/tree/0.4.59) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.57...0.4.59)

## [0.4.57](https://github.com/sonos/dinghy/tree/0.4.57) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.55...0.4.57)

## [0.4.55](https://github.com/sonos/dinghy/tree/0.4.55) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.54...0.4.55)

## [0.4.54](https://github.com/sonos/dinghy/tree/0.4.54) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.53...0.4.54)

## [0.4.53](https://github.com/sonos/dinghy/tree/0.4.53) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.52...0.4.53)

## [0.4.52](https://github.com/sonos/dinghy/tree/0.4.52) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.51...0.4.52)

## [0.4.51](https://github.com/sonos/dinghy/tree/0.4.51) (2021-02-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.50...0.4.51)

## [0.4.50](https://github.com/sonos/dinghy/tree/0.4.50) (2021-02-12)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.48...0.4.50)

## [0.4.48](https://github.com/sonos/dinghy/tree/0.4.48) (2021-02-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.47...0.4.48)

## [0.4.47](https://github.com/sonos/dinghy/tree/0.4.47) (2021-02-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.46...0.4.47)

## [0.4.46](https://github.com/sonos/dinghy/tree/0.4.46) (2021-02-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.45...0.4.46)

## [0.4.45](https://github.com/sonos/dinghy/tree/0.4.45) (2021-02-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.43...0.4.45)

## [0.4.43](https://github.com/sonos/dinghy/tree/0.4.43) (2021-02-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.41...0.4.43)

**Closed issues:**

- don't work for ios 13.3 on osx 10.15.5 [\#116](https://github.com/sonos/dinghy/issues/116)

**Merged pull requests:**

- Maintenance [\#130](https://github.com/sonos/dinghy/pull/130) ([kali](https://github.com/kali))
- Better error message on rsync exec [\#128](https://github.com/sonos/dinghy/pull/128) ([fredszaq](https://github.com/fredszaq))
- Fix some grammar and typos in README.md [\#126](https://github.com/sonos/dinghy/pull/126) ([geophree](https://github.com/geophree))

## [0.4.41](https://github.com/sonos/dinghy/tree/0.4.41) (2020-10-16)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.40...0.4.41)

**Closed issues:**

- Copies libdl.so to Android, causing failure to run [\#124](https://github.com/sonos/dinghy/issues/124)
- Dinghy on Linux [\#121](https://github.com/sonos/dinghy/issues/121)

**Merged pull requests:**

- Prevent deployment of libdl.so on Android [\#125](https://github.com/sonos/dinghy/pull/125) ([tom-bowles](https://github.com/tom-bowles))

## [0.4.40](https://github.com/sonos/dinghy/tree/0.4.40) (2020-09-10)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.39...0.4.40)

**Merged pull requests:**

- Check Android NDK in non legacy path [\#123](https://github.com/sonos/dinghy/pull/123) ([kafji](https://github.com/kafji))
- Log on Android NDK not found [\#122](https://github.com/sonos/dinghy/pull/122) ([kafji](https://github.com/kafji))

## [0.4.39](https://github.com/sonos/dinghy/tree/0.4.39) (2020-08-05)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.38...0.4.39)

**Merged pull requests:**

- iOS environment variable support [\#120](https://github.com/sonos/dinghy/pull/120) ([Jasper-Bekkers](https://github.com/Jasper-Bekkers))
- Add arm64e support [\#119](https://github.com/sonos/dinghy/pull/119) ([Jasper-Bekkers](https://github.com/Jasper-Bekkers))

## [0.4.38](https://github.com/sonos/dinghy/tree/0.4.38) (2020-06-18)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.37...0.4.38)

**Merged pull requests:**

- Ssh arg port order [\#118](https://github.com/sonos/dinghy/pull/118) ([kali](https://github.com/kali))
- Features bugfix [\#117](https://github.com/sonos/dinghy/pull/117) ([kali](https://github.com/kali))

## [0.4.37](https://github.com/sonos/dinghy/tree/0.4.37) (2020-05-29)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.35...0.4.37)

**Closed issues:**

- Long term solution for URL regression [\#101](https://github.com/sonos/dinghy/issues/101)

**Merged pull requests:**

- add missing x86\_64 android device [\#115](https://github.com/sonos/dinghy/pull/115) ([MarcTreySonos](https://github.com/MarcTreySonos))
- experimental shell expansion in run [\#114](https://github.com/sonos/dinghy/pull/114) ([kali](https://github.com/kali))
- avoid createing huge and useless PKG\_CONFIG\_LIBDIR [\#113](https://github.com/sonos/dinghy/pull/113) ([kali](https://github.com/kali))
- Allow SSH devices to use the host toolchain [\#112](https://github.com/sonos/dinghy/pull/112) ([fredszaq](https://github.com/fredszaq))

## [0.4.35](https://github.com/sonos/dinghy/tree/0.4.35) (2020-04-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.34...0.4.35)

**Merged pull requests:**

- bump cargo, and switch to anyhow [\#111](https://github.com/sonos/dinghy/pull/111) ([kali](https://github.com/kali))
- eradicate two warnings [\#108](https://github.com/sonos/dinghy/pull/108) ([kali](https://github.com/kali))

## [0.4.34](https://github.com/sonos/dinghy/tree/0.4.34) (2020-04-07)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.33...0.4.34)

## [0.4.33](https://github.com/sonos/dinghy/tree/0.4.33) (2020-04-07)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.31...0.4.33)

**Merged pull requests:**

- build bench in release mode [\#110](https://github.com/sonos/dinghy/pull/110) ([kali](https://github.com/kali))
- some android auto target fixes [\#91](https://github.com/sonos/dinghy/pull/91) ([kali](https://github.com/kali))

## [0.4.31](https://github.com/sonos/dinghy/tree/0.4.31) (2020-04-07)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.30...0.4.31)

## [0.4.30](https://github.com/sonos/dinghy/tree/0.4.30) (2020-04-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.29...0.4.30)

## [0.4.29](https://github.com/sonos/dinghy/tree/0.4.29) (2020-04-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.28...0.4.29)

## [0.4.28](https://github.com/sonos/dinghy/tree/0.4.28) (2020-04-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.27...0.4.28)

## [0.4.27](https://github.com/sonos/dinghy/tree/0.4.27) (2020-04-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.26...0.4.27)

## [0.4.26](https://github.com/sonos/dinghy/tree/0.4.26) (2020-04-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.25...0.4.26)

## [0.4.25](https://github.com/sonos/dinghy/tree/0.4.25) (2020-04-06)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.24...0.4.25)

**Merged pull requests:**

- Daxpedia windows fix [\#107](https://github.com/sonos/dinghy/pull/107) ([kali](https://github.com/kali))
- Fix targets which does not have sysroot [\#106](https://github.com/sonos/dinghy/pull/106) ([MarcTreySonos](https://github.com/MarcTreySonos))
- add upload rsync binary step for the ssh devices [\#105](https://github.com/sonos/dinghy/pull/105) ([MarcTreySonos](https://github.com/MarcTreySonos))
- Android ndk 19+ documentation [\#86](https://github.com/sonos/dinghy/pull/86) ([Deluvi](https://github.com/Deluvi))

## [0.4.24](https://github.com/sonos/dinghy/tree/0.4.24) (2020-02-13)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.23...0.4.24)

**Merged pull requests:**

- libc++\_shared.so must be included for android deployment  [\#103](https://github.com/sonos/dinghy/pull/103) ([MarcTreySonos](https://github.com/MarcTreySonos))

## [0.4.23](https://github.com/sonos/dinghy/tree/0.4.23) (2020-02-13)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.22...0.4.23)

**Fixed bugs:**

- Revert `url` lib bump to 2.1.0 [\#104](https://github.com/sonos/dinghy/pull/104) ([Deluvi](https://github.com/Deluvi))

## [0.4.22](https://github.com/sonos/dinghy/tree/0.4.22) (2020-01-30)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.21...0.4.22)

**Closed issues:**

- dinghy\_test::test\_file\_path doesn't work when running tests natively on desktop platform [\#99](https://github.com/sonos/dinghy/issues/99)

**Merged pull requests:**

- fix ci [\#102](https://github.com/sonos/dinghy/pull/102) ([kali](https://github.com/kali))
- use crate url 2.0.0 [\#97](https://github.com/sonos/dinghy/pull/97) ([MarcTreySonos](https://github.com/MarcTreySonos))

## [0.4.21](https://github.com/sonos/dinghy/tree/0.4.21) (2020-01-27)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.20...0.4.21)

**Merged pull requests:**

- Refactor bindgen related code  [\#98](https://github.com/sonos/dinghy/pull/98) ([Deluvi](https://github.com/Deluvi))

## [0.4.20](https://github.com/sonos/dinghy/tree/0.4.20) (2020-01-16)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.19...0.4.20)

**Merged pull requests:**

- Change IosSimDevice to use xcrun simctl launch rather than lldb [\#96](https://github.com/sonos/dinghy/pull/96) ([simlay](https://github.com/simlay))

## [0.4.19](https://github.com/sonos/dinghy/tree/0.4.19) (2020-01-13)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.18...0.4.19)

**Merged pull requests:**

- Updated dependencies [\#95](https://github.com/sonos/dinghy/pull/95) ([simlay](https://github.com/simlay))

## [0.4.18](https://github.com/sonos/dinghy/tree/0.4.18) (2019-10-28)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.17...0.4.18)

**Merged pull requests:**

- add a "private" framework search path [\#94](https://github.com/sonos/dinghy/pull/94) ([kali](https://github.com/kali))

## [0.4.17](https://github.com/sonos/dinghy/tree/0.4.17) (2019-10-24)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.16...0.4.17)

## [0.4.16](https://github.com/sonos/dinghy/tree/0.4.16) (2019-09-16)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.15...0.4.16)

## [0.4.15](https://github.com/sonos/dinghy/tree/0.4.15) (2019-09-16)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.14...0.4.15)

**Merged pull requests:**

- try on bionic [\#93](https://github.com/sonos/dinghy/pull/93) ([kali](https://github.com/kali))

## [0.4.14](https://github.com/sonos/dinghy/tree/0.4.14) (2019-09-16)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.13...0.4.14)

## [0.4.13](https://github.com/sonos/dinghy/tree/0.4.13) (2019-09-08)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.12...0.4.13)

## [0.4.12](https://github.com/sonos/dinghy/tree/0.4.12) (2019-08-29)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.11...0.4.12)

## [0.4.11](https://github.com/sonos/dinghy/tree/0.4.11) (2019-05-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.10...0.4.11)

**Closed issues:**

- Add android app platform version to specify/match a minSdk Version [\#85](https://github.com/sonos/dinghy/issues/85)

**Merged pull requests:**

- bump osx [\#90](https://github.com/sonos/dinghy/pull/90) ([kali](https://github.com/kali))
- Prevent library copy of lib file in the toolchain's sysroot when running on a remote device [\#88](https://github.com/sonos/dinghy/pull/88) ([Deluvi](https://github.com/Deluvi))
- Make automatic platform decision for a given device deterministic [\#84](https://github.com/sonos/dinghy/pull/84) ([Deluvi](https://github.com/Deluvi))

## [0.4.10](https://github.com/sonos/dinghy/tree/0.4.10) (2019-02-26)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.8...0.4.10)

**Closed issues:**

- Fails to sign when there is any non .mobileprovision file in Profiles folder [\#77](https://github.com/sonos/dinghy/issues/77)

**Merged pull requests:**

- Android ndk 19+ API level choice [\#83](https://github.com/sonos/dinghy/pull/83) ([Deluvi](https://github.com/Deluvi))
- Fix multiple versions of the same dynamic lib used [\#82](https://github.com/sonos/dinghy/pull/82) ([Deluvi](https://github.com/Deluvi))

## [0.4.8](https://github.com/sonos/dinghy/tree/0.4.8) (2019-02-14)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.7...0.4.8)

**Merged pull requests:**

- reactivate toolchain binary shims as they are needed by some projects [\#79](https://github.com/sonos/dinghy/pull/79) ([fredszaq](https://github.com/fredszaq))

## [0.4.7](https://github.com/sonos/dinghy/tree/0.4.7) (2019-02-13)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.6...0.4.7)

## [0.4.6](https://github.com/sonos/dinghy/tree/0.4.6) (2019-02-12)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.4.5...0.4.6)

**Merged pull requests:**

- various fix for ios [\#78](https://github.com/sonos/dinghy/pull/78) ([kali](https://github.com/kali))

## [0.4.5](https://github.com/sonos/dinghy/tree/0.4.5) (2019-02-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.16...0.4.5)

**Closed issues:**

- Failure linking backtrace-sys on android: undefined reference to 'getpagesize' [\#61](https://github.com/sonos/dinghy/issues/61)
- Error when I try to use dinghy [\#48](https://github.com/sonos/dinghy/issues/48)
- Should benchmarks return performance numbers? [\#44](https://github.com/sonos/dinghy/issues/44)
- MacOS: Unexpected end of JSON with Xcode CommandLineTools [\#32](https://github.com/sonos/dinghy/issues/32)

**Merged pull requests:**

- more android fixes and debug info [\#76](https://github.com/sonos/dinghy/pull/76) ([kali](https://github.com/kali))
- refactor + android clang support [\#75](https://github.com/sonos/dinghy/pull/75) ([kali](https://github.com/kali))
- Script device [\#74](https://github.com/sonos/dinghy/pull/74) ([kali](https://github.com/kali))
- update cargo to 0.32 - edition 2018 support [\#73](https://github.com/sonos/dinghy/pull/73) ([fredszaq](https://github.com/fredszaq))
- Fix typos in documentation [\#69](https://github.com/sonos/dinghy/pull/69) ([adrienball](https://github.com/adrienball))
- support --no-run [\#67](https://github.com/sonos/dinghy/pull/67) ([kali](https://github.com/kali))
- filename conflict in "run" [\#65](https://github.com/sonos/dinghy/pull/65) ([kali](https://github.com/kali))
- some trace! level info, plus move exe to target/ in bundle [\#64](https://github.com/sonos/dinghy/pull/64) ([kali](https://github.com/kali))
- Bump dependencies [\#63](https://github.com/sonos/dinghy/pull/63) ([Eijebong](https://github.com/Eijebong))
- add static linking helper [\#62](https://github.com/sonos/dinghy/pull/62) ([MarcTreySonos](https://github.com/MarcTreySonos))
- Define defualt toolchain directory in .dinghy [\#60](https://github.com/sonos/dinghy/pull/60) ([rtmvc](https://github.com/rtmvc))
- do not copy target in target [\#58](https://github.com/sonos/dinghy/pull/58) ([kali](https://github.com/kali))
- Update build\_env.rs [\#57](https://github.com/sonos/dinghy/pull/57) ([warent](https://github.com/warent))
- Update dinghy crate to cargo-dinghy [\#56](https://github.com/sonos/dinghy/pull/56) ([nebuto](https://github.com/nebuto))
- Allow debug build mode arg [\#55](https://github.com/sonos/dinghy/pull/55) ([rtmvc](https://github.com/rtmvc))
- Set permissions before copy [\#54](https://github.com/sonos/dinghy/pull/54) ([rtmvc](https://github.com/rtmvc))
- Copy libs for host platform too [\#53](https://github.com/sonos/dinghy/pull/53) ([rtmvc](https://github.com/rtmvc))
- Warn if package filtered on platform [\#52](https://github.com/sonos/dinghy/pull/52) ([rtmvc](https://github.com/rtmvc))
- Copy ios libs [\#51](https://github.com/sonos/dinghy/pull/51) ([rtmvc](https://github.com/rtmvc))
- Copy .so dependencies in target directory for builds [\#50](https://github.com/sonos/dinghy/pull/50) ([rtmvc](https://github.com/rtmvc))
- Strip executable copy and not original as cargo might not regenerate … [\#49](https://github.com/sonos/dinghy/pull/49) ([rtmvc](https://github.com/rtmvc))
- Copy .so dependencies in target directory for builds [\#47](https://github.com/sonos/dinghy/pull/47) ([kali](https://github.com/kali))
- Strip [\#46](https://github.com/sonos/dinghy/pull/46) ([kali](https://github.com/kali))
- Fix nightly compiler error [\#45](https://github.com/sonos/dinghy/pull/45) ([pitdicker](https://github.com/pitdicker))
- Task/cc rs compat [\#42](https://github.com/sonos/dinghy/pull/42) ([kali](https://github.com/kali))
- 0.3 [\#40](https://github.com/sonos/dinghy/pull/40) ([rtmvc](https://github.com/rtmvc))

## [0.2.16](https://github.com/sonos/dinghy/tree/0.2.16) (2018-02-05)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.15...0.2.16)

**Closed issues:**

- Android emulator support? [\#37](https://github.com/sonos/dinghy/issues/37)
- Error installing dinghy [\#36](https://github.com/sonos/dinghy/issues/36)

## [0.2.15](https://github.com/sonos/dinghy/tree/0.2.15) (2017-11-23)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.14...0.2.15)

**Closed issues:**

- MacOS Cannot build with Xcode 9 \(9A235\) [\#29](https://github.com/sonos/dinghy/issues/29)

**Merged pull requests:**

- Update to cargo 0.22, fix a few warnings [\#33](https://github.com/sonos/dinghy/pull/33) ([ryanpresciense](https://github.com/ryanpresciense))

## [0.2.14](https://github.com/sonos/dinghy/tree/0.2.14) (2017-10-05)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.13...0.2.14)

**Closed issues:**

- Refactoring some common logic into a library [\#4](https://github.com/sonos/dinghy/issues/4)

**Merged pull requests:**

- Bump to xcode 9 [\#31](https://github.com/sonos/dinghy/pull/31) ([klefevre](https://github.com/klefevre))
- Bump dependencies [\#30](https://github.com/sonos/dinghy/pull/30) ([klefevre](https://github.com/klefevre))

## [0.2.13](https://github.com/sonos/dinghy/tree/0.2.13) (2017-07-04)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.12...0.2.13)

## [0.2.12](https://github.com/sonos/dinghy/tree/0.2.12) (2017-07-04)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.11...0.2.12)

**Closed issues:**

- Allow specifying port number for ssh [\#26](https://github.com/sonos/dinghy/issues/26)
- Need some way of deploying test\_data even if they are included in .gitignore [\#24](https://github.com/sonos/dinghy/issues/24)

**Merged pull requests:**

- Bump dependencies [\#28](https://github.com/sonos/dinghy/pull/28) ([klefevre](https://github.com/klefevre))
- Implement clean\_app in ssh, add path and port options [\#27](https://github.com/sonos/dinghy/pull/27) ([azdlowry](https://github.com/azdlowry))
- Allow copy\_git\_ignored to be added to test data [\#25](https://github.com/sonos/dinghy/pull/25) ([azdlowry](https://github.com/azdlowry))
- Android Improvements [\#23](https://github.com/sonos/dinghy/pull/23) ([azdlowry](https://github.com/azdlowry))

## [0.2.11](https://github.com/sonos/dinghy/tree/0.2.11) (2017-06-09)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.10...0.2.11)

**Merged pull requests:**

- add DINGHY=1 env var when running on Android [\#22](https://github.com/sonos/dinghy/pull/22) ([fredszaq](https://github.com/fredszaq))

## [0.2.10](https://github.com/sonos/dinghy/tree/0.2.10) (2017-05-29)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.9...0.2.10)

**Merged pull requests:**

- More options [\#21](https://github.com/sonos/dinghy/pull/21) ([klefevre](https://github.com/klefevre))

## [0.2.9](https://github.com/sonos/dinghy/tree/0.2.9) (2017-04-21)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.8...0.2.9)

**Closed issues:**

- make\_linux\_app is creates more and more files each time [\#19](https://github.com/sonos/dinghy/issues/19)

**Merged pull requests:**

- support more android architechtures [\#20](https://github.com/sonos/dinghy/pull/20) ([dten](https://github.com/dten))

## [0.2.8](https://github.com/sonos/dinghy/tree/0.2.8) (2017-04-21)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.7...0.2.8)

## [0.2.7](https://github.com/sonos/dinghy/tree/0.2.7) (2017-04-18)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.6...0.2.7)

**Merged pull requests:**

- Automatic versioning from Cargo.toml [\#18](https://github.com/sonos/dinghy/pull/18) ([klefevre](https://github.com/klefevre))

## [0.2.6](https://github.com/sonos/dinghy/tree/0.2.6) (2017-04-18)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.5...0.2.6)

**Closed issues:**

- consider hiding scp/rsyinc output  [\#16](https://github.com/sonos/dinghy/issues/16)
- paths for .dinghy.toml [\#14](https://github.com/sonos/dinghy/issues/14)

## [0.2.5](https://github.com/sonos/dinghy/tree/0.2.5) (2017-04-15)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.4...0.2.5)

**Closed issues:**

- run cargo examples? [\#11](https://github.com/sonos/dinghy/issues/11)

**Merged pull requests:**

- Workspace test data [\#17](https://github.com/sonos/dinghy/pull/17) ([kali](https://github.com/kali))

## [0.2.4](https://github.com/sonos/dinghy/tree/0.2.4) (2017-04-11)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.3...0.2.4)

**Merged pull requests:**

- Add spec support for build, test and bench subcommands [\#15](https://github.com/sonos/dinghy/pull/15) ([klefevre](https://github.com/klefevre))

## [0.2.3](https://github.com/sonos/dinghy/tree/0.2.3) (2017-03-31)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.2...0.2.3)

**Merged pull requests:**

- fix ssh target triple that was hardcoded instead of read from config [\#12](https://github.com/sonos/dinghy/pull/12) ([fredszaq](https://github.com/fredszaq))

## [0.2.2](https://github.com/sonos/dinghy/tree/0.2.2) (2017-03-31)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.1...0.2.2)

## [0.2.1](https://github.com/sonos/dinghy/tree/0.2.1) (2017-03-29)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.2.0...0.2.1)

**Merged pull requests:**

- chmod from windows needs fixing [\#13](https://github.com/sonos/dinghy/pull/13) ([dten](https://github.com/dten))

## [0.2.0](https://github.com/sonos/dinghy/tree/0.2.0) (2017-02-10)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.13...0.2.0)

**Merged pull requests:**

- Ssh support [\#10](https://github.com/sonos/dinghy/pull/10) ([kali](https://github.com/kali))
- make dinghy work on linux [\#9](https://github.com/sonos/dinghy/pull/9) ([fredszaq](https://github.com/fredszaq))

## [0.1.13](https://github.com/sonos/dinghy/tree/0.1.13) (2017-01-31)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.12...0.1.13)

**Merged pull requests:**

- Support for multiple travis configurations [\#8](https://github.com/sonos/dinghy/pull/8) ([kali](https://github.com/kali))

## [0.1.12](https://github.com/sonos/dinghy/tree/0.1.12) (2017-01-26)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.11...0.1.12)

## [0.1.11](https://github.com/sonos/dinghy/tree/0.1.11) (2017-01-26)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.10...0.1.11)

**Closed issues:**

- unresolved name `ensure_shim` when trying to cargo install [\#5](https://github.com/sonos/dinghy/issues/5)

**Merged pull requests:**

- accept any android device name that isn't whitespace [\#7](https://github.com/sonos/dinghy/pull/7) ([dten](https://github.com/dten))

## [0.1.10](https://github.com/sonos/dinghy/tree/0.1.10) (2017-01-24)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.9...0.1.10)

## [0.1.9](https://github.com/sonos/dinghy/tree/0.1.9) (2017-01-23)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.8...0.1.9)

## [0.1.8](https://github.com/sonos/dinghy/tree/0.1.8) (2017-01-23)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.7...0.1.8)

## [0.1.7](https://github.com/sonos/dinghy/tree/0.1.7) (2017-01-23)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.6...0.1.7)

## [0.1.6](https://github.com/sonos/dinghy/tree/0.1.6) (2017-01-18)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.5...0.1.6)

**Closed issues:**

- Link error when executing 'cargo install dinghy' [\#3](https://github.com/sonos/dinghy/issues/3)

## [0.1.5](https://github.com/sonos/dinghy/tree/0.1.5) (2017-01-03)

[Full Changelog](https://github.com/sonos/dinghy/compare/0.1.4...0.1.5)

## [0.1.4](https://github.com/sonos/dinghy/tree/0.1.4) (2016-12-16)

[Full Changelog](https://github.com/sonos/dinghy/compare/29e2f6c0b21a11575a4af2a9aba993ab3e2bf549...0.1.4)

**Merged pull requests:**

- missing env error message is wrong [\#2](https://github.com/sonos/dinghy/pull/2) ([dten](https://github.com/dten))
- Windows build [\#1](https://github.com/sonos/dinghy/pull/1) ([dten](https://github.com/dten))



\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*
