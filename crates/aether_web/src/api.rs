// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! HTTP API surface.
//!
//! Every `/api/*` route requires a bearer token. Static assets are
//! unauthenticated but contain no engineering data. All command execution
//! goes through [`CliRunner`] and the curated [`Catalog`].

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
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
use crate::catalog::{Catalog, CatalogError};
use crate::cli::{CliError, CliRunner, CommandRequest, CommandResponse, DiscoveredCommand};

/// Shared application state.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Bearer token verifier.
    pub token: Arc<TokenAuth>,
    /// CLI subprocess adapter.
    pub runner: Arc<CliRunner>,
    /// Curated command catalog.
    pub catalog: Arc<Catalog>,
    /// Short-lived response cache.
    pub cache: Arc<RwLock<HashMap<String, (Instant, Value)>>>,
}

impl AppState {
    /// Build state, loading the embedded catalog.
    pub fn new(token: TokenAuth, runner: CliRunner) -> Result<Self, CatalogError> {
        Ok(Self {
            token: Arc::new(token),
            runner: Arc::new(runner),
            catalog: Arc::new(Catalog::load()?),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn cached(&self, key: &str, ttl: Duration) -> Option<Value> {
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

    async fn remember(&self, key: &str, value: Value) {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), (Instant::now(), value));
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
        .route("/invoke", post(invoke_handler))
        .route("/raw", post(raw_handler))
        .route_layer(from_fn_with_state(state.clone(), authenticate));

    Router::new()
        .route("/", get(assets::index))
        .route("/assets/app.css", get(assets::css))
        .route("/assets/app.js", get(assets::javascript))
        .nest("/api", api)
        .with_state(state)
}

async fn commands_handler(State(state): State<AppState>) -> Response {
    let ttl = Duration::from_secs(15);
    if let Some(value) = state.cached("commands", ttl).await {
        return Json(value).into_response();
    }
    match state.runner.discovered().await {
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

async fn status_handler(State(state): State<AppState>) -> Response {
    let ttl = Duration::from_secs(5);
    if let Some(value) = state.cached("status", ttl).await {
        return Json(value).into_response();
    }
    match state
        .runner
        .run(&CommandRequest {
            command: vec!["status".to_string()],
            flags: vec!["--json".to_string()],
        })
        .await
    {
        Ok(response) if response.state == crate::cli::CommandState::Ok => {
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
    let mut flags = vec!["--json".to_string()];
    let kind = state.catalog.get(&body.name).map(|c| c.kind).ok();
    if kind == Some(crate::catalog::CommandKind::Long) {
        flags.push("--offline".to_string());
    }
    let request = CommandRequest { command, flags };
    run_and_render(&state, request).await
}

async fn raw_handler(State(state): State<AppState>, Json(body): Json<RawRequest>) -> Response {
    if body.command.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "empty_command", "command is empty");
    }
    // Restrict to commands the CLI actually advertises. No shell is used,
    // but this prevents the UI being turned into a generic argv launcher.
    let known = match state.runner.discovered().await {
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
    match state.runner.run(&request).await {
        Ok(response) if response.state == crate::cli::CommandState::Ok => {
            Json(response).into_response()
        }
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
            flags: vec!["--json".to_string()],
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

    #[tokio::test]
    async fn api_requires_token() {
        let app = router(
            TokenAuth::new(b"test-token".to_vec()),
            CliRunner::with_binary(PathBuf::from("/bin/true"), Duration::from_secs(1)),
        );
        let response = app
            .oneshot(Request::builder().uri("/api/status").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_accepts_valid_token_and_serves_status() {
        let app = router(
            TokenAuth::new(b"test-token".to_vec()),
            match crate::cli::find_cli_binary() {
                Some(bin) => CliRunner::with_binary(bin, Duration::from_secs(30)),
                None => CliRunner::with_binary(PathBuf::from("/bin/true"), Duration::from_secs(1)),
            },
        );
        let response = app
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

    fn fixture(relative: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
            .to_string_lossy()
            .to_string()
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
        let body = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["state"], "blocked");
        assert_eq!(value["exit_code"], 30);
    }
}