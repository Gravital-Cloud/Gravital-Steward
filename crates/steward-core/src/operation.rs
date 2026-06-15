//! The `Operation` lifecycle contract.
//!
//! Every effectful action in Gravital-Steward implements [`Operation`]. The
//! engine (`steward-ops`) drives the uniform lifecycle:
//!
//! ```text
//! inspect -> plan -> validate -> apply -> verify -> (rollback on failure)
//! ```
//!
//! This contract is the heart of the data-oriented architecture: the engine
//! integrates policy, sandboxing and auditing around these six steps and rolls
//! back automatically when `verify` fails.

use crate::capability::CapabilitySet;
use crate::error::Result;
use crate::ids::OperationId;
use crate::risk::RiskLevel;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Execution context threaded through every lifecycle step.
///
/// In Phase 0 this is a marker carrying the invoking actor and target server.
/// Later phases extend it with handles to the audit log, secret store, sandbox
/// controller and state store, without changing the [`Operation`] contract.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OpContext {
    /// Identifier of the agent or principal invoking the operation.
    pub actor: String,
    /// Server the operation targets.
    pub server: crate::ids::ServerId,
}

impl OpContext {
    /// Builds a minimal context for the given actor and server.
    pub fn new(actor: impl Into<String>, server: impl Into<crate::ids::ServerId>) -> Self {
        Self {
            actor: actor.into(),
            server: server.into(),
        }
    }
}

/// Snapshot of the real system state read during `inspect`, before any mutation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurrentState {
    /// Free-form, operation-specific description of the observed state.
    pub summary: String,
    /// Structured details for the planner and the audit record.
    pub details: serde_json::Value,
}

/// Result of validating a plan. Validation may block an otherwise authorized
/// plan when it would violate a secure-default rule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Validation {
    /// Whether the plan is safe to apply.
    pub approved: bool,
    /// Human- and machine-readable reasons (warnings or blocking findings).
    pub findings: Vec<String>,
}

impl Validation {
    /// A passing validation with no findings.
    #[must_use]
    pub fn approved() -> Self {
        Self {
            approved: true,
            findings: Vec::new(),
        }
    }

    /// A blocking validation carrying the reason it failed.
    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            approved: false,
            findings: vec![reason.into()],
        }
    }
}

/// Opaque handle to the checkpoint created before `apply`, used by `rollback`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Checkpoint {
    /// Stable identifier of the checkpoint within the state store.
    pub id: String,
    /// Operation-specific recovery metadata (e.g. previous config, prior release).
    pub recovery: serde_json::Value,
}

/// Result of verifying an applied outcome (healthcheck, query, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Verification {
    /// Whether the post-apply state matches expectations.
    pub healthy: bool,
    /// Evidence gathered during verification.
    pub evidence: Vec<String>,
}

impl Verification {
    /// A passing verification.
    #[must_use]
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            evidence: Vec::new(),
        }
    }

    /// A failing verification carrying the reason; the engine will roll back.
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            healthy: false,
            evidence: vec![reason.into()],
        }
    }
}

/// Static metadata describing an operation, independent of any specific input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationMeta {
    /// Canonical operation id, e.g. `db.create`.
    pub id: OperationId,
    /// Capabilities a token must grant to invoke this operation.
    pub required_capabilities: CapabilitySet,
}

/// The uniform contract every effectful operation must implement.
///
/// The associated types make the operation self-describing: the engine can
/// derive JSON Schemas for `Input`, `Plan` and `Outcome` to expose the
/// operation as an MCP tool, and the LLM consumes the structured `Plan` and
/// `Outcome` rather than raw shell output.
#[async_trait]
pub trait Operation: Send + Sync {
    /// Parameters supplied by the agent.
    type Input: DeserializeOwned + JsonSchema + Send + Sync;
    /// A plan the LLM (and a human) can read before anything is applied.
    type Plan: Serialize + JsonSchema + Send + Sync;
    /// The typed result of a successful application.
    type Outcome: Serialize + JsonSchema + Send + Sync;

    /// Canonical operation id, e.g. `db.create`.
    fn id(&self) -> OperationId;

    /// Risk classification for the given input. The same operation can carry
    /// different risk depending on its parameters.
    fn risk_level(&self, input: &Self::Input) -> RiskLevel;

    /// Capabilities a token must grant to invoke this operation.
    fn required_capabilities(&self) -> CapabilitySet;

    /// Whether the operation requires explicit human confirmation at this risk.
    fn requires_human_confirmation(&self, risk: RiskLevel) -> bool {
        risk.requires_confirmation_by_default()
    }

    /// Step 1 — read the real state without mutating anything.
    async fn inspect(&self, ctx: &OpContext, input: &Self::Input) -> Result<CurrentState>;

    /// Step 2 — compute an idempotent plan (the diff between current and desired).
    async fn plan(
        &self,
        ctx: &OpContext,
        current: &CurrentState,
        input: &Self::Input,
    ) -> Result<Self::Plan>;

    /// Step 3 — validate the plan for safety and coherence. May block.
    async fn validate(&self, ctx: &OpContext, plan: &Self::Plan) -> Result<Validation>;

    /// Step 4 — create a checkpoint and apply the plan under the sandbox.
    async fn apply(
        &self,
        ctx: &OpContext,
        plan: &Self::Plan,
    ) -> Result<(Checkpoint, Self::Outcome)>;

    /// Step 5 — verify the applied outcome (healthcheck, queries, ...).
    async fn verify(&self, ctx: &OpContext, outcome: &Self::Outcome) -> Result<Verification>;

    /// Step 6 — revert using the checkpoint created during `apply`.
    async fn rollback(&self, ctx: &OpContext, checkpoint: &Checkpoint) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::ids::ServerId;

    /// A trivial, read-only operation used to exercise the lifecycle contract.
    struct Inspect;

    #[derive(Deserialize, JsonSchema)]
    struct InspectInput {
        path: String,
    }

    #[derive(Serialize, JsonSchema)]
    struct InspectPlan {
        note: String,
    }

    #[derive(Serialize, JsonSchema)]
    struct InspectOutcome {
        observed: String,
    }

    #[async_trait]
    impl Operation for Inspect {
        type Input = InspectInput;
        type Plan = InspectPlan;
        type Outcome = InspectOutcome;

        fn id(&self) -> OperationId {
            OperationId::new("server.inspect")
        }

        fn risk_level(&self, _input: &Self::Input) -> RiskLevel {
            RiskLevel::Info
        }

        fn required_capabilities(&self) -> CapabilitySet {
            CapabilitySet::empty().with(Capability::new(OperationId::new("server.inspect")))
        }

        async fn inspect(&self, _ctx: &OpContext, input: &Self::Input) -> Result<CurrentState> {
            Ok(CurrentState {
                summary: format!("read {}", input.path),
                details: serde_json::json!({ "path": input.path }),
            })
        }

        async fn plan(
            &self,
            _ctx: &OpContext,
            _current: &CurrentState,
            _input: &Self::Input,
        ) -> Result<Self::Plan> {
            Ok(InspectPlan {
                note: "no changes; read-only".to_owned(),
            })
        }

        async fn validate(&self, _ctx: &OpContext, _plan: &Self::Plan) -> Result<Validation> {
            Ok(Validation::approved())
        }

        async fn apply(
            &self,
            _ctx: &OpContext,
            _plan: &Self::Plan,
        ) -> Result<(Checkpoint, Self::Outcome)> {
            Ok((
                Checkpoint {
                    id: "cp-noop".to_owned(),
                    recovery: serde_json::Value::Null,
                },
                InspectOutcome {
                    observed: "ok".to_owned(),
                },
            ))
        }

        async fn verify(&self, _ctx: &OpContext, _outcome: &Self::Outcome) -> Result<Verification> {
            Ok(Verification::healthy())
        }

        async fn rollback(&self, _ctx: &OpContext, _checkpoint: &Checkpoint) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn read_only_operation_is_info_risk_and_needs_no_confirmation() {
        let op = Inspect;
        let input = InspectInput {
            path: "/etc/os-release".to_owned(),
        };
        let risk = op.risk_level(&input);
        assert_eq!(risk, RiskLevel::Info);
        assert!(!op.requires_human_confirmation(risk));
    }

    #[test]
    fn lifecycle_runs_end_to_end() {
        // Drive the full lifecycle on a tiny executor to prove the contract holds.
        let op = Inspect;
        let ctx = OpContext::new("agent-test", ServerId::new("srv-test"));
        let input = InspectInput {
            path: "/etc/hostname".to_owned(),
        };

        let result = pollster::block_on(async {
            let current = op.inspect(&ctx, &input).await?;
            let plan = op.plan(&ctx, &current, &input).await?;
            let validation = op.validate(&ctx, &plan).await?;
            assert!(validation.approved);
            let (checkpoint, outcome) = op.apply(&ctx, &plan).await?;
            let verification = op.verify(&ctx, &outcome).await?;
            assert!(verification.healthy);
            op.rollback(&ctx, &checkpoint).await?;
            Ok::<_, crate::error::CoreError>(())
        });

        result.expect("lifecycle should succeed");
    }
}
