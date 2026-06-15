//! The hash-chained, signed audit chain.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use steward_proto::audit::{AuditEvent, AuditOutcome};
use thiserror::Error;

/// The caller-supplied fields of an audit entry. The log fills in `prev_hash`,
/// computes the entry hash, and signs it.
#[derive(Debug, Clone)]
pub struct AuditDraft {
    /// Unique event identifier.
    pub audit_id: String,
    /// RFC 3339 timestamp.
    pub timestamp: String,
    /// The agent or principal that triggered the action.
    pub actor: String,
    /// The token id presented (never the token material).
    pub token_id: String,
    /// Capabilities the token carried.
    pub capabilities: Vec<String>,
    /// Canonical operation id.
    pub operation: String,
    /// Risk level at execution time.
    pub risk: String,
    /// Terminal outcome.
    pub outcome: AuditOutcome,
    /// Whether a rollback checkpoint was available.
    pub rollback_available: bool,
}

/// An appended record: the event plus its hex-encoded entry hash and signature.
#[derive(Debug, Clone)]
pub struct SignedEnvelope {
    /// The audit event (wire shape), including its `prev_hash` link.
    pub event: AuditEvent,
    /// Hex-encoded BLAKE3 hash of the canonical event bytes.
    pub entry_hash: String,
    /// Hex-encoded Ed25519 signature over the entry hash digest.
    pub signature: String,
}

/// An append-only audit log that chains and signs entries.
pub struct AuditLog {
    signing_key: SigningKey,
    head: String,
}

impl AuditLog {
    /// Creates a fresh log whose next entry is the genesis entry (empty
    /// `prev_hash`).
    #[must_use]
    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            signing_key,
            head: String::new(),
        }
    }

    /// Creates a log that continues from a persisted head hash.
    #[must_use]
    pub fn with_head(signing_key: SigningKey, head: impl Into<String>) -> Self {
        Self {
            signing_key,
            head: head.into(),
        }
    }

    /// The current head hash (the entry hash of the last appended entry; empty
    /// before the first append).
    #[must_use]
    pub fn head(&self) -> &str {
        &self.head
    }

    /// Appends a new entry, advancing the head, and returns the signed envelope.
    pub fn append(&mut self, draft: AuditDraft) -> SignedEnvelope {
        let event = AuditEvent {
            audit_id: draft.audit_id,
            timestamp: draft.timestamp,
            actor: draft.actor,
            token_id: draft.token_id,
            capabilities: draft.capabilities,
            operation: draft.operation,
            risk: draft.risk,
            outcome: draft.outcome,
            rollback_available: draft.rollback_available,
            prev_hash: self.head.clone(),
        };

        let digest = digest_of(&event);
        let entry_hash = hex::encode(digest);
        let signature = self.signing_key.sign(&digest);

        self.head.clone_from(&entry_hash);
        SignedEnvelope {
            event,
            entry_hash,
            signature: hex::encode(signature.to_bytes()),
        }
    }
}

/// Errors returned when verifying an audit chain.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VerifyError {
    /// The stored entry hash does not match the recomputed hash of the event.
    #[error("hash mismatch at entry {index}")]
    HashMismatch {
        /// Zero-based index of the offending entry.
        index: usize,
    },
    /// The entry's `prev_hash` does not link to the previous entry's hash.
    #[error("broken chain at entry {index}")]
    BrokenChain {
        /// Zero-based index of the offending entry.
        index: usize,
    },
    /// The signature is malformed or does not verify against the key.
    #[error("bad signature at entry {index}")]
    BadSignature {
        /// Zero-based index of the offending entry.
        index: usize,
    },
}

/// Verifies a complete chain against a public key: recomputes every entry hash,
/// checks the `prev_hash` linkage, and validates every signature.
///
/// # Errors
///
/// Returns the first [`VerifyError`] encountered, identifying the entry index
/// and the failure mode.
pub fn verify(chain: &[SignedEnvelope], key: &VerifyingKey) -> Result<(), VerifyError> {
    let mut expected_prev = String::new();
    for (index, envelope) in chain.iter().enumerate() {
        if envelope.event.prev_hash != expected_prev {
            return Err(VerifyError::BrokenChain { index });
        }

        let digest = digest_of(&envelope.event);
        let recomputed = hex::encode(digest);
        if recomputed != envelope.entry_hash {
            return Err(VerifyError::HashMismatch { index });
        }

        let signature =
            parse_signature(&envelope.signature).ok_or(VerifyError::BadSignature { index })?;
        key.verify(&digest, &signature)
            .map_err(|_| VerifyError::BadSignature { index })?;

        expected_prev.clone_from(&envelope.entry_hash);
    }
    Ok(())
}

/// Canonical BLAKE3 digest of an event. `AuditEvent` contains no maps, so
/// `serde_json` field order is stable and the digest is reproducible.
fn digest_of(event: &AuditEvent) -> [u8; 32] {
    let canonical = serde_json::to_vec(event).expect("AuditEvent always serializes");
    *blake3::hash(&canonical).as_bytes()
}

fn parse_signature(hex_sig: &str) -> Option<Signature> {
    let bytes = hex::decode(hex_sig).ok()?;
    let array: [u8; 64] = bytes.try_into().ok()?;
    Some(Signature::from_bytes(&array))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn draft(id: &str, op: &str) -> AuditDraft {
        AuditDraft {
            audit_id: id.to_owned(),
            timestamp: "2026-06-15T08:00:00Z".to_owned(),
            actor: "agent".to_owned(),
            token_id: "tok_abc".to_owned(),
            capabilities: vec!["server.inspect".to_owned()],
            operation: op.to_owned(),
            risk: "info".to_owned(),
            outcome: AuditOutcome::Succeeded,
            rollback_available: false,
        }
    }

    fn sample_chain() -> (Vec<SignedEnvelope>, VerifyingKey) {
        let signing = key();
        let verifying = signing.verifying_key();
        let mut log = AuditLog::new(signing);
        let chain = vec![
            log.append(draft("evt_1", "server.inspect")),
            log.append(draft("evt_2", "package.install")),
            log.append(draft("evt_3", "service.restart")),
        ];
        (chain, verifying)
    }

    #[test]
    fn genesis_entry_has_empty_prev_hash_and_head_advances() {
        let signing = key();
        let mut log = AuditLog::new(signing);
        assert_eq!(log.head(), "");
        let first = log.append(draft("evt_1", "server.inspect"));
        assert_eq!(first.event.prev_hash, "");
        assert_eq!(log.head(), first.entry_hash);
        let second = log.append(draft("evt_2", "package.install"));
        assert_eq!(second.event.prev_hash, first.entry_hash);
    }

    #[test]
    fn valid_chain_verifies() {
        let (chain, verifying) = sample_chain();
        assert_eq!(verify(&chain, &verifying), Ok(()));
    }

    #[test]
    fn tampered_payload_is_detected() {
        let (mut chain, verifying) = sample_chain();
        // Mutate an event after the fact without recomputing its hash.
        chain[1].event.operation = "disk.wipe".to_owned();
        assert_eq!(
            verify(&chain, &verifying),
            Err(VerifyError::HashMismatch { index: 1 })
        );
    }

    #[test]
    fn broken_link_is_detected() {
        let (mut chain, verifying) = sample_chain();
        // Rewrite a prev_hash link to point nowhere; the hash also changes, but
        // the chain check runs first and reports the broken link.
        chain[2].event.prev_hash = "deadbeef".to_owned();
        assert_eq!(
            verify(&chain, &verifying),
            Err(VerifyError::BrokenChain { index: 2 })
        );
    }

    #[test]
    fn wrong_key_fails_signature() {
        let (chain, _) = sample_chain();
        let other = SigningKey::from_bytes(&[9u8; 32]).verifying_key();
        assert_eq!(
            verify(&chain, &other),
            Err(VerifyError::BadSignature { index: 0 })
        );
    }

    #[test]
    fn malformed_signature_fails() {
        let (mut chain, verifying) = sample_chain();
        chain[0].signature = "not-hex".to_owned();
        assert_eq!(
            verify(&chain, &verifying),
            Err(VerifyError::BadSignature { index: 0 })
        );
    }
}
