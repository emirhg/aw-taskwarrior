# Dependencies

The workspace declares shared versions in the root `Cargo.toml` under `[workspace.dependencies]`. Each crate refers to them with `workspace = true`.

## Crate dependencies by crate

| Crate | Dependency | Version / source | Purpose | Used by |
|-------|------------|------------------|---------|---------|
| aw-taskwarrior-core | (none) | | Zero dependencies by design | |
| aw-taskwarrior-provider-cli | aw-taskwarrior-core | path | `ActiveTask`, `TaskProvider`, `ProviderError` | `lib.rs` |
| aw-taskwarrior-provider-cli | chrono | 0.4 (serde) | Parse TaskWarrior timestamps | `lib.rs` |
| aw-taskwarrior-provider-cli | serde | 1 (derive) | Derive for the `task` export JSON struct | `lib.rs` |
| aw-taskwarrior-provider-cli | serde_json | 1 | Parse `task` export output | `lib.rs` |
| aw-taskwarrior-provider-cli | toml | 0.8 | `toml::Table` settings for `create()` | `lib.rs` |
| aw-taskwarrior-provider-http | aw-taskwarrior-core | path | `ActiveTask`, `TaskProvider`, `ProviderError` | `lib.rs` |
| aw-taskwarrior-provider-http | reqwest | 0.11 (`blocking`, `json`, `rustls-tls-native-roots`; no default features) | HTTP client for task-rest. Same version and features as `aw-client-rust`, so no new crate is compiled. | `lib.rs` |
| aw-taskwarrior-provider-http | chrono | 0.4 (serde) | Parse RFC 2822 dates from task-rest | `lib.rs` |
| aw-taskwarrior-provider-http | serde / serde_json | 1 | Read task-rest JSON | `lib.rs` |
| aw-taskwarrior-provider-http | toml | 0.8 | `toml::Table` settings for `create()` | `lib.rs` |
| aw-taskwarrior (watcher) | aw-taskwarrior-core | path | Shared model and trait | `activitywatch.rs`, `main.rs`, `providers.rs` |
| aw-taskwarrior (watcher) | aw-taskwarrior-provider-http | path, optional (feature `provider-http`) | HTTP provider for task-rest | `providers.rs` |
| aw-taskwarrior (watcher) | aw-taskwarrior-provider-cli | path, optional (feature `provider-cli`) | CLI provider | `providers.rs` |
| aw-taskwarrior (watcher) | aw-client-rust | git `ActivityWatch/aw-server-rust` rev `2875df5` | ActivityWatch HTTP client (blocking) | `activitywatch.rs` |
| aw-taskwarrior (watcher) | aw-models | git `ActivityWatch/aw-server-rust` rev `2875df5` | `Event` model | `activitywatch.rs` |
| aw-taskwarrior (watcher) | serde | 1 (derive) | Config struct derives | `config.rs` |
| aw-taskwarrior (watcher) | serde_json | 1 | Heartbeat data | `activitywatch.rs` |
| aw-taskwarrior (watcher) | toml | 0.8 | Config file parsing, provider settings table | `config.rs` |
| aw-taskwarrior (watcher) | dirs | 5 | Locate the config directory | `config.rs` |
| aw-taskwarrior (watcher) | ctrlc | 3 | Ctrl-C and SIGTERM handler | `main.rs` |
| aw-taskwarrior (watcher) | chrono | 0.4 (serde) | Timestamps | `activitywatch.rs`, `log.rs` |
| aw-taskwarrior (watcher) | gethostname | 0.4 | Hostname for the bucket id | `activitywatch.rs` |

Release profile (root `Cargo.toml`, applies to the whole workspace): `opt-level = 3`, `lto = true`, `codegen-units = 1`.

The git dependencies are pinned to one commit. Updating `rev` changes the ActivityWatch API surface and needs a code check.

## Feature flags

| Crate | Feature | Default | Enables |
|-------|---------|---------|---------|
| aw-taskwarrior | `provider-cli` | yes | `aw-taskwarrior-provider-cli`; selectable as `provider = "cli"` |
| aw-taskwarrior | `provider-http` | yes | `aw-taskwarrior-provider-http`; selectable as `provider = "http"` |
| aw-taskwarrior | `provider-lib` | planned (REQ-008) | `aw-taskwarrior-provider-lib` |

Build without providers (for testing the plumbing): `cargo check -p aw-taskwarrior --no-default-features`. The binary then fails at startup with the "not available" message for any provider name.

## Dependency graph

```mermaid
flowchart LR
    subgraph crates[external crates]
        awc[aw-client-rust]
        awm[aw-models]
        serde[serde / serde_json]
        toml[toml]
        dirs[dirs]
        ctrlc[ctrlc]
        chrono[chrono]
        gh[gethostname]
    end
    core[aw-taskwarrior-core<br/>no dependencies]
    cli[aw-taskwarrior-provider-cli] --> core
    cli --> serde
    cli --> toml
    cli --> chrono
    http[aw-taskwarrior-provider-http] --> core
    http --> reqwest[reqwest blocking]
    http --> serde
    http --> chrono
    watcher[aw-taskwarrior] --> core
    watcher -. provider-cli .-> cli
    watcher -. provider-http .-> http
    watcher --> ctrlc
    watcher --> dirs
    watcher --> toml
    watcher --> awc
    watcher --> awm
    watcher --> serde
    watcher --> chrono
    watcher --> gh
```

## Build-time dependencies

| Dependency | Required for | Notes |
|------------|--------------|-------|
| Rust 1.99.0 (Arch `rust` package used for Linux builds) | All builds | Same compiler version for the isolated Windows toolchain |
| `x86_64-pc-windows-gnu` Rust standard library | Windows build (`scripts/build-windows.sh`) | Not in the Arch package. Use an isolated rustup install (kept outside `PATH`, in `~/.local/share/rust-cross/`). |
| `mingw-w64-gcc` (`x86_64-w64-mingw32-gcc`) | Windows build | Linker for `x86_64-pc-windows-gnu`, set in `.cargo/config.toml` |
| Network on the first Windows build | Windows build | Windows-only crates (`windows-sys`, `schannel`, `winreg`, ...) are not in the local cache |

The Windows binary imports only Windows system DLLs. No MinGW runtime DLLs are needed on the target machine.

## Runtime dependencies

| Dependency | Required | Used for | Failure behavior |
|------------|----------|----------|------------------|
| `task` (TaskWarrior CLI) on `PATH` | Yes, when provider is `cli` | Active-task query, every poll | Logged; last known task is reused |
| `aw-server` at the host and port from `aw-client.toml` (default `127.0.0.1:5600`) | Yes (retried) | Bucket creation, heartbeats | Startup retries every 30 s; heartbeat errors are logged |
| `~/.config/activitywatch/aw-client/aw-client.toml` | No | Single source of truth for the server host and port (`[server]`). Read-only. | Missing: defaults. Malformed: fatal. |
| `~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml` | No | Daemon settings (`[aw-taskwarrior]`), provider selection | Missing: defaults. Malformed or unknown key: fatal. |
| task-rest API (`base_url`, e.g. `http://127.0.0.1:5000`) | Yes, when provider is `http` | Active-task query, every poll: `GET /?status=pending&start.any=`, preceded by `GET /auth` when `username`/`password` are set | Logged; last known task is reused. Wrong password: `authentication rejected`. No credentials against an authenticated server: `server requires authentication`. |
| aw-qt | No | Optional discovery and autostart | None |
