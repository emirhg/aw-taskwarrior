# aw-taskwarrior - Linux release (x86_64-unknown-linux-gnu)

Binary: `aw-taskwarrior` (x86-64 ELF, dynamically linked). Checksum in `SHA256SUMS`.

## Provenance

- Source commit: `3f612e2` on `main` of aw-taskwarrior (built with `cargo build --release` at that commit, release profile with LTO).
- Built with: Rust 1.99.0 (Arch Linux package), glibc 2.44.
- Shared libraries: `libc.so.6`, `libgcc_s.so.1` (both present on standard Linux distributions).
- Tested: unit tests pass, and a startup smoke test with a closed ActivityWatch port.

## Install

1. Put `task` (TaskWarrior 3.x) on `PATH`.
2. Copy `aw-taskwarrior` to a folder on `PATH`, for example `~/.local/bin`, or to the folder aw-qt searches.
3. Optional settings: `~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml` and `~/.config/activitywatch/aw-client/aw-client.toml`. See the README of the repository for a full example.
4. Stop the daemon with Ctrl+C or SIGTERM.
