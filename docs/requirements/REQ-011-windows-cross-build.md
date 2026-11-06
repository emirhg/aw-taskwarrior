# REQ-011: Cross-compile aw-taskwarrior for Windows 11 from Linux

- **Status:** Implemented
- **Added:** 2026-11-06
- **Owner:** maintainer

## Rationale

The daemon is used on Windows 11 as well as Linux. Building the Windows binary on the Linux development machine avoids a Windows build setup for each change. The build must be reproducible and must not change the Linux toolchain or the Linux build cache.

## Design

### Target

- Rust target triple `x86_64-pc-windows-gnu`, which links with the MinGW-w64 toolchain. Windows 11 runs the result natively.
- Output: `target/windows/x86_64-pc-windows-gnu/release/aw-taskwarrior.exe`.
- A separate target directory (`target/windows`) keeps the Windows build apart from the Linux build cache in `target/`.

### Prerequisites

| Item | Why | Check |
|------|-----|-------|
| `mingw-w64` GCC (`x86_64-w64-mingw32-gcc`) | Linker and C toolchain for the target | `x86_64-w64-mingw32-gcc --version` |
| A Rust toolchain with the `x86_64-pc-windows-gnu` standard library | The target's `std` must be built by the same compiler version | `rustc --print target-libdir --target x86_64-pc-windows-gnu` |
| Network on the first build | Windows-only crates (`windows-sys`, `schannel`, `winreg`, ...) are not in the local cache | - |

The distribution Rust package (Arch `rust`) does not ship the Windows standard library. Use an isolated rustup installation of the same version (1.99.0 at the time of writing), so the system `rustc` is not changed. The toolchain lives outside `PATH`, in `~/.local/share/rust-cross/`.

### Build

- `.cargo/config.toml` sets the linker for this target only: `linker = "x86_64-w64-mingw32-gcc"`.
- `scripts/build-windows.sh` checks the prerequisites, uses the isolated toolchain when `WINDOWS_TOOLCHAIN_BIN` is set, and runs `cargo build --release --target x86_64-pc-windows-gnu --target-dir target/windows -p aw-taskwarrior`.
- The Linux build (`cargo build`, `cargo test`) is unchanged.

### Runtime notes for Windows

- Settings are read from `dirs::config_dir()`, which is `%APPDATA%` on Windows: `%APPDATA%\activitywatch\aw-client\aw-client.toml` and `%APPDATA%\activitywatch\aw-taskwarrior\aw-taskwarrior.toml`. Check that ActivityWatch for Windows uses the same folder for `aw-client.toml`.
- The `task` command must be `task.exe` on `PATH`.
- Ctrl+C stops the daemon. SIGTERM does not exist on Windows.
- aw-qt discovers `aw-*` executables by name. Place `aw-taskwarrior.exe` where aw-qt looks for it, or on `PATH`.

## Verification and limits

Verified on 2026-11-06 with Rust 1.99.0 (isolated rustup install) and MinGW-w64 GCC 16.2.0:
- The release build succeeds. The output is `PE32+ executable ... (console), x86-64`, 4.2 MB.
- The import list has only Windows system DLLs (kernel32, ws2_32, bcrypt, crypt32, advapi32, shell32, ntdll, combase, and the `api-ms-win-*` API sets). No `libgcc` or `libwinpthread`.
- The Linux build still passes its tests (`cargo test --offline`, 31 + 11 + 36 passing, 1 live test ignored) and `cargo fmt --check` is clean.
- Under Wine (scratch prefix, closed ActivityWatch port 5699): the executable reads `%APPDATA%\activitywatch\aw-client\aw-client.toml`, logs `Using task provider 'cli'`, and retries the bucket on the closed port.
- The unit-test binaries for the Windows target build and pass under Wine: watcher 31, CLI provider 11, HTTP provider 36 (1 live test ignored), core 0.

Not verified: a run on a real Windows 11 machine, autostart from aw-qt, and the folder ActivityWatch for Windows uses for `aw-client.toml`.

## Acceptance criteria

- [x] `scripts/build-windows.sh` produces `aw-taskwarrior.exe` on a Linux machine with the prerequisites.
- [x] The Linux build and tests still pass, and the Linux `target/` cache is not invalidated by the Windows build (separate `target/windows`).
- [x] The import list has no MinGW runtime DLLs.
- [x] The Windows build is documented in the README, with the runtime notes above.

## Code references

- Implemented in: `scripts/build-windows.sh`, `.cargo/config.toml` (linker for `x86_64-pc-windows-gnu`), README section "Building for Windows 11", `CLAUDE.md` "Windows build".
