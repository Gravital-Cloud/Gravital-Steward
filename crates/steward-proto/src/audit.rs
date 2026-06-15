//! Audit event schema.
//!
//! The audit log is append-only, hash-chained (BLAKE3) and signed (Ed25519).
//! This module defines the *shape* of a single event. The cryptographic
//! chaining and signing live in the `steward-audit` crate (Phase 1); here we
//! pin the serialized contract so producers and SIEM consumers agree on it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single append-only audit event.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditEvent {
    /// Unique event identifier, e.g. `evt_01J...`.
    pub audit_id: String,
    /// RFC 3339 timestamp of the event.
    pub timestamp: String,
    /// The agent or principal that triggered the action.
    pub actor: String,
    /// The token id presented, for traceability (never the token material).
    pub token_id: String,
    /// Capabilities the token carried at the time of the action.
    pub capabilities: Vec<String>,
    /// Canonical operation id, e.g. `db.create`.
    pub operation: String,
    /// Risk level of the operation at execution time.
    pub risk: String,
    /// Outcome of the action.
    pub outcome: AuditOutcome,
    /// Whether a rollback checkpoint was available.
    pub rollback_available: bool,
    /// BLAKE3 hash of the previous event, forming the integrity chain.
    /// Empty for the genesis event.
    #[serde(default)]
    pub prev_hash: String,
}

/// Terminal outcome recorded for an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// The operation applied and verified successfully.
    Succeeded,
    /// The operation failed and the system was left consistent.
    Failed,
    /// The operation failed and an automatic rollback was performed.
    RolledBack,
    /// The operation was denied by policy before any effect.
    Denied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serializes_with_chain_field() {
        let event = AuditEvent {
            audit_id: "evt_0001".to_owned(),
            timestamp: "2026-06-15T08:00:00Z".to_owned(),
            actor: "agent-claude".to_owned(),
            token_id: "tok_abc".to_owned(),
            capabilities: vec!["server.inspect".to_owned()],
            operation: "server.inspect".to_owned(),
            risk: "info".to_owned(),
            outcome: AuditOutcome::Succeeded,
            rollback_available: false,
            prev_hash: String::new(),
        };
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["outcome"], "succeeded");
        assert_eq!(value["operation"], "server.inspect");
        assert!(value.get("prev_hash").is_some());
    }

    #[test]
    fn outcome_uses_snake_case() {
        let json = serde_json::to_string(&AuditOutcome::RolledBack).unwrap();
        assert_eq!(json, "\"rolled_back\"");
    }
}
