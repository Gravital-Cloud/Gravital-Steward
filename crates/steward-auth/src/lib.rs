//! # steward-auth
//!
//! Issues, verifies and attenuates the runtime's capability tokens using
//! [Biscuit](https://www.biscuitsec.org/). A token is an Ed25519-signed,
//! self-contained bearer of capabilities that can be **attenuated offline**
//! (restricted further without contacting the server). Verification yields a
//! [`steward_policy::TokenGrant`] that the policy engine consumes.
//!
//! See `docs/rfc/RFC-0005-steward-auth.md` for the full design. Key management
//! is external: the root [`KeyPair`] is owned by the runtime; this crate never
//! generates or persists keys on its own.

#![forbid(unsafe_code)]

mod engine;
mod error;
mod spec;

pub use biscuit_auth::{Algorithm, KeyPair, PrivateKey, PublicKey};
pub use engine::{attenuate_expiry, verify, AuthEngine, Issued, Verified};
pub use error::AuthError;
pub use spec::TokenSpec;
