//! Declarative desired-state manifest.
//!
//! An agent submits a [`Manifest`] describing the *desired* state; the
//! reconciler computes the diff against the current state and converges by
//! running the necessary operations idempotently. Re-submitting the same
//! manifest produces no changes.
//!
//! This module models the schema documented in the blueprint (§10). Phase 0
//! pins the types and round-trips them through YAML/JSON; the reconciler that
//! consumes them arrives in Phase 3.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Top-level manifest envelope.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Manifest {
    /// Schema version, e.g. `steward/v1`.
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    /// The kind of resource described.
    pub kind: ManifestKind,
    /// Identifying metadata.
    pub metadata: Metadata,
    /// Desired-state specification.
    pub spec: serde_json::Value,
}

/// The kind of resource a manifest describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub enum ManifestKind {
    /// A deployable application with its dependencies and exposure.
    Deployment,
}

/// Manifest metadata identifying the project and target server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Metadata {
    /// Project / application name, e.g. `app-web`.
    pub project: String,
    /// Target server identifier, e.g. `srv-prod-1`.
    pub server: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // The wire encoding is JSON (JSON-RPC over MCP). YAML is purely a
    // human-authoring convenience handled by the CLI, so this crate exercises
    // the canonical JSON form documented in the protocol spec.
    const EXAMPLE: &str = r#"{
      "apiVersion": "steward/v1",
      "kind": "Deployment",
      "metadata": { "project": "app-web", "server": "srv-prod-1" },
      "spec": { "expose": { "domain": "app.example.com", "tls": "auto" } }
    }"#;

    #[test]
    fn deserializes_canonical_example() {
        let manifest: Manifest = serde_json::from_str(EXAMPLE).unwrap();
        assert_eq!(manifest.api_version, "steward/v1");
        assert_eq!(manifest.kind, ManifestKind::Deployment);
        assert_eq!(manifest.metadata.project, "app-web");
        assert_eq!(manifest.spec["expose"]["domain"], "app.example.com");
    }

    #[test]
    fn round_trips_through_json() {
        let manifest: Manifest = serde_json::from_str(EXAMPLE).unwrap();
        let json = serde_json::to_string(&manifest).unwrap();
        let back: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.metadata.server, "srv-prod-1");
        assert_eq!(back.api_version, manifest.api_version);
    }
}
