//! # steward-core
//!
//! Core domain types and the `Operation` lifecycle contract that govern the
//! entire Gravital-Steward runtime.
//!
//! This crate is intentionally free of domain logic. It defines *what* an
//! operation is, *how risky* it is, *which capabilities* it requires, and the
//! mandatory `inspect -> plan -> validate -> apply -> verify -> rollback`
//! lifecycle every effectful operation must implement. Domain crates
//! (`steward-system`, `steward-db`, ...) depend on this crate and provide the
//! concrete implementations.
//!
//! ## Design invariants
//!
//! - **Idempotency is mandatory.** Re-applying the same desired state produces
//!   no changes. See ADR-0003.
//! - **Risk is explicit.** An operation cannot exist without declaring its
//!   [`RiskLevel`] and the [`CapabilitySet`] it requires.
//! - **Reversibility is first-class.** Every effectful operation produces a
//!   [`Checkpoint`] so the engine can roll back on a failed verification.
//!
//! These guarantees are encoded in the type system so that invalid states are
//! difficult to represent.

#![forbid(unsafe_code)]

pub mod capability;
pub mod error;
pub mod ids;
pub mod operation;
pub mod risk;

pub use capability::{Capability, CapabilitySet};
pub use error::{CoreError, Result};
pub use ids::{AuditId, OperationId, ProjectId, ServerId, TokenId};
pub use operation::{
    Checkpoint, CurrentState, OpContext, Operation, OperationMeta, Validation, Verification,
};
pub use risk::RiskLevel;
