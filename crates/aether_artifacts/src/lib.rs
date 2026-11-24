// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Content-addressed artifact store.
//!
//! Large engineering data (CAD, meshes, fields, scans, telemetry) lives in
//! artifacts, not in the event graph. The graph records hashes and
//! references. This crate provides one integrity path: SHA-256 content
//! addressing, atomic local writes, deduplication, and scoped handles.
//!
//! The `ArtifactStore` trait is deliberately small so object-store and
//! federated adapters can be added without changing callers.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A content-addressed reference to an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Lowercase hex SHA-256 of the artifact bytes.
    pub hash: String,
    /// Size in bytes.
    pub size: u64,
}

impl ArtifactRef {
    /// The sharded relative path used by the local store.
    pub fn shard_path(&self) -> PathBuf {
        PathBuf::from(&self.hash[0..2])
            .join(&self.hash[2..4])
            .join(&self.hash)
    }
}

/// Artifact errors.
#[derive(Debug, Error)]
pub enum ArtifactError {
    /// I/O failure.
    #[error("artifact io: {0}")]
    Io(#[from] std::io::Error),
    /// The artifact does not exist.
    #[error("artifact not found: {0}")]
    NotFound(String),
    /// The bytes do not match the expected hash.
    #[error("artifact integrity failure: expected {expected}, computed {computed}")]
    Integrity {
        /// Expected hash.
        expected: String,
        /// Computed hash.
        computed: String,
    },
    /// Scope denied the operation.
    #[error("artifact scope denied: {0}")]
    ScopeDenied(String),
    /// Malformed hash or path.
    #[error("invalid artifact reference: {0}")]
    Invalid(String),
}

/// Result alias.
pub type ArtifactResult<T> = Result<T, ArtifactError>;

/// Compute the SHA-256 hash of bytes as lowercase hex.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The storage boundary.
pub trait ArtifactStore {
    /// Store bytes and return their content address. Storing the same bytes
    /// twice is idempotent.
    fn put(&self, bytes: &[u8]) -> ArtifactResult<ArtifactRef>;
    /// Read bytes by reference, verifying integrity.
    fn get(&self, reference: &ArtifactRef) -> ArtifactResult<Vec<u8>>;
    /// Whether the artifact exists.
    fn exists(&self, reference: &ArtifactRef) -> bool;
    /// Re-hash stored bytes and compare.
    fn verify(&self, reference: &ArtifactRef) -> ArtifactResult<bool>;
}

/// Local filesystem content-addressed store.
#[derive(Debug, Clone)]
pub struct LocalArtifactStore {
    root: PathBuf,
}

impl LocalArtifactStore {
    /// Open (and create) a store rooted at `root`.
    pub fn open(root: impl AsRef<Path>) -> ArtifactResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Store root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Absolute path for a reference.
    pub fn path_for(&self, reference: &ArtifactRef) -> PathBuf {
        self.root.join(reference.shard_path())
    }

    /// Iterate stored references (used by maintenance and verification).
    pub fn list(&self) -> ArtifactResult<Vec<ArtifactRef>> {
        let mut out = Vec::new();
        for entry in walk(&self.root)? {
            if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
                if name.len() == 64 && name.chars().all(|c| c.is_ascii_hexdigit()) {
                    let size = entry.metadata()?.len();
                    out.push(ArtifactRef {
                        hash: name.to_string(),
                        size,
                    });
                }
            }
        }
        Ok(out)
    }
}

fn walk(dir: &Path) -> ArtifactResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

impl ArtifactStore for LocalArtifactStore {
    fn put(&self, bytes: &[u8]) -> ArtifactResult<ArtifactRef> {
        let hash = hash_bytes(bytes);
        let reference = ArtifactRef {
            hash: hash.clone(),
            size: bytes.len() as u64,
        };
        let path = self.path_for(&reference);
        if path.exists() {
            // Deduplicate, but verify the existing object first.
            if !self.verify(&reference)? {
                return Err(ArtifactError::Integrity {
                    expected: hash.clone(),
                    computed: hash_bytes(&fs::read(&path)?),
                });
            }
            return Ok(reference);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        Ok(reference)
    }

    fn get(&self, reference: &ArtifactRef) -> ArtifactResult<Vec<u8>> {
        let path = self.path_for(reference);
        if !path.exists() {
            return Err(ArtifactError::NotFound(reference.hash.clone()));
        }
        let bytes = fs::read(&path)?;
        let computed = hash_bytes(&bytes);
        if computed != reference.hash {
            return Err(ArtifactError::Integrity {
                expected: reference.hash.clone(),
                computed,
            });
        }
        Ok(bytes)
    }

    fn exists(&self, reference: &ArtifactRef) -> bool {
        self.path_for(reference).exists()
    }

    fn verify(&self, reference: &ArtifactRef) -> ArtifactResult<bool> {
        let path = self.path_for(reference);
        if !path.exists() {
            return Err(ArtifactError::NotFound(reference.hash.clone()));
        }
        Ok(hash_bytes(&fs::read(&path)?) == reference.hash)
    }
}

/// A scoped view of a store. Reads are limited to declared hashes; writes
/// are permitted only if the scope allows them.
#[derive(Debug, Clone)]
pub struct ScopedStore<'a, S: ArtifactStore> {
    store: &'a S,
    readable: BTreeSet<String>,
    writable: bool,
}

impl<'a, S: ArtifactStore> ScopedStore<'a, S> {
    /// Create a read-only scope over specific hashes.
    pub fn read_only(store: &'a S, readable: impl IntoIterator<Item = String>) -> Self {
        Self {
            store,
            readable: readable.into_iter().collect(),
            writable: false,
        }
    }

    /// Create a read-write scope. Write access does not grant reads of
    /// arbitrary existing objects.
    pub fn read_write(store: &'a S, readable: impl IntoIterator<Item = String>) -> Self {
        Self {
            store,
            readable: readable.into_iter().collect(),
            writable: true,
        }
    }

    /// Store bytes through the scope.
    pub fn put(&self, bytes: &[u8]) -> ArtifactResult<ArtifactRef> {
        if !self.writable {
            return Err(ArtifactError::ScopeDenied(
                "store is read-only in this scope".to_string(),
            ));
        }
        self.store.put(bytes)
    }

    /// Read bytes through the scope.
    pub fn get(&self, reference: &ArtifactRef) -> ArtifactResult<Vec<u8>> {
        if !self.readable.contains(&reference.hash) {
            return Err(ArtifactError::ScopeDenied(format!(
                "hash {} is not in the read scope",
                reference.hash
            )));
        }
        self.store.get(reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "aether-artifacts-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn put_get_round_trip_and_dedupe() {
        let root = temp_root("roundtrip");
        let store = LocalArtifactStore::open(&root).unwrap();
        let a = store.put(b"wing-box mesh").unwrap();
        let b = store.put(b"wing-box mesh").unwrap();
        assert_eq!(a, b);
        assert_eq!(store.get(&a).unwrap(), b"wing-box mesh");
        assert!(store.exists(&a));
        assert_eq!(store.list().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tamper_is_detected() {
        let root = temp_root("tamper");
        let store = LocalArtifactStore::open(&root).unwrap();
        let reference = store.put(b"original").unwrap();
        fs::write(store.path_for(&reference), b"tampered").unwrap();
        assert!(matches!(
            store.get(&reference),
            Err(ArtifactError::Integrity { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_artifact_is_an_error() {
        let root = temp_root("missing");
        let store = LocalArtifactStore::open(&root).unwrap();
        let reference = ArtifactRef {
            hash: "0".repeat(64),
            size: 0,
        };
        assert!(matches!(
            store.get(&reference),
            Err(ArtifactError::NotFound(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scope_denies_unlisted_read_and_all_writes() {
        let root = temp_root("scope");
        let store = LocalArtifactStore::open(&root).unwrap();
        let reference = store.put(b"input").unwrap();
        let scope = ScopedStore::read_only(&store, [reference.hash.clone()]);
        assert_eq!(scope.get(&reference).unwrap(), b"input");
        assert!(matches!(
            scope.put(b"output"),
            Err(ArtifactError::ScopeDenied(_))
        ));

        let other = ArtifactRef {
            hash: "f".repeat(64),
            size: 0,
        };
        assert!(matches!(
            scope.get(&other),
            Err(ArtifactError::ScopeDenied(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
