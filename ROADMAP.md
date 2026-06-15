# Gravital-Steward — Roadmap

This roadmap turns the blueprint's phased plan (§18) into a trackable, ordered
program of work. It is governed by two rules:

1. **Documentation-driven.** Every crate and feature starts with an accepted
   RFC (and any ADRs it needs). No code that depends on a decision is merged
   before that decision is recorded.
2. **Sequential phases with exit criteria.** Phase N+1 does not start until
   **every** exit criterion of Phase N passes in CI.

Progress is tracked exclusively through **GitHub issues and milestones** — one
milestone per phase, one issue per crate/feature/known debt — not through
scattered status files in the repository. This document is the high-level map;
the issues are the live state.

Legend: ☐ not started · ◐ in progress · ☑ done

---

## Phase 0 — Foundations and governance

**Goal:** a compiling, lint-clean, statically buildable workspace with the core
contracts and the governance scaffolding in place.

**Build**

- ☑ Cargo workspace, static `musl` build targets, release profile.
- ☑ `steward-core`: `Operation` trait, `RiskLevel`, capability model, typed ids,
  core error.
- ☑ `steward-proto`: structured error taxonomy, manifest envelope, audit event
  schema.
- ☑ `docs/` scaffolding: ADR-0001 (pure Rust / static binary), ADR-0002
  (licensing), ADR-0003 (idempotency + reversibility); RFC template + RFC-0001.
- ☑ CI: `build` (musl), `test`, `clippy -D warnings`, `fmt --check`,
  `cargo-deny`. Automatic PR-description generation from commits.
- ☑ `deny.toml` enforcing OSI licenses and the OpenSSL ban.
- ☑ README, CONTRIBUTING, SECURITY, dual licensing.

**Exit criteria**

- Workspace compiles to a static binary on `x86_64`/`aarch64` `musl`.
- CI is green: build, test, clippy, fmt, `cargo-deny`.
- ADRs 0001–0003 accepted; RFC-0001 accepted.

---

## Phase 1 — Security core + minimal MCP

**Goal:** a runtime an agent can install, authenticate to, and use for a single
read-only operation, with every action signed in the audit log.

**Build**

- ☑ `steward-state` — embedded store (`redb`): tokens, revocation list,
  checkpoints, audit head. (RFC-0004)
- ☐ `steward-auth` — Biscuit capability tokens: issue, present, validate,
  attenuate, revoke/expire; lifecycle CLI surface.
- ☑ `steward-policy` — allow/deny engine: capability superset check, max-risk
  check, explicit denies, fail-closed default. (RFC-0002)
- ☑ `steward-audit` — append-only, BLAKE3 hash-chained, Ed25519-signed log;
  `audit verify`. (RFC-0003)
- ☐ `steward-mcp` — Streamable HTTP, JSON-RPC 2.0, stateless design; OAuth 2.1
  resource server (audience RFC 8707, metadata RFC 8414, `iss` RFC 9207, DCR
  RFC 7591); 401 on invalid token.
- ☐ `steward-cli` — `steward init`, `steward token issue|list|revoke`,
  `steward daemon`, `steward audit verify`.
- ☐ `install/install.sh` + systemd unit; `server.inspect` (read-only) as the
  first real operation.

**Exit criteria**

- `curl | sh` installs the runtime on a clean VM.
- `steward init` generates a scoped token.
- An agent connects over MCP, authenticates, runs `server.inspect`.
- The action is signed in the audit log; `steward audit verify` passes.

**RFCs:** state store, auth/tokens, policy engine, audit log, MCP transport, CLI.

---

## Phase 2 — System operations + containment + full lifecycle

**Goal:** the complete `inspect → … → rollback` engine running real,
idempotent system operations inside an enforced blast radius.

**Build**

- ☐ `steward-ops` — the lifecycle orchestrator integrating policy, sandbox,
  audit, and automatic rollback on failed `verify`.
- ☐ `steward-sandbox` — cgroups v2 resource limits + namespaces; default-deny of
  dangerous patterns.
- ☐ `steward-system` — users, packages, services (systemd via `zbus`), firewall
  (`nftables`), SSH hardening, system updates.
- ☐ `steward-observe` — logs, healthchecks, `diagnose`.

**Exit criteria**

- `server.harden` runs idempotently on a raw VM (re-apply = no changes) with a
  working checkpoint and rollback.
- An operation that fails `verify` rolls back automatically.
- A build with an infinite loop is contained by a cgroup without taking down the
  host.

**RFCs:** operation engine, sandbox/containment, system executor, observability.

---

## Phase 3 — MVP "laser scalpel": containers + proxy + deploy + secrets

**Goal (validation milestone):** from a declarative manifest, deploy a
containerized app from GitHub to an HTTPS domain, with an injected secret the
agent never sees, healthchecked and reversible.

**Build**

- ☐ `steward-containers` — Docker via `bollard`.
- ☐ `steward-web` — Caddy (automatic HTTPS) first, then Nginx; reverse proxy,
  TLS, security headers.
- ☐ `steward-deploy` — `gix` clone, stack detection, build, release,
  healthcheck, rollback.
- ☐ `steward-secrets` — generation, encryption at rest (`chacha20poly1305`),
  runtime injection, opaque references.
- ☐ `steward-reconciler` + the `apply_manifest` MCP tool — desired-state diff
  and convergence.

**Exit criteria**

- From a manifest, the agent deploys a containerized GitHub app on a domain with
  automatic HTTPS, a generated-and-injected secret the agent never sees, a
  healthcheck and rollback.
- Re-applying the manifest converges with zero changes.
- **Product validated: resolves ~80% of modern cases.**

**RFCs:** containers, web/proxy, deploy pipeline, secrets, reconciler + manifest.

---

## Phase 4 — Databases + backups

**Goal:** provision databases, connect apps by injected secret, and prove
verifiable backup/restore.

**Build**

- ☐ `steward-db` — PostgreSQL → MySQL/MariaDB → Redis/Valkey (`sqlx`, `fred`).
- ☐ `steward-backup` — `pg_dump`/`mysqldump`, volume snapshots, **verified
  restore**.

**Exit criteria**

- The agent provisions Postgres, creates DB + user, connects the app via an
  injected secret, runs `db.backup` and a **verifiable** `db.restore`.
- A migration runs with checkpoint and rollback.

**RFCs:** database executor, backup/restore.

---

## Phase 5 — Compliance and enterprise

**Goal:** end-to-end signed, verifiable audit export and optional multi-server
operation without breaking single-node.

**Build**

- ☐ `audit.export` to SIEM (Splunk/Datadog) and signed immutable file export.
- ☐ KMS integration for secret master keys.
- ☐ Policy packs.
- ☐ (Optional) multi-server control plane — always additive.

**Exit criteria**

- Verifiable end-to-end signed export.
- Documented compliance pack.
- Optional multi-server mode that never breaks the single-node experience.

---

## Phase 6+ — Expansion

- Podman; more databases (MongoDB, SQL Server, Cassandra); more proxies (Apache,
  Traefik); more operating systems (RHEL/Alma, Arch); a blueprint marketplace;
  premium plugins.

---

## Cross-cutting tracks (run continuously)

- **Security:** threat-model each crate in its RFC; `cargo-deny` advisories;
  fuzz the protocol parsers from Phase 1.
- **Observability of the runtime itself:** structured `tracing` from day one.
- **Reproducibility tests:** every `Medium+` operation gets a "re-apply = no
  change" and a "verify-fails → rollback" test (success criteria §23).
- **Documentation:** an RFC precedes each crate; runbooks land with the feature
  they document.

## Success metrics (from the blueprint §23)

1. Raw VM to HTTPS app in < 10 min with a single command + agent.
2. Re-applying a manifest yields zero changes.
3. 100% of `Medium+` operations have rollback tested in CI.
4. `audit verify` always passes; the chain is never broken.
5. Single binary < ~20–30 MB, no host dependencies.
6. Zero operations expose secrets in clear text to the agent; zero dangerous
   defaults permitted.
7. A developer with no SRE experience deploys and operates a real app without
   touching the shell.
