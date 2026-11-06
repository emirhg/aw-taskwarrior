# aw-taskwarrior: Rust Standalone Polling Daemon

A standalone Rust daemon that tracks TaskWarrior activity into ActivityWatch, replacing the Python hook-based approach with a simple polling model.

## Quick Start

**Prerequisites:**
- Rust 1.70+ (install via [rustup.rs](https://rustup.rs/))
- TaskWarrior installed (`task --version`)
- ActivityWatch server running (`aw-server`) — start it before running the daemon
- Git (to clone this repo)

**Build and run:**
```bash
cargo build --release
./target/release/aw-taskwarrior  # Start the daemon, press Ctrl-C to stop
```

The daemon will create an ActivityWatch bucket called `aw-watcher-taskwarrior_{your-hostname}` and send heartbeats tracking your active TaskWarrior tasks.

## Features

- **Continuous polling**: Queries `task rc.hooks:off +ACTIVE export` every 5 seconds (configurable)
- **Dynamic pulsetime**: Online tasks (120s pulsetime) vs. OFFLINE-tagged tasks (12h pulsetime) for gap-tolerant tracking
- **Graceful shutdown**: Responds to SIGINT/SIGTERM with clean shutdown
- **Resilient polling**: Transient task CLI failures fall back to last known task state
- **ActivityWatch ready**: Discoverable by aw-qt via `aw-*` naming convention, ready for manual or automatic start

## Building

```bash
cargo build --release
# Binary location: target/release/aw-taskwarrior (3.6 MB, fully optimized)
```

**Optional: Install to PATH**

For easier access, copy the binary to a directory on your `$PATH`:
```bash
# Option 1: Copy to ~/.local/bin (personal user binaries)
mkdir -p ~/.local/bin
cp target/release/aw-taskwarrior ~/.local/bin/

# Option 2: Copy to /usr/local/bin (system-wide, requires sudo)
sudo cp target/release/aw-taskwarrior /usr/local/bin/

# Option 3: Create a symlink (keeps the binary in the repo)
ln -s $(pwd)/target/release/aw-taskwarrior ~/.local/bin/aw-taskwarrior
```

Then you can run the daemon from anywhere:
```bash
aw-taskwarrior
```

### Building for Windows 11 (cross-compiling from Linux)

The Windows build is made on Linux with the MinGW-w64 toolchain. See [REQ-011](docs/requirements/REQ-011-windows-cross-build.md).

Prerequisites:
- MinGW-w64 GCC (`mingw-w64-gcc`; the `x86_64-w64-mingw32-gcc` command must exist).
- A Rust toolchain with the `x86_64-pc-windows-gnu` standard library. The distribution Rust package may not include it. The build used a separate rustup install of Rust 1.99.0, kept outside `PATH`, in `~/.local/share/rust-cross/`. Add the target with `rustup target add x86_64-pc-windows-gnu --toolchain <version>`.
- Network access for the first build, because the Windows-only crates are downloaded then.

```bash
WINDOWS_TOOLCHAIN_BIN=~/.local/share/rust-cross/rustup/toolchains/1.99.0-x86_64-unknown-linux-gnu/bin \
  scripts/build-windows.sh
# Output: target/windows/x86_64-pc-windows-gnu/release/aw-taskwarrior.exe
```

The script checks the linker and the Windows standard library first. Its build directory (`target/windows`) is separate from the Linux build. The Windows binary imports only Windows system DLLs.

**Running on Windows 11**
- Settings are read from `%APPDATA%\activitywatch\aw-client\aw-client.toml` and `%APPDATA%\activitywatch\aw-taskwarrior\aw-taskwarrior.toml`.
- The `task` command must be `task.exe` on `PATH`.
- Stop the daemon with Ctrl+C.
- aw-qt finds `aw-taskwarrior.exe` by name if it is on `PATH` or in the folder aw-qt searches.

This was tested by building the Windows test binaries and running them under Wine. It has not been run on a real Windows 11 machine.

## Configuration

The daemon works **out of the box with zero configuration**. All settings have sensible hardcoded defaults.

The config follows the ActivityWatch convention: one file per component under `~/.config/activitywatch/`, with a section named after the component.

| File | Purpose |
|------|---------|
| `~/.config/activitywatch/aw-client/aw-client.toml` | ActivityWatch server host and port (`[server]`). Shared with every ActivityWatch client; this daemon only reads it. |
| `~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml` | This daemon's settings (`[aw-taskwarrior]`) |

Both files are optional. Without `aw-client.toml` the server is `127.0.0.1:5600`. The old path `~/.config/aw-taskwarrior/config.toml` is no longer read; move any settings into the new file.

To change the server, edit `aw-client.toml`:

```toml
[server]
hostname = "127.0.0.1"
port = "5600"
```

Daemon settings, in `aw-taskwarrior.toml`:

```toml
[aw-taskwarrior]
# Where the active task comes from. "cli" runs the `task` command (the only provider today).
provider = "cli"

# How often to poll TaskWarrior for active tasks (seconds)
poll_interval_seconds = 5

# TaskWarrior tag that marks a task as "offline-trackable"
# Tasks with this tag tolerate long gaps in tracking (e.g., system suspend)
offline_tag = "OFFLINE"

# Pulsetime (heartbeat merge window in ActivityWatch)
# Online tasks: gap > 120s creates a visible gap in activity
# Offline tasks: gap > 12h creates a visible gap (tolerates suspend/sleep)
online_pulsetime_seconds = 120
offline_pulsetime_seconds = 43200

# Optional: API key if ActivityWatch requires authentication
# api_key = "your-api-key-here"

# Settings for the selected provider
[aw-taskwarrior.providers.cli]
timeout_seconds = 5          # max time for one `task export` call
```

### Complete example

Copy these two files to get a full configuration. Every key is optional: a missing key uses the default shown in the comment. Lines starting with `#` are not active.

`~/.config/activitywatch/aw-client/aw-client.toml` (shared with ActivityWatch; this daemon only reads `[server]`):

```toml
[server]
hostname = "127.0.0.1"   # default
port = "5600"            # default; a string or an integer both work

[client]
#commit_interval = 10    # read by ActivityWatch clients, ignored here
```

`~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml`:

```toml
[aw-taskwarrior]
# Where the active task comes from: "cli" (the `task` command) or "http" (task-rest).
provider = "cli"                  # default

poll_interval_seconds = 5         # default: how often to read the active task
offline_tag = "OFFLINE"           # default: tasks with this tag get the long pulsetime
online_pulsetime_seconds = 120    # default
offline_pulsetime_seconds = 43200 # default (12 h)
#api_key = "your-api-key"         # only if ActivityWatch requires one

# Provider settings. The table for the provider you use is read; the other is ignored.
[aw-taskwarrior.providers.cli]
timeout_seconds = 5               # default: max time for one `task export`

[aw-taskwarrior.providers.http]
#base_url = "http://127.0.0.1:5000"  # required when provider = "http"
#username = "dev"                    # optional: set both or neither (local mode)
#password = "password"
#timeout_seconds = 5
```

To use task-rest instead of the `task` command, change the first line of `[aw-taskwarrior]` to `provider = "http"` and uncomment the `base_url` line (and `username` and `password`, if task-rest runs with authentication). See the next section for details.

### Reading tasks from task-rest (HTTP provider)

To read the active task from the task-rest API instead of the `task` command, set the provider and its settings in `aw-taskwarrior.toml`:

```toml
[aw-taskwarrior]
provider = "http"

[aw-taskwarrior.providers.http]
base_url = "http://127.0.0.1:5000"   # http or https; no user info; a path prefix is allowed
username = "dev"                     # optional; set together with password
password = "password"               # optional; set together with username
timeout_seconds = 5                  # optional, default 5
```

- Without `username` and `password`, the provider talks to a task-rest started with `python -m task_rest.serve --no-auth` (local mode, REQ-012). It never logs in.
- With them, the provider logs in with `GET /auth` and sends the token. Setting only one of the two is an error at startup.
- The password is stored in plain text, like `api_key`. Make the file readable only by your user (`chmod 600` on Linux, or a per-user permission on Windows).
- Each poll asks task-rest for the active tasks only: `GET /?status=pending&start.any=` (REQ-010). The provider still applies its own selection rule to the result.
- The provider needs a running task-rest. If it is unreachable or the login fails, the last known task is reused and the error is logged.

Unknown keys in `[aw-taskwarrior]` are rejected at startup. In particular, `aw_server_host` and `aw_server_port` are no longer settings here; set the server in `aw-client.toml`. A `[aw-taskwarrior-testing]` section is reserved for the future `--testing` mode.

**Default behavior when no task is active:**
- A heartbeat is sent with only the title "No task tracked"
- No project or tags are included (empty values omitted)
- This shows in ActivityWatch that tracking is running but no task is active

## Running

```bash
./target/release/aw-taskwarrior
```

The daemon will:
1. Load config from `aw-client.toml` (server) and `aw-taskwarrior.toml` (daemon), or use built-in defaults
2. Connect to ActivityWatch server (default: `127.0.0.1:5600`)
3. Create/connect to bucket `aw-watcher-taskwarrior_{hostname}` (holds your task tracking data)
4. Enter poll loop:
   - Every 5 seconds: query TaskWarrior for active tasks via `task rc.hooks:off +ACTIVE export`
   - Find the most recently started task
   - Send a heartbeat to ActivityWatch with task details
   - If no task is active, send "No task tracked" indicator
5. Exit cleanly on Ctrl-C or when receiving SIGTERM

**Example output:**
```
[2026-10-01T19:04:57.236951738+00:00] INFO  Starting aw-taskwarrior, bucket=aw-watcher-taskwarrior_HerreraMonroy
[2026-10-01T19:04:57.271857185+00:00] INFO  Bucket aw-watcher-taskwarrior_HerreraMonroy ready
[2026-10-01T19:05:02.123456789+00:00] INFO  Received shutdown signal, exiting cleanly
```

**Running in the background:**
```bash
./target/release/aw-taskwarrior &  # Run in background
# or
nohup ./target/release/aw-taskwarrior > /tmp/aw-taskwarrior.log 2>&1 &
```

## Integration with aw-qt

1. Build the binary and place on `$PATH` (e.g., `~/.local/bin/`)
2. Start aw-qt, which auto-discovers `aw-taskwarrior` via naming convention
3. Start manually from aw-qt's UI, or add to `autostart_modules` in `~/.config/activitywatch/aw-qt/aw-qt.toml`:

```toml
[aw-qt]
autostart_modules = ["aw-server", "aw-watcher-afk", "aw-watcher-window", "aw-taskwarrior"]
```

## Testing

All 70 unit tests pass, run from the repository root:

```bash
cargo test
```

Tests cover:
- Config loading, defaults, TOML overlays, provider selection
- TaskWarrior JSON parsing, task selection by recency, provider settings
- task-rest responses: selection rules on captured responses, token handling, error mapping (run against a loopback mock, with no external network)
- Pulsetime selection (online vs. offline logic)
- Provider selection and the last-known-task fallback

A live test against a running task-rest is ignored by default:

```bash
TASK_REST_URL=http://127.0.0.1:5000 TASK_REST_USER=dev TASK_REST_PASSWORD=... \
  cargo test -p aw-taskwarrior-provider-http -- --ignored --nocapture
```

## Project Structure

Cargo workspace; all commands run from the repository root.

- `crates/aw-taskwarrior-core/` - Shared `ActiveTask`, `TaskProvider` trait, `ProviderError` (no dependencies)
- `crates/aw-taskwarrior-provider-cli/` - CLI provider: runs `task` and parses its JSON
- `crates/aw-taskwarrior-provider-http/` - HTTP provider: reads tasks from the task-rest API
- `crates/aw-taskwarrior/` - The watcher binary:
  - `crates/aw-taskwarrior/src/main.rs` - Poll loop, signal handling, orchestration
  - `crates/aw-taskwarrior/src/config.rs` - Config struct, TOML loading, defaults
  - `crates/aw-taskwarrior/src/providers.rs` - Provider selection and the last-known-task fallback
  - `crates/aw-taskwarrior/src/activitywatch.rs` - AW client wrapper, bucket/heartbeat logic
  - `crates/aw-taskwarrior/src/log.rs` - Simple stdout/stderr logging

## Adding a task provider

1. Write the requirement in `docs/requirements/` and commit it first.
2. Create `crates/aw-taskwarrior-provider-<name>/` implementing `TaskProvider`, with `pub const NAME` and `pub fn create(settings: Option<&toml::Table>)`.
3. Add a feature to `crates/aw-taskwarrior/Cargo.toml` and one `cfg` arm in `crates/aw-taskwarrior/src/providers.rs`.

## Dependencies

- `aw-client-rust` (git, ActivityWatch HTTP client)
- `aw-taskwarrior-core`, `aw-taskwarrior-provider-cli` and `aw-taskwarrior-provider-http` (workspace crates)
- `reqwest` (blocking HTTP client for the task-rest provider, same version as `aw-client-rust`)
- `chrono` (datetime parsing/formatting)
- `serde`/`serde_json` (JSON serialization)
- `toml` (config file parsing)
- `dirs` (XDG config path resolution)
- `ctrlc` (signal handling)
- `gethostname` (bucket naming)

## Troubleshooting

**"Bucket aw-watcher-taskwarrior_... ready" but no heartbeats showing in ActivityWatch**
- Ensure ActivityWatch server is actually running: `curl http://localhost:5600/api/0/info`
- Check that the daemon is polling TaskWarrior: `task +ACTIVE export` should show your active tasks
- Verify the daemon isn't crashing silently by running it in foreground: `./target/release/aw-taskwarrior`

**"task provider 'cli' failed, reusing last known task" in logs**
- TaskWarrior binary not found or not in `$PATH`
- Verify: `which task` and `task --version`
- The daemon will keep using the last known task state, so it's resilient to transient failures

**Config file not loading (using defaults instead)**
- Verify the config file locations: `cat ~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml` and `cat ~/.config/activitywatch/aw-client/aw-client.toml`
- The old `~/.config/aw-taskwarrior/config.toml` is not read any more
- A startup error naming `aw_server_host` or `aw_server_port` means those keys must be removed from `[aw-taskwarrior]`; set the server in `aw-client.toml`
- Check for TOML syntax errors (run `cargo test` in the repo to validate TOML parsing)
- Permission issue: `ls -la ~/.config/aw-taskwarrior/`

**"Fatal: failed to create AW client"**
- ActivityWatch server is not running or not reachable at the configured address
- Start aw-server: `aw-server` in another terminal
- Check network: `ping localhost` and `curl -I http://localhost:5600/api/0/info`

**Daemon exits immediately without error**
- Check if already running: `pgrep -a aw-taskwarrior`
- Kill any existing daemon: `pkill aw-taskwarrior`
- Run in foreground to see error messages: `./target/release/aw-taskwarrior`

## Known Limitations

- No `--testing` flag support (future enhancement: could swap to `_testing` bucket suffix)
- No in-process heartbeat retry backoff (naturally retried on next poll cycle)
- Hostname resolved once at startup; if hostname changes, restart needed

## Design Decisions

- **No Unix socket, no per-task daemons** — Single continuous polling daemon instead of hook-spawned processes
- **Dynamic pulsetime** — Replaces afk-bucket-based gap backfilling; server-side heartbeat merge handles gaps automatically
- **Graceful degradation** — Transient task CLI failures reuse last known state; AW server unreachable retries indefinitely
- **Pure stdout/stderr logging** — No file logging by default, compatible with aw-qt's log retrieval

## Future Work

- [ ] Add `--testing` flag for aw-qt testing mode
- [ ] CLI arg parsing for host/port/api-key override
- [ ] Structured logging with configurable levels
- [ ] Performance metrics/health reporting
- [ ] Event categorization by task type or project
