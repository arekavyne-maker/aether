// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Live command jobs for the local console.
//!
//! A job is still exactly one `aether` invocation with discrete argv. Unlike
//! the request/response endpoints, a job exposes its lifecycle and the bytes
//! the CLI writes while it runs. Nothing here invents progress: when the CLI
//! writes nothing, the browser sees an elapsed timer and no output events.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Notify, oneshot};
use tracing::warn;

use crate::cli::{self, CliRunner, CommandResponse, CommandState};

const MAX_EVENTS: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const JOB_TIMEOUT: Duration = Duration::from_secs(900);
const POLL_WINDOW: Duration = Duration::from_secs(4);

#[cfg(unix)]
#[allow(unsafe_code)] // the only syscall needed to cancel a whole process group
fn terminate_group(pid: u32) {
    if pid == 0 { return; }
    // The child is spawned as a process-group leader so its Python/plugin
    // children are cancelled with it rather than being orphaned.
    unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
}

#[cfg(not(unix))]
fn terminate_group(_pid: u32) {}

/// Terminal states never produce more output events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// The process is still running.
    Running,
    /// Exit 0.
    Ok,
    /// Exit 2.
    UsageError,
    /// Exit 30 — fail-closed denial.
    Blocked,
    /// Any other non-zero exit.
    Error,
    /// Explicitly cancelled by the user.
    Cancelled,
    /// Exceeded the job deadline.
    TimedOut,
    /// Could not be spawned.
    Failed,
}

impl JobState {
    fn from_exit(code: i32) -> Self {
        match cli::state_from_exit(code) {
            CommandState::Ok => Self::Ok,
            CommandState::UsageError => Self::UsageError,
            CommandState::Blocked => Self::Blocked,
            CommandState::Error => Self::Error,
        }
    }

    /// True once no further events can be emitted.
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }
}

/// Public snapshot of a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobView {
    /// Opaque job id.
    pub id: String,
    /// Exact argv after the binary.
    pub argv: Vec<String>,
    /// Current lifecycle state.
    pub state: JobState,
    /// Wall-clock start, epoch milliseconds.
    pub started_ms: u64,
    /// Wall-clock end, epoch milliseconds.
    pub finished_ms: Option<u64>,
    /// Final duration in milliseconds.
    pub duration_ms: Option<u128>,
    /// CLI exit code.
    pub exit_code: Option<i32>,
    /// Number of recorded events.
    pub event_count: usize,
    /// Final structured result, when the CLI emitted JSON.
    pub result: Option<CommandResponse>,
    /// Human-readable failure reason.
    pub error: Option<String>,
    /// True when output volume exceeded the in-memory caps.
    pub truncated: bool,
}

#[derive(Debug)]
struct Meta {
    view: JobView,
    cancel: Option<oneshot::Sender<()>>,
}

#[derive(Debug)]
struct Inner {
    meta: Meta,
    events: Vec<Value>,
}

#[derive(Debug)]
struct Job {
    inner: Mutex<Inner>,
    changed: Notify,
}

impl Job {
    fn view(&self) -> JobView {
        let inner = self.inner.lock().expect("job lock poisoned");
        let mut view = inner.meta.view.clone();
        view.event_count = inner.events.len();
        view
    }

    fn push(&self, event: Value) {
        let mut inner = self.inner.lock().expect("job lock poisoned");
        if inner.events.len() < MAX_EVENTS {
            inner.events.push(event);
        } else {
            inner.meta.view.truncated = true;
        }
        self.changed.notify_waiters();
    }
}

/// In-memory registry of live and recently finished jobs.
#[derive(Clone, Debug)]
pub struct JobManager {
    jobs: Arc<std::sync::RwLock<HashMap<String, Arc<Job>>>>,
    runner: CliRunner,
}

impl JobManager {
    /// Create a registry bound to the canonical CLI runner.
    pub fn new(runner: CliRunner) -> Self {
        Self {
            jobs: Arc::new(std::sync::RwLock::new(HashMap::new())),
            runner,
        }
    }

    fn get(&self, id: &str) -> Option<Arc<Job>> {
        self.jobs.read().expect("job registry poisoned").get(id).cloned()
    }

    /// Snapshots of all jobs, oldest first.
    pub fn list(&self) -> Vec<JobView> {
        let jobs: Vec<_> = self.jobs.read().expect("job registry poisoned").values().cloned().collect();
        let mut views: Vec<_> = jobs.iter().map(|job| job.view()).collect();
        views.sort_by_key(|view| view.started_ms);
        views
    }

    /// Start one CLI invocation and return its initial snapshot.
    pub fn start(&self, argv: Vec<String>) -> Result<JobView, String> {
        let mut argv = argv;
        if !argv.iter().any(|a| a == "--json" || a == "--format") {
            argv.push("--json".to_string());
        }
        if !argv.iter().any(|a| a == "--offline") {
            argv.push("--offline".to_string());
        }
        cli::validate_argv(&argv).map_err(|e| e.to_string())?;

        let id = format!("{:016x}", {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
            let seed = std::process::id() as u128;
            (nanos ^ (seed << 64)) as u64
        });
        let started_ms = unix_ms();
        let view = JobView {
            id: id.clone(),
            argv: argv.clone(),
            state: JobState::Running,
            started_ms,
            finished_ms: None,
            duration_ms: None,
            exit_code: None,
            event_count: 0,
            result: None,
            error: None,
            truncated: false,
        };
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let job = Arc::new(Job {
            inner: Mutex::new(Inner {
                meta: Meta { view: view.clone(), cancel: Some(cancel_tx) },
                events: Vec::new(),
            }),
            changed: Notify::new(),
        });
        self.jobs.write().expect("job registry poisoned").insert(id.clone(), job.clone());
        job.push(json!({"type": "started", "argv": argv, "at": started_ms}));

        let mut command = Command::new(self.runner.binary());
        command.args(&argv).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        if let Some(project) = self.runner.project() {
            command.current_dir(project);
        }
        #[cfg(unix)]
        command.process_group(0);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) => {
                finish(&job, JobState::Failed, None, None, Some(e.to_string()));
                return Ok(job.view());
            }
        };
        let pid = child.id().expect("spawned child has a pid");
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let job_for_task = job.clone();
        tokio::spawn(async move {
            let out_job = job_for_task.clone();
            let err_job = job_for_task.clone();
            let stdout_task = tokio::spawn(async move {
                match stdout { Some(out) => read_stream(out, "stdout", out_job).await, None => Vec::new() }
            });
            let stderr_task = tokio::spawn(async move {
                match stderr { Some(err) => read_stream(err, "stderr", err_job).await, None => Vec::new() }
            });

            let mut wait = Box::pin(child.wait());
            let outcome = tokio::select! {
                status = wait.as_mut() => ("exit", status.ok()),
                _ = cancel_rx => {
                    terminate_group(pid);
                    let _ = wait.as_mut().await;
                    ("cancel", None)
                }
                _ = tokio::time::sleep(JOB_TIMEOUT) => {
                    terminate_group(pid);
                    let _ = wait.as_mut().await;
                    ("timeout", None)
                }
            };
            let (stdout_lines, stderr_lines) = tokio::join!(stdout_task, stderr_task);
            let stdout_text = stdout_lines.unwrap_or_default().join("\n");
            let stderr_text = stderr_lines.unwrap_or_default().join("\n");
            match outcome.0 {
                "cancel" => {
                    finish(&job_for_task, JobState::Cancelled, None, None, Some("cancelled by user".into()));
                    job_for_task.push(json!({"type":"cancelled","at":unix_ms()}));
                }
                "timeout" => {
                    finish(&job_for_task, JobState::TimedOut, None, None, Some(format!("command exceeded {:?}", JOB_TIMEOUT)));
                    job_for_task.push(json!({"type":"timeout","at":unix_ms()}));
                }
                _ => {
                    let exit_code = outcome.1.and_then(|s| s.code()).unwrap_or(-1);
                    let state = JobState::from_exit(exit_code);
                    let (value, text) = match serde_json::from_str::<Value>(&stdout_text) {
                        Ok(value) => (value, None),
                        Err(_) => (Value::Null, Some(stdout_text)),
                    };
                    let result = CommandResponse {
                        command: {
                            let inner = job_for_task.inner.lock().expect("job lock poisoned");
                            inner.meta.view.argv.iter().take_while(|a| !a.starts_with("--")).cloned().collect::<Vec<_>>().join(" ")
                        },
                        state: cli::state_from_exit(exit_code),
                        exit_code,
                        value,
                        text,
                        stderr: stderr_text,
                        duration_ms: 0,
                    };
                    finish(&job_for_task, state, Some(exit_code), Some(result), None);
                }
            }
        });
        Ok(job.view())
    }

    /// Cancel a running job's process group.
    pub fn cancel(&self, id: &str) -> Result<JobView, String> {
        let job = self.get(id).ok_or_else(|| "unknown job".to_string())?;
        let signal = {
            let mut inner = job.inner.lock().expect("job lock poisoned");
            if inner.meta.view.state.is_terminal() {
                return Ok(job.view());
            }
            inner.meta.cancel.take()
        };
        if let Some(signal) = signal {
            let _ = signal.send(());
        }
        Ok(job.view())
    }

    /// Events since an index, waiting briefly for new ones.
    pub async fn events_since(&self, id: &str, after: usize) -> Result<EventsResponse, String> {
        let job = self.get(id).ok_or_else(|| "unknown job".to_string())?;
        let deadline = Instant::now() + POLL_WINDOW;
        loop {
            let (events, state, event_count) = {
                let inner = job.inner.lock().expect("job lock poisoned");
                let from = after.min(inner.events.len());
                (inner.events[from..].to_vec(), inner.meta.view.state, inner.events.len())
            };
            if event_count > after || state.is_terminal() {
                return Ok(EventsResponse {
                    id: id.to_string(),
                    after,
                    events,
                    state: Some(state),
                    event_count,
                    finished: state.is_terminal(),
                });
            }
            if Instant::now() >= deadline {
                return Ok(EventsResponse {
                    id: id.to_string(),
                    after,
                    events,
                    state: None,
                    event_count,
                    finished: false,
                });
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

/// Response from the long-poll event feed.
#[derive(Debug, Serialize, Deserialize)]
pub struct EventsResponse {
    /// Job id.
    pub id: String,
    /// Index the caller already had.
    pub after: usize,
    /// Events after that index.
    pub events: Vec<Value>,
    /// State at the time of the response, if terminal.
    pub state: Option<JobState>,
    /// Total number of events.
    pub event_count: usize,
    /// True once the job is terminal.
    pub finished: bool,
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn finish(job: &Job, state: JobState, exit_code: Option<i32>, result: Option<CommandResponse>, error: Option<String>) {
    let mut inner = job.inner.lock().expect("job lock poisoned");
    inner.meta.cancel = None;
    inner.meta.view.state = state;
    inner.meta.view.finished_ms = Some(unix_ms());
    let now = unix_ms();
    inner.meta.view.duration_ms = Some(now.saturating_sub(inner.meta.view.started_ms) as u128);
    inner.meta.view.exit_code = exit_code;
    inner.meta.view.result = result;
    inner.meta.view.error = error;
    job.changed.notify_waiters();
}

async fn read_stream<R: tokio::io::AsyncRead + Unpin>(reader: R, stream: &str, job: Arc<Job>) -> Vec<String> {
    let mut lines = BufReader::new(reader).lines();
    let mut collected = Vec::new();
    let mut bytes = 0usize;
    while let Ok(Some(line)) = lines.next_line().await {
        bytes += line.len() + 1;
        if bytes <= MAX_OUTPUT_BYTES {
            collected.push(line.clone());
        } else {
            let mut inner = job.inner.lock().expect("job lock poisoned");
            inner.meta.view.truncated = true;
        }
        if collected.len() <= MAX_EVENTS {
            let data = if line.len() > 4096 { &line[..4096] } else { &line };
            job.push(json!({"type":"output","stream":stream,"data":data,"truncated":line.len()>4096}));
        }
    }
    collected
}

/// Request body that starts a job.
#[derive(Debug, Deserialize)]
pub struct StartBody {
    /// Exact argv after the binary.
    pub argv: Vec<String>,
}

/// Long-poll cursor for a job's event feed.
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    /// Event index the client already has.
    #[serde(default)]
    pub after: usize,
}

/// List all jobs.
pub async fn list_handler(State(manager): State<Arc<JobManager>>) -> Response {
    Json(manager.list()).into_response()
}

/// Start a job.
pub async fn start_handler(State(manager): State<Arc<JobManager>>, Json(body): Json<StartBody>) -> Response {
    match manager.start(body.argv) {
        Ok(view) => (StatusCode::CREATED, Json(view)).into_response(),
        Err(message) => (StatusCode::BAD_REQUEST, Json(json!({"error":"invalid_request","message":message}))).into_response(),
    }
}

/// Snapshot one job.
pub async fn get_handler(State(manager): State<Arc<JobManager>>, Path(id): Path<String>) -> Response {
    match manager.get(&id) {
        Some(job) => Json(job.view()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Long-poll the event feed.
pub async fn events_handler(
    State(manager): State<Arc<JobManager>>,
    Path(id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Response {
    match manager.events_since(&id, query.after).await {
        Ok(response) => Json(response).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Cancel a running job.
pub async fn cancel_handler(State(manager): State<Arc<JobManager>>, Path(id): Path<String>) -> Response {
    match manager.cancel(&id) {
        Ok(view) => Json(view).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_is_terminal_only_after_completion() {
        assert!(!JobState::Running.is_terminal());
        assert!(JobState::Ok.is_terminal());
        assert!(JobState::Blocked.is_terminal());
        assert!(JobState::Cancelled.is_terminal());
    }

    #[tokio::test]
    async fn a_job_streams_output_and_finishes() {
        let bin = cli::find_cli_binary().expect("CLI binary for job test");
        let runner = CliRunner::with_binary(bin, Duration::from_secs(30));
        let manager = JobManager::new(runner);
        let job = manager.start(vec!["status".into()]).unwrap();
        let mut after = 0;
        let mut final_view = job.clone();
        for _ in 0..200 {
            let feed = manager.events_since(&job.id, after).await.unwrap();
            after = feed.event_count;
            let view = manager.get(&job.id).unwrap().view();
            if view.state.is_terminal() { final_view = view; break; }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(final_view.state.is_terminal());
        assert_eq!(final_view.exit_code, Some(0));
        assert!(final_view.event_count > 1);
        assert!(final_view.result.unwrap().value.is_object());
    }

    #[tokio::test]
    async fn unknown_job_is_a_clean_404() {
        let runner = CliRunner::with_binary(std::path::PathBuf::from("/bin/true"), Duration::from_secs(1));
        let manager = JobManager::new(runner);
        assert!(manager.events_since("missing", 0).await.is_err());
    }
}
