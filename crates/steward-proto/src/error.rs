//! Structured error protocol.
//!
//! The runtime never returns raw shell or kernel output to an agent. Every
//! failure is a typed, actionable [`StructuredError`] that an LLM can reason
//! about: a stable [`ErrorCode`], a [`Severity`], a human-readable message,
//! machine-readable `context`, and a list of [`SuggestedAction`]s the agent may
//! take next.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Versioned taxonomy of error codes. The string representation is stable and
/// part of the public protocol contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ErrorCode {
    /// A required network port is already bound by another process.
    PortInUse,
    /// The presented token does not grant the required capability.
    CapabilityDenied,
    /// The operation's risk exceeds the token's maximum permitted risk.
    RiskExceedsToken,
    /// Dependency resolution produced an unsatisfiable conflict.
    DepConflict,
    /// Post-apply verification failed.
    VerifyFailed,
    /// A checkpoint was restored as part of an automatic rollback.
    CheckpointRestored,
    /// The submitted manifest is malformed or internally inconsistent.
    InvalidManifest,
    /// The operation requires human confirmation that was not provided.
    ConfirmationRequired,
    /// An unexpected internal error the agent cannot act upon directly.
    Internal,
}

impl ErrorCode {
    /// Stable string representation, e.g. `"PORT_IN_USE"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::PortInUse => "PORT_IN_USE",
            ErrorCode::CapabilityDenied => "CAPABILITY_DENIED",
            ErrorCode::RiskExceedsToken => "RISK_EXCEEDS_TOKEN",
            ErrorCode::DepConflict => "DEP_CONFLICT",
            ErrorCode::VerifyFailed => "VERIFY_FAILED",
            ErrorCode::CheckpointRestored => "CHECKPOINT_RESTORED",
            ErrorCode::InvalidManifest => "INVALID_MANIFEST",
            ErrorCode::ConfirmationRequired => "CONFIRMATION_REQUIRED",
            ErrorCode::Internal => "INTERNAL",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Severity of a structured error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational; the operation can continue.
    Low,
    /// The operation failed but the system is consistent.
    Medium,
    /// The operation failed and may have required a rollback.
    High,
    /// A safety or integrity invariant was at risk.
    Critical,
}

/// A concrete action the agent may take to recover, with its own risk profile.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuggestedAction {
    /// Machine-readable action verb, e.g. `"change_port"`.
    pub action: String,
    /// Parameters for the suggested action.
    #[serde(default)]
    pub params: serde_json::Value,
    /// Optional risk hint for the suggested action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    /// Whether taking this action requires human confirmation.
    #[serde(default)]
    pub requires_confirmation: bool,
}

/// The structured error returned to the agent.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StructuredError {
    /// Stable error code from the taxonomy.
    pub code: ErrorCode,
    /// Severity of the failure.
    pub severity: Severity,
    /// Domain that produced the error, e.g. `"containers"`.
    pub domain: String,
    /// Human-readable explanation suitable for an LLM and a log.
    pub message: String,
    /// Structured context for programmatic handling.
    #[serde(default)]
    pub context: serde_json::Value,
    /// Ordered list of recovery options.
    #[serde(default)]
    pub suggested_actions: Vec<SuggestedAction>,
    /// Whether an automatic rollback was performed.
    #[serde(default)]
    pub rollback_performed: bool,
    /// Identifier of the audit event recording this failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_id: Option<String>,
}

impl StructuredError {
    /// Builds a minimal structured error with a code, severity, domain and message.
    pub fn new(
        code: ErrorCode,
        severity: Severity,
        domain: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            domain: domain.into(),
            message: message.into(),
            context: serde_json::Value::Null,
            suggested_actions: Vec::new(),
            rollback_performed: false,
            audit_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_round_trips_as_screaming_snake_case() {
        let json = serde_json::to_string(&ErrorCode::PortInUse).unwrap();
        assert_eq!(json, "\"PORT_IN_USE\"");
        let parsed: ErrorCode = serde_json::from_str("\"VERIFY_FAILED\"").unwrap();
        assert_eq!(parsed, ErrorCode::VerifyFailed);
    }

    #[test]
    fn structured_error_serializes_with_expected_shape() {
        let err = StructuredError {
            suggested_actions: vec![SuggestedAction {
                action: "change_port".to_owned(),
                params: serde_json::json!({ "to_range": "8081-8090" }),
                risk: None,
                requires_confirmation: false,
            }],
            ..StructuredError::new(
                ErrorCode::PortInUse,
                Severity::High,
                "containers",
                "Port 8080 is already in use.",
            )
        };
        let value = serde_json::to_value(&err).unwrap();
        assert_eq!(value["code"], "PORT_IN_USE");
        assert_eq!(value["severity"], "high");
        assert_eq!(value["suggested_actions"][0]["action"], "change_port");
    }

    #[test]
    fn schema_generates_for_structured_error() {
        // The MCP layer relies on JsonSchema derivation for tool discovery.
        let schema = schemars::schema_for!(StructuredError);
        let as_value = serde_json::to_value(schema).unwrap();
        assert!(as_value.get("properties").is_some());
    }
}
