// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Rust contract validation harness CLI (G1 deliverable: two-impl rule).
//!
//! Usage:
//!   aether-contract-validate                  # validate all contract examples
//!   aether-contract-validate --contract X --schema-id Y --document path.json
//!   aether-contract-validate --json          # JSON output

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;

use aether_contract_validate::{find_monorepo_root, Harness, ValidationResult};
use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(name = "aether-contract-validate", about = "Aether contract validation (Rust)")]
struct Args {
    /// Validate every example document in every contract's `examples/`.
    #[arg(long, default_value_t = true)]
    examples: bool,
    /// Validate a specific document.
    #[arg(long)]
    document: Option<PathBuf>,
    /// Contract id.
    #[arg(long)]
    contract: Option<String>,
    /// Schema id (the `$id` of the JSON Schema).
    #[arg(long = "schema-id")]
    schema_id: Option<String>,
    /// Output JSON (default is text).
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    harness: String,
    results: Vec<ValidationResult>,
    failures: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = find_monorepo_root().context("find monorepo")?;
    let harness = Harness::from_monorepo(&root).context("build harness")?;

    let mut results: Vec<ValidationResult> = Vec::new();

    if let (Some(document), Some(contract), Some(schema_id)) =
        (args.document.as_ref(), args.contract.as_ref(), args.schema_id.as_ref())
    {
        let raw = std::fs::read_to_string(document)
            .with_context(|| format!("read {}", document.display()))?;
        let value: Value = if document.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&raw)
                .with_context(|| format!("parse JSON {}", document.display()))?
        } else {
            serde_yaml::from_str(&raw)
                .with_context(|| format!("parse YAML {}", document.display()))?
        };
        let mut r = harness.validate(contract, schema_id, &value)?;
        r.document_path = document.display().to_string();
        results.push(r);
    } else     if args.examples {
        results = harness.validate_examples(&root);
    }

    let failures = results.iter().filter(|r| !r.valid).count();
    let report = Report {
        harness: format!("aether-contract-validate 0.1.0 (Rust)"),
        results: results.clone(),
        failures,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        for r in &results {
            let mark = if r.valid { "OK " } else { "FAIL" };
            println!(
                "[{mark}] {contract} :: {schema} :: {path}",
                contract = r.contract,
                schema = r.schema_path,
                path = r.document_path
            );
            for e in &r.errors {
                println!("       {e}");
            }
        }
        println!();
        println!(
            "{pass} passed, {fail} failed (total {total})",
            pass = results.len() - failures,
            fail = failures,
            total = results.len()
        );
    }
    if failures > 0 {
        std::process::exit(10); // validation (SDP §15.1)
    }
    Ok(())
}
