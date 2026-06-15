//! Risk classification for operations.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The risk an operation carries, used by the policy engine and the token
/// capability model to decide whether an operation may proceed and whether it
/// requires human confirmation.
///
/// The ordering is meaningful: `Info < Low < Medium < High < Critical`. A token
/// that allows up to `Medium` must deny any operation classified `High` or
/// `Critical`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Read-only, no side effects (e.g. `server.inspect`).
    Info,
    /// Reversible change with negligible blast radius.
    Low,
    /// Reversible change that touches a running service or config.
    Medium,
    /// Potentially disruptive; requires confirmation by default.
    High,
    /// Destructive or irreversible without a checkpoint; always confirmed.
    Critical,
}

impl RiskLevel {
    /// Returns `true` if this risk level is at or above `threshold`.
    #[must_use]
    pub fn at_least(self, threshold: RiskLevel) -> bool {
        self >= threshold
    }

    /// The default confirmation threshold: operations at `High` or above
    /// require explicit human confirmation unless policy overrides it.
    #[must_use]
    pub fn requires_confirmation_by_default(self) -> bool {
        self >= RiskLevel::High
    }

    /// Stable string identifier used in audit records and the wire protocol.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RiskLevel::Info => "info",
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_total_and_intuitive() {
        assert!(RiskLevel::Info < RiskLevel::Low);
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn confirmation_threshold_starts_at_high() {
        assert!(!RiskLevel::Medium.requires_confirmation_by_default());
        assert!(RiskLevel::High.requires_confirmation_by_default());
        assert!(RiskLevel::Critical.requires_confirmation_by_default());
    }

    #[test]
    fn at_least_compares_against_threshold() {
        assert!(RiskLevel::High.at_least(RiskLevel::Medium));
        assert!(!RiskLevel::Low.at_least(RiskLevel::High));
    }

    #[test]
    fn serializes_to_lowercase_tag() {
        let json = serde_json::to_string(&RiskLevel::Critical).unwrap();
        assert_eq!(json, "\"critical\"");
        let parsed: RiskLevel = serde_json::from_str("\"medium\"").unwrap();
        assert_eq!(parsed, RiskLevel::Medium);
    }
}
