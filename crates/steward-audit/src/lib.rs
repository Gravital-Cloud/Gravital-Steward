//! # steward-audit
//!
//! Append-only audit log with cryptographically verifiable integrity. Each entry
//! is BLAKE3 hash-chained to the previous one and Ed25519-signed by the runtime
//! key, so any tampering is detectable without trusting the producer.
//!
//! This crate owns the cryptography and the chain invariants, not storage:
//! persisting envelopes (to disk or `steward-state`) lives elsewhere. The head
//! hash can be persisted and restored via [`AuditLog::with_head`] so the chain
//! survives a restart.
//!
//! ```
//! use ed25519_dalek::SigningKey;
//! use steward_audit::{AuditDraft, AuditLog, verify};
//! use steward_proto::audit::AuditOutcome;
//!
//! let key = SigningKey::from_bytes(&[7u8; 32]);
//! let verifying = key.verifying_key();
//! let mut log = AuditLog::new(key);
//!
//! let envelope = log.append(AuditDraft {
//!     audit_id: "evt_0001".into(),
//!     timestamp: "2026-06-15T08:00:00Z".into(),
//!     actor: "agent".into(),
//!     token_id: "tok_abc".into(),
//!     capabilities: vec!["server.inspect".into()],
//!     operation: "server.inspect".into(),
//!     risk: "info".into(),
//!     outcome: AuditOutcome::Succeeded,
//!     rollback_available: false,
//! });
//!
//! assert!(verify(std::slice::from_ref(&envelope), &verifying).is_ok());
//! ```

#![forbid(unsafe_code)]

mod chain;

pub use chain::{verify, AuditDraft, AuditLog, SignedEnvelope, VerifyError};
