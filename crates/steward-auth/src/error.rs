//! Error type for token issuance, verification and attenuation.

use thiserror::Error;

/// Errors raised while issuing, verifying or attenuating a capability token.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AuthError {
    /// A Biscuit-level error (parsing, signature, serialization, datalog build).
    #[error("biscuit error: {0}")]
    Biscuit(#[from] biscuit_auth::error::Token),

    /// The token failed authorization: it is expired or an attenuation check
    /// rejected it.
    #[error("token not authorized (expired or restricted): {0}")]
    Unauthorized(String),

    /// The token verified but is missing or carries a malformed required fact.
    #[error("malformed token: {0}")]
    Malformed(String),
}
