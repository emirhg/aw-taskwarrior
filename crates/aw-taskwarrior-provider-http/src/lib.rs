//! HTTP task provider: reads the active task from the task-rest API (REQ-007).
//!
//! Flow: `GET /auth` with Basic credentials returns a JWT. `GET /` with the JWT in
//! `x-access-tokens` returns every task. The active task is the latest pending task
//! that has a start date, the same meaning as TaskWarrior's `+ACTIVE`.
//!
//! The filter is applied client-side for now. REQ-010 tracks the server-side filter.

use aw_taskwarrior_core::{ActiveTask, ProviderError, TaskProvider};
use chrono::{DateTime, FixedOffset};
use reqwest::blocking::{Client, Response};
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use serde_json::Value;
use std::fmt;
use std::sync::Mutex;
use std::time::Duration;

/// Provider name used in config (`provider = "http"`) and logs.
pub const NAME: &str = "http";

const DEFAULT_TIMEOUT_SECONDS: u64 = 5;
const TOKEN_HEADER: &str = "x-access-tokens";
const AUTH_REJECTED: &str = "authentication rejected: check username and password";

/// Talks to a task-rest server and keeps the JWT between polls.
pub struct HttpProvider {
    client: Client,
    base_url: Url,
    username: String,
    password: String,
    token: Mutex<Option<String>>,
}

impl fmt::Debug for HttpProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpProvider")
            .field("base_url", &self.base_url.as_str())
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("token", &"<redacted>")
            .finish()
    }
}

/// Builds the HTTP provider from `[aw-taskwarrior.providers.http]` settings.
/// Does no network I/O.
pub fn create(settings: Option<&toml::Table>) -> Result<Box<dyn TaskProvider>, ProviderError> {
    Ok(Box::new(build(settings)?))
}

fn build(settings: Option<&toml::Table>) -> Result<HttpProvider, ProviderError> {
    let empty = toml::Table::new();
    let settings = settings.unwrap_or(&empty);

    let base_url = parse_base_url(&required_string(settings, "base_url")?)?;
    let username = required_string(settings, "username")?;
    let password = required_string(settings, "password")?;
    let timeout_seconds = timeout_setting(settings)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .map_err(|e| ProviderError::Unavailable(format!("cannot create HTTP client: {}", e)))?;

    Ok(HttpProvider {
        client,
        base_url,
        username,
        password,
        token: Mutex::new(None),
    })
}

impl TaskProvider for HttpProvider {
    fn name(&self) -> &'static str {
        NAME
    }

    fn active_task(&self) -> Result<ActiveTask, ProviderError> {
        let token = self.cached_or_fresh_token()?;
        let response = self.fetch_list(&token)?;
        if response.status() != StatusCode::UNAUTHORIZED {
            return read_active_task(response);
        }
        // The token expired or was rejected: authenticate once more and retry once.
        let token = self.refresh_token()?;
        read_active_task(self.fetch_list(&token)?)
    }
}

impl HttpProvider {
    fn endpoint(&self, path: &str) -> Result<Url, ProviderError> {
        self.base_url.join(path).map_err(|e| {
            ProviderError::InvalidConfig(format!("cannot build URL for '{}': {}", path, e))
        })
    }

    fn authenticate(&self) -> Result<String, ProviderError> {
        let response = self
            .client
            .get(self.endpoint("auth")?)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(transport_error)?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(ProviderError::Unavailable(AUTH_REJECTED.to_string()));
        }
        if !status.is_success() {
            return Err(ProviderError::Unavailable(format!(
                "authentication failed with HTTP {}",
                status.as_u16()
            )));
        }
        let body: TokenBody = response
            .json()
            .map_err(|e| ProviderError::InvalidResponse(format!("auth response: {}", e)))?;
        Ok(body.token)
    }

    fn cached_or_fresh_token(&self) -> Result<String, ProviderError> {
        if let Some(token) = self.lock_token().clone() {
            return Ok(token);
        }
        self.refresh_token()
    }

    fn refresh_token(&self) -> Result<String, ProviderError> {
        let token = self.authenticate()?;
        *self.lock_token() = Some(token.clone());
        Ok(token)
    }

    fn fetch_list(&self, token: &str) -> Result<Response, ProviderError> {
        self.client
            .get(self.endpoint("")?)
            .header(TOKEN_HEADER, token)
            .send()
            .map_err(transport_error)
    }

    fn lock_token(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.token.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Deserialize)]
struct TokenBody {
    token: String,
}

/// One task as returned by task-rest. Empty fields are omitted by the server,
/// so every optional field defaults.
#[derive(Deserialize)]
struct ApiTask {
    #[serde(default)]
    id: u64,
    description: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    start: Option<String>,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    uuid: Option<String>,
}

impl ApiTask {
    fn start_time(&self) -> Option<DateTime<FixedOffset>> {
        self.start
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc2822(s).ok())
    }
}

fn read_active_task(response: Response) -> Result<ActiveTask, ProviderError> {
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(ProviderError::Unavailable(AUTH_REJECTED.to_string()));
    }

    let body = response.text().map_err(transport_error)?;
    let json: Value = match serde_json::from_str(&body) {
        Ok(json) => json,
        Err(e) if status.is_success() => {
            return Err(ProviderError::InvalidResponse(format!(
                "body is not JSON: {}",
                e
            )))
        }
        Err(_) => {
            return Err(ProviderError::Unavailable(format!(
                "HTTP {}",
                status.as_u16()
            )))
        }
    };

    if let Some(description) = error_message(&json) {
        return Err(ProviderError::Unavailable(description));
    }
    if !status.is_success() {
        return Err(ProviderError::Unavailable(format!(
            "HTTP {}",
            status.as_u16()
        )));
    }

    let tasks = json.get("tasks").cloned().ok_or_else(|| {
        ProviderError::InvalidResponse("response has no 'tasks' list".to_string())
    })?;
    let tasks: Vec<ApiTask> = serde_json::from_value(tasks)
        .map_err(|e| ProviderError::InvalidResponse(format!("cannot read tasks: {}", e)))?;
    Ok(select_active(tasks))
}

/// Returns the description of a task-rest error object, if the body has one.
fn error_message(json: &Value) -> Option<String> {
    let message = json.get("message")?;
    if message.get("type")?.as_str()? != "error" {
        return None;
    }
    message.get("description")?.as_str().map(str::to_string)
}

/// Latest pending task with a start date. Ties on the start time go to the higher id.
/// Unparseable start dates sort last.
fn select_active(tasks: Vec<ApiTask>) -> ActiveTask {
    tasks
        .into_iter()
        .filter(|t| t.status == "pending" && t.start.is_some())
        .max_by_key(|t| (t.start_time(), t.id))
        .map(|t| ActiveTask {
            uuid: t.uuid,
            title: t.description,
            project: t.project.unwrap_or_else(|| "No project".to_string()),
            tags: t.tags,
        })
        .unwrap_or_else(ActiveTask::default_task)
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Unavailable(error.to_string())
    }
}

fn key_error(key: &str, message: impl Into<String>) -> ProviderError {
    ProviderError::InvalidConfig(format!(
        "aw-taskwarrior.providers.http.{}: {}",
        key,
        message.into()
    ))
}

fn required_string(settings: &toml::Table, key: &str) -> Result<String, ProviderError> {
    match settings.get(key) {
        None => Err(key_error(key, "is required")),
        Some(toml::Value::String(s)) if s.is_empty() => Err(key_error(key, "must not be empty")),
        Some(toml::Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(key_error(key, format!("must be a string, got {}", other))),
    }
}

fn timeout_setting(settings: &toml::Table) -> Result<u64, ProviderError> {
    match settings.get("timeout_seconds") {
        None => Ok(DEFAULT_TIMEOUT_SECONDS),
        Some(toml::Value::Integer(n)) if *n >= 1 => Ok(*n as u64),
        Some(other) => Err(key_error(
            "timeout_seconds",
            format!("must be a positive integer, got {}", other),
        )),
    }
}

/// Checks the base URL and makes sure its path ends with `/`, so `join` keeps a
/// reverse-proxy prefix. User info is rejected so credentials never sit in a URL.
fn parse_base_url(raw: &str) -> Result<Url, ProviderError> {
    let mut url =
        Url::parse(raw).map_err(|e| key_error("base_url", format!("is not a valid URL: {}", e)))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(key_error(
            "base_url",
            format!("must use http or https, got '{}'", url.scheme()),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(key_error(
            "base_url",
            "must not contain user info; set username and password instead",
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(key_error(
            "base_url",
            "must not contain a query or fragment",
        ));
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    /// Real responses captured from task-rest (see tests/fixtures).
    const FIXTURE: &str = include_str!("../tests/fixtures/task_list.json");
    const PASSWORD: &str = "s3cret-pass";

    fn settings(text: &str) -> toml::Table {
        toml::from_str(text).unwrap()
    }

    fn valid_settings(base_url: &str) -> toml::Table {
        settings(&format!(
            "base_url = \"{}\"\nusername = \"dev\"\npassword = \"{}\"\n",
            base_url, PASSWORD
        ))
    }

    fn provider_for(base_url: &str) -> HttpProvider {
        build(Some(&valid_settings(base_url))).unwrap()
    }

    fn fixture_tasks() -> Vec<ApiTask> {
        let json: Value = serde_json::from_str(FIXTURE).unwrap();
        serde_json::from_value(json["tasks"].clone()).unwrap()
    }

    fn select_from(json: &str) -> ActiveTask {
        let tasks: Vec<ApiTask> = serde_json::from_str(json).unwrap();
        select_active(tasks)
    }

    // ---- settings ----

    #[test]
    fn test_create_returns_http_provider() {
        let provider = create(Some(&valid_settings("http://127.0.0.1:5000"))).unwrap();
        assert_eq!(provider.name(), "http");
    }

    #[test]
    fn test_requires_base_url() {
        let err = build(Some(&settings("username = \"dev\"\npassword = \"x\"\n"))).unwrap_err();
        assert!(err.to_string().contains("base_url"));
    }

    #[test]
    fn test_rejects_non_http_scheme() {
        let err = build(Some(&valid_settings("ftp://127.0.0.1/"))).unwrap_err();
        assert!(matches!(err, ProviderError::InvalidConfig(_)));
        assert!(err.to_string().contains("http or https"));
    }

    #[test]
    fn test_rejects_user_info_in_base_url() {
        let err = build(Some(&valid_settings("http://dev:pw@127.0.0.1:5000/"))).unwrap_err();
        assert!(err.to_string().contains("user info"));
    }

    #[test]
    fn test_rejects_query_in_base_url() {
        let err = build(Some(&valid_settings("http://127.0.0.1:5000/?x=1"))).unwrap_err();
        assert!(err.to_string().contains("query"));
    }

    #[test]
    fn test_requires_username_and_password() {
        let err = build(Some(&settings(
            "base_url = \"http://127.0.0.1:5000\"\npassword = \"x\"\n",
        )))
        .unwrap_err();
        assert!(err.to_string().contains("username"));

        let err = build(Some(&settings(
            "base_url = \"http://127.0.0.1:5000\"\nusername = \"dev\"\npassword = \"\"\n",
        )))
        .unwrap_err();
        assert!(err.to_string().contains("password"));
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn test_rejects_bad_timeout() {
        let text = |t: &str| {
            format!(
                "base_url = \"http://127.0.0.1:5000\"\nusername = \"dev\"\npassword = \"x\"\ntimeout_seconds = {}\n",
                t
            )
        };
        assert!(build(Some(&settings(&text("0")))).is_err());
        assert!(build(Some(&settings(&text("\"5\"")))).is_err());
        assert!(build(Some(&settings(&text("3")))).is_ok());
    }

    #[test]
    fn test_base_url_path_gets_trailing_slash() {
        let provider = provider_for("http://127.0.0.1:5000/api");
        assert_eq!(provider.base_url.path(), "/api/");
        assert_eq!(provider.endpoint("auth").unwrap().path(), "/api/auth");
    }

    #[test]
    fn test_debug_hides_secrets() {
        let provider = provider_for("http://127.0.0.1:5000");
        *provider.lock_token() = Some("tok-secret".to_string());
        let text = format!("{:?}", provider);
        assert!(!text.contains(PASSWORD));
        assert!(!text.contains("tok-secret"));
        assert!(text.contains("dev"));
    }

    // ---- date parsing ----

    #[test]
    fn test_rfc2822_offsets_are_the_same_instant() {
        let local = DateTime::parse_from_rfc2822("Thu, 05 Nov 2026 23:27:01 -0600").unwrap();
        let utc = DateTime::parse_from_rfc2822("Fri, 06 Nov 2026 05:27:01 +0000").unwrap();
        let gmt = DateTime::parse_from_rfc2822("Fri, 06 Nov 2026 05:27:01 GMT").unwrap();
        let neg_zero = DateTime::parse_from_rfc2822("Fri, 06 Nov 2026 05:27:01 -0000").unwrap();
        assert_eq!(local, utc);
        assert_eq!(utc, gmt);
        assert_eq!(gmt, neg_zero);
    }

    // ---- selection (from the captured fixture) ----

    #[test]
    fn test_fixture_selects_latest_start_with_id_tie_break() {
        // Tasks 3 (E), 4 (F) and 5 (G) all start in the same second; the higher id wins.
        let task = select_active(fixture_tasks());
        assert_eq!(task.title, "G no project started");
        assert_eq!(task.project, "No project");
        assert!(task.tags.is_empty());
    }

    #[test]
    fn test_fixture_active_set_is_pending_with_start() {
        let active: Vec<String> = fixture_tasks()
            .into_iter()
            .filter(|t| t.status == "pending" && t.start.is_some())
            .map(|t| t.description)
            .collect();
        assert_eq!(
            active,
            vec![
                "A pending started",
                "E started later",
                "F work with tags",
                "G no project started"
            ]
        );
    }

    #[test]
    fn test_project_and_tags_are_mapped() {
        let task = select_from(
            r#"[{"id":4,"uuid":"u-4","description":"F work","status":"pending",
                 "start":"Thu, 05 Nov 2026 23:27:03 -0600",
                 "project":"Organizacion.Comunicacion","tags":["reporte","work"]}]"#,
        );
        assert_eq!(task.uuid.as_deref(), Some("u-4"));
        assert_eq!(task.title, "F work");
        assert_eq!(task.project, "Organizacion.Comunicacion");
        assert_eq!(task.tags, vec!["reporte", "work"]);
    }

    #[test]
    fn test_pending_without_start_is_not_active() {
        let task = select_from(r#"[{"id":2,"uuid":"u-2","description":"B","status":"pending"}]"#);
        assert_eq!(task, ActiveTask::default_task());
    }

    #[test]
    fn test_completed_or_deleted_with_start_is_not_active() {
        let task = select_from(
            r#"[{"id":3,"description":"C","status":"completed","start":"Thu, 05 Nov 2026 23:27:01 -0600"},
                {"id":4,"description":"D","status":"deleted","start":"Thu, 05 Nov 2026 23:27:01 -0600"}]"#,
        );
        assert_eq!(task, ActiveTask::default_task());
    }

    #[test]
    fn test_empty_list_is_default_task() {
        assert_eq!(select_from("[]"), ActiveTask::default_task());
    }

    #[test]
    fn test_unparseable_start_sorts_last() {
        let task = select_from(
            r#"[{"id":9,"uuid":"bad","description":"bad date","status":"pending","start":"not a date"},
                {"id":1,"uuid":"good","description":"good date","status":"pending","start":"Thu, 05 Nov 2026 23:27:01 -0600"}]"#,
        );
        assert_eq!(task.uuid.as_deref(), Some("good"));
    }

    // ---- HTTP flow against a loopback mock server ----

    struct Reply {
        status: u16,
        body: String,
    }

    fn token_reply(token: &str) -> Reply {
        Reply {
            status: 200,
            body: format!("{{\"token\": \"{}\"}}", token),
        }
    }

    fn list_reply(body: &str) -> Reply {
        Reply {
            status: 200,
            body: body.to_string(),
        }
    }

    fn unauthorized() -> Reply {
        Reply {
            status: 401,
            body: "Could not verify".to_string(),
        }
    }

    fn reason(code: u16) -> &'static str {
        match code {
            200 => "OK",
            401 => "Unauthorized",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Status",
        }
    }

    /// One summary line per request: method, path, then the auth or token header.
    fn read_request(stream: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut chunk).unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        let text = String::from_utf8_lossy(&buf).to_string();
        let mut lines = text.lines();
        let request_line = lines.next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");
        let mut summary = format!("{} {}", method, path);
        for line in lines {
            let lower = line.to_ascii_lowercase();
            let value = line.split_once(':').map_or("", |(_, v)| v).trim();
            if lower.starts_with("authorization:") {
                summary.push_str(&format!(" auth={}", value));
            }
            if lower.starts_with("x-access-tokens:") {
                summary.push_str(&format!(" token={}", value));
            }
        }
        summary
    }

    /// Serves one canned reply per connection. Returns the base URL and a handle that
    /// yields one summary line per request seen.
    fn serve(replies: Vec<Reply>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let mut seen = Vec::new();
            for reply in replies {
                let (mut stream, _) = listener.accept().unwrap();
                seen.push(read_request(&mut stream));
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    reply.status,
                    reason(reply.status),
                    reply.body.len(),
                    reply.body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
            seen
        });
        (url, handle)
    }

    #[test]
    fn test_authenticates_then_lists_with_token() {
        let (url, server) = serve(vec![token_reply("tok-1"), list_reply(FIXTURE)]);
        let task = provider_for(&url).active_task().unwrap();
        assert_eq!(task.title, "G no project started");

        let seen = server.join().unwrap();
        assert!(seen[0].starts_with("GET /auth auth=Basic "));
        assert!(!seen[0].contains(PASSWORD));
        assert_eq!(seen[1], "GET / token=tok-1");
    }

    #[test]
    fn test_reuses_cached_token() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            list_reply(FIXTURE),
            list_reply(FIXTURE),
        ]);
        let provider = provider_for(&url);
        provider.active_task().unwrap();
        provider.active_task().unwrap();

        let seen = server.join().unwrap();
        assert_eq!(
            seen.iter().filter(|l| l.starts_with("GET /auth")).count(),
            1
        );
        assert_eq!(seen[2], "GET / token=tok-1");
    }

    #[test]
    fn test_expired_token_is_replaced_and_retried() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            unauthorized(),
            token_reply("tok-2"),
            list_reply(FIXTURE),
        ]);
        let task = provider_for(&url).active_task().unwrap();
        assert_eq!(task.title, "G no project started");

        let seen = server.join().unwrap();
        assert_eq!(seen[1], "GET / token=tok-1");
        assert!(seen[2].starts_with("GET /auth"));
        assert_eq!(seen[3], "GET / token=tok-2");
    }

    #[test]
    fn test_second_rejection_is_auth_rejected_without_secrets() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            unauthorized(),
            token_reply("tok-2"),
            unauthorized(),
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();

        assert_eq!(err, ProviderError::Unavailable(AUTH_REJECTED.to_string()));
        let text = err.to_string();
        assert!(!text.contains(PASSWORD));
        assert!(!text.contains("tok-1") && !text.contains("tok-2"));
    }

    #[test]
    fn test_wrong_password_is_auth_rejected() {
        let (url, server) = serve(vec![unauthorized()]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(err, ProviderError::Unavailable(AUTH_REJECTED.to_string()));
        assert!(!err.to_string().contains(PASSWORD));
    }

    #[test]
    fn test_task_error_message_is_unavailable() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            list_reply(r#"{"message":{"description":"Task database locked","type":"error"}}"#),
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(
            err,
            ProviderError::Unavailable("Task database locked".to_string())
        );
    }

    #[test]
    fn test_http_error_with_message_is_unavailable() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            Reply {
                status: 404,
                body: r#"{"message":{"description":"Task matching query does not exist.","type":"error"}}"#
                    .to_string(),
            },
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(
            err,
            ProviderError::Unavailable("Task matching query does not exist.".to_string())
        );
    }

    #[test]
    fn test_non_json_success_is_invalid_response() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            list_reply("<html>proxy page</html>"),
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert!(matches!(err, ProviderError::InvalidResponse(_)));
    }

    #[test]
    fn test_non_json_error_reports_status() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            Reply {
                status: 500,
                body: "oops".to_string(),
            },
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(err, ProviderError::Unavailable("HTTP 500".to_string()));
    }

    #[test]
    fn test_connection_refused_is_unavailable() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let err = provider_for(&format!("http://127.0.0.1:{}/", port))
            .active_task()
            .unwrap_err();
        assert!(matches!(err, ProviderError::Unavailable(_)));
        assert!(!err.to_string().contains(PASSWORD));
    }

    #[test]
    fn test_slow_server_is_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        thread::spawn(move || {
            let _connection = listener.accept();
            thread::sleep(Duration::from_secs(3));
        });
        let provider = build(Some(&settings(&format!(
            "base_url = \"{}\"\nusername = \"dev\"\npassword = \"{}\"\ntimeout_seconds = 1\n",
            url, PASSWORD
        ))))
        .unwrap();
        assert_eq!(provider.active_task().unwrap_err(), ProviderError::Timeout);
    }

    // ---- task-rest HEAD (f7a64b3): newer response shape and status codes ----

    /// Response captured from task-rest HEAD f7a64b3. It has the recurring template and
    /// instance, a waiting task, and an annotated started task.
    const FIXTURE_HEAD: &str = include_str!("../tests/fixtures/task_list_head.json");

    #[test]
    fn test_head_fixture_selects_most_recently_started_task() {
        // K was started last, after the captured E/F/G starts.
        let json: Value = serde_json::from_str(FIXTURE_HEAD).unwrap();
        let tasks: Vec<ApiTask> = serde_json::from_value(json["tasks"].clone()).unwrap();
        let task = select_active(tasks);
        assert_eq!(task.title, "K started annotated");
        assert_eq!(
            task.uuid.as_deref(),
            Some("c9dcd2fc-4753-40b1-ae77-4e0fcbc513e0")
        );
    }

    #[test]
    fn test_head_fixture_recurring_template_and_waiting_are_not_active() {
        let json: Value = serde_json::from_str(FIXTURE_HEAD).unwrap();
        let tasks: Vec<ApiTask> = serde_json::from_value(json["tasks"].clone()).unwrap();
        let active: Vec<String> = tasks
            .iter()
            .filter(|t| t.status == "pending" && t.start.is_some())
            .map(|t| t.description.clone())
            .collect();
        // Template (status recurring), instance (pending, no start) and waiting (no start)
        // are all excluded.
        assert!(!active
            .iter()
            .any(|d| d == "H recurring daily" || d == "I waiting"));
    }

    #[test]
    fn test_newer_fields_are_ignored() {
        // annotations, recur, parent, due, mask and imask are not read by the provider.
        let task = select_from(
            r#"[{"id":10,"uuid":"u-10","description":"K","status":"pending",
                 "start":"Thu, 05 Nov 2026 23:40:42 -0600",
                 "annotations":[{"description":"a note","entry":"Thu, 05 Nov 2026 23:40:42 -0600"}],
                 "recur":"daily","due":"Fri, 06 Nov 2026 23:00:00 -0600",
                 "mask":"+","imask":0,
                 "parent":{"description":"template","uuid":"u-p"}}]"#,
        );
        assert_eq!(task.title, "K");
        assert_eq!(task.project, "No project");
    }

    #[test]
    fn test_400_with_message_is_unavailable() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            Reply {
                status: 400,
                body: r#"{"message":{"description":"Invalid input: bad filter","type":"error"}}"#
                    .to_string(),
            },
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(
            err,
            ProviderError::Unavailable("Invalid input: bad filter".to_string())
        );
    }

    #[test]
    fn test_500_with_message_is_unavailable() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            Reply {
                status: 500,
                body: r#"{"message":{"description":"TaskWarrior crashed","type":"error"}}"#
                    .to_string(),
            },
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(
            err,
            ProviderError::Unavailable("TaskWarrior crashed".to_string())
        );
    }

    #[test]
    fn test_503_html_reports_status_only() {
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            Reply {
                status: 503,
                body: "<html>maintenance</html>".to_string(),
            },
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(err, ProviderError::Unavailable("HTTP 503".to_string()));
    }

    #[test]
    fn test_404_html_reports_status_only() {
        // Typical symptom of a base_url that points at the wrong path.
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            Reply {
                status: 404,
                body: "<html>not found</html>".to_string(),
            },
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(err, ProviderError::Unavailable("HTTP 404".to_string()));
    }

    #[test]
    fn test_sync_style_200_error_is_unavailable() {
        // task-rest answers GET /sync failures with 200 and a message (open BUG upstream).
        // The provider must treat the same body as an error on any endpoint.
        let (url, server) = serve(vec![
            token_reply("tok-1"),
            list_reply(
                r#"{"message":{"description":"No sync.* settings are configured. See task-sync(5).","type":"error"}}"#,
            ),
        ]);
        let err = provider_for(&url).active_task().unwrap_err();
        server.join().unwrap();
        assert_eq!(
            err,
            ProviderError::Unavailable(
                "No sync.* settings are configured. See task-sync(5).".to_string()
            )
        );
    }

    // ---- live test (run manually) ----

    /// Needs a running task-rest. Run with:
    /// TASK_REST_URL=http://127.0.0.1:5055 TASK_REST_USER=dev TASK_REST_PASSWORD=password \
    ///   cargo test -p aw-taskwarrior-provider-http -- --ignored
    #[test]
    #[ignore]
    fn live_task_rest_returns_active_task() {
        let url = std::env::var("TASK_REST_URL").expect("set TASK_REST_URL");
        let user = std::env::var("TASK_REST_USER").expect("set TASK_REST_USER");
        let password = std::env::var("TASK_REST_PASSWORD").expect("set TASK_REST_PASSWORD");
        let provider = build(Some(&settings(&format!(
            "base_url = \"{}\"\nusername = \"{}\"\npassword = \"{}\"\n",
            url, user, password
        ))))
        .unwrap();
        let task = provider.active_task().unwrap();
        eprintln!("live active task: {:?}", task);
        assert!(
            task.uuid.is_some(),
            "expected an active task, got {:?}",
            task
        );
    }
}
