// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! `aether-web` binary entry point.

use anyhow::Result;
use aether_web::{api, auth, cli};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let token = auth::load_or_create_token()?;
    let runner = cli::CliRunner::discover()?;
    let bind = std::env::var("AETHER_WEB_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string());

    let app = api::router(token, runner);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("aether-web listening on http://{bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
