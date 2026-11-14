// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! CLI golden output tests (G1 conformance).
//!
//! These tests run the `aether` CLI binary end-to-end and check that
//! the JSON output matches expected golden snapshots. The golden files
//! live in `tests/golden/` and are committed alongside the source.

use std::path::PathBuf;
use std::process::Command;

fn find_platform_root() -> PathBuf {
    let mut here = std::env::current_exe().expect("current_exe");
    loop {
        if here.join("Cargo.toml").is_file() && here.join("contracts").is_dir() {
            return here;
        }
        if !here.pop() {
            panic!("could not locate monorepo root");
        }
    }
}

fn canonicalize(value: &serde_json::Value) -> serde_json::Value {
    // Replace fields that are non-deterministic (timestamps, UUIDs) with
    // stable placeholders so the golden comparison is reproducible.
    fn walk(v: &serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Object(m) => {
                let mut out = serde_json::Map::new();
                for (k, vv) in m {
                    out.insert(k.clone(), replace_field(k, walk(vv)));
                }
                serde_json::Value::Object(out)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(walk).collect())
            }
            other => other.clone(),
        }
    }

    fn replace_field(k: &str, v: serde_json::Value) -> serde_json::Value {
        let placeholder_keys = [
            "id",
            "node_id",
            "event_id",
            "record_id",
            "created_at",
            "occurred_at",
            "produced_at",
            "started_at",
            "ended_at",
            "head",
            "chain_head",
        ];
        if placeholder_keys.contains(&k) && v.is_string() {
            return serde_json::Value::String(format!("<{k}>"));
        }
        v
    }

    walk(value)
}

fn assert_golden(label: &str, actual: &str, golden_path: &PathBuf) {
    let actual_value: serde_json::Value =
        serde_json::from_str(actual).unwrap_or_else(|e| panic!("{label}: not JSON: {e}: {actual}"));
    let actual_canonical = canonicalize(&actual_value);
    let actual_str = serde_json::to_string_pretty(&actual_canonical).unwrap();

    if std::env::var("AETHER_UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        std::fs::write(golden_path, &actual_str).unwrap();
        eprintln!("[golden] updated {}", golden_path.display());
        return;
    }

    let golden = match std::fs::read_to_string(golden_path) {
        Ok(g) => g,
        Err(_) => {
            std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
            std::fs::write(golden_path, &actual_str).unwrap();
            panic!(
                "golden file not found; wrote initial golden to {}. Re-run to compare.",
                golden_path.display()
            );
        }
    };
    assert_eq!(
        actual_str.trim(),
        golden.trim(),
        "CLI output differs from golden ({label}). Set AETHER_UPDATE_GOLDEN=1 to update."
    );
}

#[test]
fn cli_graph_query_golden() {
    let root = find_platform_root();
    let aether = root.join("target/debug/aether");
    if !aether.exists() {
        panic!("aether CLI not built; run `cargo build -p aether_cli` first");
    }
    // Use a unique project directory per test so they don't share state.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let project = std::env::temp_dir().join(format!("aether_cli_golden_query_{stamp}"));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).unwrap();

    // Init the project.
    Command::new(&aether)
        .arg("init")
        .arg(&project)
        .arg("--json")
        .output()
        .expect("init");

    // Add two requirements.
    for (text, crit) in [
        ("The wing rib shall sustain the limit load", "catastrophic"),
        ("All fasteners shall be torque-marked", "major"),
    ] {
        Command::new(&aether)
            .args([
                "requirement", "add",
                "--project", project.to_str().unwrap(),
                "--text", text,
                "--verification-method", "test",
                "--criticality", crit,
                "--actor", "user:alice",
                "--json",
            ])
            .output()
            .expect("requirement add");
    }

    // Query.
    let out = Command::new(&aether)
        .args([
            "graph", "query",
            "--project", project.to_str().unwrap(),
            "--type", "Requirement",
            "--limit", "10",
            "--json",
        ])
        .output()
        .expect("query");
    assert!(out.status.success(), "query failed: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

    let golden = root.join("crates/aether_cli/tests/golden/graph_query.json");
    assert_golden("graph_query", &stdout, &golden);
}

#[test]
fn cli_graph_chain_golden() {
    let root = find_platform_root();
    let aether = root.join("target/debug/aether");
    if !aether.exists() {
        panic!("aether CLI not built");
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let project = std::env::temp_dir().join(format!("aether_cli_golden_chain_{stamp}"));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).unwrap();
    Command::new(&aether).arg("init").arg(&project).arg("--json").output().expect("init");
    Command::new(&aether)
        .args([
            "requirement", "add",
            "--project", project.to_str().unwrap(),
            "--text", "first requirement",
            "--verification-method", "analysis",
            "--criticality", "major",
            "--actor", "user:bob",
            "--json",
        ])
        .output()
        .expect("add");
    let out = Command::new(&aether)
        .args(["graph", "chain", "--project", project.to_str().unwrap(), "--json"])
        .output()
        .expect("chain");
    assert!(out.status.success(), "chain failed: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let golden = root.join("crates/aether_cli/tests/golden/graph_chain.json");
    assert_golden("graph_chain", &stdout, &golden);
}
