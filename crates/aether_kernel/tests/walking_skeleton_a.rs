// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Walking skeleton (a) — stdio JSON-RPC substrate test.
//!
//! Spawns the `aether_kernel` binary against the reference Python plugin
//! and asserts the round-trip is successful. This is the executable form
//! of the IPC substrate test required by SDP §18 Phase 0.

use std::process::Command;

fn find_platform_root() -> std::path::PathBuf {
    // Prefer the env var that cargo sets for integration tests.
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        if let Some(root) = std::path::Path::new(&manifest)
            .parent()
            .and_then(|p| p.parent())
        {
            return root.to_path_buf();
        }
    }
    // Fallback: walk upwards from the current executable until we find Cargo.toml.
    let mut here = std::env::current_exe().expect("current_exe");
    loop {
        if here.join("Cargo.toml").is_file() && here.join("contracts").is_dir() {
            return here;
        }
        if !here.pop() {
            panic!("could not locate monorepo root from {}", here.display());
        }
    }
}

#[test]
fn stdio_jsonrpc_round_trip() {
    let platform_root = find_platform_root();
    let kernel_bin = platform_root.join("target/debug/aether_kernel");
    if !kernel_bin.exists() {
        eprintln!(
            "kernel binary not built at {}; run `cargo build -p aether_kernel` first",
            kernel_bin.display()
        );
        panic!("kernel binary not built");
    }

    let plugin_path = platform_root.join(
        "reference_plugins/cfd-reference-minimal/python/stdio_plugin.py",
    );

    let output = Command::new(&kernel_bin)
        .arg("python3")
        .arg(&plugin_path)
        .env("RUST_LOG", "warn")
        .output()
        .expect("failed to execute aether_kernel");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "kernel exited non-zero: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        output.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains("walking-skeleton-(a): stdio JSON-RPC substrate OK"),
        "missing success line in stdout:\n{stdout}"
    );
}
