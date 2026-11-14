// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Walking skeleton (c) — offline TUF bundle round-trip test.
//!
//! Invokes the Python `scripts/author_tuf_bundle.py` to build a fresh
//! TUF-style bundle, then runs the Rust verifier against it. The
//! round-trip proves that the Python author and the Rust verifier agree
//! on canonical JSON and on the Ed25519 signature semantics (ADR-0021).

use std::path::PathBuf;
use std::process::Command;

fn find_platform_root() -> std::path::PathBuf {
    // For integration tests, the binary lives in target/debug/deps/ while
    // the test source lives in crates/<name>/tests/. Walking up from the
    // current_exe is the most reliable way to find the monorepo root.
    let mut here = std::env::current_exe().expect("current_exe");
    loop {
        // The monorepo root has both Cargo.toml and contracts/.
        if here.join("Cargo.toml").is_file() && here.join("contracts").is_dir() {
            return here;
        }
        if !here.pop() {
            panic!("could not locate monorepo root from {}", here.display());
        }
    }
}

fn venv_python(root: &std::path::Path) -> PathBuf {
    let venv = root.join("python/.venv/bin/python");
    if venv.is_file() {
        return venv;
    }
    PathBuf::from("python3")
}

#[test]
fn tuf_author_verify_round_trip() {
    let root = find_platform_root();
    eprintln!("[test] platform root: {}", root.display());
    let aether_bin = root.join("target/debug/aether");
    eprintln!("[test] aether bin: {} (exists: {})", aether_bin.display(), aether_bin.exists());
    if !aether_bin.exists() {
        panic!("aether CLI not built; run `cargo build -p aether_cli` first");
    }
    let python = venv_python(&root);

    // Stage an artifact to bundle.
    let tmp = std::env::temp_dir().join("aether_tuf_roundtrip");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let artifact = tmp.join("hello.txt");
    std::fs::write(&artifact, b"walking skeleton (c) artifact\n").unwrap();
    let bundle = tmp.join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();

    // Author a bundle.
    let author_out = Command::new(&python)
        .arg(root.join("scripts/author_tuf_bundle.py"))
        .arg("--out")
        .arg(&bundle)
        .arg("--artifact")
        .arg(format!("hello.txt={}", artifact.display()))
        .arg("--seq")
        .arg("3")
        .output()
        .expect("run author");
    assert!(
        author_out.status.success(),
        "author failed: {}\nstdout: {}\nstderr: {}",
        author_out.status,
        String::from_utf8_lossy(&author_out.stdout),
        String::from_utf8_lossy(&author_out.stderr)
    );

    // Verify the bundle.
    let verify_out = Command::new(&aether_bin)
        .arg("verify-tuf")
        .arg(&bundle)
        .arg(bundle.join("root.json"))
        .output()
        .expect("run verify");
    let stdout = String::from_utf8_lossy(&verify_out.stdout);
    assert!(
        verify_out.status.success(),
        "verify failed: {}\nstdout: {}\nstderr: {}",
        verify_out.status,
        stdout,
        String::from_utf8_lossy(&verify_out.stderr)
    );
    assert!(
        stdout.contains("\"valid\": true"),
        "expected valid:true in verify output, got: {stdout}"
    );
    assert!(
        stdout.contains("hello.txt"),
        "expected artifact:hello.txt check in output: {stdout}"
    );
}
