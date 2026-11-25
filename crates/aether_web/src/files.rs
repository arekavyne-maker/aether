// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Read-only, root-confined file discovery for path arguments.
//!
//! The browser never gets arbitrary filesystem access: only the configured
//! project root is walked, symlinks are skipped, and the token middleware
//! still guards the endpoint.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One file or directory offered to the path picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Absolute path, safe to pass to the CLI.
    pub path: String,
    /// Path relative to the project root, for display.
    pub display: String,
    /// `file` or `dir`.
    pub kind: String,
}

const IGNORED: &[&str] = &[
    ".git", ".aether", "target", "node_modules", "vendor", "__pycache__", ".venv", "venv",
    "dist", "build", "logs",
];

/// Default project root: `AETHER_WEB_PROJECT` or the server's working dir.
pub fn default_root() -> Result<PathBuf, String> {
    let raw = std::env::var("AETHER_WEB_PROJECT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    raw.canonicalize()
        .map_err(|e| format!("project root {} cannot be resolved: {e}", raw.display()))
}

/// True when a candidate path stays inside the root.
pub fn within_root(root: &Path, candidate: &Path) -> bool {
    candidate.starts_with(root)
}

fn ignored(name: &str) -> bool {
    name.starts_with('.') || IGNORED.contains(&name)
}

/// Walk the root and return matching entries, newest paths not required.
pub fn list_files(root: &Path, query: &str, limit: usize) -> Result<Vec<FileEntry>, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("invalid root: {e}"))?;
    let needle = query.trim().to_lowercase();
    let cap = limit.clamp(1, 200);
    let mut found: Vec<FileEntry> = Vec::new();
    let mut scanned = 0usize;
    let mut stack = vec![(root.clone(), 0usize)];

    while let Some((dir, depth)) = stack.pop() {
        if scanned >= 5000 || found.len() >= cap * 4 {
            break;
        }
        let read = std::fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        for item in read.flatten() {
            scanned += 1;
            let name = item.file_name().to_string_lossy().to_string();
            if ignored(&name) {
                continue;
            }
            let meta = match std::fs::symlink_metadata(item.path()) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            let path = match item.path().canonicalize() {
                Ok(path) if within_root(&root, &path) => path,
                _ => continue,
            };
            let display = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let kind = if meta.is_dir() { "dir" } else { "file" };
            if needle.is_empty() || display.to_lowercase().contains(&needle) {
                found.push(FileEntry {
                    path: path.to_string_lossy().to_string(),
                    display,
                    kind: kind.to_string(),
                });
            }
            if meta.is_dir() && depth < 4 {
                stack.push((path, depth + 1));
            }
        }
    }

    found.sort_by(|a, b| {
        a.display
            .len()
            .cmp(&b.display.len())
            .then_with(|| a.display.cmp(&b.display))
    });
    found.truncate(cap);
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escaping_paths() {
        let root = Path::new("/tmp/aether-project");
        assert!(within_root(root, &root.join("examples/wingbox/a.yaml")));
        assert!(!within_root(root, Path::new("/tmp/other/a.yaml")));
        assert!(!within_root(root, Path::new("/tmp/aether-project-evil/a.yaml")));
    }

    #[test]
    fn ignores_hidden_and_generated_directories() {
        assert!(ignored(".git"));
        assert!(ignored("target"));
        assert!(ignored("node_modules"));
        assert!(!ignored("examples"));
    }

    #[test]
    fn lists_a_real_directory() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let entries = list_files(&root, "classification-denied", 10).unwrap();
        assert!(entries.iter().any(|e| e.path.contains("classification-denied.yaml")));
        assert!(entries.iter().all(|e| within_root(&root, Path::new(&e.path))));
    }
}
