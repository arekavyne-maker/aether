// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Manufacturing genealogy and as-built traceability.
//!
//! The design graph answers *what was intended*. This crate answers *what
//! was actually built*: which design revision, process plan revision,
//! material lots, operator qualification, inspections, and dispositions
//! produced a physical part.
//!
//! It deliberately does not control machines. It records and checks the
//! evidence that machine control must later produce, so an as-built record
//! can be traced back to a released configuration and forward to the
//! inspections and deviations that affect it.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Nonconformance severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Cosmetic or documentation-only.
    Minor,
    /// Requires disposition before release.
    Major,
    /// Safety or airworthiness impact; requires approval.
    Critical,
}

/// Disposition of a nonconformance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// Acceptable without change, with justification.
    UseAsIs,
    /// Bring the part back into drawing tolerance.
    Rework,
    /// Restore the part to an acceptable condition by an approved repair.
    Repair,
    /// Reject the part.
    Scrap,
    /// Return the material or part to the supplier.
    ReturnToSupplier,
}

impl Disposition {
    /// Whether the disposition resolves the nonconformance for release.
    pub fn resolves(self) -> bool {
        true
    }

    /// Whether the disposition needs an approver signature.
    pub fn needs_approval(self) -> bool {
        matches!(
            self,
            Disposition::UseAsIs | Disposition::Repair | Disposition::ReturnToSupplier
        )
    }
}

/// Inspection outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionResult {
    /// Within acceptance criteria.
    Pass,
    /// Outside acceptance criteria.
    Fail,
    /// Accepted with a recorded condition.
    Conditional,
}

/// A material lot consumed by a build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialLot {
    /// Supplier or internal lot identifier.
    pub lot: String,
    /// Material specification, e.g. `Al-2024-T3`.
    pub material: String,
    /// Hash of the mill certificate or incoming inspection record.
    pub certificate_hash: Option<String>,
}

/// One inspection performed on a build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectionRecord {
    /// Inspection identifier.
    pub id: String,
    /// Method, e.g. `ultrasonic`, `cmm`, `visual`.
    pub method: String,
    /// Outcome.
    pub result: InspectionResult,
    /// Measured values, kept as data rather than prose.
    pub measured: Option<Value>,
}

/// A recorded nonconformance and its disposition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nonconformance {
    /// Nonconformance identifier.
    pub id: String,
    /// Severity.
    pub severity: Severity,
    /// Human description.
    pub description: String,
    /// Disposition, if one has been decided.
    pub disposition: Option<Disposition>,
    /// Approver identity for dispositions that require approval.
    pub approved_by: Option<String>,
}

/// One physical build of a part at a design revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildRecord {
    /// Build identifier.
    pub id: String,
    /// Part identifier shared with the design graph.
    pub part_id: String,
    /// Design revision that was released for this build.
    pub design_revision: u32,
    /// Process plan identifier.
    pub process_plan_id: String,
    /// Process plan revision.
    pub process_plan_revision: u32,
    /// Material lots consumed.
    pub material_lots: Vec<MaterialLot>,
    /// Operator qualification reference, required for critical processes.
    pub operator_qualification: Option<String>,
    /// Whether the process is critical (requires a qualified operator).
    pub critical_process: bool,
    /// Inspections performed.
    pub inspections: Vec<InspectionRecord>,
    /// Nonconformances found.
    pub nonconformances: Vec<Nonconformance>,
    /// Revision actually built and inspected.
    pub as_built_revision: u32,
    /// ISO-8601 build timestamp.
    pub built_at: String,
}

/// One verification problem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenealogyIssue {
    /// Build the issue belongs to, or empty for a genealogy-wide issue.
    pub build_id: String,
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Result of checking a genealogy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenealogyReport {
    /// Whether every build record is traceable and dispositioned.
    pub valid: bool,
    /// Number of build records checked.
    pub builds_checked: usize,
    /// Problems found. Empty when `valid` is true.
    pub issues: Vec<GenealogyIssue>,
}

/// One entry in an as-built configuration snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsBuiltEntry {
    /// Build identifier.
    pub build_id: String,
    /// Released design revision.
    pub design_revision: u32,
    /// Revision actually built.
    pub as_built_revision: u32,
    /// Material lots consumed.
    pub material_lots: Vec<String>,
    /// Inspection identifiers.
    pub inspections: Vec<String>,
    /// Nonconformance identifiers that are still open.
    pub open_nonconformances: Vec<String>,
}

/// An as-built configuration for one part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsBuiltSnapshot {
    /// Part identifier.
    pub part_id: String,
    /// Build entries, ordered by build id.
    pub entries: Vec<AsBuiltEntry>,
    /// SHA-256 of the canonical entries.
    pub manifest_hash: String,
}

/// Impact of a design revision change on existing builds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpactReport {
    /// Part identifier.
    pub part_id: String,
    /// Revision the fleet is moving to.
    pub to_revision: u32,
    /// Builds whose design revision is older than `to_revision`.
    pub affected_builds: Vec<String>,
    /// Affected builds that were built at an older revision and need
    /// revalidation against the new revision.
    pub revalidation_required: Vec<String>,
    /// Builds already at or beyond the new revision.
    pub unaffected_builds: Vec<String>,
    /// Plain-language explanation of the result.
    pub reason: String,
}

/// Genealogy errors.
#[derive(Debug, Error, PartialEq)]
pub enum ManufacturingError {
    /// A build id appears more than once.
    #[error("duplicate build id {0}")]
    DuplicateBuild(String),
    /// A build references an unknown part.
    #[error("no build records for part {0}")]
    UnknownPart(String),
}

/// A collection of build records for one or more parts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Genealogy {
    /// Build records in input order.
    pub records: Vec<BuildRecord>,
}

impl Genealogy {
    /// Construct a genealogy.
    pub fn new(records: Vec<BuildRecord>) -> Self {
        Self { records }
    }

    /// Check every build record for traceability and disposition completeness.
    pub fn verify(&self) -> GenealogyReport {
        let mut issues = Vec::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();

        for record in &self.records {
            if !seen.insert(record.id.as_str()) {
                issues.push(issue(
                    &record.id,
                    "duplicate_build_id",
                    format!("build id {} is not unique", record.id),
                ));
            }
            if record.part_id.trim().is_empty() {
                issues.push(issue(&record.id, "missing_part", "part_id is empty"));
            }
            if record.design_revision == 0 {
                issues.push(issue(
                    &record.id,
                    "invalid_design_revision",
                    "design_revision must be >= 1",
                ));
            }
            if record.process_plan_id.trim().is_empty() {
                issues.push(issue(
                    &record.id,
                    "missing_process_plan",
                    "process_plan_id is empty",
                ));
            }
            if record.process_plan_revision == 0 {
                issues.push(issue(
                    &record.id,
                    "invalid_process_plan_revision",
                    "process_plan_revision must be >= 1",
                ));
            }
            if record.as_built_revision != record.design_revision {
                issues.push(issue(
                    &record.id,
                    "as_built_revision_mismatch",
                    format!(
                        "as_built_revision {} does not match design_revision {}",
                        record.as_built_revision, record.design_revision
                    ),
                ));
            }
            if record.material_lots.is_empty() {
                issues.push(issue(
                    &record.id,
                    "missing_material_lot",
                    "no material lot is recorded",
                ));
            }
            for lot in &record.material_lots {
                if lot.lot.trim().is_empty() || lot.material.trim().is_empty() {
                    issues.push(issue(
                        &record.id,
                        "incomplete_material_lot",
                        "material lot must have a lot id and material specification",
                    ));
                }
            }
            if record.critical_process
                && record
                    .operator_qualification
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .is_none()
            {
                issues.push(issue(
                    &record.id,
                    "missing_operator_qualification",
                    "critical processes require a qualification reference",
                ));
            }

            let failed_inspection = record
                .inspections
                .iter()
                .any(|i| i.result == InspectionResult::Fail);
            let resolved_nonconformance = record.nonconformances.iter().any(|n| {
                n.disposition
                    .map(Disposition::resolves)
                    .unwrap_or(false)
            });
            if failed_inspection && !resolved_nonconformance {
                issues.push(issue(
                    &record.id,
                    "failed_inspection_unresolved",
                    "a failed inspection has no dispositioned nonconformance",
                ));
            }

            for nonconformance in &record.nonconformances {
                match nonconformance.disposition {
                    None => issues.push(issue(
                        &record.id,
                        "open_nonconformance",
                        format!(
                            "nonconformance {} has no disposition",
                            nonconformance.id
                        ),
                    )),
                    Some(disposition) => {
                        let requires_approval = disposition.needs_approval()
                            || nonconformance.severity == Severity::Critical;
                        if requires_approval
                            && nonconformance
                                .approved_by
                                .as_deref()
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .is_none()
                        {
                            issues.push(issue(
                                &record.id,
                                "unapproved_disposition",
                                format!(
                                    "nonconformance {} disposition {:?} requires an approver",
                                    nonconformance.id, disposition
                                ),
                            ));
                        }
                    }
                }
            }
        }

        GenealogyReport {
            valid: issues.is_empty(),
            builds_checked: self.records.len(),
            issues,
        }
    }

    /// Build an as-built configuration snapshot for a part.
    pub fn as_built(&self, part_id: &str) -> Option<AsBuiltSnapshot> {
        let mut entries: Vec<AsBuiltEntry> = self
            .records
            .iter()
            .filter(|r| r.part_id == part_id)
            .map(|r| AsBuiltEntry {
                build_id: r.id.clone(),
                design_revision: r.design_revision,
                as_built_revision: r.as_built_revision,
                material_lots: r.material_lots.iter().map(|l| l.lot.clone()).collect(),
                inspections: r.inspections.iter().map(|i| i.id.clone()).collect(),
                open_nonconformances: r
                    .nonconformances
                    .iter()
                    .filter(|n| n.disposition.is_none())
                    .map(|n| n.id.clone())
                    .collect(),
            })
            .collect();
        if entries.is_empty() {
            return None;
        }
        entries.sort_by(|a, b| a.build_id.cmp(&b.build_id));
        let manifest_hash = hash_entries(&entries);
        Some(AsBuiltSnapshot {
            part_id: part_id.to_string(),
            entries,
            manifest_hash,
        })
    }

    /// Explain which builds a design revision change affects.
    pub fn impact(&self, part_id: &str, to_revision: u32) -> Result<ImpactReport, ManufacturingError> {
        let mut affected = Vec::new();
        let mut revalidation = Vec::new();
        let mut unaffected = Vec::new();
        for record in self.records.iter().filter(|r| r.part_id == part_id) {
            if record.design_revision < to_revision {
                affected.push(record.id.clone());
                if record.as_built_revision < to_revision {
                    revalidation.push(record.id.clone());
                }
            } else {
                unaffected.push(record.id.clone());
            }
        }
        if affected.is_empty() && unaffected.is_empty() {
            return Err(ManufacturingError::UnknownPart(part_id.to_string()));
        }
        affected.sort();
        revalidation.sort();
        unaffected.sort();
        let reason = if affected.is_empty() {
            format!(
                "all recorded builds of {part_id} are already at revision {to_revision} or later"
            )
        } else {
            format!(
                "{} build(s) of {part_id} predate revision {to_revision}; {} require revalidation",
                affected.len(),
                revalidation.len()
            )
        };
        Ok(ImpactReport {
            part_id: part_id.to_string(),
            to_revision,
            affected_builds: affected,
            revalidation_required: revalidation,
            unaffected_builds: unaffected,
            reason,
        })
    }
}

fn issue(build_id: &str, code: &str, message: impl Into<String>) -> GenealogyIssue {
    GenealogyIssue {
        build_id: build_id.to_string(),
        code: code.to_string(),
        message: message.into(),
    }
}

fn hash_entries(entries: &[AsBuiltEntry]) -> String {
    // Serialize a map keyed by build id so the hash is stable even if the
    // input order changes.
    let mut canonical: BTreeMap<&str, &AsBuiltEntry> = BTreeMap::new();
    for entry in entries {
        canonical.insert(entry.build_id.as_str(), entry);
    }
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lot(lot: &str) -> MaterialLot {
        MaterialLot {
            lot: lot.to_string(),
            material: "Al-2024-T3".to_string(),
            certificate_hash: Some("00".to_string()),
        }
    }

    fn record(id: &str) -> BuildRecord {
        BuildRecord {
            id: id.to_string(),
            part_id: "rib-07".to_string(),
            design_revision: 2,
            process_plan_id: "pp-rib-07".to_string(),
            process_plan_revision: 3,
            material_lots: vec![lot("LOT-1")],
            operator_qualification: Some("op-qual-1".to_string()),
            critical_process: true,
            inspections: vec![InspectionRecord {
                id: "insp-1".to_string(),
                method: "cmm".to_string(),
                result: InspectionResult::Pass,
                measured: None,
            }],
            nonconformances: Vec::new(),
            as_built_revision: 2,
            built_at: "2026-11-24T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn complete_build_verifies() {
        let genealogy = Genealogy::new(vec![record("build-1")]);
        let report = genealogy.verify();
        assert!(report.valid, "{:?}", report.issues);
        assert_eq!(report.builds_checked, 1);
    }

    #[test]
    fn missing_material_fails() {
        let mut build = record("build-1");
        build.material_lots.clear();
        let report = Genealogy::new(vec![build]).verify();
        assert!(!report.valid);
        assert!(report.issues.iter().any(|i| i.code == "missing_material_lot"));
    }

    #[test]
    fn failed_inspection_needs_disposition() {
        let mut build = record("build-1");
        build.inspections[0].result = InspectionResult::Fail;
        let report = Genealogy::new(vec![build.clone()]).verify();
        assert!(!report.valid);
        assert!(report
            .issues
            .iter()
            .any(|i| i.code == "failed_inspection_unresolved"));

        build.nonconformances.push(Nonconformance {
            id: "nc-1".to_string(),
            severity: Severity::Major,
            description: "porosity".to_string(),
            disposition: Some(Disposition::Rework),
            approved_by: Some("quality-auditor".to_string()),
        });
        assert!(Genealogy::new(vec![build]).verify().valid);
    }

    #[test]
    fn critical_disposition_requires_approval() {
        let mut build = record("build-1");
        build.nonconformances.push(Nonconformance {
            id: "nc-1".to_string(),
            severity: Severity::Critical,
            description: "crack".to_string(),
            disposition: Some(Disposition::Scrap),
            approved_by: None,
        });
        let report = Genealogy::new(vec![build]).verify();
        assert!(!report.valid);
        assert!(report
            .issues
            .iter()
            .any(|i| i.code == "unapproved_disposition"));
    }

    #[test]
    fn critical_process_needs_operator_qualification() {
        let mut build = record("build-1");
        build.operator_qualification = None;
        let report = Genealogy::new(vec![build]).verify();
        assert!(!report.valid);
        assert!(report
            .issues
            .iter()
            .any(|i| i.code == "missing_operator_qualification"));
    }

    #[test]
    fn impact_reports_old_builds() {
        let mut old = record("build-old");
        old.design_revision = 1;
        old.as_built_revision = 1;
        let new = record("build-new");
        let genealogy = Genealogy::new(vec![old, new]);
        let impact = genealogy.impact("rib-07", 2).unwrap();
        assert_eq!(impact.affected_builds, vec!["build-old"]);
        assert_eq!(impact.revalidation_required, vec!["build-old"]);
        assert_eq!(impact.unaffected_builds, vec!["build-new"]);
        assert!(impact.reason.contains("predate revision 2"));
    }

    #[test]
    fn impact_rejects_unknown_part() {
        let genealogy = Genealogy::new(vec![record("build-1")]);
        assert!(matches!(
            genealogy.impact("spar-99", 2),
            Err(ManufacturingError::UnknownPart(_))
        ));
    }

    #[test]
    fn as_built_snapshot_is_order_independent() {
        let a = record("build-a");
        let b = record("build-b");
        let first = Genealogy::new(vec![a.clone(), b.clone()])
            .as_built("rib-07")
            .unwrap();
        let second = Genealogy::new(vec![b, a]).as_built("rib-07").unwrap();
        assert_eq!(first.manifest_hash, second.manifest_hash);
        assert_eq!(first.entries.len(), 2);
    }
}
