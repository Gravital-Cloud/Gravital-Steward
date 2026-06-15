//! The authorization predicate.

use crate::decision::{Decision, DenyReason};
use crate::grant::{PolicyRequest, TokenGrant};

/// Decides whether a request is authorized against a token grant.
///
/// Checks run in a fixed precedence; the first failing check denies (fail
/// closed). The order matters: an explicit deny wins over a granted capability,
/// and an expired token is reported as `Expired` regardless of any other issue.
///
/// 1. expired → `Deny(Expired)`
/// 2. server out of scope → `Deny(ServerOutOfScope)`
/// 3. explicitly denied → `Deny(ExplicitlyDenied)`
/// 4. risk above the token maximum → `Deny(RiskExceedsMax)`
/// 5. missing capability → `Deny(CapabilityMissing)`
/// 6. risk at/above the confirmation threshold and not confirmed →
///    `RequiresConfirmation`
/// 7. otherwise → `Allow`
#[must_use]
pub fn decide(grant: &TokenGrant, request: &PolicyRequest) -> Decision {
    if request.now_unix >= grant.expires_unix {
        return Decision::Deny(DenyReason::Expired);
    }

    if !grant.scope_servers.contains(&request.server) {
        return Decision::Deny(DenyReason::ServerOutOfScope);
    }

    if grant.denied.contains(&request.operation) {
        return Decision::Deny(DenyReason::ExplicitlyDenied);
    }

    if request.risk > grant.max_risk {
        return Decision::Deny(DenyReason::RiskExceedsMax);
    }

    if !grant.granted.grants_all(&request.required) {
        return Decision::Deny(DenyReason::CapabilityMissing);
    }

    if request.risk >= grant.confirm_above && !request.confirmed {
        return Decision::RequiresConfirmation;
    }

    Decision::Allow
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grant::ServerScope;
    use std::collections::BTreeSet;
    use steward_core::{Capability, CapabilitySet, OperationId, RiskLevel, ServerId};

    const NOW: i64 = 1_000_000;
    const LATER: i64 = 2_000_000;

    fn caps(ids: &[&str]) -> CapabilitySet {
        ids.iter()
            .map(|id| Capability::new(OperationId::new(*id)))
            .collect()
    }

    fn grant() -> TokenGrant {
        TokenGrant {
            granted: caps(&["server.inspect", "deploy.from_github"]),
            denied: BTreeSet::new(),
            max_risk: RiskLevel::Medium,
            confirm_above: RiskLevel::High,
            scope_servers: ServerScope::only([ServerId::new("srv-prod-1")]),
            expires_unix: LATER,
        }
    }

    fn request() -> PolicyRequest {
        PolicyRequest {
            operation: OperationId::new("server.inspect"),
            required: caps(&["server.inspect"]),
            risk: RiskLevel::Info,
            server: ServerId::new("srv-prod-1"),
            confirmed: false,
            now_unix: NOW,
        }
    }

    #[test]
    fn allows_a_valid_request() {
        assert_eq!(decide(&grant(), &request()), Decision::Allow);
    }

    #[test]
    fn denies_expired_token() {
        let mut req = request();
        req.now_unix = LATER; // exactly at expiry is already expired
        assert_eq!(decide(&grant(), &req), Decision::Deny(DenyReason::Expired));
    }

    #[test]
    fn denies_server_out_of_scope() {
        let mut req = request();
        req.server = ServerId::new("srv-other");
        assert_eq!(
            decide(&grant(), &req),
            Decision::Deny(DenyReason::ServerOutOfScope)
        );
    }

    #[test]
    fn explicit_deny_beats_granted_capability() {
        let mut g = grant();
        g.denied.insert(OperationId::new("server.inspect"));
        // The capability is granted, but the explicit deny must win.
        assert_eq!(
            decide(&g, &request()),
            Decision::Deny(DenyReason::ExplicitlyDenied)
        );
    }

    #[test]
    fn denies_risk_above_max() {
        let mut req = request();
        req.operation = OperationId::new("deploy.from_github");
        req.required = caps(&["deploy.from_github"]);
        req.risk = RiskLevel::High; // token max is Medium
        assert_eq!(
            decide(&grant(), &req),
            Decision::Deny(DenyReason::RiskExceedsMax)
        );
    }

    #[test]
    fn denies_missing_capability() {
        let mut req = request();
        req.operation = OperationId::new("db.drop");
        req.required = caps(&["db.drop"]);
        assert_eq!(
            decide(&grant(), &req),
            Decision::Deny(DenyReason::CapabilityMissing)
        );
    }

    #[test]
    fn requires_confirmation_at_threshold_when_unconfirmed() {
        let mut g = grant();
        g.max_risk = RiskLevel::Critical; // allow high-risk so we reach the confirm check
        let mut req = request();
        req.risk = RiskLevel::High; // == confirm_above
        req.confirmed = false;
        assert_eq!(decide(&g, &req), Decision::RequiresConfirmation);
    }

    #[test]
    fn allows_high_risk_when_confirmed() {
        let mut g = grant();
        g.max_risk = RiskLevel::Critical;
        let mut req = request();
        req.risk = RiskLevel::High;
        req.confirmed = true;
        assert_eq!(decide(&g, &req), Decision::Allow);
    }

    #[test]
    fn expiry_precedence_wins_over_other_failures() {
        // A request that also targets a denied operation on an out-of-scope
        // server must still report Expired, the highest-precedence reason.
        let mut g = grant();
        g.denied.insert(OperationId::new("server.inspect"));
        let mut req = request();
        req.now_unix = LATER;
        req.server = ServerId::new("srv-other");
        assert_eq!(decide(&g, &req), Decision::Deny(DenyReason::Expired));
    }

    #[test]
    fn any_scope_allows_any_server() {
        let mut g = grant();
        g.scope_servers = ServerScope::Any;
        let mut req = request();
        req.server = ServerId::new("srv-anything");
        assert_eq!(decide(&g, &req), Decision::Allow);
    }
}
