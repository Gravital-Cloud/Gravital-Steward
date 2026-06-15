//! # steward-state
//!
//! The runtime's single durable store, backed by an embedded `redb` database
//! (pure Rust, no C — ADR-0001). It persists the facts the security core needs
//! across restarts:
//!
//! - issued token records ([`TokenRecord`]),
//! - the token revocation list,
//! - operation checkpoints (rollback data),
//! - the audit log head hash.
//!
//! All writes are single ACID transactions, so a crash mid-write cannot corrupt
//! the store. Values are stored as JSON, so the schema can evolve additively.
//!
//! Secrets never live here: [`TokenRecord`] holds metadata only. Token material
//! and application secrets belong to `steward-secrets`.

#![forbid(unsafe_code)]
// `redb`'s error enums are large, so wrapping them makes `StateError` large too.
// Boxing every variant would lose the ergonomic `?`/`#[from]` conversions for no
// real benefit in a store whose calls are I/O-bound, so we accept the size here.
#![allow(clippy::result_large_err)]

mod error;
mod store;
mod token;

pub use error::{Result, StateError};
pub use store::StateStore;
pub use token::TokenRecord;
