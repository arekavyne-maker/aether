// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Phase 2 — project-local engineering state (sovereign projects).
//!
//! Verifies the state-location contract of `engine_bridge::open_engine`:
//!
//! 1. A standalone project (outside the monorepo) stores its graph DB
//!    at `<project>/.aether/state/graph.db` — the project directory is
//!    self-contained and can be exported/replayed offline.
//! 2. `AETHER_STATE_DIR` redirects state when set.
//! 3. Graph writes and reads work on the standalone project (a
//!    requirement added is queryable afterwards).

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

fn temp_project(label: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let project = std::env::temp_dir().join(format!("aether_project_state_{label}_{stamp}"));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).unwrap();
    project
}

fn aether(root: &PathBuf) -> Command {
    let bin = root.join("target/debug/aether");
    assert!(
        bin.exists(),
        "aether CLI not built; run `cargo build -p aether_cli` first"
    );
    Command::new(bin)
}

#[test]
fn standalone_project_state_is_project_local() {
    let root = find_platform_root();
    let project = temp_project("local");

    // `aether init` on a directory outside the monorepo.
    let out = aether(&root)
        .arg("init")
        .arg(&project)
        .arg("--json")
        .output()
        .expect("init run");
    assert!(out.status.success(), "init failed: {}", out.status);

    // State is created lazily on first graph operation; the DB must
    // then live inside the project, not in the monorepo.
    let db = project.join(".aether/state/graph.db");

    // Adding a requirement must not scatter state back into the monorepo.
    let out = aether(&root)
        .args([
            "requirement",
            "add",
            "--project",
            project.to_str().unwrap(),
            "--text",
            "The wing shall carry 2.5g limit load without failure",
            "--json",
        ])
        .output()
        .expect("requirement add run");
    assert!(
        out.status.success(),
        "requirement add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The requirement is queryable from the project-local state.
    let out = aether(&root)
        .args([
            "graph",
            "query",
            "--type",
            "Requirement",
            "--project",
            project.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("graph query run");
    assert!(out.status.success(), "graph query failed");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("query JSON");
    assert_eq!(v["count"], 1, "expected exactly one Requirement node");

    assert!(db.exists(), "project-local graph DB missing at {}", db.display());
}

#[test]
fn aether_state_dir_override_redirects_state() {
    let root = find_platform_root();
    let project = temp_project("override");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let state_dir = std::env::temp_dir().join(format!("aether_state_dir_{stamp}"));
    let _ = std::fs::remove_dir_all(&state_dir);

    let out = aether(&root)
        .arg("init")
        .arg(&project)
        .arg("--json")
        .output()
        .expect("init run");
    assert!(out.status.success(), "init failed: {}", out.status);

    let out = aether(&root)
        .args([
            "requirement",
            "add",
            "--project",
            project.to_str().unwrap(),
            "--text",
            "Override target state dir",
            "--json",
        ])
        .env("AETHER_STATE_DIR", &state_dir)
        .output()
        .expect("requirement add run");
    assert!(out.status.success(), "requirement add failed: {}", String::from_utf8_lossy(&out.stderr));

    let entries = std::fs::read_dir(&state_dir).expect("state dir readable");
    let count = entries.count();
    assert!(count >= 1, "AETHER_STATE_DIR override produced no state");
    assert!(!project.join(".aether/state/graph.db").exists(),
        "project DB created despite AETHER_STATE_DIR override");
}
