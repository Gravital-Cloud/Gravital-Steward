# Gravital-Steward

> The open-source server operator, agent-native by design.

Gravital-Steward is an **agent-native server operations runtime**: a single
static Rust binary you install on any Linux host so that an AI coding agent
(Claude Code, Codex, Cursor) can deploy, secure and operate applications,
databases and web services through **safe, idempotent, auditable and reversible
operations** — never a free shell.

It encapsulates the combined knowledge of an architect, a software engineer, a
DBA and an SRE as an installable local operator. The agent does not improvise
with SSH; it invokes predefined operations that are governed by policy, planned
before execution, verified after, and rolled back on failure.

## Why

Giving a language model power over a server without a layer that encodes best
practices, containment and auditability is unsafe by definition. LLMs hallucinate
imperative sequences, consume resources without limit, and do not guarantee
reversibility. Gravital-Steward is that missing layer.

## Principles

- **Agent-native.** Operations, errors and outputs are structured for an LLM to
  consume, not for a human dashboard.
- **Capability security.** Attenuable capability tokens (Biscuit) + OAuth 2.1 —
  not flat API keys.
- **Declarative + idempotent.** The agent submits a desired state; the runtime
  computes the diff and converges. Immune to imperative drift.
- **Bounded blast radius.** Each operation runs under cgroups/namespaces.
- **Cryptographic audit.** An append-only, hash-chained, signed log, exportable
  to a SIEM.
- **One pure-Rust binary.** `curl | sh`, no Node/Python/JVM, no OpenSSL, no host
  dependencies.
- **100% open source, no vendor cloud required.** The multi-server control plane
  is optional and additive.

This is not a PaaS, a dashboard, or "Docker with AI." It is a new layer: a
server-operations runtime designed for an agent to act as a disciplined
infrastructure engineer.

## Status

Early development. The project is built **documentation-first** and **phase by
phase** — see [`ROADMAP.md`](ROADMAP.md). The current milestone is **Phase 0:
foundations and governance**, which establishes the workspace, the core
contracts, and the governance scaffolding.

What exists today:

- `steward-core` — the `Operation` lifecycle trait, risk classification, the
  capability model, typed identifiers, and the core error type.
- `steward-proto` — the wire-protocol types: the structured error taxonomy, the
  desired-state manifest, and the audit-event schema.

## Architecture

The runtime is a workspace of focused crates layered foundations → execution &
security core → domain executors → invocation surfaces. The full design is in
[`ARCHITECTURE.md`](ARCHITECTURE.md). Every effectful action implements the
uniform lifecycle:

```
inspect → plan → validate → apply → verify → (rollback on failure)
```

## Building

Requires a stable Rust toolchain.

```sh
# Native build, tests, and lints
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Static binary targets
cargo build --workspace --target x86_64-unknown-linux-musl
```

## Governance

- **ADRs** record cross-cutting decisions: [`docs/adr`](docs/adr).
- **RFCs** design each crate/feature before it is built: [`docs/rfc`](docs/rfc).
- **Spec** is the normative protocol/operations contract: [`docs/spec`](docs/spec).
- **Runbooks** document operations: [`docs/runbooks`](docs/runbooks).
- Roadmap, milestones and known debt are tracked in **GitHub issues**, not in
  scattered files.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) to get involved and
[`SECURITY.md`](SECURITY.md) to report vulnerabilities.

## License

Dual-licensed by component: the core runtime is **AGPL-3.0-or-later**; the
protocol and SDK crates and the documentation are **Apache-2.0**. See
[`LICENSE`](LICENSE) and [ADR-0002](docs/adr/ADR-0002-licensing.md).

Copyright (c) Angel Nereira and contributors.
