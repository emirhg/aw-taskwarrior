//! CLI task provider: reads the active task by running `task rc.hooks:off +ACTIVE export`.

use aw_taskwarrior_core::{ActiveTask, ProviderError, TaskProvider};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Provider name used in config (`provider = "cli"`) and logs.
pub const NAME: &str = "cli";

/// Default time allowed for one `task export` call.
const DEFAULT_TIMEOUT_SECONDS: u64 = 5;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawTwTask {
    pub uuid: String,
    pub description: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub start: String, // "20260723T200958Z"
}

#[derive(Debug)]
pub enum TaskQueryError {
    Spawn(std::io::Error),
    Timeout,
    NonZeroExit { code: Option<i32>, stderr: String },
    InvalidJson(serde_json::Error),
}

impl std::fmt::Display for TaskQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskQueryError::Spawn(e) => write!(f, "failed to spawn task command: {}", e),
            TaskQueryError::Timeout => write!(f, "task command timed out"),
            TaskQueryError::NonZeroExit { code, stderr } => {
                write!(
                    f,
                    "task command exited with status {}: {}",
                    code.map(|c| c.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    if stderr.is_empty() {
                        "(no stderr)"
                    } else {
                        stderr
                    }
                )
            }
            TaskQueryError::InvalidJson(e) => write!(f, "failed to parse task JSON: {}", e),
        }
    }
}

/// Reads the active task by running `task` through this provider.
pub struct CliProvider {
    timeout: Duration,
}

impl CliProvider {
    pub fn new(timeout: Duration) -> Self {
        CliProvider { timeout }
    }

    fn query(&self) -> Result<ActiveTask, TaskQueryError> {
        let output = run_task_export_with_timeout(self.timeout)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Ok(ActiveTask::default_task());
        }

        let tasks: Vec<RawTwTask> =
            serde_json::from_str(trimmed).map_err(TaskQueryError::InvalidJson)?;

        if tasks.is_empty() {
            return Ok(ActiveTask::default_task());
        }

        Ok(pick_latest(tasks))
    }
}

impl TaskProvider for CliProvider {
    fn name(&self) -> &'static str {
        NAME
    }

    fn active_task(&self) -> Result<ActiveTask, ProviderError> {
        self.query().map_err(|e| {
            let msg = e.to_string();
            match e {
                TaskQueryError::Timeout => ProviderError::Timeout,
                TaskQueryError::InvalidJson(_) => ProviderError::InvalidResponse(msg),
                TaskQueryError::Spawn(_) | TaskQueryError::NonZeroExit { .. } => {
                    ProviderError::Unavailable(msg)
                }
            }
        })
    }
}

/// Builds the CLI provider from `[aw-taskwarrior.providers.cli]` settings.
/// Supported key: `timeout_seconds` (positive integer, default 5).
pub fn create(settings: Option<&toml::Table>) -> Result<Box<dyn TaskProvider>, ProviderError> {
    let timeout_seconds = match settings.and_then(|t| t.get("timeout_seconds")) {
        None => DEFAULT_TIMEOUT_SECONDS,
        Some(toml::Value::Integer(n)) if *n >= 1 => *n as u64,
        Some(other) => {
            return Err(ProviderError::InvalidConfig(format!(
                "aw-taskwarrior.providers.cli.timeout_seconds must be a positive integer, got {}",
                other
            )))
        }
    };
    Ok(Box::new(CliProvider::new(Duration::from_secs(
        timeout_seconds,
    ))))
}

fn run_task_export_with_timeout(timeout: Duration) -> Result<std::process::Output, TaskQueryError> {
    let mut child = Command::new("task")
        .args(["rc.hooks:off", "+ACTIVE", "export"])
        .spawn()
        .map_err(TaskQueryError::Spawn)?;

    let start = std::time::Instant::now();
    let check_interval = Duration::from_millis(50);

    loop {
        match child.try_wait().map_err(TaskQueryError::Spawn)? {
            Some(_status) => {
                let output = Command::new("task")
                    .args(["rc.hooks:off", "+ACTIVE", "export"])
                    .output()
                    .map_err(TaskQueryError::Spawn)?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    return Err(TaskQueryError::NonZeroExit {
                        code: output.status.code(),
                        stderr,
                    });
                }
                return Ok(output);
            }
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(TaskQueryError::Timeout);
                }
                thread::sleep(check_interval);
            }
        }
    }
}

fn pick_latest(mut tasks: Vec<RawTwTask>) -> ActiveTask {
    tasks.sort_by(|a, b| {
        let ta = parse_tw_timestamp(&a.start);
        let tb = parse_tw_timestamp(&b.start);
        tb.cmp(&ta) // descending: most-recent first
    });
    let chosen = tasks.into_iter().next().unwrap();
    ActiveTask {
        uuid: Some(chosen.uuid),
        title: chosen.description,
        project: chosen.project.unwrap_or_else(|| "No project".to_string()),
        tags: chosen.tags,
    }
}

fn parse_tw_timestamp(s: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_empty_array_yields_default_task() {
        let json = "[]";
        let tasks: Vec<RawTwTask> = serde_json::from_str(json).unwrap();
        let result = if tasks.is_empty() {
            ActiveTask::default_task()
        } else {
            pick_latest(tasks)
        };

        assert_eq!(result.title, "No task tracked");
        assert_eq!(result.project, "");
        assert_eq!(result.tags, vec![] as Vec<String>);
        assert_eq!(result.uuid, None);
    }

    #[test]
    fn test_single_task_parses_correctly() {
        let json = r#"[{
            "uuid": "test-uuid-123",
            "description": "Test task",
            "project": "MyProject",
            "tags": ["tag1", "tag2"],
            "start": "20260723T200958Z"
        }]"#;

        let tasks: Vec<RawTwTask> = serde_json::from_str(json).unwrap();
        let result = pick_latest(tasks);

        assert_eq!(result.uuid, Some("test-uuid-123".to_string()));
        assert_eq!(result.title, "Test task");
        assert_eq!(result.project, "MyProject");
        assert_eq!(result.tags, vec!["tag1", "tag2"]);
    }

    #[test]
    fn test_missing_project_defaults_to_no_project() {
        let json = r#"[{
            "uuid": "test-uuid",
            "description": "Task without project",
            "tags": [],
            "start": "20260723T200958Z"
        }]"#;

        let tasks: Vec<RawTwTask> = serde_json::from_str(json).unwrap();
        let result = pick_latest(tasks);

        assert_eq!(result.project, "No project");
    }

    #[test]
    fn test_multiple_active_tasks_picks_most_recent_start() {
        let json = r#"[
            {
                "uuid": "uuid-1",
                "description": "Older task",
                "tags": [],
                "start": "20260723T100000Z"
            },
            {
                "uuid": "uuid-2",
                "description": "Newer task",
                "tags": [],
                "start": "20260723T200000Z"
            }
        ]"#;

        let tasks: Vec<RawTwTask> = serde_json::from_str(json).unwrap();
        let result = pick_latest(tasks);

        assert_eq!(result.uuid, Some("uuid-2".to_string()));
        assert_eq!(result.title, "Newer task");
    }

    #[test]
    fn test_unparseable_start_sorts_last() {
        let json = r#"[
            {
                "uuid": "uuid-1",
                "description": "Valid start",
                "tags": [],
                "start": "20260723T200000Z"
            },
            {
                "uuid": "uuid-2",
                "description": "Invalid start",
                "tags": [],
                "start": "not-a-timestamp"
            }
        ]"#;

        let tasks: Vec<RawTwTask> = serde_json::from_str(json).unwrap();
        let result = pick_latest(tasks);

        // Valid timestamp should be picked over invalid one
        assert_eq!(result.uuid, Some("uuid-1".to_string()));
    }

    #[test]
    fn test_parse_tw_timestamp_format() {
        let ts = "20260723T200958Z";
        let result = parse_tw_timestamp(ts);
        assert!(result.is_some());

        let dt = result.unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 7);
        assert_eq!(dt.day(), 23);
    }

    #[test]
    fn test_parse_tw_timestamp_invalid() {
        let ts = "garbage";
        let result = parse_tw_timestamp(ts);
        assert!(result.is_none());
    }

    #[test]
    fn test_create_without_settings_uses_default() {
        let provider = create(None).unwrap();
        assert_eq!(provider.name(), NAME);
    }

    #[test]
    fn test_create_accepts_timeout_seconds() {
        let table: toml::Table = toml::from_str("timeout_seconds = 9").unwrap();
        assert!(create(Some(&table)).is_ok());
    }

    #[test]
    fn test_create_rejects_non_positive_timeout() {
        let table: toml::Table = toml::from_str("timeout_seconds = 0").unwrap();
        assert!(matches!(
            create(Some(&table)),
            Err(ProviderError::InvalidConfig(_))
        ));
    }

    #[test]
    fn test_create_rejects_non_integer_timeout() {
        let table: toml::Table = toml::from_str("timeout_seconds = \"5\"").unwrap();
        assert!(matches!(
            create(Some(&table)),
            Err(ProviderError::InvalidConfig(_))
        ));
    }
}
