// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Agent tool contracts and permission gating.
//!
//! AI assistance is an advisor, not an authority (A3-I21). An agent may
//! propose a tool call, but the call executes only if a deterministic
//! policy allows the proposed action for the agent's role. Agents never
//! receive raw graph-write or machine-control authority.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A tool an agent may invoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTool {
    /// Query the graph read-only.
    GraphQuery,
    /// Compile and explain a workflow.
    WorkflowExplain,
    /// Run an analysis workflow.
    WorkflowRun,
    /// Evaluate a design candidate.
    DesignEvaluate,
    /// Propose a design candidate.
    DesignPropose,
    /// Propose a change request.
    ProposeChange,
    /// Request a release gate evaluation.
    ReleaseGateEvaluate,
    /// Approve or release anything.
    Approve,
    /// Modify policy or roles.
    PolicyEdit,
    /// Send machine or process commands.
    MachineControl,
}

impl AgentTool {
    /// Stable wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            AgentTool::GraphQuery => "graph.query",
            AgentTool::WorkflowExplain => "workflow.explain",
            AgentTool::WorkflowRun => "workflow.run",
            AgentTool::DesignEvaluate => "design.evaluate",
            AgentTool::DesignPropose => "design.propose",
            AgentTool::ProposeChange => "propose.change",
            AgentTool::ReleaseGateEvaluate => "release.gate.evaluate",
            AgentTool::Approve => "approve",
            AgentTool::PolicyEdit => "policy.edit",
            AgentTool::MachineControl => "machine.control",
        }
    }

    /// Parse a wire label.
    pub fn parse(label: &str) -> Option<Self> {
        Some(match label {
            "graph.query" => AgentTool::GraphQuery,
            "workflow.explain" => AgentTool::WorkflowExplain,
            "workflow.run" => AgentTool::WorkflowRun,
            "design.evaluate" => AgentTool::DesignEvaluate,
            "design.propose" => AgentTool::DesignPropose,
            "propose.change" => AgentTool::ProposeChange,
            "release.gate.evaluate" => AgentTool::ReleaseGateEvaluate,
            "approve" => AgentTool::Approve,
            "policy.edit" => AgentTool::PolicyEdit,
            "machine.control" => AgentTool::MachineControl,
            _ => return None,
        })
    }

    /// Whether this tool can change controlled state.
    pub fn is_mutating(self) -> bool {
        matches!(
            self,
            AgentTool::WorkflowRun
                | AgentTool::DesignPropose
                | AgentTool::ProposeChange
                | AgentTool::Approve
                | AgentTool::PolicyEdit
                | AgentTool::MachineControl
        )
    }
}

/// An agent role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Read-only exploration assistant.
    Explorer,
    /// Proposes design candidates and change requests.
    Designer,
    /// Runs workflows inside an approved envelope.
    Analyst,
    /// Reviews evidence and recommends decisions.
    Reviewer,
}

impl AgentRole {
    /// Tools this role may call.
    pub fn allowed_tools(self) -> BTreeSet<AgentTool> {
        use AgentTool::*;
        match self {
            AgentRole::Explorer => {
                [GraphQuery, WorkflowExplain, DesignEvaluate, ReleaseGateEvaluate].into()
            }
            AgentRole::Designer => [
                GraphQuery,
                WorkflowExplain,
                DesignEvaluate,
                DesignPropose,
                ProposeChange,
                ReleaseGateEvaluate,
            ]
            .into(),
            AgentRole::Analyst => [
                GraphQuery,
                WorkflowExplain,
                WorkflowRun,
                DesignEvaluate,
                ReleaseGateEvaluate,
            ]
            .into(),
            AgentRole::Reviewer => {
                [GraphQuery, WorkflowExplain, DesignEvaluate, ReleaseGateEvaluate].into()
            }
        }
    }

    /// Label.
    pub fn as_str(self) -> &'static str {
        match self {
            AgentRole::Explorer => "explorer",
            AgentRole::Designer => "designer",
            AgentRole::Analyst => "analyst",
            AgentRole::Reviewer => "reviewer",
        }
    }
}

/// An agent proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentProposal {
    /// Proposing agent identity.
    pub agent_id: String,
    /// Agent role.
    pub role: AgentRole,
    /// Tool to invoke.
    pub tool: AgentTool,
    /// Tool arguments. Treated as untrusted input.
    pub arguments: serde_json::Value,
    /// Human-readable rationale.
    pub rationale: String,
}

/// Decision on an agent proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDecision {
    /// Whether the proposal may execute.
    pub allowed: bool,
    /// Agent identity.
    pub agent_id: String,
    /// Role.
    pub role: AgentRole,
    /// Tool.
    pub tool: AgentTool,
    /// Stable reason code.
    pub reason_code: String,
    /// Human explanation.
    pub reason: String,
    /// Whether a human must approve before execution.
    pub requires_human_approval: bool,
}

/// Agent governance errors.
#[derive(Debug, Error, PartialEq)]
pub enum AgentError {
    /// Unknown tool label.
    #[error("unknown agent tool: {0}")]
    UnknownTool(String),
    /// Empty agent identity.
    #[error("agent id is empty")]
    EmptyAgent,
}

/// Evaluate an agent proposal against the deterministic permission matrix.
///
/// The function never executes the tool. It returns a decision that a
/// separate, authorized executor must honor.
pub fn evaluate_proposal(proposal: &AgentProposal) -> Result<AgentDecision, AgentError> {
    if proposal.agent_id.trim().is_empty() {
        return Err(AgentError::EmptyAgent);
    }
    let tool = proposal.tool;
    let allowed_by_role = proposal.role.allowed_tools().contains(&tool);
    let mut reason_code = "allowed";
    let mut reason = format!(
        "role {:?} may call {:?}",
        proposal.role, tool
    );
    let mut allowed = allowed_by_role;

    if !allowed_by_role {
        reason_code = "tool_not_in_role";
        reason = format!(
            "role {:?} may not call {:?}; allowed tools are {:?}",
            proposal.role,
            tool,
            proposal.role.allowed_tools().iter().map(|t| t.as_str()).collect::<Vec<_>>()
        );
    }

    // Mutating tools always need an explicit human approval decision, even
    // for roles that may propose them.
    let requires_human_approval = tool.is_mutating();
    if requires_human_approval {
        reason.push_str("; execution requires an authorized human approval decision");
    }

    // A few tools are never available to any agent role, regardless of
    // future role catalog edits.
    let prohibited = matches!(tool, AgentTool::Approve | AgentTool::PolicyEdit | AgentTool::MachineControl);
    if prohibited {
        allowed = false;
        reason_code = "prohibited_for_agents";
        reason = format!(
            "{:?} is reserved for authorized humans and is never granted to agents",
            tool
        );
    }

    Ok(AgentDecision {
        allowed,
        agent_id: proposal.agent_id.clone(),
        role: proposal.role,
        tool,
        reason_code: reason_code.to_string(),
        reason,
        requires_human_approval,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(role: AgentRole, tool: AgentTool) -> AgentProposal {
        AgentProposal {
            agent_id: "agent-1".to_string(),
            role,
            tool,
            arguments: serde_json::json!({}),
            rationale: "test".to_string(),
        }
    }

    #[test]
    fn explorer_can_query() {
        let decision = evaluate_proposal(&proposal(AgentRole::Explorer, AgentTool::GraphQuery)).unwrap();
        assert!(decision.allowed);
        assert!(!decision.requires_human_approval);
    }

    #[test]
    fn explorer_cannot_run_workflows() {
        let decision =
            evaluate_proposal(&proposal(AgentRole::Explorer, AgentTool::WorkflowRun)).unwrap();
        assert!(!decision.allowed);
        assert_eq!(decision.reason_code, "tool_not_in_role");
    }

    #[test]
    fn designer_proposals_need_human_approval() {
        let decision =
            evaluate_proposal(&proposal(AgentRole::Designer, AgentTool::DesignPropose)).unwrap();
        assert!(decision.allowed);
        assert!(decision.requires_human_approval);
    }

    #[test]
    fn no_agent_can_approve_or_control_machines() {
        for role in [
            AgentRole::Explorer,
            AgentRole::Designer,
            AgentRole::Analyst,
            AgentRole::Reviewer,
        ] {
            for tool in [AgentTool::Approve, AgentTool::PolicyEdit, AgentTool::MachineControl] {
                let decision = evaluate_proposal(&proposal(role, tool)).unwrap();
                assert!(!decision.allowed, "{role:?} {tool:?}");
                assert_eq!(decision.reason_code, "prohibited_for_agents");
            }
        }
    }

    #[test]
    fn empty_agent_is_rejected() {
        let mut p = proposal(AgentRole::Explorer, AgentTool::GraphQuery);
        p.agent_id = "  ".to_string();
        assert!(matches!(evaluate_proposal(&p), Err(AgentError::EmptyAgent)));
    }

    #[test]
    fn tool_labels_round_trip() {
        for tool in [
            AgentTool::GraphQuery,
            AgentTool::WorkflowExplain,
            AgentTool::WorkflowRun,
            AgentTool::DesignEvaluate,
            AgentTool::DesignPropose,
            AgentTool::ProposeChange,
            AgentTool::ReleaseGateEvaluate,
            AgentTool::Approve,
            AgentTool::PolicyEdit,
            AgentTool::MachineControl,
        ] {
            assert_eq!(AgentTool::parse(tool.as_str()), Some(tool));
        }
    }
}