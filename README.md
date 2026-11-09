# aw-taskwarrior - Windows 11 release (x86_64-pc-windows-gnu)

Binary: `aw-taskwarrior.exe` (console executable, x86-64). Checksum in `SHA256SUMS`.

## Provenance

- Source commit: `17f13cc` on `main` of aw-taskwarrior (built from the clean working tree at that commit; includes REQ-013, REQ-014 and REQ-015).
- Built with: Rust 1.99.0 (isolated rustup install, windows-gnu standard library) and MinGW-w64 GCC 16.2.0, using `scripts/build-windows.sh` (REQ-011).
- Imports: Windows system DLLs only (no MinGW runtime DLLs).
- Tested: not run under Wine or on Windows for this build. The Linux test suite passes on the source commit. Not tested on a real Windows 11 machine.

## Install on Windows 11

1. Put `task.exe` (TaskWarrior 3.x) on `PATH`.
2. Copy `aw-taskwarrior.exe` to a folder on `PATH` or to the folder aw-qt searches.
3. Optional settings: `%LOCALAPPDATA%\activitywatch\aw-taskwarrior\aw-taskwarrior.toml` and `%LOCALAPPDATA%\activitywatch\aw-client\aw-client.toml`. If the daemon's file is missing, it is created on first run with every setting commented out.
4. While no task is active, nothing is sent, so idle time shows as a gap. To send a marker instead, set `no_task_title = "No task tracked"` (or any text) in the daemon's file (REQ-015).
5. Stop the daemon with Ctrl+C.
