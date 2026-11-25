// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Declarative command catalog for the local web interface.
//!
//! This is interface metadata only. The CLI binary remains the single
//! source of truth for behaviour; the catalog tells the UI how to render
//! arguments for a curated subset of the discovered command surface.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// How an argument is rendered in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    /// Free text input.
    Text,
    /// Local file path, validated for existence on the server.
    Path,
}

/// An argument in a curated command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgSpec {
    /// Token name, e.g. `--project` or `id`.
    pub name: String,
    /// Human label.
    pub label: String,
    /// Rendering kind.
    pub kind: ArgKind,
    /// Whether the argument is required.
    pub required: bool,
}

/// Interaction class of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    /// Read-only command.
    Read,
    /// Command requiring explicit human approval intent.
    Approval,
    /// Long-running command; use streaming where available.
    Long,
}

/// A curated command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandSpec {
    /// Full command name, e.g. `schema list`.
    pub name: String,
    /// Help text.
    pub about: String,
    /// Workspace id.
    pub workspace: String,
    /// Interaction class.
    pub kind: CommandKind,
    /// Argument templates in argv order.
    #[serde(default)]
    pub args: Vec<ArgSpec>,
}

/// A UI workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    /// Stable id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Description.
    pub description: String,
}

/// The full catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    /// Catalog version.
    pub version: String,
    /// Workspaces in display order.
    pub workspaces: Vec<Workspace>,
    /// Curated commands.
    pub commands: Vec<CommandSpec>,
}

/// Catalog errors.
#[derive(Debug, Error)]
pub enum CatalogError {
    /// The embedded catalog could not be parsed.
    #[error("invalid embedded catalog: {0}")]
    Parse(#[from] serde_json::Error),
    /// A requested command is not curated.
    #[error("unknown command {0}")]
    Unknown(String),
    /// A required argument is missing.
    #[error("missing required argument {0}")]
    MissingArg(String),
    /// A path argument does not exist on this host.
    #[error("path {0} does not exist")]
    BadPath(String),
}

const CATALOG_JSON: &str = include_str!("../catalog.json");

impl Catalog {
    /// Parse the embedded catalog.
    pub fn load() -> Result<Self, CatalogError> {
        Ok(serde_json::from_str(CATALOG_JSON)?)
    }

    /// Look up a curated command.
    pub fn get(&self, name: &str) -> Result<&CommandSpec, CatalogError> {
        self.commands
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| CatalogError::Unknown(name.to_string()))
    }

    /// Commands belonging to a workspace.
    pub fn for_workspace(&self, id: &str) -> Vec<&CommandSpec> {
        self.commands.iter().filter(|c| c.workspace == id).collect()
    }

    /// Build argv from a curated command and supplied argument values.
    ///
    /// Values are passed as discrete argv elements; no shell is involved.
    pub fn build_command(
        &self,
        name: &str,
        values: &std::collections::BTreeMap<String, String>,
    ) -> Result<Vec<String>, CatalogError> {
        let spec = self.get(name)?;
        let mut argv: Vec<String> = spec.name.split_whitespace().map(str::to_string).collect();
        for arg in &spec.args {
            let value = values.get(&arg.name);
            match value {
                Some(v) if !v.is_empty() => {
                    if arg.kind == ArgKind::Path && !std::path::Path::new(v).exists() {
                        return Err(CatalogError::BadPath(v.clone()));
                    }
                    argv.push(v.clone());
                }
                _ if arg.required => return Err(CatalogError::MissingArg(arg.name.clone())),
                _ => {}
            }
        }
        Ok(argv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn catalog_parses() {
        let catalog = Catalog::load().unwrap();
        assert_eq!(catalog.workspaces.len(), 9);
        assert!(catalog.commands.len() >= 40);
    }

    #[test]
    fn build_command_no_args() {
        let catalog = Catalog::load().unwrap();
        let argv = catalog.build_command("status", &BTreeMap::new()).unwrap();
        assert_eq!(argv, vec!["status"]);
    }

    #[test]
    fn build_command_with_args() {
        let catalog = Catalog::load().unwrap();
        let values = BTreeMap::from([("id".to_string(), "aether.asm".to_string())]);
        let argv = catalog.build_command("schema show", &values).unwrap();
        assert_eq!(argv, vec!["schema", "show", "aether.asm"]);
    }

    #[test]
    fn missing_required_arg_is_rejected() {
        let catalog = Catalog::load().unwrap();
        assert!(catalog.build_command("schema show", &BTreeMap::new()).is_err());
    }

    #[test]
    fn bad_path_is_rejected() {
        let catalog = Catalog::load().unwrap();
        let values = BTreeMap::from([(
            "file".to_string(),
            "/definitely/not/a/real/file.yaml".to_string(),
        )]);
        assert!(matches!(
            catalog.build_command("design evaluate", &values),
            Err(CatalogError::BadPath(_))
        ));
    }
}