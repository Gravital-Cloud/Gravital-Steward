//! Core error type.
//!
//! Domain crates surface failures through richer, structured protocol errors
//! (see `steward-proto`). At the core level we only need a small, dependency-light
//! error type for lifecycle plumbing.

use thiserror::Error;

/// Convenience result alias for core operations.
pub type Result<T> = std::result::Result<T, CoreError>;

/// Errors that can arise while orchestrating the operation lifecycle.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// The requested operation is not registered in the operation registry.
    #[error("unknown operation: {0}")]
    UnknownOperation(String),

    /// The presented token does not grant the capabilities the operation requires.
    #[error(
        "capability denied: operation '{operation}' requires capabilities not granted by the token"
    )]
    CapabilityDenied {
        /// The operation that was denied.
        operation: String,
    },

    /// The operation's risk exceeds the maximum risk permitted by the token.
    #[error(
        "risk exceeds token: operation risk '{operation_risk}' exceeds token maximum '{token_max}'"
    )]
    RiskExceedsToken {
        /// Risk level the operation declared.
        operation_risk: String,
        /// Maximum risk the token permits.
        token_max: String,
    },

    /// The plan failed validation and must not be applied.
    #[error("plan validation failed: {0}")]
    ValidationFailed(String),

    /// Post-apply verification failed; the engine will roll back.
    #[error("verification failed: {0}")]
    VerificationFailed(String),

    /// An error occurred while serializing or deserializing a payload.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A catch-all for domain executor failures, carrying a human-readable message.
    #[error("operation failed: {0}")]
    Operation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_denied_renders_operation() {
        let err = CoreError::CapabilityDenied {
            operation: "db.drop".to_owned(),
        };
        assert!(err.to_string().contains("db.drop"));
    }

    #[test]
    fn serialization_error_converts_from_serde() {
        let serde_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let core: CoreError = serde_err.into();
        assert!(matches!(core, CoreError::Serialization(_)));
    }
}
