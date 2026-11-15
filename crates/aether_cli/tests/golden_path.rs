// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Phase 2 — golden-path regression (G1 conformance).
//!
//! Exercises the spine end-to-end through the real binary:
//!
//!   aether doctor
//!   aether demo aircraft / structure / reproducibility
//!   aether init my-wing --profile aerospace
//!   aether run
//!   aether explain
//!   aether evidence export
//!   aether benchmark
//!
//! The point is to lock the golden path: if any step regresses, this
//! file fails before a release can claim the path works.

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

fn aether(root: &PathBuf) -> Command {
    let bin = root.join("target/debug/aether");
    assert!(
        bin.exists(),
        "aether CLI not built; run `cargo build -p aether_cli` first"
    );
    Command::new(bin)
}

fn temp_root(label: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aether_golden_path_{label}_{stamp}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn doctor_reports_core_operational() {
    let root = find_platform_root();
    let out = aether(&root)
        .arg("doctor")
        .arg("--json")
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("doctor run");
    assert!(
        out.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("doctor JSON");
    assert_eq!(v["core_operational"], true, "doctor: core not operational");
    let checks = v["checks"].as_array().expect("checks array");
    assert!(checks.len() >= 5, "doctor produced too few probes");
    let names: Vec<String> = checks
        .iter()
        .filter_map(|c| c["name"].as_str().map(|s| s.to_ascii_lowercase()))
        .collect();
    assert!(
        names.iter().any(|n| n.contains("runtime")) && names.iter().any(|n| n.contains("graph")),
        "doctor probe names drifted: {names:?}"
    );
}

#[test]
fn demo_aircraft_runs_deterministically() {
    let root = find_platform_root();
    for kind in ["aircraft", "structure", "reproducibility"] {
        let out = aether(&root)
            .args(["demo", kind, "--json"])
            .env("AETHER_MONOREPO", &root)
            .output()
            .expect("demo run");
        assert!(
            out.status.success(),
            "demo {kind} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("demo {kind}: not JSON ({e}): {stdout}"));
        if kind == "reproducibility" {
            assert_eq!(
                v["reproducible"], true,
                "reproducibility demo did not confirm reproducibility"
            );
        } else {
            assert!(
                v["steps"].as_object().map(|s| !s.is_empty()).unwrap_or(false),
                "demo {kind} reported no steps"
            );
        }
    }
}

#[test]
fn full_golden_path_end_to_end() {
    let root = find_platform_root();
    let envroot = temp_root("env");
    let project = envroot.join("wing-study");

    // aether init wing-study --profile aerospace
    let out = aether(&root)
        .args(["init", project.to_str().unwrap(), "--profile", "aerospace", "--json"])
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("init run");
    assert!(out.status.success(), "init failed: {}", String::from_utf8_lossy(&out.stderr));

    // Scaffold exists.
    assert!(project.join("workflows/wing-baseline.awl.yaml").exists());
    assert!(project.join("experiment.yaml").exists());

    // aether run
    let out = aether(&root)
        .arg("run")
        .arg("--project")
        .arg(&project)
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("run");
    assert!(out.status.success(), "run failed: {}", String::from_utf8_lossy(&out.stderr));
    let run_out: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("run JSON");
    assert_eq!(
        run_out["status"], "succeeded",
        "run reported failure: {run_out}"
    );
    assert!(
        run_out["recorded"].as_array().map(|r| !r.is_empty()).unwrap_or(false),
        "run recorded no engineering results: {run_out}"
    );
    assert!(
        run_out["recorded"][0]["simulation_run"].is_string(),
        "recorded entry missing simulation_run id: {run_out}"
    );

    // aether explain
    let out = aether(&root)
        .arg("explain")
        .arg("--project")
        .arg(&project)
        .arg("--json")
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("explain");
    assert!(out.status.success(), "explain failed");
    let explain: serde_json::Value = serde_json::from_slice(&out.stdout).expect("explain JSON");
    // `explain` returns the top engineering claim(s) with full provenance.
    if explain.is_array() {
        assert!(!explain.as_array().unwrap().is_empty(), "explain found no engineering results");
    } else {
        assert!(
            explain.get("claim").is_some(),
            "explain returned no engineering claim: {explain}"
        );
        assert_eq!(
            explain["validation_status"], "verified",
            "explained claim is not verified: {explain}"
        );
    }

    // aether evidence export
    let out = aether(&root)
        .args(["evidence", "export", "--project", project.to_str().unwrap(), "--json"])
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("evidence export");
    assert!(out.status.success(), "evidence export failed");
    let ev: serde_json::Value = serde_json::from_slice(&out.stdout).expect("evidence JSON");
    assert!(
        ev["records"].as_u64().unwrap_or(0) >= 1,
        "evidence export produced no records: {ev}"
    );
    // The full bundle is written to disk and carries the chain verdict.
    let bundle_path = project.join("evidence/evidence.json");
    assert!(bundle_path.exists(), "evidence bundle not written");
    let bundle: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle_path).unwrap())
            .expect("bundle JSON");
    assert_eq!(
        bundle["graph_chain"]["valid"], true,
        "evidence: graph chain not valid"
    );
    assert!(
        !bundle["records"].as_array().unwrap().is_empty(),
        "bundle contains no records"
    );
}

#[test]
fn benchmarks_execute_for_real() {
    let root = find_platform_root();
    let envroot = temp_root("bench");
    let project = envroot.join("wing-study");
    let out = aether(&root)
        .args(["init", project.to_str().unwrap(), "--profile", "aerospace", "--json"])
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("init run");
    assert!(out.status.success(), "init failed");

    let out = aether(&root)
        .arg("benchmark")
        .env("AETHER_MONOREPO", &root)
        .current_dir(&project)
        .output()
        .expect("benchmark");
    assert!(out.status.success(), "benchmark failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("AETHER-BENCH-01"), "BENCH-01 missing");
    assert!(stdout.contains("AETHER-BENCH-08"), "BENCH-08 missing");
    assert!(stdout.contains("PASS"), "no PASS lines in benchmark output");
}

/// H3 / DEMO 3: the golden path project exports as a self-verifying bundle
/// that replays offline — every hash verified, every workflow re-executed,
/// and any tampering detected.
#[test]
fn export_replay_sovereign_bundle() {
    let root = find_platform_root();
    let envroot = temp_root("sovereign");
    let project = envroot.join("wing-study");

    let out = aether(&root)
        .args(["init", project.to_str().unwrap(), "--profile", "aerospace", "--json"])
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("init run");
    assert!(out.status.success(), "init failed");
    let out = aether(&root)
        .arg("run")
        .arg("--project")
        .arg(&project)
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("run");
    assert!(out.status.success(), "run failed");

    // Export: deterministic bundle with manifest hash and chain head.
    let out = aether(&root)
        .args(["export", "--project", project.to_str().unwrap(), "--json"])
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("export");
    assert!(out.status.success(), "export failed: {}",
        String::from_utf8_lossy(&out.stderr));
    let exp: serde_json::Value = serde_json::from_slice(&out.stdout).expect("export JSON");
    let bundle_dir = project.join(exp["path"].as_str().unwrap());
    assert!(bundle_dir.join("manifest.json").exists(), "bundle missing manifest");

    // Replay: verifies every hash and re-executes the workflow.
    let out = aether(&root)
        .args(["replay", bundle_dir.to_str().unwrap(), "--json"])
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("replay");
    assert!(out.status.success(), "replay failed: {}",
        String::from_utf8_lossy(&out.stderr));
    let rep: serde_json::Value = serde_json::from_slice(&out.stdout).expect("replay JSON");
    assert!(rep["verified"].as_bool().unwrap_or(false),
        "replay did not verify the bundle: {rep}");

    // Tampering must be detected: corrupt one workflow file.
    let wf = std::fs::read_dir(bundle_dir.join("workflows"))
        .expect("workflows dir")
        .next()
        .expect("no workflow in bundle")
        .unwrap()
        .path();
    let original = std::fs::read_to_string(&wf).unwrap();
    std::fs::write(&wf, format!("{original}\n# tampered\n")).unwrap();
    let out = aether(&root)
        .args(["replay", bundle_dir.to_str().unwrap(), "--json"])
        .env("AETHER_MONOREPO", &root)
        .output()
        .expect("replay after tamper");
    assert!(!out.status.success(), "replay accepted a tampered bundle");
}
