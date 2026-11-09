# REQ-014: Windows config location and first-run config file

- **Status:** Implemented
- **Added:** 2026-11-08
- **Owner:** maintainer

## Rationale

Two gaps in REQ-009 show up on Windows:

1. **Wrong folder on Windows.** The daemon reads `dirs::config_dir()`, which is `%APPDATA%` (Roaming). The ActivityWatch Python components (`aw-client`, aw-qt) use `platformdirs.user_config_dir()`, which on Windows is `%LOCALAPPDATA%\activitywatch` (Local). Checked in `~/src/activitywatch`: `aw-core/aw_core/dirs.py` and `venv/.../platformdirs/windows.py`. So on Windows the daemon never sees the `aw-client.toml` that the Python clients write, and it falls back to `127.0.0.1:5600`.
2. **No file on first run.** A missing config file is silently treated as all defaults, so users do not know the file exists or what it can hold. `aw-core` (`load_config_toml`) and awatcher both write a default file when none exists.

## Design

### Base directory

| Platform | Base directory | Source |
|----------|----------------|--------|
| Linux | `$XDG_CONFIG_HOME` or `~/.config`, then `activitywatch/` | `dirs::config_dir()` |
| macOS | `~/Library/Application Support/activitywatch/` | `dirs::config_dir()` |
| Windows | `%LOCALAPPDATA%\activitywatch\` | `dirs::data_local_dir()` |

Windows uses the Local folder, not the Roaming one, to match `platformdirs.user_config_dir()`. The Linux and macOS paths do not change. The base directory is the same for `aw-client.toml` and `aw-taskwarrior.toml`.

### First run

When `aw-taskwarrior.toml` does not exist, the daemon writes it before it loads the configuration:

- The contents are `examples/aw-taskwarrior.toml` (embedded at build time with `include_str!`), with every key line commented out (`#` prefix) and the section headers left active. This is the same approach as `aw-core`'s `_comment_out_toml`. The file is valid TOML and loads as the built-in defaults.
- The parent directory is created if it is missing.
- If the file exists, it is never changed.
- `aw-client.toml` is not created. The daemon only reads it, and the other ActivityWatch clients own it.
- If the file cannot be written (for example, a read-only folder), the daemon logs a warning and continues with the built-in defaults. An unreadable or invalid existing file still stops the start, as in REQ-009.

### Modules affected

- `crates/aw-taskwarrior/src/config.rs`: `config_path` / `client_config_path` use the platform base directory; a new function writes the default file when it is missing; `load` calls it before reading.
- `examples/aw-taskwarrior.toml`: the source of the embedded default.
- See [../modules.md](../modules.md) and [../dependencies.md](../dependencies.md). No new dependency: `dirs` is already used.

## Acceptance criteria

- [x] On Linux, the paths are unchanged (`~/.config/activitywatch/...`). Checked by the first-run run with `XDG_CONFIG_HOME`.
- [x] On Windows, both config paths are under `%LOCALAPPDATA%\activitywatch\`. The Windows binary builds; the path choice is covered by a unit test. Not run on a Windows machine.
- [x] With no config file, the daemon creates `aw-taskwarrior.toml` with every key commented out, and the file loads to the built-in defaults (test).
- [x] An existing `aw-taskwarrior.toml` is not modified by startup (test).
- [x] A missing config directory is created before the file is written (test).
- [x] A write failure is returned as an error (test). The startup path that logs it and continues with defaults is checked by reading the code, not by a test.
- [x] `aw-client.toml` is never created by the daemon (code only creates `aw-taskwarrior.toml`).

## Code references

- Implemented in: `crates/aw-taskwarrior/src/config.rs` (`base_config_dir`, `commented_example`, `create_default_file`, `load`); embedded `examples/aw-taskwarrior.toml`.
