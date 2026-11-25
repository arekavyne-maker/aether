// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! `aether-web` binary entry point.

use std::io::IsTerminal;

use anyhow::{Result, anyhow};

use aether_web::{api, auth, cli};

fn validate_bind(bind: &str) -> Result<std::net::IpAddr> {
    let address: std::net::SocketAddr = bind
        .parse()
        .map_err(|_| anyhow!("AETHER_WEB_BIND must be an address like 127.0.0.1:8787"))?;
    if !address.ip().is_loopback() && std::env::var("AETHER_WEB_ALLOW_REMOTE").as_deref() != Ok("1") {
        return Err(anyhow!(
            "refusing non-loopback bind {}; set AETHER_WEB_ALLOW_REMOTE=1 only with an independent security review",
            address.ip()
        ));
    }
    Ok(address.ip())
}

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
    let ip = validate_bind(&bind)?;
    if !ip.is_loopback() {
        tracing::warn!("non-loopback bind explicitly enabled: {bind}");
    }

    let app = api::router(token.clone(), runner);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("aether-web listening on http://{bind}");

    // Print the secret-bearing URL only on an interactive terminal. Service
    // logs must never contain the bearer token.
    if std::io::stdout().is_terminal() {
        if std::env::var("AETHER_WEB_TOKEN").is_ok() {
            println!("Open http://{bind}/#token=<AETHER_WEB_TOKEN>");
        } else {
            println!("Open http://{bind}/#token={}", token.as_str());
        }
    } else {
        println!(
            "Open http://{bind}/#token=<contents of {}>",
            auth::token_path()?.display()
        );
    }

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_is_required_by_default() {
        assert!(validate_bind("127.0.0.1:8787").is_ok());
        assert!(validate_bind("[::1]:8787").is_ok());
        // The test process inherits no AETHER_WEB_ALLOW_REMOTE in CI; if an
        // operator sets it, the override is intentional and logged.
        if std::env::var("AETHER_WEB_ALLOW_REMOTE").as_deref() != Ok("1") {
            assert!(validate_bind("0.0.0.0:8787").is_err());
        }
    }
}
