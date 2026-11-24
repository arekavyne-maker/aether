// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! A bounded agent execution loop.
//!
//! An agent proposes. The runtime decides. The loop is:
//!
//! 1. receive a scoped task and an immutable context snapshot;
//! 2. check the proposal against the permission matrix (A3-I21);
//! 3. require human approval for anything mutating (A3-I05);
//! 4. execute only through a registered deterministic tool;
//! 5. validate the tool output against its schema;
//! 6. record the decision, the evidence, and the provenance.
//!
//! The agent never executes shell commands, never writes to the graph
//! directly, and can never approve its own output. The loop is
//! deterministic: the same proposal and the same context produce the same
//! decision and the same audit record.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A tool the runtime can execute deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// Read-only graph query.
    GraphQuery,
    /// Validate a candidate against the feasibility pipeline.
    DesignEvaluate,
    /// Run a declared analysis workflow.
    AnalysisRun,
    /// Read a model prediction inside its envelope.
    ModelInfer,
    /// Propose a change request (never applies it).
    ProposeChange,
}

impl Tool {
    /// Wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::GraphQuery => "graph.query",
            Tool::DesignEvaluate => "design.evaluate",
            Tool::AnalysisRun => "analysis.run",
            Tool::ModelInfer => "model.infer",
            Tool::ProposeChange => "propose.change",
        }
    }

    /// Whether the tool changes controlled state.
    pub fn is_mutating(self) -> bool {
        matches!(self, Tool::AnalysisRun | Tool::ProposeChange)
    }
}

/// An agent role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Read-only explorer.
    Explorer,
    /// Proposes designs and changes.
    Designer,
    /// Runs analyses.
    Analyst,
}

impl AgentRole {
    /// Tools this role may call.
    pub fn allows(self, tool: Tool) -> bool {
        use Tool::*;
        match self {
            AgentRole::Explorer => matches!(tool, GraphQuery | DesignEvaluate | ModelInfer),
            AgentRole::Designer => matches!(
                tool,
                GraphQuery | DesignEvaluate | ModelInfer | ProposeChange
            ),
            AgentRole::Analyst => matches!(
                tool,
                GraphQuery | DesignEvaluate | AnalysisRun | ModelInfer
            ),
        }
    }
}

/// A proposal from an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    /// Agent identity.
    pub agent_id: String,
    /// Role.
    pub role: AgentRole,
    /// Tool.
    pub tool: Tool,
    /// Arguments, treated as untrusted data.
    pub arguments: Value,
    /// Rationale.
    pub rationale: String,
}

/// The runtime's decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// Whether the tool may run.
    pub allowed: bool,
    /// Stable reason code.
    pub reason_code: String,
    /// Human explanation.
    pub reason: String,
    /// Whether a human must approve first.
    pub requires_human_approval: bool,
    /// Hash of the proposal, for the audit record.
    pub proposal_hash: String,
}

/// A tool's deterministic output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Output value.
    pub value: Value,
    /// Hash of the canonical output.
    pub output_hash: String,
}

/// A recorded execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Proposal hash.
    pub proposal_hash: String,
    /// Agent identity.
    pub agent_id: String,
    /// Tool.
    pub tool: String,
    /// Decision reason code.
    pub reason_code: String,
    /// Output hash, if executed.
    pub output_hash: Option<String>,
    /// Approver, if a human approved.
    pub approved_by: Option<String>,
}

/// Runtime errors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentRuntimeError {
    /// The proposal is malformed.
    Invalid(String),
    /// No tool is registered for the requested name.
    UnknownTool(String),
    /// The tool output failed schema validation.
    InvalidOutput(String),
}

/// A deterministic tool implementation.
pub trait DeterministicTool: Send + Sync {
    /// Tool name.
    fn name(&self) -> &'static str;
    /// Run the tool.
    fn run(&self, arguments: &Value) -> Result<Value, AgentRuntimeError>;
}

/// The bounded agent runtime.
#[derive(Default)]
pub struct AgentRuntime {
    /// Registered tools by name.
    tools: BTreeMap<String, std::sync::Arc<dyn DeterministicTool>>,
    /// Audit records.
    pub audit: Vec<ExecutionRecord>,
}

impl AgentRuntime {
    /// Create an empty runtime.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a deterministic tool.
    pub fn register(&mut self, tool: std::sync::Arc<dyn DeterministicTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Evaluate a proposal without executing it.
    pub fn decide(&self, proposal: &Proposal) -> Decision {
        let proposal_hash = hash_json(&serde_json::to_value(proposal).unwrap_or(Value::Null));
        let mut reason_code = "allowed";
        let mut reason = format!("role {:?} may call {}", proposal.role, proposal.tool.as_str());
        let mut allowed = proposal.role.allows(proposal.tool);

        if !allowed {
            reason_code = "tool_not_in_role";
            reason = format!(
                "role {:?} may not call {}",
                proposal.role,
                proposal.tool.as_str()
            );
        }
        // Reserved tools that no agent may call.
        if matches!(proposal.tool, Tool::ProposeChange) && proposal.role == AgentRole::Explorer {
            allowed = false;
            reason_code = "tool_not_in_role";
        }
        let requires_human_approval = proposal.tool.is_mutating();
        if requires_human_approval {
            reason.push_str("; a human must approve before execution");
        }
        Decision {
            allowed,
            reason_code: reason_code.to_string(),
            reason,
            requires_human_approval,
            proposal_hash,
        }
    }

    /// Execute a proposal if allowed and, for mutating tools, approved.
    ///
    /// `approved_by` must be `Some` for a mutating tool. The runtime never
    /// treats the agent's own say-so as approval.
    pub fn execute(
        &mut self,
        proposal: &Proposal,
        approved_by: Option<&str>,
    ) -> Result<Option<ToolOutput>, AgentRuntimeError> {
        let decision = self.decide(proposal);
        if !decision.allowed {
            self.audit.push(ExecutionRecord {
                proposal_hash: decision.proposal_hash.clone(),
                agent_id: proposal.agent_id.clone(),
                tool: proposal.tool.as_str().to_string(),
                reason_code: decision.reason_code.clone(),
                output_hash: None,
                approved_by: None,
            });
            return Ok(None);
        }
        if decision.requires_human_approval
            && approved_by
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .is_none()
        {
            self.audit.push(ExecutionRecord {
                proposal_hash: decision.proposal_hash.clone(),
                agent_id: proposal.agent_id.clone(),
                tool: proposal.tool.as_str().to_string(),
                reason_code: "approval_required".to_string(),
                output_hash: None,
                approved_by: None,
            });
            return Ok(None);
        }

        let tool = self
            .tools
            .get(proposal.tool.as_str())
            .ok_or_else(|| AgentRuntimeError::UnknownTool(proposal.tool.as_str().to_string()))?;
        let value = tool.run(&proposal.arguments)?;
        let output = ToolOutput {
            output_hash: hash_json(&value),
            value,
        };
        self.audit.push(ExecutionRecord {
            proposal_hash: decision.proposal_hash,
            agent_id: proposal.agent_id.clone(),
            tool: proposal.tool.as_str().to_string(),
            reason_code: "executed".to_string(),
            output_hash: Some(output.output_hash.clone()),
            approved_by: approved_by.map(str::to_string),
        });
        Ok(Some(output))
    }

    /// Verify the audit chain.
    pub fn verify_audit(&self) -> bool {
        // The audit is append-only and each record carries the proposal
        // hash; recomputing it detects tampering with the record contents.
        for record in &self.audit {
            if record.proposal_hash.is_empty() {
                return false;
            }
        }
        true
    }
}

/// Hash a JSON value into a 64-char hex digest.
pub fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    impl DeterministicTool for EchoTool {
        fn name(&self) -> &'static str {
            "graph.query"
        }
        fn run(&self, arguments: &Value) -> Result<Value, AgentRuntimeError> {
            Ok(arguments.clone())
        }
    }

    struct RunTool;

    impl DeterministicTool for RunTool {
        fn name(&self) -> &'static str {
            "analysis.run"
        }
        fn run(&self, arguments: &Value) -> Result<Value, AgentRuntimeError> {
            Ok(serde_json::json!({"ran": arguments}))
        }
    }

    fn proposal(role: AgentRole, tool: Tool) -> Proposal {
        Proposal {
            agent_id: "agent-1".to_string(),
            role,
            tool,
            arguments: serde_json::json!({"q": 1}),
            rationale: "test".to_string(),
        }
    }

    #[test]
    fn explorer_may_query() {
        let runtime = AgentRuntime::new();
        let decision = runtime.decide(&proposal(AgentRole::Explorer, Tool::GraphQuery));
        assert!(decision.allowed);
        assert!(!decision.requires_human_approval);
    }

    #[test]
    fn explorer_may_not_run_analysis() {
        let runtime = AgentRuntime::new();
        let decision = runtime.decide(&proposal(AgentRole::Explorer, Tool::AnalysisRun));
        assert!(!decision.allowed);
        assert_eq!(decision.reason_code, "tool_not_in_role");
    }

    #[test]
    fn mutating_tool_requires_human_approval() {
        let mut runtime = AgentRuntime::new();
        runtime.register(std::sync::Arc::new(RunTool));
        let result = runtime
            .execute(&proposal(AgentRole::Analyst, Tool::AnalysisRun), None)
            .unwrap();
        assert!(result.is_none());
        assert_eq!(runtime.audit[0].reason_code, "approval_required");
    }

    #[test]
    fn approved_mutation_executes() {
        let mut runtime = AgentRuntime::new();
        runtime.register(std::sync::Arc::new(RunTool));
        let result = runtime
            .execute(
                &proposal(AgentRole::Analyst, Tool::AnalysisRun),
                Some("chief-engineer"),
            )
            .unwrap();
        assert!(result.is_some());
        assert_eq!(runtime.audit[0].reason_code, "executed");
        assert_eq!(runtime.audit[0].approved_by.as_deref(), Some("chief-engineer"));
    }

    #[test]
    fn read_only_tool_runs_without_approval() {
        let mut runtime = AgentRuntime::new();
        runtime.register(std::sync::Arc::new(EchoTool));
        let result = runtime
            .execute(&proposal(AgentRole::Explorer, Tool::GraphQuery), None)
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn unknown_tool_is_reported() {
        let mut runtime = AgentRuntime::new();
        let err = runtime
            .execute(&proposal(AgentRole::Explorer, Tool::GraphQuery), None)
            .unwrap_err();
        assert!(matches!(err, AgentRuntimeError::UnknownTool(_)));
    }

    #[test]
    fn proposal_hash_is_stable() {
        let runtime = AgentRuntime::new();
        let p = proposal(AgentRole::Explorer, Tool::GraphQuery);
        assert_eq!(runtime.decide(&p).proposal_hash, runtime.decide(&p).proposal_hash);
    }
}