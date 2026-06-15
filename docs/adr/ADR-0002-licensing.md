# ADR-0002: Dual licensing — AGPL core, Apache protocol

- Status: Accepted
- Date: 2026-06-15
- Deciders: Angel Nereira

## Context

Gravital-Steward is open source and must stay open source. At the same time we
want the widest possible ecosystem of clients, agents and integrations built on
top of its wire protocol. These two goals pull in opposite directions:

- A permissive license everywhere invites a third party to run the core runtime
  as a closed hosted service without contributing improvements back.
- A strong copyleft license everywhere discourages building proprietary clients,
  SDKs and tooling that merely speak the protocol.

The product must also work **completely** without any vendor cloud, so licensing
must not depend on a hosted control plane.

## Decision

License the project by component:

| Component | License |
|---|---|
| Core runtime crates (`steward-core` and all execution crates) | `AGPL-3.0-or-later` |
| Protocol and client SDK crates (`steward-proto`, `steward-sdk`) | `Apache-2.0` |
| Documentation and examples | `Apache-2.0` |

Each crate declares its own `license` field in `Cargo.toml`. `cargo-deny`
allows exactly the set of licenses in `deny.toml`, including
`AGPL-3.0-or-later` for first-party core crates.

## Alternatives considered

- **MIT/Apache everywhere.** Maximum adoption, but no protection against a
  closed SaaS fork of the core. Rejected.
- **AGPL everywhere.** Protects the core but deters building proprietary agents
  and SDKs against the protocol, shrinking the ecosystem. Rejected.
- **A Business Source License (BSL).** Source-available but not OSI-approved;
  conflicts with the "100% open source" principle. Rejected.

## Consequences

- Positive: the value-bearing runtime is protected by AGPL; the integration
  surface is frictionless under Apache-2.0.
- Positive: clear, mechanical enforcement via `cargo-deny`.
- Negative: contributors must be conscious of which crate they are editing and
  which license applies; the workspace cannot assume a single license header.
- Neutral: AGPL obligations only bind those who modify and distribute or host
  the core, which is the intended boundary.
