//! The specification used to issue a token.

use steward_core::RiskLevel;

/// Describes a token to be issued. All fields are minimal by default; the
/// caller grants capabilities and scope explicitly (least privilege).
#[derive(Debug, Clone)]
pub struct TokenSpec {
    /// Stable identifier embedded in the token.
    pub token_id: String,
    /// Optional human-readable label (kept by the caller for its record).
    pub label: Option<String>,
    /// Capabilities (operation ids) the token grants.
    pub capabilities: Vec<String>,
    /// Operations the token explicitly denies.
    pub denied: Vec<String>,
    /// Highest risk the token may invoke.
    pub max_risk: RiskLevel,
    /// Risk at or above which human confirmation is required.
    pub confirm_above: RiskLevel,
    /// Whether the token may act on any server.
    pub scope_any: bool,
    /// Explicit server scope when `scope_any` is false.
    pub scope_servers: Vec<String>,
    /// Time-to-live in seconds; the expiry is `now + ttl_seconds`.
    pub ttl_seconds: i64,
}
