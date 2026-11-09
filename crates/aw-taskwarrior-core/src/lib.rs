//! Shared model and interface for active-task providers.
//!
//! This crate has no dependencies. The watcher and every provider depend on it;
//! it depends on none of them.

use std::fmt;

/// The task currently being tracked.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTask {
    pub uuid: Option<String>,
    pub title: String,
    pub project: String,
    pub tags: Vec<String>,
}

impl ActiveTask {
    /// Placeholder used when no task is active. The title is empty; the watcher decides
    /// from `no_task_title` in config whether anything is sent while idle (REQ-015).
    pub fn default_task() -> Self {
        ActiveTask {
            uuid: None,
            title: String::new(),
            project: String::new(),
            tags: Vec::new(),
        }
    }
}

/// A source of the active task.
pub trait TaskProvider: Send {
    /// Short provider name used in logs and config, e.g. "cli".
    fn name(&self) -> &'static str;

    /// Returns the currently active task, or `ActiveTask::default_task()` when none is active.
    fn active_task(&self) -> Result<ActiveTask, ProviderError>;
}

/// Errors a provider can report.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderError {
    /// The source cannot be reached or failed to run.
    Unavailable(String),
    /// The source did not answer in time.
    Timeout,
    /// The source answered with data that cannot be parsed.
    InvalidResponse(String),
    /// The provider's settings are invalid.
    InvalidConfig(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::Unavailable(msg) => write!(f, "provider unavailable: {}", msg),
            ProviderError::Timeout => write!(f, "provider timed out"),
            ProviderError::InvalidResponse(msg) => write!(f, "invalid provider response: {}", msg),
            ProviderError::InvalidConfig(msg) => write!(f, "invalid provider config: {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}
