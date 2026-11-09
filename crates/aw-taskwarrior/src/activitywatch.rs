use crate::config::Config;
use crate::log;
use aw_client_rust::blocking::AwClient;
use aw_models::Event;
use aw_taskwarrior_core::ActiveTask;
use chrono::Utc;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub struct AwTaskWatcherClient {
    client: AwClient,
    bucket_id: String,
}

impl AwTaskWatcherClient {
    pub fn new(cfg: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let client = AwClient::new_with_api_key(
            &cfg.aw_server_host,
            cfg.aw_server_port,
            "aw-taskwarrior",
            cfg.api_key.clone(),
        )?;

        let hostname = gethostname::gethostname();
        let bucket_id = format!("aw-watcher-taskwarrior_{}", hostname.to_string_lossy());

        Ok(AwTaskWatcherClient { client, bucket_id })
    }

    pub fn bucket_id(&self) -> &str {
        &self.bucket_id
    }

    pub fn ensure_bucket_with_shutdown_check(&self, shutdown: &std::sync::Arc<AtomicBool>) -> bool {
        loop {
            if shutdown.load(Ordering::SeqCst) {
                return false;
            }

            match self
                .client
                .create_bucket_simple(&self.bucket_id, "task-activity")
            {
                Ok(_) => {
                    log::log_info(&format!("Bucket {} ready", self.bucket_id));
                    return true;
                }
                Err(e) => {
                    // Check if it's already exists (304 or similar)
                    let err_str = e.to_string();
                    if err_str.contains("304") || err_str.contains("already exists") {
                        log::log_info(&format!("Bucket {} already exists", self.bucket_id));
                        return true;
                    }
                    log::log_error(&format!(
                        "Failed to reach AW server, retrying in 30s: {}",
                        e
                    ));
                }
            }

            // Sleep in short intervals to honor shutdown signal
            for _ in 0..30 {
                if shutdown.load(Ordering::SeqCst) {
                    return false;
                }
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    pub fn send_heartbeat(&self, task: &ActiveTask, pulsetime: f64) -> Result<(), String> {
        // Build data map: only include non-empty values
        let mut data = serde_json::Map::new();

        // Always include title if non-empty
        if !task.title.is_empty() {
            data.insert("title".to_string(), Value::String(task.title.clone()));
        }

        // Include project only if non-empty
        if !task.project.is_empty() {
            data.insert("project".to_string(), Value::String(task.project.clone()));
        }

        // Include tags only if non-empty
        if !task.tags.is_empty() {
            data.insert(
                "tags".to_string(),
                Value::Array(task.tags.iter().map(|t| Value::String(t.clone())).collect()),
            );
        }

        // Include uuid only if present
        if let Some(uuid) = &task.uuid {
            data.insert("uuid".to_string(), Value::String(uuid.clone()));
        }

        let event = Event {
            id: None,
            timestamp: Utc::now(),
            duration: chrono::Duration::zero(),
            data,
        };

        self.client
            .heartbeat(&self.bucket_id, &event, pulsetime)
            .map_err(|e| format!("Heartbeat failed: {}", e))
    }
}

pub fn select_pulsetime(task: &ActiveTask, cfg: &Config) -> f64 {
    if task.tags.iter().any(|t| t == &cfg.offline_tag) {
        cfg.offline_pulsetime_seconds as f64
    } else {
        cfg.online_pulsetime_seconds as f64
    }
}

/// Decides what to send for `task` (REQ-015). `None` means no heartbeat.
///
/// A real task is returned unchanged. The idle placeholder (`default_task()`) gives `None`
/// when `no_task_title` is empty after trimming. Otherwise it becomes a task whose only
/// content is the trimmed `no_task_title`.
pub fn heartbeat_task(task: &ActiveTask, cfg: &Config) -> Option<ActiveTask> {
    if *task != ActiveTask::default_task() {
        return Some(task.clone());
    }
    let title = cfg.no_task_title.trim();
    if title.is_empty() {
        return None;
    }
    Some(ActiveTask {
        uuid: None,
        title: title.to_string(),
        project: String::new(),
        tags: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_task() -> ActiveTask {
        ActiveTask {
            uuid: Some("abc".to_string()),
            title: "Write report".to_string(),
            project: "Work".to_string(),
            tags: vec!["urgent".to_string()],
        }
    }

    #[test]
    fn test_heartbeat_task_idle_and_title_unset_sends_nothing() {
        let cfg = Config::default();
        assert_eq!(heartbeat_task(&ActiveTask::default_task(), &cfg), None);
    }

    #[test]
    fn test_heartbeat_task_idle_and_title_empty_or_blank_sends_nothing() {
        let mut cfg = Config::default();
        for value in ["", "   ", "\t"] {
            cfg.no_task_title = value.to_string();
            assert_eq!(heartbeat_task(&ActiveTask::default_task(), &cfg), None);
        }
    }

    #[test]
    fn test_heartbeat_task_idle_with_title_sends_only_title() {
        let cfg = Config {
            no_task_title: "  Idle  ".to_string(),
            ..Config::default()
        };

        let sent = heartbeat_task(&ActiveTask::default_task(), &cfg).unwrap();
        assert_eq!(sent.title, "Idle");
        assert_eq!(sent.uuid, None);
        assert_eq!(sent.project, "");
        assert!(sent.tags.is_empty());
    }

    #[test]
    fn test_heartbeat_task_real_task_unchanged_whatever_the_title() {
        let mut cfg = Config::default();
        assert_eq!(heartbeat_task(&real_task(), &cfg), Some(real_task()));

        cfg.no_task_title = "Idle".to_string();
        assert_eq!(heartbeat_task(&real_task(), &cfg), Some(real_task()));
    }

    #[test]
    fn test_select_pulsetime_offline_tag_present() {
        let cfg = Config::default();
        let task = ActiveTask {
            uuid: Some("test".to_string()),
            title: "Test".to_string(),
            project: "Test".to_string(),
            tags: vec!["OFFLINE".to_string()],
        };

        let pulsetime = select_pulsetime(&task, &cfg);
        assert_eq!(pulsetime, cfg.offline_pulsetime_seconds as f64);
    }

    #[test]
    fn test_select_pulsetime_offline_tag_absent() {
        let cfg = Config::default();
        let task = ActiveTask {
            uuid: Some("test".to_string()),
            title: "Test".to_string(),
            project: "Test".to_string(),
            tags: vec!["OTHER".to_string()],
        };

        let pulsetime = select_pulsetime(&task, &cfg);
        assert_eq!(pulsetime, cfg.online_pulsetime_seconds as f64);
    }

    #[test]
    fn test_select_pulsetime_custom_offline_tag_name() {
        let mut cfg = Config::default();
        cfg.offline_tag = "AWAY".to_string();

        // Task with custom AWAY tag should use offline pulsetime
        let task_away = ActiveTask {
            uuid: Some("test".to_string()),
            title: "Test".to_string(),
            project: "Test".to_string(),
            tags: vec!["AWAY".to_string()],
        };
        assert_eq!(
            select_pulsetime(&task_away, &cfg),
            cfg.offline_pulsetime_seconds as f64
        );

        // Task with default OFFLINE tag should use online pulsetime (not matched)
        let task_offline = ActiveTask {
            uuid: Some("test".to_string()),
            title: "Test".to_string(),
            project: "Test".to_string(),
            tags: vec!["OFFLINE".to_string()],
        };
        assert_eq!(
            select_pulsetime(&task_offline, &cfg),
            cfg.online_pulsetime_seconds as f64
        );
    }
}
