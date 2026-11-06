#!/usr/bin/env bash
# Cross-compile aw-taskwarrior for Windows 11 (x86_64-pc-windows-gnu).
# See docs/requirements/REQ-011-windows-cross-build.md.
#
# Environment:
#   WINDOWS_TOOLCHAIN_BIN  directory with a cargo and rustc that have the
#                          x86_64-pc-windows-gnu standard library (optional;
#                          defaults to the cargo and rustc on PATH).
#
# Output: target/windows/x86_64-pc-windows-gnu/release/aw-taskwarrior.exe
set -euo pipefail

TARGET=x86_64-pc-windows-gnu
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    echo "error: $*" >&2
    exit 1
}

# 1. MinGW-w64 C compiler, used as the linker for this target.
if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    fail "x86_64-w64-mingw32-gcc not found. Install the mingw-w64-gcc package."
fi

# 2. Rust toolchain with the Windows standard library.
if [ -n "${WINDOWS_TOOLCHAIN_BIN:-}" ]; then
    export PATH="$WINDOWS_TOOLCHAIN_BIN:$PATH"
fi
if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    fail "cargo and rustc not found. Set WINDOWS_TOOLCHAIN_BIN to a toolchain bin directory."
fi
LIBDIR="$(rustc --print target-libdir --target "$TARGET")"
if ! compgen -G "$LIBDIR/libstd-*.rlib" >/dev/null; then
    fail "no Rust standard library for $TARGET in $LIBDIR. Add it with: rustup target add $TARGET"
fi

# 3. Build from the repository root, into a separate target directory so the
#    Linux build cache in target/ is not touched.
cd "$REPO"
cargo build --release --target "$TARGET" --target-dir target/windows -p aw-taskwarrior "$@"

OUT="target/windows/$TARGET/release/aw-taskwarrior.exe"
echo "built: $REPO/$OUT"
