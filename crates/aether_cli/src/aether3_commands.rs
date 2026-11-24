// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Aether 3.0 substrate commands.
//!
//! These commands expose the Phase 1 primitives without changing the
//! existing golden-path surface:
//!
//! - `aether schema {list|show}`      — contract and graph-catalog discovery
//! - `aether status`                  — honest capability/status summary
//! - `aether policy explain`          — deterministic RBAC decision
//! - `aether provenance explain`      — node lineage from the event graph
//! - `aether validation report`       — requirement verification coverage
//! - `aether plan explain`            — compiled workflow DAG
//! - `aether config {snapshot|verify}`— ASM configuration snapshots

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use serde_json::{Value, json};

use aether_asm::{
    AsmKind, AsmObject, ConfigurationSnapshot, LifecycleState, ObjectId,
};
use aether_policy::{PolicyDecision, PolicyEngine, PolicyRequest, Subject};

use crate::commands::{emit, OutputMode};
use crate::engine_bridge;

/// `aether schema list` — list contract packages and graph catalog types.
pub fn cmd_schema_list(project: PathBuf, mode: OutputMode) -> Result<()> {
    let root = engine_bridge::find_monorepo_root(&project);
    let contracts = list_contracts(&root)?;
    let (nodes, edges) = list_graph_types(&root)?;
    emit(
        mode,
        &json!({
            "contracts": contracts,
            "contract_count": contracts.len(),
            "graph_node_types": nodes,
            "graph_node_count": nodes.len(),
            "graph_edge_types": edges,
            "graph_edge_count": edges.len(),
        }),
    )
}

/// `aether schema show <id>` — show a contract manifest, node type, or edge type.
pub fn cmd_schema_show(project: PathBuf, id: String, mode: OutputMode) -> Result<()> {
    let root = engine_bridge::find_monorepo_root(&project);
    // Contract package.
    for contract in list_contracts(&root)? {
        if contract["id"] == id || contract["contract_id"] == id {
            let manifest = contract["manifest"].clone();
            emit(mode, &json!({"kind": "contract", "id": id, "manifest": manifest}))?;
            return Ok(());
        }
    }
    // Graph node or edge type.
    let catalog = root.join("contracts/graph.model/catalog");
    for (dir, kind) in [("nodes", "node_type"), ("edges", "edge_type")] {
        for entry in std::fs::read_dir(catalog.join(dir))? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)?;
            let value: Value = serde_json::from_str(&raw)?;
            if value.get("type_id").and_then(|v| v.as_str()) == Some(id.as_str()) {
                emit(mode, &json!({"kind": kind, "id": id, "definition": value}))?;
                return Ok(());
            }
        }
    }
    Err(anyhow!(
        "unknown schema id {id:?}; run `aether schema list` to see available ids"
    ))
}

fn list_contracts(root: &Path) -> Result<Vec<Value>> {
    let contracts_dir = root.join("contracts");
    let mut out = Vec::new();
    if !contracts_dir.is_dir() {
        return Ok(out);
    }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&contracts_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let manifest = dir.join("manifest.json");
        if !manifest.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&manifest)?;
        let value: Value = serde_json::from_str(&raw)?;
        out.push(json!({
            "id": value.get("contract_id").cloned().unwrap_or(Value::Null),
            "contract_id": value.get("contract_id").cloned().unwrap_or(Value::Null),
            "version": value.get("version").cloned().unwrap_or(Value::Null),
            "title": value.get("title").cloned().unwrap_or(Value::Null),
            "path": manifest.strip_prefix(root).unwrap_or(&manifest).display().to_string(),
            "manifest": value,
        }));
    }
    Ok(out)
}

fn list_graph_types(root: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let catalog = root.join("contracts/graph.model/catalog");
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for entry in std::fs::read_dir(catalog.join("nodes"))? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let raw = std::fs::read_to_string(&path)?;
            let value: Value = serde_json::from_str(&raw)?;
            if let Some(id) = value.get("type_id").and_then(|v| v.as_str()) {
                nodes.push(id.to_string());
            }
        }
    }
    for entry in std::fs::read_dir(catalog.join("edges"))? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let raw = std::fs::read_to_string(&path)?;
            let value: Value = serde_json::from_str(&raw)?;
            if let Some(id) = value.get("type_id").and_then(|v| v.as_str()) {
                edges.push(id.to_string());
            }
        }
    }
    nodes.sort();
    edges.sort();
    Ok((nodes, edges))
}

/// `aether status` — the honest current capability line.
pub fn cmd_status(project: PathBuf, mode: OutputMode) -> Result<()> {
    let root = engine_bridge::find_monorepo_root(&project);
    let contracts = list_contracts(&root)?;
    let (nodes, edges) = list_graph_types(&root)?;
    let graph = match engine_bridge::open_engine(&project) {
        Ok((engine, db)) => json!({
            "available": true,
            "database": db.display().to_string(),
            "nodes": engine.node_count().unwrap_or(0),
            "edges": engine.edge_count().unwrap_or(0),
            "events": engine.event_count().unwrap_or(0),
            "chain_head": engine.storage_head().unwrap_or(None),
        }),
        Err(_) => json!({"available": false}),
    };
    emit(
        mode,
        &json!({
            "product": "aether",
            "version": env!("CARGO_PKG_VERSION"),
            "phase": "Phase 0–2 substrate + Phase 1 primitives",
            "architecture_target": "docs/architecture/AETHER-3.0.md",
            "contracts": contracts.len(),
            "graph_node_types": nodes.len(),
            "graph_edge_types": edges.len(),
            "graph": graph,
            "policy": "fail_closed_rbac",
            "plugin_sandbox": "not_implemented",
            "evidence_gates": "implemented_as_library",
            "configuration_snapshots": "implemented_as_library",
            "agent_runtime": "not_implemented",
            "certification_claim": false,
        }),
    )
}

/// `aether policy explain <action>` — deterministic RBAC decision.
pub fn cmd_policy_explain(
    action: String,
    role: String,
    subject: String,
    mode: OutputMode,
) -> Result<()> {
    let engine = PolicyEngine::new();
    let request = PolicyRequest {
        subject: Subject { id: subject, role },
        action,
        context: json!({}),
    };
    let decision = engine.evaluate(&request);
    let (allow, reason) = match &decision {
        PolicyDecision::Allow => (true, "allowed by role catalog".to_string()),
        PolicyDecision::Deny { reason } => (false, reason.clone()),
    };
    emit(
        mode,
        &json!({
            "subject": request.subject,
            "action": request.action,
            "decision": if allow { "allow" } else { "deny" },
            "allow": allow,
            "reason": reason,
            "engine": "rbac_fail_closed",
        }),
    )
}

/// `aether provenance explain <object-id>` — node lineage and chain status.
pub fn cmd_provenance_explain(project: PathBuf, object_id: String, mode: OutputMode) -> Result<()> {
    let (engine, db) = engine_bridge::open_engine(&project)?;
    let uuid = uuid::Uuid::parse_str(&object_id)
        .with_context(|| format!("object id {object_id:?} is not a UUID"))?;
    let node = engine
        .get_node(uuid)
        .map_err(|e| anyhow!("graph lookup: {e}"))?
        .ok_or_else(|| anyhow!("object {object_id} not found in {}", db.display()))?;
    let mut out_edges = Vec::new();
    let mut in_edges = Vec::new();
    for edge in engine.out_edges(uuid).map_err(|e| anyhow!("out edges: {e}"))? {
        let target = engine
            .get_node(edge.target)
            .ok()
            .flatten()
            .map(|n| json!({"id": n.id.to_string(), "type_id": n.type_id, "state": n.state.as_str()}));
        out_edges.push(json!({
            "edge_id": edge.id.to_string(),
            "type_id": edge.type_id,
            "target": target,
        }));
    }
    for edge in engine.in_edges(uuid).map_err(|e| anyhow!("in edges: {e}"))? {
        let source = engine
            .get_node(edge.source)
            .ok()
            .flatten()
            .map(|n| json!({"id": n.id.to_string(), "type_id": n.type_id, "state": n.state.as_str()}));
        in_edges.push(json!({
            "edge_id": edge.id.to_string(),
            "type_id": edge.type_id,
            "source": source,
        }));
    }
    let chain = engine
        .verify_chain()
        .map_err(|e| anyhow!("chain verify: {e}"))?;
    emit(
        mode,
        &json!({
            "object": engine_bridge::node_to_value(&node),
            "out_edges": out_edges,
            "in_edges": in_edges,
            "chain": {
                "valid": chain.valid,
                "event_count": chain.event_count,
                "errors": chain.errors,
            },
            "database": db.display().to_string(),
        }),
    )
}

/// `aether validation report` — requirement verification coverage.
pub fn cmd_validation_report(project: PathBuf, mode: OutputMode) -> Result<()> {
    let (engine, _) = engine_bridge::open_engine(&project)?;
    let requirements = engine
        .nodes_of_type("Requirement")
        .map_err(|e| anyhow!("requirements: {e}"))?;
    let mut verified = Vec::new();
    let mut gaps = Vec::new();
    for requirement in &requirements {
        let mut evidence = Vec::new();
        for edge in engine
            .out_edges(requirement.id)
            .map_err(|e| anyhow!("requirement edges: {e}"))?
        {
            if matches!(
                edge.type_id.as_str(),
                "verifies" | "verified_by" | "validated_by"
            ) {
                evidence.push(json!({"edge": edge.type_id, "target": edge.target.to_string()}));
            }
        }
        for edge in engine
            .in_edges(requirement.id)
            .map_err(|e| anyhow!("requirement edges: {e}"))?
        {
            if matches!(edge.type_id.as_str(), "verifies" | "verified_by") {
                evidence.push(json!({"edge": edge.type_id, "source": edge.source.to_string()}));
            }
        }
        if evidence.is_empty() {
            gaps.push(json!({
                "requirement_id": requirement.id.to_string(),
                "status": requirement.properties.get("status").cloned().unwrap_or(Value::Null),
                "reason": "no verifies/verified_by edge",
            }));
        } else {
            verified.push(json!({
                "requirement_id": requirement.id.to_string(),
                "evidence": evidence,
            }));
        }
    }
    emit(
        mode,
        &json!({
            "requirements_total": requirements.len(),
            "requirements_verified": verified.len(),
            "verified": verified,
            "gaps": gaps,
            "claim": "coverage only; this is not certification",
        }),
    )
}

/// `aether plan explain <workflow>` — compile an AWL document and show the DAG.
pub fn cmd_plan_explain(workflow: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&workflow)
        .with_context(|| format!("read {}", workflow.display()))?;
    let awl = aether_workflow::awl::Awl::parse(&raw)
        .with_context(|| format!("parse {}", workflow.display()))?;
    let compiled = aether_workflow::dag::CompiledWorkflow::from_awl(&awl)?;
    let steps: Vec<Value> = compiled
        .steps
        .values()
        .map(|step| {
            json!({
                "id": step.id,
                "step_type": format!("{:?}", step.kind),
                "plugin": step.plugin,
                "contract": step.contract,
                "depends_on": step.depends_on,
                "outputs": step.outputs,
            })
        })
        .collect();
    emit(
        mode,
        &json!({
            "workflow_id": compiled.workflow_id,
            "awl_version": compiled.awl_version,
            "step_count": compiled.steps.len(),
            "topological_order": compiled.topological_order,
            "reproducibility": compiled.reproducibility,
            "env_manifest_hash": compiled.env_manifest_hash,
            "steps": steps,
        }),
    )
}

/// `aether config snapshot <file> --output <path>` — capture an ASM
/// configuration snapshot from a released object set.
pub fn cmd_config_snapshot(file: PathBuf, output: Option<PathBuf>, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&file)
        .with_context(|| format!("read {}", file.display()))?;
    let doc: Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse {}", file.display()))?;
    let objects = parse_asm_objects(&doc)?;
    let id = doc
        .get("snapshot")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("snapshot-{}", chrono::Utc::now().timestamp()));
    let author = doc
        .get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("aether-cli")
        .to_string();
    let snapshot = ConfigurationSnapshot::capture(
        id,
        author,
        chrono::Utc::now().to_rfc3339(),
        &objects,
    )
    .map_err(|e| anyhow!("capture snapshot: {e}"))?;
    let hash = snapshot.manifest_hash();
    let out_path = output.unwrap_or_else(|| PathBuf::from(format!("{}.snapshot.json", snapshot.id)));
    let bytes = serde_json::to_vec_pretty(&snapshot)?;
    std::fs::write(&out_path, &bytes)
        .with_context(|| format!("write {}", out_path.display()))?;
    emit(
        mode,
        &json!({
            "snapshot": snapshot,
            "manifest_hash": hash,
            "output": out_path.display().to_string(),
            "objects": objects.len(),
            "artifact_size": bytes.len(),
        }),
    )
}

/// `aether config verify <snapshot> <file>` — verify an object set against
/// a recorded snapshot.
pub fn cmd_config_verify(snapshot_path: PathBuf, file: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&snapshot_path)
        .with_context(|| format!("read {}", snapshot_path.display()))?;
    let snapshot: ConfigurationSnapshot = serde_json::from_str(&raw)
        .with_context(|| format!("parse {}", snapshot_path.display()))?;
    let doc_raw = std::fs::read_to_string(&file)
        .with_context(|| format!("read {}", file.display()))?;
    let doc: Value = serde_yaml::from_str(&doc_raw)
        .with_context(|| format!("parse {}", file.display()))?;
    let objects = parse_asm_objects(&doc)?;
    let result = snapshot.verify(&objects);
    let (valid, error) = match &result {
        Ok(()) => (true, Value::Null),
        Err(e) => (false, Value::String(e.to_string())),
    };
    emit(
        mode,
        &json!({
            "valid": valid,
            "snapshot": snapshot.id,
            "manifest_hash": snapshot.manifest_hash(),
            "error": error,
        }),
    )?;
    if !valid {
        std::process::exit(30);
    }
    Ok(())
}

fn parse_asm_objects(doc: &Value) -> Result<Vec<AsmObject>> {
    let entries = doc
        .get("objects")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("document must contain an `objects` array"))?;
    let mut out = Vec::new();
    for entry in entries {
        let id = entry
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("object missing id"))?;
        let kind = entry
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("object {id} missing kind"))?;
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(id);
        let revision = entry
            .get("revision")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        let state = entry
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("released");
        if state != "released" && state != "superseded" {
            return Err(anyhow!(
                "object {id} is {state:?}; snapshots pin released or superseded revisions only"
            ));
        }
        let content = entry.get("content").cloned().unwrap_or_else(|| json!({}));
        let mut object = AsmObject::draft(id, parse_kind(kind)?, name, "aether-cli", content);
        object.transition(LifecycleState::InReview)?;
        object.transition(LifecycleState::Released)?;
        // A document may pin a revision number; the local object is the
        // content for that revision.
        if revision > 1 {
            let content = object.content.clone();
            object.revise("aether-cli", content, "2026-11-24T00:00:00Z")?;
            object.transition(LifecycleState::InReview)?;
            object.transition(LifecycleState::Released)?;
        }
        out.push(object);
    }
    Ok(out)
}

fn parse_kind(value: &str) -> Result<AsmKind> {
    let kind = match value {
        "program" => AsmKind::Program,
        "aircraft_configuration" => AsmKind::AircraftConfiguration,
        "product" => AsmKind::Product,
        "assembly" => AsmKind::Assembly,
        "part" => AsmKind::Part,
        "interface" => AsmKind::Interface,
        "variant" => AsmKind::Variant,
        "configuration_baseline" => AsmKind::ConfigurationBaseline,
        "change_request" => AsmKind::ChangeRequest,
        "effectivity" => AsmKind::Effectivity,
        "requirement" => AsmKind::Requirement,
        "verification_case" => AsmKind::VerificationCase,
        "validation_case" => AsmKind::ValidationCase,
        "material_assignment" => AsmKind::MaterialAssignment,
        "environment_case" => AsmKind::EnvironmentCase,
        other => return Err(anyhow!("unknown ASM kind {other:?}")),
    };
    Ok(kind)
}

/// `aether evidence gate <file>` — evaluate a release gate over evidence.
pub fn cmd_evidence_gate(file: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&file)
        .with_context(|| format!("read {}", file.display()))?;
    let doc: Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse {}", file.display()))?;
    let snapshot = doc
        .get("snapshot")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("evidence file must declare `snapshot`"))?;
    let gate_value = doc
        .get("gate")
        .ok_or_else(|| anyhow!("evidence file must declare `gate`"))?;
    let gate: aether_evidence::Gate = serde_json::from_value(gate_value.clone())
        .with_context(|| "parse gate")?;
    let mut items = Vec::new();
    if let Some(list) = doc.get("evidence").and_then(|v| v.as_array()) {
        for value in list {
            items.push(
                serde_json::from_value::<aether_evidence::EvidenceItem>(value.clone())
                    .with_context(|| "parse evidence item")?,
            );
        }
    }
    let mut credits = BTreeMap::new();
    if let Some(list) = doc.get("credibility").and_then(|v| v.as_array()) {
        for value in list {
            let record: aether_evidence::CredibilityRecord = serde_json::from_value(value.clone())
                .with_context(|| "parse credibility record")?;
            credits.insert(record.id.clone(), record);
        }
    }
    let waiver: Option<aether_evidence::Waiver> = doc
        .get("waiver")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .with_context(|| "parse waiver")?;
    let verdict = aether_evidence::evaluate_gate(&gate, snapshot, &items, &credits, waiver.as_ref());
    emit(mode, &verdict)?;
    if !verdict.passed {
        std::process::exit(30);
    }
    Ok(())
}

/// `aether manufacturing verify <file>` — check build genealogy and
/// disposition completeness.
pub fn cmd_manufacturing_verify(file: PathBuf, mode: OutputMode) -> Result<()> {
    let genealogy = read_genealogy(&file)?;
    let report = genealogy.verify();
    emit(mode, &report)?;
    if !report.valid {
        std::process::exit(30);
    }
    Ok(())
}

/// `aether manufacturing as-built <file> --part <id>` — produce an
/// as-built configuration snapshot for one part.
pub fn cmd_manufacturing_as_built(file: PathBuf, part: String, mode: OutputMode) -> Result<()> {
    let genealogy = read_genealogy(&file)?;
    let snapshot = genealogy
        .as_built(&part)
        .ok_or_else(|| anyhow!("no build records for part {part:?}"))?;
    emit(mode, &snapshot)
}

/// `aether manufacturing impact <file> --part <id> --to-revision <n>` —
/// explain which builds a design revision change affects.
pub fn cmd_manufacturing_impact(
    file: PathBuf,
    part: String,
    to_revision: u32,
    mode: OutputMode,
) -> Result<()> {
    let genealogy = read_genealogy(&file)?;
    let impact = genealogy
        .impact(&part, to_revision)
        .map_err(|e| anyhow!("{e}"))?;
    emit(mode, &impact)
}

fn read_genealogy(file: &Path) -> Result<aether_manufacturing::Genealogy> {
    let raw = std::fs::read_to_string(file)
        .with_context(|| format!("read {}", file.display()))?;
    let doc: Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse {}", file.display()))?;
    let records = if let Some(array) = doc.as_array() {
        array.clone()
    } else {
        doc.get("records")
            .or_else(|| doc.get("builds"))
            .and_then(|v| v.as_array())
            .cloned()
            .ok_or_else(|| anyhow!("document must contain a `builds` or `records` array"))?
    };
    let parsed: Vec<aether_manufacturing::BuildRecord> =
        serde_json::from_value(Value::Array(records)).context("parse build records")?;
    Ok(aether_manufacturing::Genealogy::new(parsed))
}

/// `aether model verify <file>` — evaluate a model-card promotion gate.
pub fn cmd_model_verify(file: PathBuf, approver_role: String, mode: OutputMode) -> Result<()> {
    let (card, gate) = read_model_doc(&file)?;
    let decision = aether_model_registry::evaluate_promotion(&card, &gate, &approver_role);
    emit(mode, &decision)?;
    if !decision.passed {
        std::process::exit(30);
    }
    Ok(())
}

/// `aether model promote <file>` — evaluate a gate and emit the promoted card.
pub fn cmd_model_promote(
    file: PathBuf,
    approver_role: String,
    output: Option<PathBuf>,
    mode: OutputMode,
) -> Result<()> {
    let (mut card, gate) = read_model_doc(&file)?;
    let decision = aether_model_registry::evaluate_promotion(&card, &gate, &approver_role);
    if !decision.passed {
        emit(mode, &decision)?;
        std::process::exit(30);
    }
    card.state = aether_model_registry::ModelState::Promoted;
    card.approved_by = Some(approver_role.clone());
    if let Some(path) = &output {
        let bytes = serde_json::to_vec_pretty(&card)?;
        std::fs::write(path, &bytes)
            .with_context(|| format!("write {}", path.display()))?;
    }
    emit(
        mode,
        &json!({
            "decision": decision,
            "model": card,
            "output": output.map(|p| p.display().to_string()),
        }),
    )
}

fn read_model_doc(
    file: &Path,
) -> Result<(
    aether_model_registry::ModelCard,
    aether_model_registry::PromotionGate,
)> {
    let raw = std::fs::read_to_string(file)
        .with_context(|| format!("read {}", file.display()))?;
    let doc: Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse {}", file.display()))?;
    let model_value = doc.get("model").cloned().unwrap_or_else(|| doc.clone());
    let card: aether_model_registry::ModelCard =
        serde_json::from_value(model_value).context("parse model card")?;
    let gate: aether_model_registry::PromotionGate = match doc.get("gate") {
        Some(value) => serde_json::from_value(value.clone()).context("parse promotion gate")?,
        None => aether_model_registry::PromotionGate::default(),
    };
    Ok((card, gate))
}

/// `aether fidelity plan <file>` — deterministic multi-fidelity selection.
pub fn cmd_fidelity_plan(file: PathBuf, mode: OutputMode) -> Result<()> {
    let raw = std::fs::read_to_string(&file)
        .with_context(|| format!("read {}", file.display()))?;
    let request: aether_fidelity::FidelityRequest = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse {}", file.display()))?;
    let plan = aether_fidelity::plan(&request).map_err(|e| anyhow!("{e}"))?;
    emit(
        mode,
        &json!({
            "request": request,
            "plan": plan,
            "claim": "declared errors are claims; validation evidence is required before use",
        }),
    )
}

/// Structured schema list result, used by tests.
#[derive(Debug, Serialize)]
pub struct SchemaSummary {
    /// Contract ids.
    pub contracts: Vec<String>,
    /// Node type ids.
    pub nodes: Vec<String>,
    /// Edge type ids.
    pub edges: Vec<String>,
}

/// Build a schema summary without printing (for tests and SDK use).
pub fn schema_summary(root: &Path) -> Result<SchemaSummary> {
    let contracts = list_contracts(root)?
        .into_iter()
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_string))
        .collect();
    let (nodes, edges) = list_graph_types(root)?;
    Ok(SchemaSummary {
        contracts,
        nodes,
        edges,
    })
}

/// A stable map of object id to revision used in snapshot reports.
pub fn snapshot_revisions(snapshot: &ConfigurationSnapshot) -> BTreeMap<String, u32> {
    snapshot
        .entries
        .iter()
        .map(|(id, entry)| (id.clone(), entry.revision))
        .collect()
}

#[allow(dead_code)]
fn _object_id(value: &str) -> ObjectId {
    ObjectId::new(value)
}
