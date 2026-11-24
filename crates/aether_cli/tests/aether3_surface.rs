// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Aether 3.0 substrate-surface regression.
//!
//! Locks the Phase 1 primitives exposed through the CLI: schema discovery,
//! fail-closed policy, configuration snapshots, evidence gates, and the
//! honest status line.

use std::path::PathBuf;
use std::process::Command;

fn platform_root() -> PathBuf {
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

fn aether() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aether"))
}

fn temp_root(label: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aether3_{label}_{stamp}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_json(mut command: Command) -> serde_json::Value {
    let out = command.output().expect("command runs");
    assert!(
        out.status.success(),
        "command failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

#[test]
fn schema_list_exposes_new_primitives() {
    let root = platform_root();
    let v = run_json({
        let mut c = aether();
        c.arg("schema")
            .arg("list")
            .arg("--project")
            .arg(&root)
            .arg("--json")
            .env("AETHER_MONOREPO", &root);
        c
    });
    let contracts: Vec<String> = v["contracts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["id"].as_str().map(str::to_string))
        .collect();
    for required in [
        "aether.quantities",
        "aether.frames",
        "aether.asm",
        "aether.evidence",
        "aether.model-card",
    ] {
        assert!(contracts.contains(&required.to_string()), "missing {required}");
    }
    let nodes: Vec<String> = v["graph_node_types"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n.as_str().map(str::to_string))
        .collect();
    for required in [
        "ConfigurationSnapshot",
        "VerificationCase",
        "MaterialBatch",
        "CandidateDesign",
    ] {
        assert!(nodes.contains(&required.to_string()), "missing node {required}");
    }
    assert!(v["graph_node_count"].as_u64().unwrap() >= 60);
}

#[test]
fn policy_is_fail_closed() {
    let denied = run_json({
        let mut c = aether();
        c.arg("policy")
            .arg("explain")
            .arg("graph.mutate")
            .arg("--role")
            .arg("quality-auditor")
            .arg("--json");
        c
    });
    assert_eq!(denied["allow"], false);

    let unknown = run_json({
        let mut c = aether();
        c.arg("policy")
            .arg("explain")
            .arg("graph.teleport")
            .arg("--role")
            .arg("admin")
            .arg("--json");
        c
    });
    assert_eq!(unknown["allow"], false);

    let allowed = run_json({
        let mut c = aether();
        c.arg("policy")
            .arg("explain")
            .arg("graph.mutate")
            .arg("--role")
            .arg("project-engineer")
            .arg("--json");
        c
    });
    assert_eq!(allowed["allow"], true);
}

#[test]
fn snapshot_round_trip_and_change_detection() {
    let root = platform_root();
    let work = temp_root("snapshot");
    let source = root.join("examples/wingbox/configuration.asm.yaml");
    let snapshot = work.join("snapshot.json");

    run_json({
        let mut c = aether();
        c.arg("config")
            .arg("snapshot")
            .arg(&source)
            .arg("--output")
            .arg(&snapshot)
            .arg("--json");
        c
    });
    assert!(snapshot.is_file());

    let verified = run_json({
        let mut c = aether();
        c.arg("config")
            .arg("verify")
            .arg(&snapshot)
            .arg(&source)
            .arg("--json");
        c
    });
    assert_eq!(verified["valid"], true);

    let changed = work.join("changed.asm.yaml");
    let text = std::fs::read_to_string(&source).unwrap().replace(
        "thickness_mm: 2.4",
        "thickness_mm: 2.9",
    );
    std::fs::write(&changed, text).unwrap();
    let out = aether()
        .arg("config")
        .arg("verify")
        .arg(&snapshot)
        .arg(&changed)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(30), "changed input must fail closed");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["valid"], false);
    std::fs::remove_dir_all(work).unwrap();
}

#[test]
fn evidence_gate_fails_closed_without_credible_validation() {
    let root = platform_root();
    let work = temp_root("evidence");
    let gate = root.join("examples/wingbox/release-gate.yaml");

    let passed = run_json({
        let mut c = aether();
        c.arg("evidence").arg("gate").arg(&gate).arg("--json");
        c
    });
    assert_eq!(passed["passed"], true);

    let text = std::fs::read_to_string(&gate).unwrap();
    let mut doc: serde_json::Value = serde_yaml::from_str(&text).unwrap();
    let evidence = doc["evidence"].as_array_mut().unwrap();
    evidence.retain(|item| item["class"] != "validation");
    let incomplete = work.join("incomplete.yaml");
    std::fs::write(&incomplete, serde_yaml::to_string(&doc).unwrap()).unwrap();
    let out = aether()
        .arg("evidence")
        .arg("gate")
        .arg(&incomplete)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(30));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["passed"], false);
    assert!(v["missing"].as_array().unwrap().iter().any(|m| m
        .as_str()
        .unwrap()
        .contains("validation")));
    std::fs::remove_dir_all(work).unwrap();
}

#[test]
fn status_does_not_claim_unsupported_capabilities() {
    let root = platform_root();
    let v = run_json({
        let mut c = aether();
        c.arg("status")
            .arg("--project")
            .arg(&root)
            .arg("--json")
            .env("AETHER_MONOREPO", &root);
        c
    });
    assert_eq!(v["plugin_sandbox"], "not_implemented");
    assert_eq!(v["agent_runtime"], "not_implemented");
    assert_eq!(v["certification_claim"], false);
    assert_eq!(v["policy"], "fail_closed_rbac");
}
