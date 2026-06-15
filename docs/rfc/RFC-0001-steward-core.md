# RFC-0001: `steward-core` — domain types and the Operation contract

- Status: Accepted
- Date: 2026-06-15
- Author: Angel Nereira
- Phase: 0 — Foundations and governance
- Tracking issue: Phase 0 milestone

## Summary

`steward-core` defines the foundational, domain-logic-free types that govern the
entire runtime: the `Operation` lifecycle trait, the `RiskLevel` classification,
the capability model, strongly typed identifiers, and the core error type. Every
domain crate depends on this crate and implements `Operation` for its concrete
verbs.

## Motivation

The blueprint (§9) establishes that the uniform operation contract is "the heart
of the data-oriented architecture." Before any domain logic exists, the shape of
an operation must be fixed so that:

- the engine can drive a single lifecycle for every operation;
- risk and required capabilities are declared at the type level (ADR-0003);
- the policy and auth engines have stable types to reason about.

## Design

### Modules

- `risk` — `RiskLevel { Info, Low, Medium, High, Critical }`, totally ordered,
  with a default confirmation threshold at `High`.
- `ids` — transparent newtype identifiers (`OperationId`, `ServerId`,
  `ProjectId`, `TokenId`, `AuditId`) so distinct ids cannot be confused.
- `capability` — `Capability` (wraps an `OperationId`) and `CapabilitySet` with
  the core authorization predicate `grants_all` (a token's granted set must be a
  superset of an operation's required set).
- `error` — `CoreError`, a small `thiserror` enum for lifecycle plumbing. Rich,
  agent-facing errors live in `steward-proto`.
- `operation` — the `Operation` trait and its lifecycle value types
  (`CurrentState`, `Validation`, `Checkpoint`, `Verification`, `OperationMeta`,
  `OpContext`).

### The `Operation` trait

`Operation` has three associated types — `Input`, `Plan`, `Outcome` — each
`Serialize`/`JsonSchema`, so the MCP layer can auto-derive tool schemas and the
LLM consumes structured plans and outcomes rather than raw output. The six
lifecycle methods (`inspect`, `plan`, `validate`, `apply`, `verify`, `rollback`)
are `async` via `async_trait`.

`apply` returns `(Checkpoint, Outcome)`, forcing every effectful operation to
produce a rollback handle.

### Invariants encoded

- `#![forbid(unsafe_code)]` at the crate level (ADR-0001).
- Risk and required capabilities are non-optional parts of the trait.
- Identifiers are distinct types, not bare strings.

## Security considerations

This crate performs no I/O and holds no secrets. Its job is to make the security
model expressible: capabilities, risk and confirmation are first-class so the
policy engine can enforce them. The capability superset check is the single
predicate all authorization builds on.

## Alternatives considered

- **A single `run(input) -> outcome` method.** Rejected: it cannot enforce the
  inspect/plan/validate/verify/rollback discipline mandated by ADR-0003.
- **Bare `String` identifiers.** Rejected: they allow mixing up a server id and
  an operation id at call sites with no compiler help.
- **Putting agent-facing error types here.** Rejected: those belong in the
  Apache-licensed `steward-proto` so clients can depend on them without AGPL.

## Test plan

- Unit tests for risk ordering and the confirmation threshold.
- Round-trip serialization tests for ids, capabilities and lifecycle types.
- A complete end-to-end drive of the lifecycle on a trivial read-only operation,
  proving the contract is implementable and composes.

## Exit criteria

- The crate compiles for `x86_64-unknown-linux-musl`.
- `cargo clippy -D warnings` is clean.
- All unit tests pass in CI.
- `cargo-deny` passes (license + bans).
