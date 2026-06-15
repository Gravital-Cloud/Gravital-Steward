//! The outcome of a policy evaluation.

/// The single decision returned by the engine for a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The operation is authorized and may proceed.
    Allow,
    /// The operation is authorized but needs explicit human confirmation first.
    RequiresConfirmation,
    /// The operation is denied; the reason is the highest-precedence failure.
    Deny(DenyReason),
}

impl Decision {
    /// Returns `true` only for [`Decision::Allow`].
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allow)
    }
}

/// Why a request was denied. Exactly one reason is reported, following the fixed
/// precedence in [`crate::decide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    /// The token's expiry is at or before the request time.
    Expired,
    /// The target server is not within the token's scope.
    ServerOutOfScope,
    /// The operation is on the token's explicit deny list.
    ExplicitlyDenied,
    /// The operation's risk exceeds the token's maximum permitted risk.
    RiskExceedsMax,
    /// The token does not grant all capabilities the operation requires.
    CapabilityMissing,
}

impl DenyReason {
    /// Stable string code, suitable for logs and mapping to a protocol error.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DenyReason::Expired => "EXPIRED",
            DenyReason::ServerOutOfScope => "SERVER_OUT_OF_SCOPE",
            DenyReason::ExplicitlyDenied => "EXPLICITLY_DENIED",
            DenyReason::RiskExceedsMax => "RISK_EXCEEDS_TOKEN",
            DenyReason::CapabilityMissing => "CAPABILITY_DENIED",
        }
    }
}
