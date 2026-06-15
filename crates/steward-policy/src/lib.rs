//! # steward-policy
//!
//! The allow/deny authorization engine. This crate is a pure decision function:
//! given a [`TokenGrant`] (what a validated token permits) and a
//! [`PolicyRequest`] (the operation being attempted), [`decide`] returns exactly
//! one [`Decision`] — `Allow`, `RequiresConfirmation`, or `Deny(reason)`.
//!
//! It contains no I/O and no domain logic; it is the executable form of the
//! authorization predicate documented in `ARCHITECTURE.md` §5.1. Every code path
//! that runs an operation routes through [`decide`], so the rule set lives in
//! exactly one place and **fails closed**.

#![forbid(unsafe_code)]

mod decision;
mod engine;
mod grant;

pub use decision::{Decision, DenyReason};
pub use engine::decide;
pub use grant::{PolicyRequest, ServerScope, TokenGrant};
