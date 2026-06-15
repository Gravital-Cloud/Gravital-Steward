//! The persisted record of an issued token (metadata only).

use serde::{Deserialize, Serialize};
use steward_core::{CapabilitySet, OperationId, RiskLevel, ServerId, TokenId};

/// Metadata of an issued token. Holds no token material and no secrets — only
/// what the authorization engine needs to reconstruct a grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRecord {
    /// Identifier of the token.
    pub id: TokenId,
    /// Optional human-readable label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Capabilities the token grants.
    pub capabilities: CapabilitySet,
    /// Operations explicitly denied to the token.
    #[serde(default)]
    pub denied: Vec<OperationId>,
    /// Highest risk the token may invoke.
    pub max_risk: RiskLevel,
    /// Risk at or above which human confirmation is required.
    pub confirm_above: RiskLevel,
    /// Whether the token may act on any server.
    pub scope_any: bool,
    /// Explicit server scope when `scope_any` is false.
    #[serde(default)]
    pub scope_servers: Vec<ServerId>,
    /// Creation time as Unix seconds.
    pub created_unix: i64,
    /// Expiry as Unix seconds.
    pub expires_unix: i64,
}
