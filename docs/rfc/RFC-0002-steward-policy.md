# RFC-0002: `steward-policy` — the allow/deny engine

- Status: Accepted
- Date: 2026-06-15
- Author: Angel Nereira
- Phase: 1 — Security core + minimal MCP
- Tracking issue: #2

## Summary

`steward-policy` is the pure decision function at the center of the security
model. Given a token's grant and a requested operation, it returns exactly one
of `Allow`, `RequiresConfirmation`, or `Deny(reason)`. It contains no I/O and no
domain logic; it is the executable form of the authorization predicate in
`ARCHITECTURE.md` §5.1.

## Motivation

The blueprint requires that "best practices are encoded as executable rules, not
comments" and that the system **fails closed**. Authorization must be a single,
auditable, deterministic function so that every code path that performs an
operation routes through the same decision, and so the decision can be unit
tested exhaustively. Centralizing it here keeps `steward-auth` (token formats,
Biscuit) and the domain executors free of ad-hoc permission checks.

## Design

### Inputs

- `TokenGrant` — what a validated token permits:
  - `granted: CapabilitySet` — capabilities the token holds.
  - `denied: BTreeSet<OperationId>` — explicit prohibitions that win over grants.
  - `max_risk: RiskLevel` — highest risk the token may invoke.
  - `confirm_above: RiskLevel` — risk at/above which human confirmation is required.
  - `scope_servers: ServerScope` — `Any` or an explicit set of `ServerId`s.
  - `expires_unix: i64` — expiry as Unix seconds.
- `PolicyRequest` — the operation being attempted:
  - `operation: OperationId`, `required: CapabilitySet`, `risk: RiskLevel`,
    `server: ServerId`, `confirmed: bool`, `now_unix: i64`.

### Decision

```rust
enum Decision { Allow, RequiresConfirmation, Deny(DenyReason) }
enum DenyReason {
    Expired, ServerOutOfScope, ExplicitlyDenied,
    RiskExceedsMax, CapabilityMissing,
}
```

### Evaluation order (fail closed)

Checks run in a fixed precedence; the first failing check denies:

1. `now_unix >= expires_unix` → `Deny(Expired)`.
2. server not in `scope_servers` → `Deny(ServerOutOfScope)`.
3. `operation ∈ denied` → `Deny(ExplicitlyDenied)` (wins even if a capability exists).
4. `risk > max_risk` → `Deny(RiskExceedsMax)`.
5. `!granted.grants_all(required)` → `Deny(CapabilityMissing)`.
6. `risk >= confirm_above && !confirmed` → `RequiresConfirmation`.
7. otherwise → `Allow`.

Each `DenyReason` maps to a `steward_proto::ErrorCode` so the agent receives a
structured, actionable error.

## Security considerations

- **Explicit deny beats capability** (step 3 before step 5), matching the
  blueprint's Biscuit `deny if …` semantics.
- **Default deny:** an empty `scope_servers` set matches no server; a missing
  capability denies. There is no implicit allow.
- The engine is deterministic and side-effect free, so a decision is fully
  reproducible from its inputs and can be recorded verbatim in the audit log.

## Alternatives considered

- **Permission checks inside each operation.** Rejected: scatters the rules,
  defeats auditability, and invites drift.
- **Returning a bool.** Rejected: it cannot express `RequiresConfirmation`, the
  human-in-the-loop state the MCP layer needs.

## Test plan

- One unit test per `DenyReason` and per terminal state.
- Precedence tests (e.g. an expired token on a denied operation reports
  `Expired`, the highest-precedence reason).
- Property: `Allow`/`RequiresConfirmation` is only ever returned when all deny
  conditions are false.

## Exit criteria

- Compiles for both musl targets; `clippy -D warnings` clean; tests pass.
- Every `Decision` and `DenyReason` is exercised by a test.
