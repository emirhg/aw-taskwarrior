//! Configuration following the ActivityWatch convention (REQ-009).
//!
//! - Server host and port come from `~/.config/activitywatch/aw-client/aw-client.toml`.
//! - Daemon settings come from
//!   `~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml`, section `[aw-taskwarrior]`.
//! - In testing mode, the `-testing` variant of each section is used instead.
//!
//! Both files are optional. The old path `~/.config/aw-taskwarrior/config.toml` is not read.
//!
//! The base directory is `dirs::config_dir()` on Linux and macOS, and `%LOCALAPPDATA%` on
//! Windows, matching the Python ActivityWatch components (REQ-014). If the daemon's own file
//! does not exist, `load` writes it with every key commented out (REQ-014).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Built-in example file. A missing `aw-taskwarrior.toml` is created from it, with the
/// keys commented out (REQ-014).
const EXAMPLE_CONFIG: &str = include_str!("../../../examples/aw-taskwarrior.toml");

const COMPONENT: &str = "aw-taskwarrior";
const CLIENT_COMPONENT: &str = "aw-client";
const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
const DEFAULT_SERVER_PORT: u16 = 5600;
const DEFAULT_TESTING_SERVER_PORT: u16 = 5666;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub poll_interval_seconds: u64,
    /// ActivityWatch server host. Read from aw-client.toml, not from this daemon's file.
    pub aw_server_host: String,
    /// ActivityWatch server port. Read from aw-client.toml, not from this daemon's file.
    pub aw_server_port: u16,
    pub api_key: Option<String>,
    pub offline_tag: String,
    pub online_pulsetime_seconds: u64,
    pub offline_pulsetime_seconds: u64,
    /// Name of the task provider to use, e.g. "cli".
    pub provider: String,
    /// Per-provider settings, keyed by provider name. Passed opaquely to the provider.
    pub providers: toml::Table,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            poll_interval_seconds: 5,
            aw_server_host: DEFAULT_SERVER_HOST.to_string(),
            aw_server_port: DEFAULT_SERVER_PORT,
            api_key: None,
            offline_tag: "OFFLINE".to_string(),
            online_pulsetime_seconds: 120,
            offline_pulsetime_seconds: 43_200,
            provider: "cli".to_string(),
            providers: toml::Table::new(),
        }
    }
}

/// `aw-client.toml`. Only the server sections are read; other sections are ignored.
#[derive(Debug, Deserialize, Default)]
struct ClientFile {
    server: Option<ServerSection>,
    #[serde(rename = "server-testing")]
    server_testing: Option<ServerSection>,
}

#[derive(Debug, Deserialize, Default)]
struct ServerSection {
    hostname: Option<String>,
    port: Option<PortValue>,
}

/// ActivityWatch writes the port as a string ("5600"); an integer is accepted too.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PortValue {
    Number(u16),
    Text(String),
}

impl PortValue {
    fn to_port(&self) -> Result<u16, String> {
        match self {
            PortValue::Number(n) => Ok(*n),
            PortValue::Text(s) => s.trim().parse::<u16>().map_err(|_| {
                format!(
                    "server port must be a number between 0 and 65535, got \"{}\"",
                    s
                )
            }),
        }
    }
}

/// `aw-taskwarrior.toml`. Sections of other names are ignored.
#[derive(Debug, Deserialize, Default)]
struct OwnFile {
    #[serde(rename = "aw-taskwarrior")]
    main: Option<Section>,
    #[serde(rename = "aw-taskwarrior-testing")]
    testing: Option<Section>,
}

/// One daemon section. Unknown keys are rejected, so a removed setting
/// (such as `aw_server_port`) fails loudly instead of being ignored.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Section {
    poll_interval_seconds: Option<u64>,
    api_key: Option<String>,
    offline_tag: Option<String>,
    online_pulsetime_seconds: Option<u64>,
    offline_pulsetime_seconds: Option<u64>,
    provider: Option<String>,
    providers: Option<toml::Table>,
}

#[derive(Debug)]
pub enum ConfigLoadError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    InvalidValue {
        path: PathBuf,
        message: String,
    },
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLoadError::Io { path, source } => {
                write!(f, "config file {} I/O error: {}", path.display(), source)
            }
            ConfigLoadError::Parse { path, source } => {
                write!(f, "config file {} parse error: {}", path.display(), source)
            }
            ConfigLoadError::InvalidValue { path, message } => {
                write!(f, "config file {}: {}", path.display(), message)
            }
        }
    }
}

/// Base directory for ActivityWatch config files (REQ-014).
///
/// Windows uses the Local app-data folder (`%LOCALAPPDATA%`), like the Python `platformdirs`
/// config path. Other platforms use `dirs::config_dir()` (XDG on Linux).
fn base_config_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        dirs::data_local_dir()
    } else {
        dirs::config_dir()
    }
}

/// `<config dir>/activitywatch/<name>/<name>.toml`
fn component_path(name: &str) -> Option<PathBuf> {
    base_config_dir().map(|d| {
        d.join("activitywatch")
            .join(name)
            .join(format!("{}.toml", name))
    })
}

/// Path of this daemon's config file.
pub fn config_path() -> Option<PathBuf> {
    component_path(COMPONENT)
}

/// Path of the ActivityWatch client config file (the source of the server settings).
pub fn client_config_path() -> Option<PathBuf> {
    component_path(CLIENT_COMPONENT)
}

/// The example file with every key line commented out. Section headers stay active, so the
/// result is valid TOML that loads as the built-in defaults.
fn commented_example() -> String {
    comment_out_keys(EXAMPLE_CONFIG)
}

fn comment_out_keys(text: &str) -> String {
    let mut out: Vec<String> = text
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
                line.to_string()
            } else {
                format!("#{}", line)
            }
        })
        .collect();
    out.push(String::new());
    out.join("\n")
}

/// Writes `contents` to `path` only if no file exists there, creating the parent directory.
/// Returns `Ok(true)` when the file was written. An existing file is never changed.
fn create_default_file(path: &Path, contents: &str) -> std::io::Result<bool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // `create_new` fails if the file exists, so a file written by someone else is kept.
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(contents.as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

/// Loads the configuration. `testing` selects the `-testing` sections.
///
/// If this daemon's config file does not exist, it is created first (REQ-014). A failure to
/// create it is logged and the defaults are used.
pub fn load(testing: bool) -> Result<Config, ConfigLoadError> {
    if let Some(path) = config_path() {
        match create_default_file(&path, &commented_example()) {
            Ok(true) => {
                crate::log::log_info(&format!("Created default config file {}", path.display()))
            }
            Ok(false) => {}
            Err(e) => crate::log::log_error(&format!(
                "Could not create default config file {}: {}",
                path.display(),
                e
            )),
        }
    }
    load_from(
        client_config_path().as_deref(),
        config_path().as_deref(),
        testing,
    )
}

fn load_from(
    client_path: Option<&Path>,
    own_path: Option<&Path>,
    testing: bool,
) -> Result<Config, ConfigLoadError> {
    let mut cfg = Config::default();
    if testing {
        cfg.aw_server_port = DEFAULT_TESTING_SERVER_PORT;
    }

    if let Some((path, contents)) = read_optional(client_path)? {
        apply_client_file(&mut cfg, &path, &contents, testing)?;
    }
    if let Some((path, contents)) = read_optional(own_path)? {
        apply_own_file(&mut cfg, &path, &contents, testing)?;
    }

    Ok(cfg)
}

/// Reads a file. A missing file (or no path) is not an error.
fn read_optional(path: Option<&Path>) -> Result<Option<(PathBuf, String)>, ConfigLoadError> {
    let Some(path) = path else {
        return Ok(None);
    };
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(Some((path.to_path_buf(), contents))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ConfigLoadError::Io {
            path: path.to_path_buf(),
            source: e,
        }),
    }
}

fn apply_client_file(
    cfg: &mut Config,
    path: &Path,
    contents: &str,
    testing: bool,
) -> Result<(), ConfigLoadError> {
    let file: ClientFile = toml::from_str(contents).map_err(|e| ConfigLoadError::Parse {
        path: path.to_path_buf(),
        source: e,
    })?;

    let section = if testing {
        file.server_testing
    } else {
        file.server
    };
    if let Some(server) = section {
        if let Some(host) = server.hostname {
            cfg.aw_server_host = host;
        }
        if let Some(port) = server.port {
            cfg.aw_server_port =
                port.to_port()
                    .map_err(|message| ConfigLoadError::InvalidValue {
                        path: path.to_path_buf(),
                        message,
                    })?;
        }
    }
    Ok(())
}

fn apply_own_file(
    cfg: &mut Config,
    path: &Path,
    contents: &str,
    testing: bool,
) -> Result<(), ConfigLoadError> {
    let file: OwnFile = toml::from_str(contents).map_err(|e| ConfigLoadError::Parse {
        path: path.to_path_buf(),
        source: e,
    })?;

    let section = if testing { file.testing } else { file.main };
    let Some(section) = section else {
        return Ok(());
    };

    if let Some(v) = section.poll_interval_seconds {
        cfg.poll_interval_seconds = v;
    }
    if let Some(v) = section.api_key {
        cfg.api_key = Some(v);
    }
    if let Some(v) = section.offline_tag {
        cfg.offline_tag = v;
    }
    if let Some(v) = section.online_pulsetime_seconds {
        cfg.online_pulsetime_seconds = v;
    }
    if let Some(v) = section.offline_pulsetime_seconds {
        cfg.offline_pulsetime_seconds = v;
    }
    if let Some(v) = section.provider {
        cfg.provider = v;
    }
    if let Some(v) = section.providers {
        cfg.providers = v;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client_path() -> PathBuf {
        PathBuf::from("aw-client.toml")
    }

    fn own_path() -> PathBuf {
        PathBuf::from("aw-taskwarrior.toml")
    }

    fn apply_client(contents: &str, testing: bool) -> Result<Config, ConfigLoadError> {
        let mut cfg = Config::default();
        apply_client_file(&mut cfg, &client_path(), contents, testing)?;
        Ok(cfg)
    }

    fn apply_own(contents: &str, testing: bool) -> Result<Config, ConfigLoadError> {
        let mut cfg = Config::default();
        apply_own_file(&mut cfg, &own_path(), contents, testing)?;
        Ok(cfg)
    }

    /// Writes a unique temp file and returns its path. The caller removes it.
    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aw-taskwarrior-test-{}-{}.toml",
            std::process::id(),
            name
        ));
        std::fs::write(&path, contents).unwrap();
        path
    }

    // ---- defaults ----

    #[test]
    fn test_default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.poll_interval_seconds, 5);
        assert_eq!(cfg.aw_server_host, "127.0.0.1");
        assert_eq!(cfg.aw_server_port, 5600);
        assert_eq!(cfg.api_key, None);
        assert_eq!(cfg.offline_tag, "OFFLINE");
        assert_eq!(cfg.online_pulsetime_seconds, 120);
        assert_eq!(cfg.offline_pulsetime_seconds, 43_200);
    }

    #[test]
    fn test_provider_defaults_to_cli() {
        let cfg = Config::default();
        assert_eq!(cfg.provider, "cli");
        assert!(cfg.providers.is_empty());
    }

    // ---- first run (REQ-014) ----

    /// A unique scratch directory for one test. The caller removes it.
    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aw-taskwarrior-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_commented_example_has_no_active_keys() {
        let text = commented_example();
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with('[') {
                continue;
            }
            panic!("active key line in default config: {line}");
        }
        // Still valid TOML.
        let _: toml::Table = toml::from_str(&text).unwrap();
    }

    #[test]
    fn test_default_file_loads_as_builtin_defaults() {
        let dir = scratch_dir("defaults");
        let path = dir.join("sub").join("aw-taskwarrior.toml");
        assert!(create_default_file(&path, &commented_example()).unwrap());

        let contents = std::fs::read_to_string(&path).unwrap();
        let mut cfg = Config::default();
        apply_own_file(&mut cfg, &path, &contents, false).unwrap();
        let defaults = Config::default();
        assert_eq!(cfg.poll_interval_seconds, defaults.poll_interval_seconds);
        assert_eq!(cfg.offline_tag, defaults.offline_tag);
        assert_eq!(cfg.provider, defaults.provider);
        assert_eq!(
            cfg.online_pulsetime_seconds,
            defaults.online_pulsetime_seconds
        );
        assert_eq!(
            cfg.offline_pulsetime_seconds,
            defaults.offline_pulsetime_seconds
        );
        // The provider table headers stay active, so each provider table exists but is empty.
        assert!(cfg
            .providers
            .values()
            .all(|v| v.as_table().is_some_and(|t| t.is_empty())));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_existing_file_is_not_overwritten() {
        let dir = scratch_dir("keep");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("aw-taskwarrior.toml");
        let mine = "[aw-taskwarrior]\npoll_interval_seconds = 9\n";
        std::fs::write(&path, mine).unwrap();

        assert!(!create_default_file(&path, &commented_example()).unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), mine);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_unwritable_location_is_an_error() {
        // The parent "directory" is a regular file, so creating the file must fail.
        let dir = scratch_dir("blocked");
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("not-a-dir");
        std::fs::write(&blocker, "x").unwrap();
        let path = blocker.join("aw-taskwarrior.toml");

        assert!(create_default_file(&path, &commented_example()).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_base_dir_follows_platform_convention() {
        if cfg!(windows) {
            assert_eq!(base_config_dir(), dirs::data_local_dir());
        } else {
            assert_eq!(base_config_dir(), dirs::config_dir());
        }
    }

    // ---- paths ----

    #[test]
    fn test_config_paths_follow_activitywatch_convention() {
        if let Some(p) = config_path() {
            assert!(p.ends_with("activitywatch/aw-taskwarrior/aw-taskwarrior.toml"));
        }
        if let Some(p) = client_config_path() {
            assert!(p.ends_with("activitywatch/aw-client/aw-client.toml"));
        }
    }

    // ---- aw-client.toml ----

    #[test]
    fn test_client_string_port() {
        let cfg = apply_client(
            "[server]\nhostname = \"10.0.0.5\"\nport = \"5699\"\n",
            false,
        )
        .unwrap();
        assert_eq!(cfg.aw_server_host, "10.0.0.5");
        assert_eq!(cfg.aw_server_port, 5699);
    }

    #[test]
    fn test_client_integer_port() {
        let cfg = apply_client("[server]\nport = 5700\n", false).unwrap();
        assert_eq!(cfg.aw_server_port, 5700);
        assert_eq!(cfg.aw_server_host, "127.0.0.1"); // unchanged
    }

    #[test]
    fn test_client_missing_keys_keep_defaults() {
        let cfg = apply_client("[server]\n#port = \"5600\"\n", false).unwrap();
        assert_eq!(cfg.aw_server_host, "127.0.0.1");
        assert_eq!(cfg.aw_server_port, 5600);
    }

    #[test]
    fn test_client_testing_section_used_only_in_testing_mode() {
        let toml_str = "[server]\nport = \"5600\"\n[server-testing]\nport = \"5777\"\n";
        assert_eq!(apply_client(toml_str, false).unwrap().aw_server_port, 5600);
        assert_eq!(apply_client(toml_str, true).unwrap().aw_server_port, 5777);
    }

    #[test]
    fn test_client_other_sections_ignored() {
        let cfg = apply_client("[client]\ncommit_interval = 10\n", false).unwrap();
        assert_eq!(cfg.aw_server_port, 5600);
    }

    #[test]
    fn test_client_non_numeric_port_is_invalid_value() {
        let err = apply_client("[server]\nport = \"abc\"\n", false).unwrap_err();
        assert!(matches!(err, ConfigLoadError::InvalidValue { .. }));
        assert!(err.to_string().contains("aw-client.toml"));
    }

    #[test]
    fn test_client_malformed_is_parse_error_naming_file() {
        let err = apply_client("[server\nport = ", false).unwrap_err();
        assert!(matches!(err, ConfigLoadError::Parse { .. }));
        assert!(err.to_string().contains("aw-client.toml"));
    }

    // ---- aw-taskwarrior.toml ----

    #[test]
    fn test_own_section_full() {
        let toml_str = r#"
[aw-taskwarrior]
poll_interval_seconds = 3
api_key = "secret"
offline_tag = "AWAY"
online_pulsetime_seconds = 60
offline_pulsetime_seconds = 86400
provider = "cli"
"#;
        let cfg = apply_own(toml_str, false).unwrap();
        assert_eq!(cfg.poll_interval_seconds, 3);
        assert_eq!(cfg.api_key.as_deref(), Some("secret"));
        assert_eq!(cfg.offline_tag, "AWAY");
        assert_eq!(cfg.online_pulsetime_seconds, 60);
        assert_eq!(cfg.offline_pulsetime_seconds, 86400);
        assert_eq!(cfg.provider, "cli");
    }

    #[test]
    fn test_own_partial_keeps_defaults() {
        let cfg = apply_own("[aw-taskwarrior]\npoll_interval_seconds = 10\n", false).unwrap();
        assert_eq!(cfg.poll_interval_seconds, 10);
        assert_eq!(cfg.offline_tag, "OFFLINE"); // unchanged
    }

    #[test]
    fn test_own_nested_provider_settings() {
        let toml_str = r#"
[aw-taskwarrior]
provider = "cli"

[aw-taskwarrior.providers.cli]
timeout_seconds = 9
"#;
        let cfg = apply_own(toml_str, false).unwrap();
        let cli = cfg.providers.get("cli").and_then(|v| v.as_table()).unwrap();
        assert_eq!(
            cli.get("timeout_seconds").and_then(|v| v.as_integer()),
            Some(9)
        );
    }

    #[test]
    fn test_own_removed_server_key_is_rejected_and_named() {
        let err = apply_own("[aw-taskwarrior]\naw_server_port = 5600\n", false).unwrap_err();
        assert!(matches!(err, ConfigLoadError::Parse { .. }));
        assert!(err.to_string().contains("aw_server_port"));
    }

    #[test]
    fn test_own_testing_section_selected_in_testing_mode() {
        let toml_str = r#"
[aw-taskwarrior]
poll_interval_seconds = 5

[aw-taskwarrior-testing]
poll_interval_seconds = 1
"#;
        assert_eq!(apply_own(toml_str, false).unwrap().poll_interval_seconds, 5);
        assert_eq!(apply_own(toml_str, true).unwrap().poll_interval_seconds, 1);
    }

    #[test]
    fn test_own_other_sections_ignored() {
        let cfg = apply_own("[something-else]\nfoo = 1\n", false).unwrap();
        assert_eq!(cfg.poll_interval_seconds, 5);
    }

    #[test]
    fn test_own_wrong_type_is_parse_error() {
        let err =
            apply_own("[aw-taskwarrior]\npoll_interval_seconds = \"x\"\n", false).unwrap_err();
        assert!(matches!(err, ConfigLoadError::Parse { .. }));
    }

    // ---- load_from (files) ----

    #[test]
    fn test_load_from_no_files_gives_defaults() {
        let cfg = load_from(None, None, false).unwrap();
        assert_eq!(cfg.aw_server_host, "127.0.0.1");
        assert_eq!(cfg.aw_server_port, 5600);
        assert_eq!(cfg.provider, "cli");
    }

    #[test]
    fn test_load_from_missing_files_give_defaults() {
        let missing = std::env::temp_dir().join("aw-taskwarrior-test-does-not-exist.toml");
        let cfg = load_from(Some(&missing), Some(&missing), false).unwrap();
        assert_eq!(cfg.aw_server_port, 5600);
    }

    #[test]
    fn test_load_from_testing_mode_defaults_to_testing_port() {
        let cfg = load_from(None, None, true).unwrap();
        assert_eq!(cfg.aw_server_port, 5666);
    }

    #[test]
    fn test_load_from_reads_both_files() {
        let client = temp_file("both-client", "[server]\nport = \"5699\"\n");
        let own = temp_file("both-own", "[aw-taskwarrior]\npoll_interval_seconds = 2\n");

        let result = load_from(Some(&client), Some(&own), false);
        let _ = std::fs::remove_file(&client);
        let _ = std::fs::remove_file(&own);

        let cfg = result.unwrap();
        assert_eq!(cfg.aw_server_port, 5699);
        assert_eq!(cfg.poll_interval_seconds, 2);
    }

    #[test]
    fn test_load_from_malformed_file_names_path() {
        let own = temp_file("malformed-own", "[aw-taskwarrior\n");

        let result = load_from(None, Some(&own), false);
        let _ = std::fs::remove_file(&own);

        let err = result.unwrap_err();
        assert!(err.to_string().contains("malformed-own"));
    }
}
