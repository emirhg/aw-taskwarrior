//! Selects the task provider from config and applies the watcher's failure policy.
//!
//! This is the only file in the watcher that knows which providers exist.
//! Adding a provider means a new crate, a new feature, and one `cfg` arm in `build`.

use crate::config::Config;
use crate::log;
use aw_taskwarrior_core::{ActiveTask, TaskProvider};

/// Builds the provider named by `cfg.provider`.
///
/// Returns an error when the name is unknown or its feature was not compiled in.
pub fn build(cfg: &Config) -> Result<Box<dyn TaskProvider>, String> {
    match cfg.provider.as_str() {
        #[cfg(feature = "provider-cli")]
        aw_taskwarrior_provider_cli::NAME => {
            let settings = cfg
                .providers
                .get(aw_taskwarrior_provider_cli::NAME)
                .and_then(|v| v.as_table());
            aw_taskwarrior_provider_cli::create(settings).map_err(|e| e.to_string())
        }
        #[cfg(feature = "provider-http")]
        aw_taskwarrior_provider_http::NAME => {
            let settings = cfg
                .providers
                .get(aw_taskwarrior_provider_http::NAME)
                .and_then(|v| v.as_table());
            aw_taskwarrior_provider_http::create(settings).map_err(|e| e.to_string())
        }
        other => Err(format!(
            "provider '{}' is not available; rebuild with the matching feature",
            other
        )),
    }
}

/// Returns the task from `provider`. On failure, logs the error and returns `last_known`.
/// A successful read replaces `last_known`.
pub fn resolve_task(provider: &dyn TaskProvider, last_known: &mut ActiveTask) -> ActiveTask {
    match provider.active_task() {
        Ok(task) => {
            *last_known = task.clone();
            task
        }
        Err(e) => {
            log::log_error(&format!(
                "task provider '{}' failed, reusing last known task: {}",
                provider.name(),
                e
            ));
            last_known.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aw_taskwarrior_core::ProviderError;

    /// Test provider: returns the task it holds, or an error when it holds none.
    struct Stub {
        task: Option<ActiveTask>,
    }

    impl TaskProvider for Stub {
        fn name(&self) -> &'static str {
            "stub"
        }

        fn active_task(&self) -> Result<ActiveTask, ProviderError> {
            self.task
                .clone()
                .ok_or_else(|| ProviderError::Unavailable("stub is down".to_string()))
        }
    }

    fn task(title: &str) -> ActiveTask {
        ActiveTask {
            uuid: Some(format!("uuid-{}", title)),
            title: title.to_string(),
            project: String::new(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn test_success_returns_task_and_updates_last_known() {
        let stub = Stub {
            task: Some(task("fresh")),
        };
        let mut last = ActiveTask::default_task();

        let result = resolve_task(&stub, &mut last);

        assert_eq!(result, task("fresh"));
        assert_eq!(last, task("fresh"));
    }

    #[test]
    fn test_failure_returns_last_known_task() {
        let stub = Stub { task: None };
        let mut last = task("previous");

        let result = resolve_task(&stub, &mut last);

        assert_eq!(result, task("previous"));
        assert_eq!(last, task("previous"));
    }

    #[test]
    fn test_failure_before_any_success_returns_default() {
        let stub = Stub { task: None };
        let mut last = ActiveTask::default_task();

        let result = resolve_task(&stub, &mut last);

        assert_eq!(result, ActiveTask::default_task());
    }

    #[test]
    fn test_unknown_provider_is_error() {
        let cfg = Config {
            provider: "no-such-provider".to_string(),
            ..Config::default()
        };

        let err = build(&cfg).err().expect("unknown provider must fail");

        assert!(err.contains("'no-such-provider' is not available"));
    }

    #[cfg(feature = "provider-http")]
    #[test]
    fn test_http_provider_without_settings_names_base_url() {
        let cfg = Config {
            provider: "http".to_string(),
            ..Config::default()
        };

        let err = build(&cfg).err().expect("http provider needs settings");

        assert!(err.contains("base_url"));
    }

    #[cfg(feature = "provider-cli")]
    #[test]
    fn test_default_provider_builds_cli() {
        let cfg = Config::default();

        let provider = build(&cfg).expect("cli provider must build by default");

        assert_eq!(provider.name(), "cli");
    }
}
