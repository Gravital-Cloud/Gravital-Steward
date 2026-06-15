//! Capability model.
//!
//! A [`Capability`] names a single operation an actor is allowed to invoke. A
//! [`CapabilitySet`] is the closed set of capabilities granted to a token. The
//! authorization engine (`steward-auth` / `steward-policy`) resolves a token to
//! a `CapabilitySet` and checks the requested operation against it. Tokens are
//! born minimal (least privilege) and capabilities are granted explicitly.

use crate::ids::OperationId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A single granted capability, identified by the operation it authorizes.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct Capability(OperationId);

impl Capability {
    /// Builds a capability for the given operation id.
    pub fn new(operation: impl Into<OperationId>) -> Self {
        Self(operation.into())
    }

    /// The operation this capability authorizes.
    #[must_use]
    pub fn operation(&self) -> &OperationId {
        &self.0
    }
}

impl From<OperationId> for Capability {
    fn from(value: OperationId) -> Self {
        Self(value)
    }
}

/// A closed set of capabilities. Used both for the capabilities an operation
/// *requires* and the capabilities a token *grants*.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct CapabilitySet {
    capabilities: BTreeSet<Capability>,
}

impl CapabilitySet {
    /// An empty capability set (grants nothing).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Inserts a capability, returning `self` for builder-style construction.
    #[must_use]
    pub fn with(mut self, capability: Capability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    /// Inserts a capability in place.
    pub fn insert(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }

    /// Returns `true` if this set contains the given capability.
    #[must_use]
    pub fn contains(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Returns `true` if every capability required by `required` is granted by
    /// this set. This is the core authorization predicate: a token's granted
    /// set must be a superset of an operation's required set.
    #[must_use]
    pub fn grants_all(&self, required: &CapabilitySet) -> bool {
        required
            .capabilities
            .iter()
            .all(|cap| self.capabilities.contains(cap))
    }

    /// Number of capabilities in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Returns `true` if the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    /// Iterates over the capabilities in deterministic (sorted) order.
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }
}

impl FromIterator<Capability> for CapabilitySet {
    fn from_iter<I: IntoIterator<Item = Capability>>(iter: I) -> Self {
        Self {
            capabilities: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(id: &str) -> Capability {
        Capability::new(OperationId::new(id))
    }

    #[test]
    fn superset_grants_all_required() {
        let granted = CapabilitySet::empty()
            .with(cap("server.inspect"))
            .with(cap("dependency.install"))
            .with(cap("deploy.from_github"));
        let required = CapabilitySet::empty().with(cap("server.inspect"));
        assert!(granted.grants_all(&required));
    }

    #[test]
    fn missing_capability_denies() {
        let granted = CapabilitySet::empty().with(cap("server.inspect"));
        let required = CapabilitySet::empty().with(cap("db.drop"));
        assert!(!granted.grants_all(&required));
    }

    #[test]
    fn empty_required_is_always_granted() {
        let granted = CapabilitySet::empty();
        assert!(granted.grants_all(&CapabilitySet::empty()));
    }

    #[test]
    fn iteration_is_deterministic() {
        let set: CapabilitySet = [cap("b"), cap("a"), cap("c")].into_iter().collect();
        let ordered: Vec<_> = set
            .iter()
            .map(|c| c.operation().as_str().to_owned())
            .collect();
        assert_eq!(ordered, vec!["a", "b", "c"]);
    }
}
