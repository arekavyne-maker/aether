// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! CLI contract regression (spec §12, §12.2).
//!
//! Locks the machine-readable discovery surface: every command must be
//! discoverable, JSON/YAML output must parse, and the versioned contract
//! must declare its exit codes and flags.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aether"))
}

fn json(args: &[&str]) -> serde_json::Value {
    let out = bin().args(args).arg("--json").output().expect("run aether");
    assert!(
        out.status.success(),
        "aether {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("valid JSON on stdout")
}

#[test]
fn discovery_lists_all_commands() {
    let v = json(&["discover", "commands"]);
    assert_eq!(v["contract_version"], "1.0");
    let commands = v["commands"].as_array().expect("command array");
    assert!(commands.len() >= 100, "only {} commands", commands.len());
    let names: Vec<&str> = commands
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    for required in [
        "schema list",
        "schema show",
        "status",
        "plan explain",
        "provenance explain",
        "policy explain",
        "validation report",
        "vv coverage",
        "infer",
        "agent-run",
    ] {
        assert!(names.contains(&required), "missing discovery entry {required}");
    }
}

#[test]
fn command_contract_declares_flags_and_exit_codes() {
    let v = json(&["commands"]);
    assert_eq!(v["contract"], "aether.cli.contract");
    assert_eq!(v["version"], "1.0");
    let flags: Vec<&str> = v["flags"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s.as_str())
        .collect();
    for flag in ["--json", "--dry-run", "--offline", "--explain", "--plan", "--yes"] {
        assert!(flags.contains(&flag), "missing flag {flag}");
    }
    assert_eq!(v["exit_codes"]["30"], "contract violation or fail-closed denial");
}

#[test]
fn yaml_output_is_valid() {
    let out = bin()
        .args(["status", "--format", "yaml"])
        .output()
        .expect("run aether");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_yaml::Value = serde_yaml::from_str(&text).expect("valid YAML");
    assert!(parsed.get("product").is_some(), "{parsed:?}");
}

#[test]
fn project_context_is_discoverable() {
    let v = json(&["discover", "project-context"]);
    assert_eq!(v["cli_contract"], "1.0");
    assert!(v["contract_count"].as_u64().unwrap() >= 25);
}

#[test]
fn completion_is_generated_from_the_command_tree() {
    let out = bin().args(["completion", "bash"]).output().expect("run aether");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("_aether()"), "not a completion script: {text}");
}
