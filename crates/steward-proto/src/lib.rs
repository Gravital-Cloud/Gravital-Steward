//! # steward-proto
//!
//! Stable wire-protocol types shared between the runtime and any client (the
//! MCP server, the CLI, and third-party SDKs). This crate is licensed
//! Apache-2.0 for maximum integrability and is deliberately isolated so the
//! protocol can evolve independently of the AGPL core runtime.
//!
//! It contains three families of types:
//!
//! - [`error`] — the structured, actionable error protocol returned to LLM
//!   agents (never raw Linux logs).
//! - [`manifest`] — the declarative desired-state manifest an agent submits.
//! - [`audit`] — the schema of an append-only, hash-chained audit event.

#![forbid(unsafe_code)]

pub mod audit;
pub mod error;
pub mod manifest;

pub use audit::AuditEvent;
pub use error::{ErrorCode, Severity, StructuredError, SuggestedAction};
pub use manifest::{Manifest, ManifestKind};
