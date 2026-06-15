//! Issuance, verification and attenuation built on Biscuit.

use crate::error::AuthError;
use crate::spec::TokenSpec;
use biscuit_auth::{AuthorizerBuilder, Biscuit, BlockBuilder, KeyPair, PublicKey};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use steward_core::{Capability, CapabilitySet, OperationId, RiskLevel, ServerId};
use steward_policy::{ServerScope, TokenGrant};

/// A freshly issued token and the metadata a caller needs to persist a record.
#[derive(Debug, Clone)]
pub struct Issued {
    /// The base64-encoded Biscuit token.
    pub token_b64: String,
    /// The token's identifier.
    pub token_id: String,
    /// Absolute expiry as Unix seconds.
    pub expires_unix: i64,
}

/// The result of verifying a token: its identity, the grant for the policy
/// engine, and the revocation ids to check against the state store.
#[derive(Debug, Clone)]
pub struct Verified {
    /// The token's identifier.
    pub token_id: String,
    /// The authorization grant reconstructed from the authority block.
    pub grant: TokenGrant,
    /// Biscuit revocation identifiers (one per block).
    pub revocation_ids: Vec<Vec<u8>>,
}

/// Issues capability tokens signed with a runtime-owned root key.
pub struct AuthEngine {
    root: KeyPair,
}

impl AuthEngine {
    /// Creates an engine that signs tokens with `root`.
    #[must_use]
    pub fn new(root: KeyPair) -> Self {
        Self { root }
    }

    /// The public key used to verify tokens issued by this engine.
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.root.public()
    }

    /// Issues a token from `spec`, expiring at `now_unix + spec.ttl_seconds`.
    ///
    /// # Errors
    /// Returns [`AuthError`] if the datalog authority block fails to build or
    /// the token cannot be encoded.
    pub fn issue(&self, spec: &TokenSpec, now_unix: i64) -> Result<Issued, AuthError> {
        let expires_unix = now_unix.saturating_add(spec.ttl_seconds);

        // Writing into a String is infallible, so the `write!` results are ignored.
        let mut src = String::new();
        let _ = writeln!(src, "token(\"{}\");", spec.token_id);
        let _ = writeln!(src, "max_risk(\"{}\");", spec.max_risk.as_str());
        let _ = writeln!(src, "confirm_above(\"{}\");", spec.confirm_above.as_str());
        let _ = writeln!(src, "expires_unix({expires_unix});");
        if spec.scope_any {
            src.push_str("scope_any(true);\n");
        }
        for server in &spec.scope_servers {
            let _ = writeln!(src, "scope_server(\"{server}\");");
        }
        for capability in &spec.capabilities {
            let _ = writeln!(src, "capability(\"{capability}\");");
        }
        for denied in &spec.denied {
            let _ = writeln!(src, "denied(\"{denied}\");");
        }
        // Enforce expiry against the integer `now` fact the verifier supplies.
        let _ = writeln!(src, "check if now($t), $t <= {expires_unix};");

        let token = Biscuit::builder().code(src)?.build(&self.root)?;
        Ok(Issued {
            token_b64: token.to_base64()?,
            token_id: spec.token_id.clone(),
            expires_unix,
        })
    }
}

/// Verifies a token's signature and expiry against `root` at `now_unix`, and
/// extracts its authority grant.
///
/// # Errors
/// Returns [`AuthError::Biscuit`] if the signature or encoding is invalid,
/// [`AuthError::Unauthorized`] if the token is expired or restricted, or
/// [`AuthError::Malformed`] if a required fact is missing or invalid.
pub fn verify(token_b64: &str, root: &PublicKey, now_unix: i64) -> Result<Verified, AuthError> {
    let biscuit = Biscuit::from_base64(token_b64, root)?;

    let mut authorizer = AuthorizerBuilder::new()
        .fact(format!("now({now_unix})").as_str())?
        .policy("allow if true")?
        .build(&biscuit)?;
    authorizer
        .authorize()
        .map_err(|e| AuthError::Unauthorized(e.to_string()))?;

    // Distinct head predicates per query so derived facts never cross-contaminate.
    let caps: Vec<(String,)> = authorizer.query("cap_out($c) <- capability($c)")?;
    let denied_q: Vec<(String,)> = authorizer.query("den_out($d) <- denied($d)")?;
    let max_risk_q: Vec<(String,)> = authorizer.query("risk_out($r) <- max_risk($r)")?;
    let confirm_q: Vec<(String,)> = authorizer.query("conf_out($r) <- confirm_above($r)")?;
    let expires_q: Vec<(i64,)> = authorizer.query("exp_out($e) <- expires_unix($e)")?;
    let token_q: Vec<(String,)> = authorizer.query("tok_out($t) <- token($t)")?;
    let any_q: Vec<(bool,)> = authorizer.query("any_out($b) <- scope_any($b)")?;
    let servers_q: Vec<(String,)> = authorizer.query("srv_out($s) <- scope_server($s)")?;

    let token_id = token_q
        .into_iter()
        .next()
        .map(|(t,)| t)
        .ok_or_else(|| AuthError::Malformed("missing token id".to_owned()))?;
    let expires_unix = expires_q
        .into_iter()
        .next()
        .map(|(e,)| e)
        .ok_or_else(|| AuthError::Malformed("missing expires_unix".to_owned()))?;
    let max_risk = parse_risk(&first(max_risk_q, "max_risk")?)?;
    let confirm_above = parse_risk(&first(confirm_q, "confirm_above")?)?;
    let scope_any = any_q.into_iter().any(|(b,)| b);

    let grant = TokenGrant {
        granted: caps
            .into_iter()
            .map(|(c,)| Capability::new(OperationId::new(c)))
            .collect::<CapabilitySet>(),
        denied: denied_q
            .into_iter()
            .map(|(d,)| OperationId::new(d))
            .collect::<BTreeSet<_>>(),
        max_risk,
        confirm_above,
        scope_servers: if scope_any {
            ServerScope::Any
        } else {
            ServerScope::only(servers_q.into_iter().map(|(s,)| ServerId::new(s)))
        },
        expires_unix,
    };

    Ok(Verified {
        token_id,
        grant,
        revocation_ids: biscuit.revocation_identifiers(),
    })
}

/// Attenuates a token by appending an expiry-shortening check. Offline: needs no
/// server round-trip. The resulting token verifies only while
/// `now <= not_after_unix` (and still within the original expiry).
///
/// # Errors
/// Returns [`AuthError`] if the token cannot be parsed or re-encoded.
pub fn attenuate_expiry(
    token_b64: &str,
    root: &PublicKey,
    not_after_unix: i64,
) -> Result<String, AuthError> {
    let biscuit = Biscuit::from_base64(token_b64, root)?;
    let block = BlockBuilder::new().code(format!("check if now($t), $t <= {not_after_unix};"))?;
    let attenuated = biscuit.append(block)?;
    Ok(attenuated.to_base64()?)
}

fn first(values: Vec<(String,)>, field: &str) -> Result<String, AuthError> {
    values
        .into_iter()
        .next()
        .map(|(v,)| v)
        .ok_or_else(|| AuthError::Malformed(format!("missing {field}")))
}

fn parse_risk(value: &str) -> Result<RiskLevel, AuthError> {
    Ok(match value {
        "info" => RiskLevel::Info,
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        "critical" => RiskLevel::Critical,
        other => return Err(AuthError::Malformed(format!("unknown risk level: {other}"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_auth::{Algorithm, PrivateKey};
    use steward_policy::{decide, Decision, PolicyRequest};

    const NOW: i64 = 1_000_000;

    fn engine() -> AuthEngine {
        let private = PrivateKey::from_bytes(&[7u8; 32], Algorithm::Ed25519).unwrap();
        AuthEngine::new(KeyPair::from(&private))
    }

    fn spec() -> TokenSpec {
        TokenSpec {
            token_id: "tok_abc".to_owned(),
            label: Some("ci".to_owned()),
            capabilities: vec!["server.inspect".to_owned(), "deploy.from_github".to_owned()],
            denied: vec!["db.drop".to_owned()],
            max_risk: RiskLevel::Medium,
            confirm_above: RiskLevel::High,
            scope_any: false,
            scope_servers: vec!["srv-prod-1".to_owned()],
            ttl_seconds: 3600,
        }
    }

    #[test]
    fn issue_then_verify_round_trips_the_grant() {
        let eng = engine();
        let issued = eng.issue(&spec(), NOW).unwrap();
        let verified = verify(&issued.token_b64, &eng.public_key(), NOW + 10).unwrap();

        assert_eq!(verified.token_id, "tok_abc");
        assert_eq!(verified.grant.max_risk, RiskLevel::Medium);
        assert_eq!(verified.grant.confirm_above, RiskLevel::High);
        assert_eq!(verified.grant.expires_unix, NOW + 3600);
        assert_eq!(verified.grant.granted.len(), 2);
        assert!(verified.grant.denied.contains(&OperationId::new("db.drop")));
        assert!(!verified.revocation_ids.is_empty());
    }

    #[test]
    fn extracted_grant_authorizes_a_valid_request() {
        let eng = engine();
        let issued = eng.issue(&spec(), NOW).unwrap();
        let verified = verify(&issued.token_b64, &eng.public_key(), NOW + 10).unwrap();

        let request = PolicyRequest {
            operation: OperationId::new("server.inspect"),
            required: [Capability::new(OperationId::new("server.inspect"))]
                .into_iter()
                .collect(),
            risk: RiskLevel::Info,
            server: ServerId::new("srv-prod-1"),
            confirmed: false,
            now_unix: NOW + 10,
        };
        assert_eq!(decide(&verified.grant, &request), Decision::Allow);
    }

    #[test]
    fn expired_token_fails_verification() {
        let eng = engine();
        let issued = eng.issue(&spec(), NOW).unwrap();
        // now is past the expiry (NOW + ttl).
        let result = verify(&issued.token_b64, &eng.public_key(), NOW + 4000);
        assert!(matches!(result, Err(AuthError::Unauthorized(_))));
    }

    #[test]
    fn wrong_public_key_fails() {
        let eng = engine();
        let issued = eng.issue(&spec(), NOW).unwrap();
        let other = KeyPair::from(&PrivateKey::from_bytes(&[9u8; 32], Algorithm::Ed25519).unwrap());
        assert!(verify(&issued.token_b64, &other.public(), NOW + 10).is_err());
    }

    #[test]
    fn expiry_attenuation_rejects_after_shortened_deadline() {
        let eng = engine();
        let issued = eng.issue(&spec(), NOW).unwrap();
        // Shorten the lifetime to NOW + 100, well before the original NOW + 3600.
        let attenuated = attenuate_expiry(&issued.token_b64, &eng.public_key(), NOW + 100).unwrap();

        // Still valid before the shortened deadline.
        assert!(verify(&attenuated, &eng.public_key(), NOW + 50).is_ok());
        // Rejected after the shortened deadline, even though the original expiry
        // (NOW + 3600) has not passed.
        assert!(matches!(
            verify(&attenuated, &eng.public_key(), NOW + 200),
            Err(AuthError::Unauthorized(_))
        ));
    }
}
