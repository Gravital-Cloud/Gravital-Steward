//! Inputs to a policy evaluation: what a token grants and what is being attempted.

use std::collections::BTreeSet;
use steward_core::{CapabilitySet, OperationId, RiskLevel, ServerId};

/// The set of servers a token may act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerScope {
    /// The token may act on any server.
    Any,
    /// The token is restricted to this explicit set of servers.
    Only(BTreeSet<ServerId>),
}

impl ServerScope {
    /// Builds an `Only` scope from an iterator of server ids.
    pub fn only<I: IntoIterator<Item = ServerId>>(servers: I) -> Self {
        ServerScope::Only(servers.into_iter().collect())
    }

    /// Returns `true` if `server` is within scope.
    #[must_use]
    pub fn contains(&self, server: &ServerId) -> bool {
        match self {
            ServerScope::Any => true,
            ServerScope::Only(set) => set.contains(server),
        }
    }
}

/// What a validated token permits. Produced by `steward-auth` after a token is
/// authenticated; consumed by [`crate::decide`].
#[derive(Debug, Clone)]
pub struct TokenGrant {
    /// Capabilities the token holds.
    pub granted: CapabilitySet,
    /// Operations explicitly prohibited; these win over any granted capability.
    pub denied: BTreeSet<OperationId>,
    /// Highest risk level the token may invoke.
    pub max_risk: RiskLevel,
    /// Risk level at or above which human confirmation is required.
    pub confirm_above: RiskLevel,
    /// Servers the token may act on.
    pub scope_servers: ServerScope,
    /// Token expiry as Unix seconds.
    pub expires_unix: i64,
}

/// A single operation attempt to be authorized.
#[derive(Debug, Clone)]
pub struct PolicyRequest {
    /// The operation being attempted.
    pub operation: OperationId,
    /// Capabilities the operation requires.
    pub required: CapabilitySet,
    /// Risk of the operation for the given input.
    pub risk: RiskLevel,
    /// The target server.
    pub server: ServerId,
    /// Whether a human confirmation has already been supplied.
    pub confirmed: bool,
    /// Current time as Unix seconds.
    pub now_unix: i64,
}
