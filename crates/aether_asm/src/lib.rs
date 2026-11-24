// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Aerospace Systems Model (ASM) helpers.
//!
//! ASM is a versioned semantic layer, not a CAD kernel or PLM database.
//! The canonical schemas live in `contracts/asm/`; this crate provides the
//! shared invariants that every domain service and plugin must obey:
//! stable object identity, revision lineage, lifecycle transitions,
//! interface compatibility, and configuration snapshots.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A stable ASM object id. Revisions of the same object keep this id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub String);

impl ObjectId {
    /// Construct an id.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ASM object kinds that carry revision and lifecycle semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsmKind {
    /// Programme or project.
    Program,
    /// Aircraft configuration.
    AircraftConfiguration,
    /// Product (a deliverable system).
    Product,
    /// Assembly.
    Assembly,
    /// Part.
    Part,
    /// Interface between two objects.
    Interface,
    /// Variant.
    Variant,
    /// Configuration baseline.
    ConfigurationBaseline,
    /// Change request.
    ChangeRequest,
    /// Effectivity statement.
    Effectivity,
    /// Requirement.
    Requirement,
    /// Verification case.
    VerificationCase,
    /// Validation case.
    ValidationCase,
    /// Material assignment.
    MaterialAssignment,
    /// Environment case.
    EnvironmentCase,
}

/// Lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Work in progress; mutable.
    Draft,
    /// Under review; mutable only by reviewers.
    InReview,
    /// Released; immutable.
    Released,
    /// Replaced by a later revision.
    Superseded,
    /// Withdrawn from use.
    Retired,
}

impl LifecycleState {
    /// Whether content may change in this state.
    pub fn is_mutable(self) -> bool {
        matches!(self, LifecycleState::Draft | LifecycleState::InReview)
    }
}

/// ASM errors.
#[derive(Debug, Error, PartialEq)]
pub enum AsmError {
    /// Revision is immutable.
    #[error("revision {revision} of {id} is {state:?} and cannot be modified")]
    Immutable {
        /// Object id.
        id: String,
        /// Revision number.
        revision: u32,
        /// Current state.
        state: LifecycleState,
    },
    /// Illegal lifecycle transition.
    #[error("illegal lifecycle transition for {id}: {from:?} -> {to:?}")]
    IllegalTransition {
        /// Object id.
        id: String,
        /// From state.
        from: LifecycleState,
        /// To state.
        to: LifecycleState,
    },
    /// Unknown or duplicate revision.
    #[error("revision error: {0}")]
    Revision(String),
    /// Interfaces are incompatible.
    #[error("interface incompatible: {0}")]
    InterfaceIncompatible(String),
    /// Snapshot integrity failure.
    #[error("snapshot integrity failure: {0}")]
    Snapshot(String),
}

/// Result alias.
pub type AsmResult<T> = Result<T, AsmError>;

/// One immutable revision of an ASM object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    /// Revision number, starting at 1.
    pub number: u32,
    /// Lifecycle state.
    pub state: LifecycleState,
    /// Author identity.
    pub author: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// SHA-256 of the canonical content.
    pub content_hash: String,
    /// Previous revision number, if any.
    pub supersedes: Option<u32>,
}

/// An ASM object with revision history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsmObject {
    /// Stable id.
    pub id: ObjectId,
    /// Object kind.
    pub kind: AsmKind,
    /// Human name.
    pub name: String,
    /// Current revision number.
    pub current: u32,
    /// All revisions in order.
    pub revisions: Vec<Revision>,
    /// Content of the current revision.
    pub content: Value,
}

impl AsmObject {
    /// Create a draft object at revision 1.
    pub fn draft(
        id: impl Into<String>,
        kind: AsmKind,
        name: impl Into<String>,
        author: impl Into<String>,
        content: Value,
    ) -> Self {
        let hash = canonical_hash(&content);
        let revision = Revision {
            number: 1,
            state: LifecycleState::Draft,
            author: author.into(),
            created_at: String::new(),
            content_hash: hash,
            supersedes: None,
        };
        Self {
            id: ObjectId::new(id),
            kind,
            name: name.into(),
            current: 1,
            revisions: vec![revision],
            content,
        }
    }

    /// The current revision.
    pub fn revision(&self) -> &Revision {
        self.revisions
            .iter()
            .find(|r| r.number == self.current)
            .expect("current revision exists")
    }

    /// Transition the current revision to a new state.
    pub fn transition(&mut self, to: LifecycleState) -> AsmResult<()> {
        let from = self.revision().state;
        if !allowed_transition(from, to) {
            return Err(AsmError::IllegalTransition {
                id: self.id.0.clone(),
                from,
                to,
            });
        }
        if let Some(revision) = self.revisions.iter_mut().find(|r| r.number == self.current) {
            revision.state = to;
        }
        Ok(())
    }

    /// Create a new draft revision that supersedes the current one.
    ///
    /// A released revision is never mutated: the old revision is marked
    /// `Superseded` and a new `Draft` revision is appended.
    pub fn revise(
        &mut self,
        author: impl Into<String>,
        content: Value,
        created_at: impl Into<String>,
    ) -> AsmResult<u32> {
        if self.revision().state.is_mutable() {
            return Err(AsmError::Revision(
                "revise the draft in place; only released revisions create a new revision"
                    .to_string(),
            ));
        }
        let old = self.current;
        if let Some(revision) = self.revisions.iter_mut().find(|r| r.number == old) {
            if revision.state == LifecycleState::Released {
                revision.state = LifecycleState::Superseded;
            }
        }
        let number = old + 1;
        let hash = canonical_hash(&content);
        self.revisions.push(Revision {
            number,
            state: LifecycleState::Draft,
            author: author.into(),
            created_at: created_at.into(),
            content_hash: hash,
            supersedes: Some(old),
        });
        self.current = number;
        self.content = content;
        Ok(number)
    }

    /// Mutate the current draft content in place. Released content is
    /// rejected.
    pub fn edit_draft(&mut self, content: Value) -> AsmResult<()> {
        if !self.revision().state.is_mutable() {
            return Err(AsmError::Immutable {
                id: self.id.0.clone(),
                revision: self.current,
                state: self.revision().state,
            });
        }
        let hash = canonical_hash(&content);
        if let Some(revision) = self.revisions.iter_mut().find(|r| r.number == self.current) {
            revision.content_hash = hash;
        }
        self.content = content;
        Ok(())
    }
}

/// Allowed lifecycle transitions. Released is a one-way gate except for
/// supersession, which happens through `revise`.
pub fn allowed_transition(from: LifecycleState, to: LifecycleState) -> bool {
    matches!(
        (from, to),
        (LifecycleState::Draft, LifecycleState::InReview)
            | (LifecycleState::Draft, LifecycleState::Retired)
            | (LifecycleState::InReview, LifecycleState::Draft)
            | (LifecycleState::InReview, LifecycleState::Released)
            | (LifecycleState::InReview, LifecycleState::Retired)
            | (LifecycleState::Released, LifecycleState::Superseded)
            | (LifecycleState::Released, LifecycleState::Retired)
            | (LifecycleState::Superseded, LifecycleState::Retired)
    )
}

/// A typed interface parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceParameter {
    /// Parameter name.
    pub name: String,
    /// Physical quantity symbol (e.g. `force`).
    pub quantity: String,
    /// Unit symbol (e.g. `N`).
    pub unit: String,
    /// Coordinate frame id, if spatially resolved.
    pub frame: Option<String>,
}

/// Direction of an interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Source provides to target.
    Provides,
    /// Source consumes from target.
    Consumes,
    /// Bidirectional.
    Bidirectional,
}

/// An interface between a provider and a consumer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    /// Interface id.
    pub id: String,
    /// Provider object.
    pub provider: ObjectId,
    /// Consumer object.
    pub consumer: ObjectId,
    /// Direction.
    pub direction: Direction,
    /// Provided parameters.
    pub provides: Vec<InterfaceParameter>,
    /// Required parameters.
    pub requires: Vec<InterfaceParameter>,
}

impl Interface {
    /// Validate that every required parameter is provided with the same
    /// quantity and unit. A mismatch is a hard error, not a warning.
    pub fn validate(&self) -> AsmResult<()> {
        for required in &self.requires {
            let provided = self
                .provides
                .iter()
                .find(|p| p.name == required.name)
                .ok_or_else(|| {
                    AsmError::InterfaceIncompatible(format!(
                        "{} requires parameter {:?} but {} does not provide it",
                        self.consumer, required.name, self.provider
                    ))
                })?;
            if provided.quantity != required.quantity {
                return Err(AsmError::InterfaceIncompatible(format!(
                    "parameter {:?}: quantity {:?} != required {:?}",
                    required.name, provided.quantity, required.quantity
                )));
            }
            if provided.unit != required.unit {
                return Err(AsmError::InterfaceIncompatible(format!(
                    "parameter {:?}: unit {:?} != required {:?}",
                    required.name, provided.unit, required.unit
                )));
            }
            if provided.frame != required.frame {
                return Err(AsmError::InterfaceIncompatible(format!(
                    "parameter {:?}: frame {:?} != required {:?}",
                    required.name, provided.frame, required.frame
                )));
            }
        }
        Ok(())
    }
}

/// One entry in a configuration snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Object id.
    pub id: String,
    /// Object kind.
    pub kind: String,
    /// Pinned revision number.
    pub revision: u32,
    /// Content hash for that revision.
    pub content_hash: String,
}

/// An immutable configuration snapshot. Runs bind to a snapshot id, not to
/// whatever happens to be "current".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationSnapshot {
    /// Snapshot id.
    pub id: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Author.
    pub author: String,
    /// Pinned entries keyed by object id.
    pub entries: BTreeMap<String, SnapshotEntry>,
}

impl ConfigurationSnapshot {
    /// Build a snapshot from objects.
    pub fn capture(
        id: impl Into<String>,
        author: impl Into<String>,
        created_at: impl Into<String>,
        objects: &[AsmObject],
    ) -> AsmResult<Self> {
        let mut entries = BTreeMap::new();
        for object in objects {
            let revision = object.revision();
            if revision.state != LifecycleState::Released
                && revision.state != LifecycleState::Superseded
            {
                return Err(AsmError::Snapshot(format!(
                    "object {} is {:?}; snapshots pin released or superseded revisions only",
                    object.id, revision.state
                )));
            }
            entries.insert(
                object.id.0.clone(),
                SnapshotEntry {
                    id: object.id.0.clone(),
                    kind: format!("{:?}", object.kind),
                    revision: revision.number,
                    content_hash: revision.content_hash.clone(),
                },
            );
        }
        Ok(Self {
            id: id.into(),
            created_at: created_at.into(),
            author: author.into(),
            entries,
        })
    }

    /// Content hash of the snapshot manifest.
    pub fn manifest_hash(&self) -> String {
        let value = serde_json::to_value(self).expect("snapshot serializes");
        canonical_hash(&value)
    }

    /// Verify a candidate object set against the snapshot.
    pub fn verify(&self, objects: &[AsmObject]) -> AsmResult<()> {
        for object in objects {
            let entry = self.entries.get(&object.id.0).ok_or_else(|| {
                AsmError::Snapshot(format!("object {} is not in the snapshot", object.id))
            })?;
            let revision = object
                .revisions
                .iter()
                .find(|r| r.number == entry.revision)
                .ok_or_else(|| {
                    AsmError::Snapshot(format!(
                        "revision {} of {} is missing",
                        entry.revision, object.id
                    ))
                })?;
            if revision.content_hash != entry.content_hash {
                return Err(AsmError::Snapshot(format!(
                    "content hash mismatch for {} revision {}",
                    object.id, entry.revision
                )));
            }
        }
        Ok(())
    }

    /// Object ids whose pinned revision or hash differs between snapshots.
    pub fn diff(&self, other: &ConfigurationSnapshot) -> Vec<String> {
        let mut changed = Vec::new();
        for (id, entry) in &self.entries {
            match other.entries.get(id) {
                Some(other_entry)
                    if other_entry.revision == entry.revision
                        && other_entry.content_hash == entry.content_hash => {}
                _ => changed.push(id.clone()),
            }
        }
        for id in other.entries.keys() {
            if !self.entries.contains_key(id) {
                changed.push(id.clone());
            }
        }
        changed.sort();
        changed.dedup();
        changed
    }
}

/// Canonical JSON hash: object keys sorted, no insignificant whitespace.
pub fn canonical_hash(value: &Value) -> String {
    let canonical = canonicalize(value);
    let bytes = serde_json::to_vec(&canonical).expect("canonical JSON serializes");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted.insert(key.clone(), canonicalize(&map[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn released(name: &str) -> AsmObject {
        let mut object = AsmObject::draft(
            format!("obj-{name}"),
            AsmKind::Part,
            name,
            "engineer",
            json!({"thickness_mm": 3.0}),
        );
        object.transition(LifecycleState::InReview).unwrap();
        object.transition(LifecycleState::Released).unwrap();
        object
    }

    #[test]
    fn released_revision_is_immutable() {
        let mut object = released("rib");
        assert!(matches!(
            object.edit_draft(json!({"thickness_mm": 4.0})),
            Err(AsmError::Immutable { .. })
        ));
    }

    #[test]
    fn revision_supersedes_without_mutating_history() {
        let mut object = released("rib");
        let old_hash = object.revision().content_hash.clone();
        let number = object
            .revise("engineer", json!({"thickness_mm": 4.0}), "2026-11-24T00:00:00Z")
            .unwrap();
        assert_eq!(number, 2);
        assert_eq!(object.revisions[0].state, LifecycleState::Superseded);
        assert_eq!(object.revisions[0].content_hash, old_hash);
        assert_eq!(object.revision().state, LifecycleState::Draft);
    }

    #[test]
    fn illegal_transition_is_rejected() {
        let mut object = released("rib");
        assert!(matches!(
            object.transition(LifecycleState::Draft),
            Err(AsmError::IllegalTransition { .. })
        ));
    }

    #[test]
    fn interface_unit_mismatch_is_rejected() {
        let interface = Interface {
            id: "if-1".into(),
            provider: ObjectId::new("wing"),
            consumer: ObjectId::new("fuselage"),
            direction: Direction::Provides,
            provides: vec![InterfaceParameter {
                name: "lift".into(),
                quantity: "force".into(),
                unit: "kN".into(),
                frame: None,
            }],
            requires: vec![InterfaceParameter {
                name: "lift".into(),
                quantity: "force".into(),
                unit: "N".into(),
                frame: None,
            }],
        };
        assert!(matches!(
            interface.validate(),
            Err(AsmError::InterfaceIncompatible(_))
        ));
    }

    #[test]
    fn snapshot_reconstructs_and_detects_change() {
        let rib = released("rib");
        let spar = released("spar");
        let snapshot = ConfigurationSnapshot::capture(
            "snap-1",
            "engineer",
            "2026-11-24T00:00:00Z",
            &[rib.clone(), spar.clone()],
        )
        .unwrap();
        snapshot.verify(&[rib.clone(), spar.clone()]).unwrap();

        let mut changed = rib.clone();
        changed
            .revise("engineer", json!({"thickness_mm": 5.0}), "2026-11-25T00:00:00Z")
            .unwrap();
        let mut newer = changed;
        newer.transition(LifecycleState::InReview).unwrap();
        newer.transition(LifecycleState::Released).unwrap();
        let next = ConfigurationSnapshot::capture(
            "snap-2",
            "engineer",
            "2026-11-25T00:00:00Z",
            &[newer, spar],
        )
        .unwrap();
        assert_eq!(snapshot.diff(&next), vec![rib.id.0.clone()]);
    }

    #[test]
    fn snapshot_refuses_draft_objects() {
        let draft = AsmObject::draft("obj", AsmKind::Part, "draft", "engineer", json!({}));
        assert!(matches!(
            ConfigurationSnapshot::capture("s", "a", "t", &[draft]),
            Err(AsmError::Snapshot(_))
        ));
    }
}
