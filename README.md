# aw-taskwarrior - Windows 11 release (x86_64-pc-windows-gnu)

Binary: `aw-taskwarrior.exe` (console executable, x86-64). Checksum in `SHA256SUMS`.

## Provenance

- Source commit: `197ad7b` on `main` of aw-taskwarrior (built from the working tree at that commit).
- Built with: Rust 1.99.0 (isolated rustup install, windows-gnu standard library) and MinGW-w64 GCC 16.2.0, using `scripts/build-windows.sh` (REQ-011).
- Imports: Windows system DLLs only (no MinGW runtime DLLs).
- Tested: unit tests run under Wine. Not tested on a real Windows 11 machine.

## Install on Windows 11

1. Put `task.exe` (TaskWarrior 3.x) on `PATH`.
2. Copy `aw-taskwarrior.exe` to a folder on `PATH` or to the folder aw-qt searches.
3. Optional settings: `%APPDATA%\activitywatch\aw-taskwarrior\aw-taskwarrior.toml` and `%APPDATA%\activitywatch\aw-client\aw-client.toml`.
4. Stop the daemon with Ctrl+C.
