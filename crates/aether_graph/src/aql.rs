// SPDX-License-Identifier: Aether-VPSL-1.0
// SPDX-FileCopyrightText: 2026 The Aether Consortium
//! Canonical JSON for graph event hashing.
//!
//! The Aether event chain is reproducible across implementations, so the
//! canonical form must be deterministic: keys sorted, no whitespace,
//! UTF-8, and `null` object fields omitted (matches the Python `json`
//! library's `json.dumps(..., sort_keys=True, separators=(",", ":"))`
//! and the same canonicalization the TUF verifier uses in `aether_cli`).
//!
//! This is a strict subset of RFC 8785 (JCS) sufficient for Aether v0.

#![deny(unsafe_code)]

use serde_json::Value;

/// Render `value` in the Aether canonical JSON form.
pub fn canonical_json(value: &Value) -> String {
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
