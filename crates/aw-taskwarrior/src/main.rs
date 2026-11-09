mod activitywatch;
mod config;
mod log;
mod providers;

use aw_taskwarrior_core::ActiveTask;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    // Load config
    let cfg = match config::load(false) {
        Ok(c) => c,
        Err(e) => {
            log::log_error(&format!("Fatal: {}", e));
            std::process::exit(1);
        }
    };

    // Build the task provider selected by config (fails fast before any network I/O)
    let provider = match providers::build(&cfg) {
        Ok(p) => p,
        Err(e) => {
            log::log_error(&format!("Fatal: {}", e));
            std::process::exit(1);
        }
    };
    log::log_info(&format!("Using task provider '{}'", provider.name()));

    // Install signal handler for clean shutdown
    let shutdown = Arc::new(AtomicBool::new(false));
    {
        let shutdown = shutdown.clone();
        ctrlc::set_handler(move || {
            shutdown.store(true, Ordering::SeqCst);
        })
        .expect("Error installing signal handler");
    }

    // Create ActivityWatch client
    let aw_client = match activitywatch::AwTaskWatcherClient::new(&cfg) {
        Ok(c) => c,
        Err(e) => {
            log::log_error(&format!("Fatal: failed to create AW client: {}", e));
            std::process::exit(1);
        }
    };

    log::log_info(&format!(
        "Starting aw-taskwarrior, bucket={}",
        aw_client.bucket_id()
    ));

    // Ensure bucket exists (retry with shutdown signal honored)
    if !aw_client.ensure_bucket_with_shutdown_check(&shutdown) {
        log::log_info("Shutdown requested during startup, exiting");
        std::process::exit(0);
    }

    // Initialize last known task to the default task
    let mut last_known_task = ActiveTask::default_task();
    // Set while idle so the "no task" line is logged once, not on every poll (REQ-015)
    let mut idle_logged = false;

    // Main poll loop
    while !shutdown.load(Ordering::SeqCst) {
        // Ask the provider for the active task; falls back to the last known task on failure
        let task = providers::resolve_task(provider.as_ref(), &mut last_known_task);

        // Decide what to send: None while idle with no no_task_title (REQ-015)
        match activitywatch::heartbeat_task(&task, &cfg) {
            Some(to_send) => {
                idle_logged = false;
                // Select pulsetime based on whether task has offline tag
                let pulsetime = activitywatch::select_pulsetime(&to_send, &cfg);

                // Send heartbeat to ActivityWatch
                if let Err(e) = aw_client.send_heartbeat(&to_send, pulsetime) {
                    log::log_error(&e);
                }
            }
            None => {
                if !idle_logged {
                    log::log_info("No active task; no heartbeat sent until one starts");
                    idle_logged = true;
                }
            }
        }

        // Sleep for the configured poll interval, checking shutdown signal frequently
        sleep_with_shutdown_check(Duration::from_secs(cfg.poll_interval_seconds), &shutdown);
    }

    log::log_info("Received shutdown signal, exiting cleanly");
    std::process::exit(0);
}

fn sleep_with_shutdown_check(duration: Duration, shutdown: &Arc<AtomicBool>) {
    let check_interval = Duration::from_millis(200);
    let mut remaining = duration;

    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        let sleep_time = if remaining > check_interval {
            check_interval
        } else {
            remaining
        };

        thread::sleep(sleep_time);
        remaining = remaining.saturating_sub(sleep_time);

        if remaining.is_zero() {
            break;
        }
    }
}
