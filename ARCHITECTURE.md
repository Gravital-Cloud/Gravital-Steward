# Gravital-Steward — Architecture

This document is the detailed architecture plan for the runtime. It expands the
blueprint into concrete engineering structure: component responsibilities,
dependencies, the data that flows between them, and the invariants each
component upholds. It is a living document; each crate's RFC refines its section.

- **Category:** agent-native server operations runtime.
- **Form factor:** a single statically linked Rust binary, `steward`.
- **Thesis:** a server is operated through *predefined, policy-governed,
  planned, verified and reversible* operations — never a free shell — so that an
  AI agent can act as a disciplined infrastructure engineer.

---

## 1. Guiding principles (non-negotiable)

These come directly from the blueprint and the ADRs and constrain every design
choice below.

- **Data-oriented architecture.** The data model — desired state, operations,
  capabilities, audit events — precedes and governs the implementation.
- **Compile-time guarantees.** Rust's type system makes invalid states hard to
  represent. A risky operation cannot exist without declaring its `RiskLevel`,
  required capabilities and rollback (ADR-0003).
- **Least privilege by default.** Every token is born minimal; capabilities are
  granted explicitly.
- **Fail closed.** On any policy ambiguity, deny.
- **Secure defaults.** Generated configurations are safe; dangerous patterns
  (privileged container, plaintext secret, unnecessary port) are blocked from
  the factory.
- **Reproducibility.** Same input → same plan → same state.
- **Total observability.** Everything the agent does is recorded and explainable.
- **Pure Rust, no OpenSSL, no host dependencies** (ADR-0001).

---

## 2. System context

```
┌──────────────────────────┐
│  GitHub repository        │  application source
└─────────────┬────────────┘
              │
              ▼
┌──────────────────────────┐
│  AI coding agent          │  Claude Code / Codex / Cursor
└─────────────┬────────────┘
              │  MCP (Streamable HTTP, JSON-RPC 2.0) + OAuth 2.1 access token
              ▼
╔════════════════════════════════════════════════════╗
║  GRAVITAL-STEWARD RUNTIME (single binary `steward`)  ║
║                                                      ║
║  Invocation surfaces:  steward-mcp · steward-cli     ║
║  Security core:        steward-auth · steward-policy ║
║                        steward-audit · steward-secrets║
║  Execution core:       steward-ops · steward-reconciler║
║                        steward-sandbox · steward-state ║
║  Domain executors:     steward-system · steward-web   ║
║                        steward-containers · steward-deploy║
║                        steward-db · steward-backup · steward-observe║
╚════════════════════════╤═════════════════════════════╝
                         ▼
┌──────────────────────────┐
│  Server resources         │
│  Linux · systemd · nft    │
│  Docker · DBs · TLS · FS  │
└──────────────────────────┘
```

The central component is the **runtime**, not the MCP server. MCP and the CLI
are two interchangeable invocation surfaces over the same operation engine.

---

## 3. Layered component model

The crates form four layers. Dependencies point downward only; nothing in a
lower layer depends on a higher one.

```
        ┌───────────────────────────────────────────────┐
  L4    │  Invocation surfaces                            │
        │    steward-mcp        steward-cli               │
        └───────────────┬───────────────┬────────────────┘
                        │               │
        ┌───────────────▼───────────────▼────────────────┐
  L3    │  Domain executors (implement Operation)         │
        │    steward-system  steward-web  steward-containers
        │    steward-deploy  steward-db   steward-backup  │
        │    steward-observe                              │
        └───────────────┬────────────────────────────────┘
                        │
        ┌───────────────▼────────────────────────────────┐
  L2    │  Execution & security core                      │
        │    steward-ops        steward-reconciler        │
        │    steward-policy     steward-auth              │
        │    steward-sandbox    steward-secrets           │
        │    steward-audit      steward-state             │
        └───────────────┬────────────────────────────────┘
                        │
        ┌───────────────▼────────────────────────────────┐
  L1    │  Foundations                                    │
        │    steward-core (traits, domain types)          │
        │    steward-proto (wire protocol, Apache-2.0)    │
        └─────────────────────────────────────────────────┘
```

### 3.1 Crate responsibilities

| Crate | Layer | License | Responsibility |
|---|---|---|---|
| `steward-core` | L1 | AGPL | `Operation` trait, `RiskLevel`, capabilities, typed ids, core error. No I/O. |
| `steward-proto` | L1 | Apache | Wire types: structured errors, manifest, audit event. Clients depend on this. |
| `steward-state` | L2 | AGPL | Embedded `redb` store: tokens, revocations, checkpoints, audit head, inventory. |
| `steward-auth` | L2 | AGPL | Biscuit capability tokens: issue/attenuate/validate/revoke. |
| `steward-policy` | L2 | AGPL | Allow/deny decisions: capability superset, max-risk, explicit denies, fail-closed. |
| `steward-audit` | L2 | AGPL | Append-only, BLAKE3-chained, Ed25519-signed log; independent `verify`. |
| `steward-secrets` | L2 | AGPL | Secret generation, encryption at rest, runtime injection, opaque references. |
| `steward-sandbox` | L2 | AGPL | cgroups v2 limits + namespaces; dangerous-pattern denial. |
| `steward-ops` | L2 | AGPL | Lifecycle orchestrator; wires policy + sandbox + audit + auto-rollback. |
| `steward-reconciler` | L2 | AGPL | Desired-state diff and ordered, idempotent convergence. |
| `steward-system` | L3 | AGPL | Users, packages, systemd (`zbus`), firewall (`nftables`), SSH, updates. |
| `steward-web` | L3 | AGPL | Caddy/Nginx reverse proxy, automatic TLS, security headers. |
| `steward-containers` | L3 | AGPL | Docker via `bollard` (later Podman). |
| `steward-deploy` | L3 | AGPL | `gix` clone, stack detection, build, release, healthcheck, rollback. |
| `steward-db` | L3 | AGPL | Postgres → MySQL/MariaDB → Redis; create/migrate/backup/restore. |
| `steward-backup` | L3 | AGPL | Verified dump/restore and volume snapshots. |
| `steward-observe` | L3 | AGPL | Logs, metrics, healthchecks, `diagnose`. |
| `steward-mcp` | L4 | AGPL | MCP server (Streamable HTTP, JSON-RPC), OAuth 2.1 resource server. |
| `steward-cli` | L4 | AGPL | `steward` binary: `init`, `token`, `daemon`, `run`, `audit verify`. |
| `steward-sdk` | — | Apache | Client SDK over the protocol (Phase 5). |

---

## 4. The operation lifecycle (core of the system)

Every effectful action implements `steward_core::Operation`, whose six steps the
engine drives uniformly:

```
inspect → plan → validate → apply → verify → (rollback on failure)
```

- **inspect** reads real state without mutation → `CurrentState`.
- **plan** computes the idempotent diff between current and desired → `Plan`
  (LLM-readable).
- **validate** checks safety/coherence; may block on a secure-default violation.
- **apply** creates a `Checkpoint`, then mutates under the sandbox →
  `(Checkpoint, Outcome)`.
- **verify** runs healthchecks/queries → `Verification`.
- **rollback** restores from the checkpoint.

### 4.1 Engine flow (`steward-ops`)

```
request (operation id + input + token)
  → steward-mcp validates the OAuth token (audience, expiry, iss)
  → steward-auth resolves the token's Biscuit capabilities
  → steward-policy decides allow/deny (+ confirmation if risk is high)
  → [if confirmation required] return the plan + confirmation_token; stop
  → steward-ops: inspect → plan → validate
  → steward-sandbox prepares containment (cgroup/namespace)
  → steward-ops: apply → verify
  → steward-audit records every step (signed)
  → if verify fails → rollback automatically + audit
  → structured response to the agent (typed success | typed error + next action)
```

The associated `Input`/`Plan`/`Outcome` types are `JsonSchema`, so `steward-mcp`
derives each operation's tool schema automatically and the agent always receives
structured data, never raw logs.

---

## 5. Security model — six enforced layers

Security is "best practice compiled into executable rules," not convention.

1. **Capability token (not root).** A token carries `duration`, `scope`
   (server/project), granted `capabilities`, explicit `denied` actions,
   `max_risk`, and `requires_confirmation_above`. Implemented with Biscuit so it
   can be *attenuated* offline (delegated more narrowly without contacting the
   server).
2. **Predefined operations, not a free shell.** The agent receives verbs
   (`server.harden`, `db.create`, `deploy.from_github`), never
   `run_command("sudo …")`.
3. **Plan before execution.** Every effectful operation passes through the full
   lifecycle; the runtime decides whether the plan is safe.
4. **Containment (blast radius).** cgroups v2 cap CPU/RAM/IO per operation;
   namespaces isolate risky work until `verify` passes. Dangerous patterns
   (privileged container, `--net=host` without justification, mounting `/`,
   plaintext secrets in a manifest) are denied by default.
5. **Cryptographic audit.** Every action is recorded in an append-only log,
   BLAKE3 hash-chained and Ed25519-signed, exportable to a SIEM.
6. **Rollback and recovery.** Every risky action checkpoints first; a failed
   `verify` triggers automatic rollback.

### 5.1 Authorization predicate

A request is authorized iff:

```
token.capabilities ⊇ operation.required_capabilities          (capability check)
∧ operation.risk(input) ≤ token.max_risk                       (max-risk check)
∧ operation.id ∉ token.denied                                  (explicit deny)
∧ now < token.expires                                          (expiry)
∧ request.server ∈ token.scope_server                          (scope)
∧ (operation.risk < confirm_threshold ∨ confirmation_present)  (human-in-the-loop)
```

Any failure denies (fail closed). The capability superset check is
`CapabilitySet::grants_all`, already implemented in `steward-core`.

### 5.2 Token model — two layers

- **Transport (toward the agent):** the runtime is an **OAuth 2.1 resource
  server**. It validates access tokens, audience (RFC 8707), issuer (RFC 9207);
  publishes Authorization Server Metadata (RFC 8414); supports Dynamic Client
  Registration (RFC 7591); returns 401 on invalid/expired tokens.
- **Capabilities (internally):** the access token maps to a **Biscuit** encoding
  attenuable capabilities. Token lifecycle:
  `issue → present → validate → (attenuate) → revoke/expire`, with a revocation
  list in `steward-state` in addition to expiry.

Two operating modes: **Local** (stdio, environment credentials, no OAuth — agent
runs on/beside the server) and **Remote** (Streamable HTTP, OAuth + Biscuit —
the main case).

---

## 6. State and data model (`steward-state`)

A single embedded `redb` database (pure Rust, no C) holds all durable runtime
state. Logical tables:

- `tokens` — issued token metadata (never the secret material) and their
  capabilities/scope.
- `revocations` — revoked token ids.
- `checkpoints` — recovery metadata keyed by checkpoint id, referenced by
  `apply`/`rollback`.
- `audit_head` — the latest audit hash, so the chain survives restarts.
- `inventory` — known services, ports, domains, and the last reconciled desired
  state per project (used to compute diffs).

Design rules: all writes are transactional; the desired state is stored so the
reconciler can compute `desired − current` deterministically; nothing here is a
secret in clear (see §8).

---

## 7. Declarative desired state and reconciliation

The agent submits a manifest (`steward/v1`, `kind: Deployment`) describing the
desired state — runtime, source repo, exposure (domain/proxy/TLS/port),
dependencies (e.g. Postgres), resources and healthcheck. `steward-reconciler`
computes `desired − current = diff` and runs the necessary `Operation`s in
order, idempotently. Re-applying the same manifest converges to no changes.

This neutralizes LLM imperative drift: the agent declares *what*, the runtime
decides *how*, and convergence is verifiable in tests.

---

## 8. Secrets (hermetic) (`steward-secrets`)

- The agent **never** sees a secret in clear. When asked to "create Postgres and
  connect the app," the runtime **generates** the password, **encrypts** it
  locally (`chacha20poly1305`, master key derived and protected — or delegated to
  a KMS in Phase 5), and **injects** it directly into the systemd unit's
  `Environment=` or the container.
- The agent receives only an opaque **reference**:
  `{"DATABASE_URL": "<SECRET_REF:db_prod_pass>"}`.
- `secret.rotate` re-generates and re-injects without downtime where possible.
- Injection happens at **runtime**, never persisted in the manifest or the
  agent's history.

---

## 9. Audit and compliance (`steward-audit`)

- **Append-only, chained:** each event carries `prev_hash` (BLAKE3) → a
  verifiable integrity chain.
- **Signed:** each event/block is Ed25519-signed with the runtime key, proving
  the automated intervention was not altered.
- **Per-event content:** `audit_id`, timestamp, actor, token id + capabilities,
  operation, input, plan, applied diff, files/services touched, outcome,
  `rollback_available`, `verify` result.
- **Export:** `audit.export` to a SIEM or a signed file for compliance.
- **Independent verification:** `steward audit verify` recomputes the chain and
  validates signatures. The `AuditEvent` shape is pinned in `steward-proto`.

---

## 10. Structured error protocol (`steward-proto::error`)

The runtime never returns raw Linux logs. It returns typed, actionable errors: a
stable `ErrorCode` from a versioned taxonomy, a `Severity`, a `domain`, a
human-readable `message`, machine-readable `context`, an ordered list of
`suggested_actions` (each with its own risk and confirmation flag),
`rollback_performed`, and the `audit_id`. Successful outputs are equally
structured and reference their `audit_id`.

---

## 11. MCP integration (`steward-mcp`)

- **Transport:** Streamable HTTP, JSON-RPC 2.0, designed against the **stateless**
  model (no sticky sessions; `tools/list` cached by TTL). stdio for local mode.
- **Auth:** OAuth 2.1 resource server (§5.2).
- **Tools:** one MCP tool per public operation in the catalog, plus
  `apply_manifest` for desired state. Each tool publishes its `schemars`-derived
  JSON Schema for automatic discovery.
- **Resources:** read-only server state, service inventory, latest audit events.
- **Confirmations:** `High`/`Critical` operations return the plan and a
  `confirmation_token`; a second confirmed call is required.

---

## 12. Containment (`steward-sandbox`)

- **cgroups v2** via `cgroups-rs`: per-operation CPU/RAM/IO ceilings so, e.g., a
  build cannot exceed a CPU budget or take down the host.
- **namespaces:** temporary network/FS isolation for risky tasks until `verify`
  passes.
- **Default denial** of dangerous patterns: privileged containers,
  `--net=host` without justification, mounting `/`, plaintext secrets in
  `env`.

---

## 13. Technology choices (all pure-Rust, OSI-licensed)

| Concern | Crate | Rationale |
|---|---|---|
| Async runtime | `tokio` | Standard, native concurrency. |
| HTTP server | `axum` + `hyper` + `tower` | MCP Streamable HTTP, middleware. |
| TLS | `rustls` | No OpenSSL → clean static binary (ADR-0001). |
| Serialization | `serde`, `serde_json`, `serde_yaml` | Manifests, protocol. |
| JSON Schema | `schemars` | Auto-derived MCP tool schemas. |
| Capability tokens | `biscuit-auth` | Attenuable offline (Apache-2.0). |
| OAuth | `oauth2` + custom RS metadata | Resource server 2.1. |
| Local state | `redb` | Embedded KV, 100% Rust. |
| Sign/hash | `ed25519-dalek`, `blake3` | Audit signing + hash-chain. |
| Secret encryption | `chacha20poly1305` / `aes-gcm` | Secrets at rest. |
| CLI | `clap` | `steward` CLI and installer. |
| Tracing | `tracing`, `tracing-subscriber` | Runtime observability. |
| systemd / D-Bus | `zbus` | Talk to systemd without a shell (pure Rust). |
| System info | `sysinfo` | Resources, processes. |
| Firewall | `nftables` crate / controlled `nft` | Network rules. |
| cgroups | `cgroups-rs` | Blast-radius containment. |
| SQL drivers | `sqlx` | Async, C-free where possible. |
| Redis/Valkey | `fred` / `redis` | Cache/KV. |
| Git | `gix` (gitoxide) | Clone repos in pure Rust. |
| Containers | `bollard` | Docker Engine API. |
| Static build | `*-unknown-linux-musl` | Binary with no dynamic libc. |

Before adding any crate: verify an OSI license (prefer MIT/Apache/BSD) and that
it does not drag in OpenSSL or unnecessary C. `cargo-deny` enforces this in CI.

---

## 14. Repository layout

```
gravital-steward/
├── Cargo.toml                  # workspace
├── LICENSE / LICENSE-AGPL / LICENSE-APACHE
├── deny.toml                   # cargo-deny: licenses + bans + advisories
├── rust-toolchain.toml         # pinned channel + musl targets
├── ROADMAP.md / ARCHITECTURE.md
├── install/
│   └── install.sh              # curl|sh bootstrap (Phase 1)
├── docs/
│   ├── adr/                    # Architecture Decision Records
│   ├── rfc/                    # one design doc per crate/feature
│   ├── runbooks/               # documented operations
│   └── spec/                   # protocol & operations specification
└── crates/
    ├── steward-core/           # L1: traits + domain types
    ├── steward-proto/          # L1: wire protocol (Apache-2.0)
    └── …                       # L2–L4 crates added phase by phase
```

Each domain crate depends on `steward-core` and registers its `Operation`s in a
registry; the final binary assembles everything.

---

## 15. Build, release and reproducibility

- **Targets:** `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` → one
  statically linked binary, no host dependencies.
- **Release profile:** `opt-level = "z"`, LTO, one codegen unit, stripped,
  `panic = "abort"` → small, predictable binary (< ~20–30 MB target).
- **Reproducibility:** same input → same plan → same state; the reconciler and
  operation tests assert "re-apply = zero changes."

---

## 16. Testing strategy

- **Unit tests** in every crate (already present in `steward-core` and
  `steward-proto`).
- **Idempotency tests:** for every `Medium+` operation, re-applying yields no
  change.
- **Rollback tests:** for every `Medium+` operation, a forced `verify` failure
  triggers and completes rollback (success criterion: 100% coverage in CI).
- **Audit integrity tests:** `audit verify` passes; tampering breaks the chain.
- **Protocol tests:** `schemars` schemas are regenerated and diffed against the
  spec; parsers are fuzzed (Phase 1+).
- **CI gates:** `build` (musl), `test`, `clippy -D warnings`, `fmt --check`,
  `cargo-deny`. A phase does not close until its exit criteria are green.

---

## 17. Risks and mitigations

| Risk | Mitigation |
|---|---|
| LLM takes a destructive action | Predefined operations + deny policy + human confirmation on `High`/`Critical` + rollback. |
| Imperative drift / hallucinated steps | Declarative model + idempotent reconciler. |
| Malicious build exhausts resources | cgroups v2 + namespaces (blast radius). |
| Secret leakage | Runtime injection + opaque references; never clear to the agent. |
| Stolen token | Short, attenuated Biscuit + revocation list + OAuth audience. |
| Log tampering | BLAKE3 chain + Ed25519 signature + independent `verify`. |
| Unexpected C dependency (OpenSSL) | `rustls` + `cargo-deny` ban in CI. |
| MCP spec changes | Design against the stateless target; isolate `steward-proto` to absorb change. |
| Accidental lock-in | Single-node works alone; the control plane is always optional. |
