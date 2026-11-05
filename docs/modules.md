# Modules

The repository is a Cargo workspace (`crates/*`) with three crates. The watcher binary (`aw-taskwarrior`) contains five modules. The provider crate and the core crate hold the task-source logic and the shared interface.

## Crate dependency graph

```mermaid
flowchart TD
    watcher[aw-taskwarrior<br/>watcher binary] --> core[aw-taskwarrior-core]
    watcher -. feature provider-cli, default .-> cli[aw-taskwarrior-provider-cli]
    cli --> core
    watcher -.-> aw[(aw-client-rust, aw-models)]
    cli -.-> tw[(task CLI)]
```

`aw-taskwarrior-core` has no dependencies. The watcher never imports provider internals; it reaches a provider only through `providers.rs` and the `TaskProvider` trait.

## Watcher module graph

```mermaid
flowchart TD
    main[main.rs] --> config[config.rs]
    main --> providers[providers.rs]
    main --> activitywatch[activitywatch.rs]
    main --> log[log.rs]
    providers --> config
    providers --> log
    providers --> core[aw-taskwarrior-core]
    activitywatch --> config
    activitywatch --> log
    activitywatch --> core
```

`config` and `log` do not depend on other project modules.

---

## aw-taskwarrior-core (`crates/aw-taskwarrior-core/src/lib.rs`)

**Responsibility:** the shared model and the interface every provider implements. Zero dependencies.

| Item | Kind | Notes |
|------|------|-------|
| `ActiveTask` | struct | `uuid: Option<String>`, `title`, `project`, `tags`. The task currently being tracked. |
| `ActiveTask::default_task()` | fn | Title "No task tracked", no uuid, empty project and tags |
| `TaskProvider` | trait (`Send`) | `name(&self) -> &'static str`; `active_task(&self) -> Result<ActiveTask, ProviderError>` |
| `ProviderError` | enum | `Unavailable(String)`, `Timeout`, `InvalidResponse(String)`, `InvalidConfig(String)`. Implements `Display` and `Error`. |

- Tests: none (behavior is covered by the watcher and provider tests).

## aw-taskwarrior-provider-cli (`crates/aw-taskwarrior-provider-cli/src/lib.rs`)

**Responsibility:** the CLI provider. Runs `task rc.hooks:off +ACTIVE export` and turns its JSON into one `ActiveTask`.

| Item | Kind | Notes |
|------|------|-------|
| `NAME` | const | `"cli"`. Used in config (`provider = "cli"`) and logs. |
| `create(settings: Option<&toml::Table>)` | fn | Builds the provider from `[providers.cli]`. Key `timeout_seconds`: positive integer, default 5. Any other value gives `InvalidConfig`. |
| `CliProvider` | struct | Holds the timeout. `new(Duration)`. Implements `TaskProvider`. |
| `CliProvider::active_task` (via trait) | fn | Maps `TaskQueryError` to `ProviderError`: `Timeout` to `Timeout`, `InvalidJson` to `InvalidResponse`, `Spawn` and `NonZeroExit` to `Unavailable` |
| `RawTwTask` | struct | JSON shape: `uuid`, `description`, `project`, `tags`, `start` (`YYYYMMDDTHHMMSSZ`) |
| `TaskQueryError` | enum | `Spawn`, `Timeout`, `NonZeroExit`, `InvalidJson`. Internal to the crate. |
| `CliProvider::query()` | fn (private) | Empty output or empty array gives the default task. Otherwise picks the latest task. |
| `run_task_export_with_timeout()` | fn (private) | Spawns `task`, polls for exit every 50 ms, kills it on timeout. See the known issue in [architecture.md](architecture.md). |
| `pick_latest()` | fn (private) | Sorts by parsed `start`, newest first |
| `parse_tw_timestamp()` | fn (private) | Parses the TaskWarrior timestamp format. Invalid values give `None`. |

- Tests (11): empty array, single task, missing project, most recent start wins, unparseable start sorts last, timestamp format, invalid timestamp, and `create` with no settings, a valid timeout, a non-positive timeout, and a non-integer timeout.

---

## aw-taskwarrior (watcher) (`crates/aw-taskwarrior/src/main.rs`)

### main.rs

**Responsibility:** entry point. Loads config, builds the provider, installs the shutdown handler, creates the ActivityWatch client, and runs the poll loop.

- Private functions: `main`, `sleep_with_shutdown_check(Duration, &Arc<AtomicBool>)`.
- Uses: `config::load`, `providers::build`, `providers::resolve_task`, `activitywatch::AwTaskWatcherClient`, `activitywatch::select_pulsetime`, `ActiveTask::default_task`, `log::*`.
- Startup order: config, provider (fails fast, before any network I/O), signal handler, ActivityWatch client, bucket.
- Tests: none.

### config.rs

**Responsibility:** runtime settings with defaults, overlaid by an optional TOML file at `~/.config/aw-taskwarrior/config.toml` (via `dirs::config_dir`).

| Item | Kind | Notes |
|------|------|-------|
| `Config` | struct | Fields: `poll_interval_seconds` (5), `aw_server_host` ("localhost"), `aw_server_port` (5600), `api_key` (None), `offline_tag` ("OFFLINE"), `online_pulsetime_seconds` (120), `offline_pulsetime_seconds` (43200), `provider` ("cli"), `providers` (empty table) |
| `ConfigLoadError` | enum | `Io`, `Parse`. Both are fatal at startup. |
| `config_path()` | fn | Returns the config file path, or None if no config dir exists |
| `load()` | fn | Missing file returns defaults. Present file overlays only the keys it sets. |

- `providers` holds the raw `[providers.<name>]` tables. The watcher does not interpret them; they are passed to the chosen provider's `create()`.
- Tests (6): defaults, partial overlay, full overlay, malformed TOML is a parse error, provider defaults to cli, provider and settings parse.

### providers.rs

**Responsibility:** the only watcher file that knows which providers exist. Selects the provider from config and applies the watcher's failure policy.

| Item | Kind | Notes |
|------|------|-------|
| `build(&Config)` | fn | Matches `cfg.provider`. Each provider is one `cfg(feature)` arm. An unknown or not-compiled name gives `Err("provider '<name>' is not available; rebuild with the matching feature")`. |
| `resolve_task(&dyn TaskProvider, &mut ActiveTask)` | fn | On success, updates `last_known` and returns the task. On error, logs it and returns `last_known`. |

- Tests (5): success updates last known, failure returns last known, failure before any success returns the default, unknown provider is an error, default provider builds `cli`.

### activitywatch.rs

**Responsibility:** ActivityWatch client wrapper: bucket creation, heartbeat construction, and pulsetime choice.

| Item | Kind | Notes |
|------|------|-------|
| `AwTaskWatcherClient` | struct | Holds `AwClient` and the bucket id |
| `new(&Config)` | fn | Bucket id is `aw-watcher-taskwarrior_{hostname}`. Client name "aw-taskwarrior". |
| `bucket_id()` | fn | Returns the bucket id |
| `ensure_bucket_with_shutdown_check(&Arc<AtomicBool>)` | fn | Creates bucket type `task-activity`. Treats "304" or "already exists" as success. Retries every 30 s. |
| `send_heartbeat(&ActiveTask, f64)` | fn | Builds event data with only non-empty fields: `title`, `project`, `tags`, `uuid`. Duration is zero. |
| `select_pulsetime(&ActiveTask, &Config)` | fn | Offline pulsetime if any tag equals `offline_tag`, otherwise online pulsetime |

- Tests (3): offline tag present, absent, custom tag name.

### log.rs

**Responsibility:** timestamped logging to stdout (info) and stderr (error). No levels beyond these two, no files.

| Item | Kind | Notes |
|------|------|-------|
| `log_info(&str)` | fn | `[RFC3339 timestamp] INFO  msg` on stdout |
| `log_error(&str)` | fn | `[RFC3339 timestamp] ERROR msg` on stderr |

- Tests: none.

---

## Test summary

25 unit tests, all run with `cargo test` from the repository root:

| Crate | Tests |
|-------|-------|
| aw-taskwarrior-core | 0 |
| aw-taskwarrior-provider-cli | 11 |
| aw-taskwarrior (watcher) | 14 (config 6, activitywatch 3, providers 5) |
