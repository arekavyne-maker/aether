// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! HTTP API surface.
//!
//! Every `/api/*` route requires a bearer token. Static assets are
//! unauthenticated but contain no engineering data. All command execution
//! goes through [`CliRunner`] and the curated [`Catalog`]; no shell and no
//! parallel implementation exists in this crate.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::{Router, middleware};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::RwLock;

use crate::assets;
use crate::auth::{self, AuthError, TokenAuth};
use crate::catalog::{Catalog, CatalogError, CommandKind};
use crate::cli::{
    CliError, CliRunner, CommandRequest, CommandState, DiscoveredCommand, resolve_command,
    validate_argv,
};
use crate::files;
use crate::help::{self, CommandHelp};
use crate::jobs::{self, JobManager};

/// Shared application state.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Bearer token verifier.
    pub token: Arc<TokenAuth>,
    /// CLI subprocess adapter.
    pub runner: Arc<CliRunner>,
    /// Live command jobs with incremental output.
    pub jobs: Arc<JobManager>,
    /// Curated command catalog.
    pub catalog: Arc<Catalog>,
    /// Short-lived response cache.
    pub cache: Arc<RwLock<HashMap<String, (Instant, Value)>>>,
    /// Cached CLI discovery surface.
    pub discovered: Arc<RwLock<Option<(Instant, Vec<DiscoveredCommand>)>>>,
    /// Parsed help, cached for the life of the process.
    pub helps: Arc<RwLock<HashMap<String, CommandHelp>>>,
}

impl AppState {
    /// Build state, loading and validating the embedded catalog.
    pub fn new(token: TokenAuth, runner: CliRunner) -> Result<Self, CatalogError> {
        let catalog = Catalog::load()?;
        catalog.validate()?;
        let runner = Arc::new(runner);
        Ok(Self {
            token: Arc::new(token),
            jobs: Arc::new(JobManager::new((*runner).clone())),
            runner,
            catalog: Arc::new(catalog),
            cache: Arc::new(RwLock::new(HashMap::new())),
            discovered: Arc::new(RwLock::new(None)),
            helps: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub(crate) async fn cached(&self, key: &str, ttl: Duration) -> Option<Value> {
        let cache = self.cache.read().await;
        cache
            .get(key)
            .and_then(|(at, value)| {
                if at.elapsed() < ttl {
                    Some(value.clone())
                } else {
                    None
                }
            })
    }

    pub(crate) async fn remember(&self, key: &str, value: Value) {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), (Instant::now(), value));
    }

    /// One discovery query shared by the surface, describe, exec and raw APIs.
    pub async fn discovered(&self) -> Result<Vec<DiscoveredCommand>, CliError> {
        const TTL: Duration = Duration::from_secs(30);
        if let Some((at, list)) = self.discovered.read().await.as_ref() {
            if at.elapsed() < TTL {
                return Ok(list.clone());
            }
        }
        let list = self.runner.discovered().await?;
        *self.discovered.write().await = Some((Instant::now(), list.clone()));
        Ok(list)
    }
}

/// Request body for a curated command invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeRequest {
    /// Curated command name.
    pub name: String,
    /// Argument values keyed by argument name.
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

/// Request body for the raw command console.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRequest {
    /// Command tokens, e.g. `["schema", "list"]`.
    pub command: Vec<String>,
    /// Extra flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Request body for the terminal. Positional arguments live in `argv`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    /// Exact argv tokens, without the leading `aether` binary.
    pub argv: Vec<String>,
    /// Inject `--json` when the caller did not request another format.
    #[serde(default = "default_true")]
    pub structured: bool,
    /// Optional deadline override in seconds, clamped to 1..=3600.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Query for the help endpoint.
#[derive(Debug, Deserialize)]
pub struct DescribeQuery {
    /// Fully qualified command name, e.g. `vv coverage`.
    pub name: String,
}

/// Query for the path picker.
#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    /// Case-insensitive substring filter.
    #[serde(default)]
    pub q: Option<String>,
    /// Maximum number of entries.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Query for the project atlas. An omitted offset uses the chain head.
#[derive(Debug, Deserialize)]
pub struct AtlasQuery {
    /// Event offset to render.
    #[serde(default)]
    pub offset: Option<u64>,
}

/// One command in the merged UI surface.
#[derive(Debug, Clone, Serialize)]
pub struct SurfaceCommand {
    /// Full command name.
    pub name: String,
    /// Discovery description.
    pub about: String,
    /// Resolved workspace id.
    pub workspace: String,
    /// Leading CLI token.
    pub family: String,
    /// Whether the command has child commands.
    pub group: bool,
    /// Whether the command has curated metadata.
    pub curated: bool,
    /// Interaction class: `group`, `read`, `approval`, `long`.
    pub kind: String,
}

/// One workspace with its command count.
#[derive(Debug, Clone, Serialize)]
pub struct SurfaceWorkspace {
    /// Stable id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Display description.
    pub description: String,
    /// Number of commands mapped here.
    pub command_count: usize,
}

/// The complete command surface available to the browser.
#[derive(Debug, Clone, Serialize)]
pub struct Surface {
    /// Catalog version.
    pub version: String,
    /// Workspaces in display order.
    pub workspaces: Vec<SurfaceWorkspace>,
    /// Every command advertised by the CLI.
    pub commands: Vec<SurfaceCommand>,
}

fn kind_name(kind: CommandKind) -> &'static str {
    match kind {
        CommandKind::Read => "read",
        CommandKind::Approval => "approval",
        CommandKind::Long => "long",
    }
}

async fn build_surface(state: &AppState) -> Result<Surface, String> {
    let discovered = state.discovered().await.map_err(|e| e.to_string())?;
    let commands: Vec<SurfaceCommand> = discovered
        .iter()
        .map(|command| {
            let family = command.name.split_whitespace().next().unwrap_or("").to_string();
            let child_prefix = format!("{} ", command.name);
            let group = discovered
                .iter()
                .any(|other| other.name.starts_with(&child_prefix));
            let curated = state.catalog.get(&command.name).ok();
            let kind = if group {
                "group"
            } else {
                curated.map(|c| kind_name(c.kind)).unwrap_or("read")
            };
            SurfaceCommand {
                name: command.name.clone(),
                about: command.about.clone(),
                workspace: state.catalog.workspace_for(&family).to_string(),
                family,
                group,
                curated: curated.is_some(),
                kind: kind.to_string(),
            }
        })
        .collect();
    let workspaces = state
        .catalog
        .workspaces
        .iter()
        .map(|w| SurfaceWorkspace {
            id: w.id.clone(),
            title: w.title.clone(),
            description: w.description.clone(),
            command_count: commands.iter().filter(|c| c.workspace == w.id).count(),
        })
        .collect();
    Ok(Surface {
        version: state.catalog.version.clone(),
        workspaces,
        commands,
    })
}

fn error_response(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({
            "error": code,
            "message": message.into(),
        })),
    )
        .into_response()
}

/// Bearer-token middleware for `/api/*`.
async fn authenticate(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let header = match headers.get(axum::http::header::AUTHORIZATION) {
        Some(value) => match value.to_str() {
            Ok(value) => value,
            Err(_) => {
                return error_response(StatusCode::UNAUTHORIZED, "invalid_token", "invalid header");
            }
        },
        None => {
            return error_response(StatusCode::UNAUTHORIZED, "missing_token", "bearer token required");
        }
    };
    match auth::bearer(header).and_then(|token| state.token.verify(token)) {
        Ok(()) => next.run(request).await,
        Err(AuthError::Invalid) => {
            error_response(StatusCode::UNAUTHORIZED, "invalid_token", "bearer token rejected")
        }
        Err(AuthError::Missing) => {
            error_response(StatusCode::UNAUTHORIZED, "missing_token", "bearer token required")
        }
    }
}

/// Build the router.
pub fn router(token: TokenAuth, runner: CliRunner) -> Router {
    let state = match AppState::new(token, runner) {
        Ok(state) => state,
        Err(e) => panic!("embedded catalog is invalid: {e}"),
    };
    let api = Router::new()
        .route("/commands", get(commands_handler))
        .route("/catalog", get(catalog_handler))
        .route("/status", get(status_handler))
        .route("/surface", get(surface_handler))
        .route("/describe", get(describe_handler))
        .route("/files", get(files_handler))
        .route("/atlas", get(atlas_handler))
        .route("/atlas/graph", get(atlas_graph_handler))
        .route("/atlas/schema", get(atlas_schema_handler))
        .route("/atlas/health", get(atlas_health_handler))
        .route("/invoke", post(invoke_handler))
        .route("/exec", post(exec_handler))
        .route("/raw", post(raw_handler))
        .route("/project", get(project_handler).post(project_switch_handler))
        .route("/jobs", get(jobs_list_handler).post(jobs_start_handler))
        .route("/jobs/:id", get(jobs_get_handler))
        .route("/jobs/:id/events", get(jobs_events_handler))
        .route("/jobs/:id/cancel", post(jobs_cancel_handler))
        .route_layer(from_fn_with_state(state.clone(), authenticate));

    Router::new()
        .route("/", get(assets::index))
        .route("/assets/app.css", get(assets::css))
        .route("/assets/js/*file", get(assets::javascript_module))
        .nest("/api", api)
        .with_state(state)
}

async fn commands_handler(State(state): State<AppState>) -> Response {
    let ttl = Duration::from_secs(15);
    if let Some(value) = state.cached("commands", ttl).await {
        return Json(value).into_response();
    }
    match state.discovered().await {
        Ok(commands) => {
            let value = serde_json::to_value(commands).unwrap_or(Value::Null);
            state.remember("commands", value.clone()).await;
            Json(value).into_response()
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "discovery_failed",
            e.to_string(),
        ),
    }
}

async fn catalog_handler(State(state): State<AppState>) -> Response {
    match serde_json::to_value(state.catalog.as_ref()) {
        Ok(value) => Json(value).into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "catalog_error",
            e.to_string(),
        ),
    }
}

async fn jobs_list_handler(State(state): State<AppState>) -> Response {
    jobs::list_handler(State(state.jobs.clone())).await
}

async fn jobs_start_handler(State(state): State<AppState>, body: Json<jobs::StartBody>) -> Response {
    jobs::start_handler(State(state.jobs.clone()), body).await
}

async fn jobs_get_handler(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    jobs::get_handler(State(state.jobs.clone()), Path(id)).await
}

async fn jobs_events_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Query<jobs::EventsQuery>,
) -> Response {
    jobs::events_handler(State(state.jobs.clone()), Path(id), query).await
}

async fn jobs_cancel_handler(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    jobs::cancel_handler(State(state.jobs.clone()), Path(id)).await
}

/// Request body for an in-app project switch.
#[derive(Debug, Deserialize)]
pub struct ProjectBody {
    /// Absolute or relative path to a project directory.
    pub path: String,
}

async fn project_handler(State(state): State<AppState>) -> Response {
    let root = state
        .runner
        .project()
        .or_else(|| std::env::current_dir().ok())
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    Json(json!({ "path": root })).into_response()
}

async fn project_switch_handler(State(state): State<AppState>, Json(body): Json<ProjectBody>) -> Response {
    let candidate = PathBuf::from(&body.path);
    let root = match candidate.canonicalize() {
        Ok(path) if path.is_dir() => path,
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "bad_project",
                format!("{} is not an existing directory", body.path),
            );
        }
    };
    state.runner.set_project(root.clone());
    state.cache.write().await.clear();
    *state.discovered.write().await = None;
    state.helps.write().await.clear();
    Json(json!({ "path": root.to_string_lossy(), "switched": true })).into_response()
}

async fn status_handler(State(state): State<AppState>) -> Response {
    let ttl = Duration::from_secs(5);
    if let Some(value) = state.cached("status", ttl).await {
        return Json(value).into_response();
    }
    match state
        .runner
        .run(&CommandRequest {
            command: vec!["status".to_string()],
            flags: vec!["--json".to_string(), "--offline".to_string()],
        })
        .await
    {
        Ok(response) if response.state == CommandState::Ok => {
            state.remember("status", response.value.clone()).await;
            Json(response.value).into_response()
        }
        Ok(response) => error_response(
            StatusCode::BAD_GATEWAY,
            "cli_failed",
            format!("exit {}: {}", response.exit_code, response.stderr.trim()),
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cli_error",
            e.to_string(),
        ),
    }
}

async fn surface_handler(State(state): State<AppState>) -> Response {
    match build_surface(&state).await {
        Ok(surface) => Json(surface).into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "surface_failed",
            e,
        ),
    }
}

async fn describe_handler(
    State(state): State<AppState>,
    Query(query): Query<DescribeQuery>,
) -> Response {
    if let Some(help) = state.helps.read().await.get(&query.name) {
        return Json(help.clone()).into_response();
    }
    let known = match state.discovered().await {
        Ok(commands) => commands,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "discovery_failed",
                e.to_string(),
            );
        }
    };
    if !known.iter().any(|c| c.name == query.name) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "unknown_command",
            format!("{} is not in the CLI discovery surface", query.name),
        );
    }
    let command: Vec<String> = query.name.split_whitespace().map(str::to_string).collect();
    let response = match state
        .runner
        .run_with_timeout(
            &CommandRequest {
                command,
                flags: vec!["--help".to_string()],
            },
            Duration::from_secs(10),
        )
        .await
    {
        Ok(response) => response,
        Err(e) => {
            return error_response(StatusCode::BAD_GATEWAY, "help_failed", e.to_string());
        }
    };
    let text = response
        .text
        .unwrap_or_else(|| serde_json::to_string(&response.value).unwrap_or_default());
    if text.trim().is_empty() {
        return error_response(StatusCode::BAD_GATEWAY, "help_failed", "empty help output");
    }
    let parsed = help::parse_help(&query.name, &text);
    state
        .helps
        .write()
        .await
        .insert(query.name.clone(), parsed.clone());
    Json(parsed).into_response()
}

async fn files_handler(State(state): State<AppState>, Query(query): Query<FilesQuery>) -> Response {
    let _ = &state;
    if std::env::var("AETHER_WEB_FILES").as_deref() == Ok("0") {
        return error_response(StatusCode::NOT_FOUND, "files_disabled", "path picker disabled");
    }
    let root = match files::default_root() {
        Ok(root) => root,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "bad_root", e),
    };
    match files::list_files(&root, query.q.as_deref().unwrap_or(""), query.limit.unwrap_or(80)) {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "files_failed", e),
    }
}

async fn atlas_handler(
    State(state): State<AppState>,
    Query(query): Query<AtlasQuery>,
) -> Response {
    Json(crate::atlas::build(&state, query.offset).await).into_response()
}

async fn atlas_graph_handler(
    State(state): State<AppState>,
    Query(query): Query<AtlasQuery>,
) -> Response {
    let (graph, source) = crate::atlas::graph(&state, query.offset).await;
    Json(json!({ "graph": graph, "source": source })).into_response()
}

async fn atlas_schema_handler(State(state): State<AppState>) -> Response {
    let (schema, source) = crate::atlas::schema(&state).await;
    Json(json!({ "schema": schema, "source": source })).into_response()
}

async fn atlas_health_handler(State(state): State<AppState>) -> Response {
    let (health, source) = crate::atlas::health(&state).await;
    Json(json!({ "health": health, "source": source })).into_response()
}

async fn invoke_handler(
    State(state): State<AppState>,
    Json(body): Json<InvokeRequest>,
) -> Response {
    let command = match state.catalog.build_command(&body.name, &body.values) {
        Ok(command) => command,
        Err(CatalogError::BadPath(path)) => {
            return error_response(StatusCode::BAD_REQUEST, "bad_path", path);
        }
        Err(CatalogError::MissingArg(arg)) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "missing_argument",
                format!("{arg} is required"),
            );
        }
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, "unknown_command", e.to_string());
        }
    };
    let request = CommandRequest {
        command,
        flags: vec!["--json".to_string(), "--offline".to_string()],
    };
    run_and_render(&state, request).await
}

async fn exec_handler(State(state): State<AppState>, Json(body): Json<ExecRequest>) -> Response {
    if let Err(e) = validate_argv(&body.argv) {
        return error_response(StatusCode::BAD_REQUEST, "invalid_argv", e.to_string());
    }
    let known = match state.discovered().await {
        Ok(commands) => commands,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "discovery_failed",
                e.to_string(),
            );
        }
    };
    let resolved = match resolve_command(&body.argv, &known) {
        Some(command) => command,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "unknown_command",
                "command is not in the CLI discovery surface",
            );
        }
    };
    let used = resolved.name.split_whitespace().count();
    let mut flags: Vec<String> = body.argv[used..].to_vec();
    if body.structured {
        let has_format = flags
            .iter()
            .any(|f| f == "--json" || f == "--format" || f.starts_with("--format="));
        if !has_format {
            flags.push("--json".to_string());
        }
    }
    if !flags.iter().any(|f| f == "--offline") {
        flags.push("--offline".to_string());
    }
    let timeout = body
        .timeout_secs
        .unwrap_or_else(|| {
            std::env::var("AETHER_WEB_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(600)
        })
        .clamp(1, 3600);
    let request = CommandRequest {
        command: body.argv[..used].to_vec(),
        flags,
    };
    run_and_render_with_timeout(&state, request, Duration::from_secs(timeout)).await
}

async fn raw_handler(State(state): State<AppState>, Json(body): Json<RawRequest>) -> Response {
    if body.command.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "empty_command", "command is empty");
    }
    // Legacy surface: restricted to exact discovery names. The terminal uses
    // /api/exec, which also accepts positional arguments.
    let known = match state.discovered().await {
        Ok(commands) => commands,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "discovery_failed",
                e.to_string(),
            );
        }
    };
    let name = body.command.join(" ");
    if !known
        .iter()
        .any(|DiscoveredCommand { name: n, .. }| n == &name)
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "unknown_command",
            format!("{name} is not in the CLI discovery surface"),
        );
    }
    let mut flags = body.flags.clone();
    if !flags.iter().any(|f| f == "--json" || f == "--format") {
        flags.push("--json".to_string());
    }
    if !flags.iter().any(|f| f == "--offline") {
        flags.push("--offline".to_string());
    }
    run_and_render(
        &state,
        CommandRequest {
            command: body.command,
            flags,
        },
    )
    .await
}

async fn run_and_render(state: &AppState, request: CommandRequest) -> Response {
    run_and_render_with_timeout(state, request, state.runner_default_timeout()).await
}

impl AppState {
    fn runner_default_timeout(&self) -> Duration {
        Duration::from_secs(600)
    }
}

async fn run_and_render_with_timeout(
    state: &AppState,
    request: CommandRequest,
    timeout: Duration,
) -> Response {
    match state.runner.run_with_timeout(&request, timeout).await {
        Ok(response) if response.state == CommandState::Ok => Json(response).into_response(),
        Ok(response) => {
            // Blocked / usage errors are still valid results: the browser
            // needs the reason, so return 200 with a non-ok state.
            Json(response).into_response()
        }
        Err(CliError::Timeout(duration)) => error_response(
            StatusCode::GATEWAY_TIMEOUT,
            "timeout",
            format!("command exceeded {duration:?}"),
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cli_error",
            e.to_string(),
        ),
    }
}

/// Render a command response for tests without an HTTP server.
pub async fn invoke_for_test(state: &AppState, request: InvokeRequest) -> Response {
    let command = match state.catalog.build_command(&request.name, &request.values) {
        Ok(command) => command,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid_request", e.to_string());
        }
    };
    run_and_render(
        state,
        CommandRequest {
            command,
            flags: vec!["--json".to_string(), "--offline".to_string()],
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn state() -> AppState {
        let token = TokenAuth::new(b"test-token".to_vec());
        let runner = match crate::cli::find_cli_binary() {
            Some(bin) => CliRunner::with_binary(bin, Duration::from_secs(30)),
            None => CliRunner::with_binary(PathBuf::from("/bin/true"), Duration::from_secs(1)),
        };
        AppState::new(token, runner).unwrap()
    }

    use std::path::PathBuf;

    fn app() -> Router {
        router(
            TokenAuth::new(b"test-token".to_vec()),
            match crate::cli::find_cli_binary() {
                Some(bin) => CliRunner::with_binary(bin, Duration::from_secs(30)),
                None => CliRunner::with_binary(PathBuf::from("/bin/true"), Duration::from_secs(1)),
            },
        )
    }

    async fn body_value(response: Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), 2_000_000)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn fixture(relative: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
            .to_string_lossy()
            .to_string()
    }

    fn post(path: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("Authorization", "Bearer test-token")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn api_requires_token() {
        let response = app()
            .oneshot(Request::builder().uri("/api/status").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_accepts_valid_token_and_serves_status() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .header("Authorization", "Bearer test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn blocked_command_returns_state_blocked() {
        let state = state();
        let response = invoke_for_test(
            &state,
            InvokeRequest {
                name: "classify check".to_string(),
                values: BTreeMap::from([(
                    "file".to_string(),
                    fixture("examples/wingbox/classification-denied.yaml"),
                )]),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let value = body_value(response).await;
        assert_eq!(value["state"], "blocked");
        assert_eq!(value["exit_code"], 30);
    }

    #[tokio::test]
    async fn surface_maps_every_command_to_a_workspace() {
        if crate::cli::find_cli_binary().is_none() {
            return;
        }
        let state = state();
        let surface = build_surface(&state).await.unwrap();
        assert_eq!(surface.workspaces.len(), 10);
        assert!(surface.commands.len() >= 100);
        assert!(surface.commands.iter().all(|c| !c.workspace.is_empty()));
        assert!(surface.commands.iter().any(|c| c.name == "vv coverage"));
        assert!(surface.commands.iter().any(|c| c.group && c.name == "graph"));
    }

    #[tokio::test]
    async fn describe_parses_a_real_command() {
        if crate::cli::find_cli_binary().is_none() {
            return;
        }
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/describe?name=vv%20coverage")
                    .header("Authorization", "Bearer test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value = body_value(response).await;
        assert_eq!(value["name"], "vv coverage");
        assert_eq!(value["positionals"][0]["name"], "FILE");
        assert_eq!(value["positionals"][0]["required"], true);
    }

    fn get(path: &str) -> Request<Body> {
        Request::builder()
            .uri(path)
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn atlas_reports_schema_and_graph_provenance() {
        if crate::cli::find_cli_binary().is_none() {
            return;
        }
        let response = app().oneshot(get("/api/atlas")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value = body_value(response).await;
        assert_eq!(value["schema"]["node_types"].as_array().unwrap().len(), 78);
        assert_eq!(value["schema"]["edge_types"].as_array().unwrap().len(), 38);
        assert_eq!(value["graph"]["offset"], value["chain"]["event_count"]);
        assert_eq!(value["sources"].as_array().unwrap().len(), 6);
        assert!(value["project"]["root"].as_str().is_some());
    }

    #[tokio::test]
    async fn atlas_subroutes_match_the_aggregate() {
        if crate::cli::find_cli_binary().is_none() {
            return;
        }
        let response = app().oneshot(get("/api/atlas/graph?offset=0")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value = body_value(response).await;
        assert_eq!(value["graph"]["offset"], 0);
        assert_eq!(value["graph"]["nodes"].as_array().unwrap().len(), 0);

        let response = app().oneshot(get("/api/atlas/schema")).await.unwrap();
        let value = body_value(response).await;
        assert_eq!(value["schema"]["node_types"].as_array().unwrap().len(), 78);

        let response = app().oneshot(get("/api/atlas/health")).await.unwrap();
        let value = body_value(response).await;
        assert!(value["health"]["checks"].as_array().is_some());
    }

    #[tokio::test]
    async fn atlas_counts_match_the_project_graph() {
        let Some(bin) = crate::cli::find_cli_binary() else { return };
        let root = std::env::temp_dir().join(format!("aether-atlas-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let steps = [
            std::process::Command::new(&bin)
                .arg("init")
                .arg(&root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status(),
            std::process::Command::new(&bin)
                .arg("run")
                .arg("--project")
                .arg(&root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status(),
        ];
        for status in steps {
            match status {
                Ok(s) if s.success() => {}
                _ => {
                    let _ = std::fs::remove_dir_all(&root);
                    return;
                }
            }
        }
        let token = TokenAuth::new(b"test-token".to_vec());
        let runner = CliRunner::with_binary(bin, Duration::from_secs(30)).with_project(root.clone());
        let state = AppState::new(token, runner).unwrap();
        let atlas = crate::atlas::build(&state, None).await;
        assert!(atlas.project.initialized);
        let reported = atlas.chain.get("event_count").and_then(Value::as_u64).unwrap_or(0);
        assert_eq!(atlas.graph.event_count, reported);
        assert!(atlas.graph.node_count >= 3, "{:?}", atlas.graph.node_count);
        assert!(atlas.graph.edge_count >= 2, "{:?}", atlas.graph.edge_count);
        assert_eq!(atlas.graph.nodes.len() as u64, atlas.graph.node_count);
        assert_eq!(atlas.graph.edges.len() as u64, atlas.graph.edge_count);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn exec_accepts_positional_arguments_and_rejects_unknown() {
        if crate::cli::find_cli_binary().is_none() {
            return;
        }
        let response = app()
            .oneshot(post(
                "/api/exec",
                json!({"argv": ["vv", "coverage", fixture("contracts/vv/examples/wingbox.requirements-incomplete.yaml")]}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value = body_value(response).await;
        assert_eq!(value["state"], "blocked");
        assert_eq!(value["exit_code"], 30);

        let response = app()
            .oneshot(post("/api/exec", json!({"argv": ["rm", "-rf", "/"]})))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
