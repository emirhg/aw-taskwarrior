use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub poll_interval_seconds: u64,
    pub aw_server_host: String,
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
            aw_server_host: "localhost".to_string(),
            aw_server_port: 5600,
            api_key: None,
            offline_tag: "OFFLINE".to_string(),
            online_pulsetime_seconds: 120,
            offline_pulsetime_seconds: 43_200,
            provider: "cli".to_string(),
            providers: toml::Table::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    poll_interval_seconds: Option<u64>,
    aw_server_host: Option<String>,
    aw_server_port: Option<u16>,
    api_key: Option<String>,
    offline_tag: Option<String>,
    online_pulsetime_seconds: Option<u64>,
    offline_pulsetime_seconds: Option<u64>,
    provider: Option<String>,
    providers: Option<toml::Table>,
}

#[derive(Debug)]
pub enum ConfigLoadError {
    Io(std::io::Error),
    Parse(toml::de::Error),
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLoadError::Io(e) => write!(f, "config file I/O error: {}", e),
            ConfigLoadError::Parse(e) => write!(f, "config file parse error: {}", e),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("aw-taskwarrior").join("config.toml"))
}

pub fn load() -> Result<Config, ConfigLoadError> {
    let mut cfg = Config::default();
    let Some(path) = config_path() else {
        return Ok(cfg);
    };

    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(cfg);
        }
        Err(e) => return Err(ConfigLoadError::Io(e)),
    };

    let file: ConfigFile = toml::from_str(&contents).map_err(ConfigLoadError::Parse)?;

    if let Some(v) = file.poll_interval_seconds {
        cfg.poll_interval_seconds = v;
    }
    if let Some(v) = file.aw_server_host {
        cfg.aw_server_host = v;
    }
    if let Some(v) = file.aw_server_port {
        cfg.aw_server_port = v;
    }
    if let Some(v) = file.api_key {
        cfg.api_key = Some(v);
    }
    if let Some(v) = file.offline_tag {
        cfg.offline_tag = v;
    }
    if let Some(v) = file.online_pulsetime_seconds {
        cfg.online_pulsetime_seconds = v;
    }
    if let Some(v) = file.offline_pulsetime_seconds {
        cfg.offline_pulsetime_seconds = v;
    }
    if let Some(v) = file.provider {
        cfg.provider = v;
    }
    if let Some(v) = file.providers {
        cfg.providers = v;
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.poll_interval_seconds, 5);
        assert_eq!(cfg.aw_server_host, "localhost");
        assert_eq!(cfg.aw_server_port, 5600);
        assert_eq!(cfg.api_key, None);
        assert_eq!(cfg.offline_tag, "OFFLINE");
        assert_eq!(cfg.online_pulsetime_seconds, 120);
        assert_eq!(cfg.offline_pulsetime_seconds, 43_200);
    }

    #[test]
    fn test_toml_overlay_partial() {
        let toml_str = r#"
poll_interval_seconds = 10
"#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        let mut cfg = Config::default();

        if let Some(v) = file.poll_interval_seconds {
            cfg.poll_interval_seconds = v;
        }

        assert_eq!(cfg.poll_interval_seconds, 10);
        assert_eq!(cfg.aw_server_host, "localhost"); // unchanged
    }

    #[test]
    fn test_toml_overlay_full() {
        let toml_str = r#"
poll_interval_seconds = 3
aw_server_host = "192.168.1.100"
aw_server_port = 5666
offline_tag = "AWAY"
online_pulsetime_seconds = 60
offline_pulsetime_seconds = 86400
"#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        let mut cfg = Config::default();

        if let Some(v) = file.poll_interval_seconds {
            cfg.poll_interval_seconds = v;
        }
        if let Some(v) = file.aw_server_host {
            cfg.aw_server_host = v;
        }
        if let Some(v) = file.aw_server_port {
            cfg.aw_server_port = v;
        }
        if let Some(v) = file.offline_tag {
            cfg.offline_tag = v;
        }
        if let Some(v) = file.online_pulsetime_seconds {
            cfg.online_pulsetime_seconds = v;
        }
        if let Some(v) = file.offline_pulsetime_seconds {
            cfg.offline_pulsetime_seconds = v;
        }

        assert_eq!(cfg.poll_interval_seconds, 3);
        assert_eq!(cfg.aw_server_host, "192.168.1.100");
        assert_eq!(cfg.aw_server_port, 5666);
        assert_eq!(cfg.offline_tag, "AWAY");
        assert_eq!(cfg.online_pulsetime_seconds, 60);
        assert_eq!(cfg.offline_pulsetime_seconds, 86400);
    }

    #[test]
    fn test_malformed_toml_is_parse_error() {
        let toml_str = r#"
poll_interval_seconds = "not-a-number"
"#;
        let result: Result<ConfigFile, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_defaults_to_cli() {
        let cfg = Config::default();
        assert_eq!(cfg.provider, "cli");
        assert!(cfg.providers.is_empty());
    }

    #[test]
    fn test_provider_and_settings_parse() {
        let toml_str = r#"
provider = "http"

[providers.cli]
timeout_seconds = 9
"#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.provider.as_deref(), Some("http"));
        let providers = file.providers.unwrap();
        let cli = providers.get("cli").and_then(|v| v.as_table()).unwrap();
        assert_eq!(
            cli.get("timeout_seconds").and_then(|v| v.as_integer()),
            Some(9)
        );
    }
}
