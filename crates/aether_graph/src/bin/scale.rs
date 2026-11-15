// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Walking skeleton (b) binary: SQLite event-sourced graph at 10M-node scale.

use std::path::PathBuf;
use std::time::Duration;

use aether_graph::{ScaleTestConfig, run_scale_test};
use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "aether_graph_scale", about = "Phase 0 walking skeleton (b)")]
struct Args {
    /// Number of node events to insert.
    #[arg(long, default_value_t = 10_000_000)]
    nodes: usize,

    /// Insert batch size.
    #[arg(long, default_value_t = 5_000)]
    batch: usize,

    /// Output DB path; if omitted, runs against :memory:.
    #[arg(long)]
    db: Option<PathBuf>,

    /// Maximum wall-clock seconds; if exceeded, exits non-zero.
    #[arg(long, default_value_t = 300.0)]
    budget_s: f64,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let args = Args::parse();
    let config = ScaleTestConfig {
        node_count: args.nodes,
        batch_size: args.batch,
        db_path: args.db.clone(),
        ..Default::default()
    };

    info!(?config, "starting scale test");
    let result = run_scale_test(&config).context("scale test")?;
    println!("{}", serde_json::to_string_pretty(&result)?);

    let total = Duration::from_secs_f64(result.total_seconds);
    if total > Duration::from_secs_f64(args.budget_s) {
        anyhow::bail!(
            "scale test exceeded budget: {:.1}s > {:.1}s",
            total.as_secs_f64(),
            args.budget_s
        );
    }
    Ok(())
}
