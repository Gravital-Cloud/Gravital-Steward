# Architecture Decision Records

An Architecture Decision Record (ADR) captures a single significant decision,
its context, the options considered, and the consequences. ADRs are immutable
once **Accepted**: to change a decision, write a new ADR that supersedes the old
one.

This is part of the project's documentation-driven governance: no code that
depends on an architectural decision is merged before the ADR that justifies it
is accepted.

## Status values

- **Proposed** — under discussion.
- **Accepted** — ratified; implementation may proceed.
- **Superseded by ADR-NNNN** — replaced by a later decision.
- **Deprecated** — no longer relevant.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](ADR-0001-pure-rust-static-binary.md) | Pure Rust, single static `musl` binary | Accepted |
| [0002](ADR-0002-licensing.md) | Dual licensing: AGPL core, Apache protocol | Accepted |
| [0003](ADR-0003-mandatory-idempotency-and-reversibility.md) | Mandatory idempotency and reversibility | Accepted |

## Template

Use the following structure for new ADRs:

```markdown
# ADR-NNNN: Title

- Status: Proposed | Accepted | Superseded by ADR-MMMM | Deprecated
- Date: YYYY-MM-DD
- Deciders: <names>

## Context
What is the problem and the forces at play?

## Decision
The decision, stated in active voice.

## Alternatives considered
Each alternative and why it was not chosen.

## Consequences
Positive, negative, and neutral results of the decision.
```
