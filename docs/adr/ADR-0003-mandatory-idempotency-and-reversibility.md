# ADR-0003: Mandatory idempotency and reversibility

- Status: Accepted
- Date: 2026-06-15
- Deciders: Angel Nereira

## Context

The primary caller of Gravital-Steward is a large language model acting as an
infrastructure engineer. LLMs are prone to two failure modes that are
catastrophic on a server:

1. **Imperative drift.** They hallucinate sequences of imperative commands whose
   combined effect is unpredictable and non-reproducible.
2. **Irreversible mistakes.** They may take destructive actions with no way back.

A server in production is a living system. The reference operations guide that
informs this project states the central rule plainly: *what is not documented,
monitored, backed up and tested does not exist operationally*. Our runtime must
encode that discipline so the agent cannot bypass it.

## Decision

Every effectful operation in the system is **idempotent** and **reversible**, and
this is enforced by the type system, not by convention.

1. **Uniform lifecycle.** Every operation implements the
   `inspect -> plan -> validate -> apply -> verify -> rollback` contract defined
   by the `Operation` trait in `steward-core`. The engine drives all six steps;
   an operation type cannot be registered without implementing them.
2. **Declarative convergence.** The agent submits a *desired state* manifest. The
   reconciler computes `desired - current = diff` and applies only the
   difference. Re-applying the same manifest produces no changes.
3. **Checkpoint before apply.** `apply` must create a `Checkpoint` before
   mutating the system (config snapshot, prior release, database backup). If
   `verify` fails, the engine calls `rollback` automatically.
4. **Risk is explicit.** An operation must declare its `RiskLevel` and required
   `CapabilitySet`; operations at `High`/`Critical` require human confirmation by
   default.

## Alternatives considered

- **Free-form shell access for the agent.** Rejected: this is precisely the
  catastrophic-risk surface the project exists to eliminate.
- **Idempotency by guideline only.** Rejected: guidelines are not enforced; the
  type system is. Making the contract a trait means non-conforming operations do
  not compile.
- **Reversibility as an optional feature.** Rejected: a `Medium`-or-higher
  operation without a tested rollback violates a success criterion (100% rollback
  coverage for `Medium+` in CI).

## Consequences

- Positive: the agent cannot drift imperatively; convergence is verifiable in
  tests (re-apply = zero changes).
- Positive: every risky action is recoverable; failed verification self-heals via
  rollback.
- Negative: every operation author pays the cost of implementing all six steps,
  including a real `rollback`. This is deliberate and non-negotiable.
- Negative: some operations have no clean rollback (e.g. destructive disk wipe);
  these are modeled as `Critical`, require confirmation, and must create a
  verifiable backup checkpoint or be denied by policy.
