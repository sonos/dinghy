# AGENTS.md — dinghy contributor guide

IMPORTANT: Read this file in full before starting any work on the project.

dinghy cross-builds, tests and runs Rust on remote devices, emulators and phones.

## Overview
- **`cargo-dinghy/`** — the CLI (`cargo dinghy …`). Parses args, selects a platform +
  device, and dispatches: `runner` (ship a prebuilt exe to a device and run it), cargo
  passthrough (`build`/`test`/…), `run-with`, and the `devices` / `all-devices` /
  `all-platforms` listings.
- **`dinghy-lib/`** — the engine. Key modules:
  - `config` — parses `.dinghy.toml` (platforms, ssh/script devices, test data).
  - `lib` — `Dinghy::probe` assembles platforms + devices from the managers; the core
    `Platform` / `Device` / `PlatformManager` traits and `Build` / `Runnable` / `SetupArgs`.
  - `platform/regular_platform` — cross-toolchain platform: sets up CC/linker/sysroot,
    strips, cross-builds.
  - `toolchain` — resolves a cross toolchain into CC/AR/linker/pkg-config/shim env.
  - `host` — the native (host) platform.
  - `android` — NDK platform + `adb` device transport.
  - `apple` — iOS/tvOS/watchOS devices + simulators (macOS only), via xcode.
  - `ssh` — remote-board transport (rsync + ssh).
  - `script` — device driven by a user-supplied script.
  - `plugin` — auto-detects `cargo-dinghy-*` executables as device/platform providers.
  - `overlay` — pkg-config overlays (extra libraries onto the sysroot).
  - `project` — cargo-metadata wrapper (target dir, workspace root).
- **`dinghy-build/`** — helpers for a cross-built crate's own `build.rs` (env/sysroot).
- **`dinghy-test/`** — test-time helpers (locate test data) for crates using dinghy.
- **`test-ws/`** — fixture workspace exercised by dinghy's tests.
- **`legacy/`** — deprecated stub; use `cargo-dinghy`.

## Contributing

### Before you commit
- Format with `cargo fmt --all`. The repo has a `rust-toolchain` file, so bare
  `cargo fmt` uses the toolchain CI checks against — don't override it.
- `cargo clippy --workspace` — don't add new warnings.

### Commit messages
- One short paragraph: what was wrong + the fix. Nothing else.
- No consequence chains ("X broke Y broke Z"), no "Result:/Symptom:" sections,
  no bullet lists of every place the bug surfaced.

### Inline comments
- Default to NONE. Names carry the meaning. A comment signals a hidden
  constraint / invariant / workaround — not narration.
- Never describe the diff or history ("used to be X", "previously…"). Comments
  describe current code only.
- No section-banner comments; split into functions instead.

### Doc comments (`///` / `//!`)
- DO add a concise one on public / non-trivial items — public fns, traits,
  config structs. State what it is, its contract, and valid inputs.
- Same anti-narration rule: document the current contract, not benchmarks,
  perf numbers, issue numbers, or history ("Measured on…", "Regression:…").

### Pull requests
- Open with a 1–2 sentence summary of what and why.
- Follow-up questions and review replies are handled by a HUMAN, not the bot.
  The maintainer wants to talk to the author, not prompt an LLM.
