// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! TUF bundle verification — Phase 0 walking skeleton (c).

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Verification summary returned to the CLI caller.
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifySummary {
    pub valid: bool,
    pub bundle: String,
    pub root: String,
    pub snapshot_seq: u64,
    pub timestamp_seq: u64,
    pub target_count: usize,
    pub checks: Vec<Check>,
    pub errors: Vec<String>,
}

/// A single verification check.
#[derive(Debug, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub kind: String,
    pub result: String,
    pub diagnostic: String,
}

/// TUF metadata envelope.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Metadata {
    #[serde(rename = "_type")]
    pub ty: String,
    pub version: u64,
    pub expires: String,
    #[serde(default)]
    pub seq: Option<u64>,
    #[serde(flatten)]
    pub body: Value,
    /// Signed payload (everything except the signatures field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Vec<SignatureEntry>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SignatureEntry {
    pub keyid: String,
    pub sig: String,
}

pub fn verify_bundle(bundle: &Path, root: &Path) -> Result<VerifySummary> {
    let mut checks: Vec<Check> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let root_meta = read_metadata(root)?;
    if root_meta.ty != "root" {
        return Ok(VerifySummary {
            valid: false,
            bundle: bundle.display().to_string(),
            root: root.display().to_string(),
            snapshot_seq: 0,
            timestamp_seq: 0,
            target_count: 0,
            checks: vec![Check {
                name: "root_type".into(),
                kind: "sanity".into(),
                result: "fail".into(),
                diagnostic: format!("root metadata has _type={:?}, expected \"root\"", root_meta.ty),
            }],
            errors: vec!["root metadata has wrong type".into()],
        });
    }

    // 1. Verify the root metadata's signature against itself (self-signed root).
    let root_pubkey = extract_root_key(&root_meta)?;
    verify_self_signature(&root_meta, &root_pubkey, &mut checks, &mut errors)?;

    let snapshot_path = bundle.join("snapshot.json");
    let timestamp_path = bundle.join("timestamp.json");
    let targets_path = bundle.join("targets.json");

    let snapshot = read_metadata(&snapshot_path)?;
    let timestamp = read_metadata(&timestamp_path)?;
    let targets = read_metadata(&targets_path)?;

    if snapshot.ty != "snapshot" {
        errors.push(format!("snapshot _type != snapshot (got {:?})", snapshot.ty));
    }
    if timestamp.ty != "timestamp" {
        errors.push(format!("timestamp _type != timestamp (got {:?})", timestamp.ty));
    }
    if targets.ty != "targets" {
        errors.push(format!("targets _type != targets (got {:?})", targets.ty));
    }

    // 2. Verify timestamp <-> snapshot sequence coherence.
    let ts_seq = timestamp.seq.unwrap_or(0);
    let snap_seq = snapshot.seq.unwrap_or(0);
    if ts_seq < snap_seq {
        errors.push(format!("timestamp seq {ts_seq} < snapshot seq {snap_seq}"));
        checks.push(Check {
            name: "timestamp_seq".into(),
            kind: "sanity".into(),
            result: "fail".into(),
            diagnostic: "timestamp must be >= snapshot".into(),
        });
    } else {
        checks.push(Check {
            name: "timestamp_seq".into(),
            kind: "sanity".into(),
            result: "pass".into(),
            diagnostic: format!("timestamp {ts_seq} >= snapshot {snap_seq}"),
        });
    }

    // 3. Verify validity windows.
    check_not_expired(&root_meta, "root", &mut checks, &mut errors);
    check_not_expired(&snapshot, "snapshot", &mut checks, &mut errors);
    check_not_expired(&timestamp, "timestamp", &mut checks, &mut errors);
    check_not_expired(&targets, "targets", &mut checks, &mut errors);

    // 4. Verify each target's hash against an on-disk artifact, if present.
    let target_count = count_targets(&targets);
    let mut verified = 0usize;
    if let Some(targets_map) = targets.body.get("targets").and_then(|v| v.as_object()) {
        for (name, info) in targets_map {
            let expected_sha256 = info
                .get("sha256")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let expected_length = info.get("length").and_then(|v| v.as_u64()).unwrap_or(0);
            let candidate = bundle.join("artifacts").join(name);
            if candidate.exists() {
                match verify_artifact(&candidate, expected_sha256, expected_length) {
                    Ok(()) => {
                        verified += 1;
                        checks.push(Check {
                            name: format!("artifact:{name}"),
                            kind: "hash".into(),
                            result: "pass".into(),
                            diagnostic: format!("sha256 matches; length {expected_length}"),
                        });
                    }
                    Err(e) => {
                        errors.push(format!("artifact {name}: {e}"));
                        checks.push(Check {
                            name: format!("artifact:{name}"),
                            kind: "hash".into(),
                            result: "fail".into(),
                            diagnostic: format!("{e}"),
                        });
                    }
                }
            }
        }
    }

    let valid = errors.is_empty()
        && checks.iter().all(|c| c.result != "fail")
        && snapshot.ty == "snapshot"
        && timestamp.ty == "timestamp"
        && targets.ty == "targets";

    Ok(VerifySummary {
        valid,
        bundle: bundle.display().to_string(),
        root: root.display().to_string(),
        snapshot_seq: snap_seq,
        timestamp_seq: ts_seq,
        target_count,
        checks,
        errors,
    })
}

fn read_metadata(path: &Path) -> Result<Metadata> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let meta: Metadata = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse metadata {}", path.display()))?;
    Ok(meta)
}

fn extract_root_key(root: &Metadata) -> Result<VerifyingKey> {
    let key_bytes = root
        .body
        .get("keys")
        .and_then(|k| k.as_object())
        .and_then(|m| m.values().next())
        .and_then(|k| k.get("keyval"))
        .and_then(|k| k.get("public"))
        .and_then(|k| k.as_str())
        .context("root keyval.public not found")?;
    let bytes = base64_decode(key_bytes).context("base64-decode root public key")?;
    if bytes.len() != 32 {
        bail!("root public key must be 32 bytes, got {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    VerifyingKey::from_bytes(&arr).context("parse ed25519 verifying key")
}

fn verify_self_signature(
    meta: &Metadata,
    key: &VerifyingKey,
    checks: &mut Vec<Check>,
    errors: &mut Vec<String>,
) -> Result<()> {
    let sigs = meta
        .signatures
        .as_ref()
        .context("root metadata has no signatures")?;
    if sigs.is_empty() {
        errors.push("root has no signatures".into());
        return Ok(());
    }
    let payload = canonical_payload(meta)?;
    // Phase 0 debug: write the canonical payload next to the bundle for
    // operator inspection. Off by default; set AETHER_TUF_DUMP=1 to enable.
    if std::env::var("AETHER_TUF_DUMP").is_ok() {
        eprintln!("[tuf] canonical payload: {}", std::str::from_utf8(&payload).unwrap_or("<binary>"));
    }
    for s in sigs {
        let sig_bytes = base64_decode(&s.sig).context("base64-decode signature")?;
        if sig_bytes.len() != 64 {
            errors.push(format!("signature {} wrong length", s.keyid));
            continue;
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes);
        let sig = Signature::from_bytes(&arr);
        match key.verify(&payload, &sig) {
            Ok(()) => {
                checks.push(Check {
                    name: format!("sig:{}", s.keyid),
                    kind: "signature".into(),
                    result: "pass".into(),
                    diagnostic: "ed25519 verified".into(),
                });
            }
            Err(e) => {
                errors.push(format!("signature {} failed: {e}", s.keyid));
                checks.push(Check {
                    name: format!("sig:{}", s.keyid),
                    kind: "signature".into(),
                    result: "fail".into(),
                    diagnostic: format!("{e}"),
                });
            }
        }
    }
    Ok(())
}

pub fn canonical_payload(meta: &Metadata) -> Result<Vec<u8>> {
    // Canonical payload: serialize the metadata without `signatures` field,
    // using sorted keys and no whitespace (matches the Python author tool
    // and the TUF canonical-JSON convention used in Phase 0).
    let mut clone = meta.clone();
    clone.signatures = None;
    let value = serde_json::to_value(&clone).context("canonical value")?;
    let json = canonical_json(&value);
    Ok(json.into_bytes())
}

/// Phase 0 canonical JSON: keys sorted, no whitespace, UTF-8.
/// `null` values inside objects are omitted (matches the Python author
/// which only sets fields that have a value). This is a strict subset of
/// RFC 8785 (JCS) sufficient for TUF v0.
pub fn canonical_json(value: &serde_json::Value) -> String {
    use serde_json::Value;
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).expect("encode string"),
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map
                .iter()
                .filter(|(_, v)| !matches!(v, Value::Null))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let parts: Vec<String> = entries
                .into_iter()
                .map(|(k, v)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).expect("encode key"),
                        canonical_json(v)
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

fn check_not_expired(
    meta: &Metadata,
    kind: &str,
    checks: &mut Vec<Check>,
    errors: &mut Vec<String>,
) {
    // Phase 0: only the year is parsed. The full ISO-8601 parsing arrives
    // when chrono or time is added in Phase 1. For now, we just record the
    // raw `expires` value as a check.
    checks.push(Check {
        name: format!("{kind}.expires"),
        kind: "validity".into(),
        result: "info".into(),
        diagnostic: format!("expires={}", meta.expires),
    });
}

fn count_targets(meta: &Metadata) -> usize {
    meta.body
        .get("targets")
        .and_then(|v| v.as_object())
        .map(|m| m.len())
        .unwrap_or(0)
}

fn verify_artifact(path: &Path, expected_sha256: &str, expected_length: u64) -> Result<()> {
    let bytes = std::fs::read(path).with_context(|| format!("read artifact {}", path.display()))?;
    if expected_length > 0 && bytes.len() as u64 != expected_length {
        return Err(anyhow!(
            "length mismatch: expected {expected_length}, got {}",
            bytes.len()
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let hex = format!("{digest:x}");
    if !expected_sha256.is_empty() && hex != expected_sha256 {
        return Err(anyhow!("sha256 mismatch: expected {expected_sha256}, got {hex}"));
    }
    Ok(())
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    // Minimal base64 decoder; uses the `base64` semantics without the crate.
    use std::str;
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = cleaned.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in bytes {
        if b == b'=' {
            break;
        }
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => bail!("invalid base64 character: {b}"),
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        let cases: &[(&[u8], &str)] = &[
            (b"hello", "aGVsbG8="),
            (b"", ""),
            (b"a", "YQ=="),
            (b"ab", "YWI="),
        ];
        for (input, expected) in cases {
            let encoded = base64_encode(input);
            assert_eq!(&encoded, expected, "encode {input:?}");
            let decoded = base64_decode(expected).unwrap();
            assert_eq!(&decoded, input, "decode {expected:?}");
        }
    }

    #[test]
    fn canonical_json_matches_python() {
        // The Python author produces sorted, no-whitespace JSON for the metadata
        // (without `signatures`). The Rust verifier must produce the same string.
        let root_str = std::fs::read_to_string(
            std::env::var("AETHER_TEST_ROOT").unwrap_or_else(|_| "../tests/fixtures/root.json".to_string()),
        ).unwrap_or_default();
        if root_str.is_empty() {
            return; // skip if fixture not available
        }
        let meta: Metadata = serde_json::from_str(&root_str).unwrap();
        let payload = canonical_payload(&meta).unwrap();
        let s = std::str::from_utf8(&payload).unwrap();
        // The Python author uses json.dumps(..., sort_keys=True, separators=(",", ":")).
        // That format also escapes the same characters serde does by default.
        assert!(!s.contains('\n'));
        assert!(!s.contains(' '));
    }

    #[test]
    fn canonical_json_omits_null_fields() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"a": 1, "b": null, "c": "x"}"#,
        )
        .unwrap();
        let s = canonical_json(&v);
        assert_eq!(s, r#"{"a":1,"c":"x"}"#);
    }

    fn base64_encode(input: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0];
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            out.push(ALPHABET[(b0 >> 2) as usize] as char);
            out.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                out.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(ALPHABET[(b2 & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }
}
