# RFC-0004: `steward-state` — embedded durable state store

- Status: Accepted
- Date: 2026-06-15
- Author: Angel Nereira
- Phase: 1 — Security core + minimal MCP
- Tracking issue: #2

## Summary

`steward-state` is the runtime's single durable store, backed by an embedded
`redb` database (pure Rust, no C). It persists the data the security core needs
across restarts: issued token records, the token revocation list, operation
checkpoints, and the audit log head hash.

## Motivation

The policy engine (`steward-policy`) and the audit log (`steward-audit`) are pure
and stateless by design. They need a place to keep durable facts:

- `steward-auth` must look up and revoke tokens.
- `steward-ops` must store a checkpoint before `apply` and read it back for
  `rollback`.
- `steward-audit` must persist its head hash so the hash chain survives a
  restart.

ADR-0001 forbids C dependencies, so `redb` (100% Rust) is the chosen embedded
store rather than an SQLite C library.

## Design

### Tables

| Table | Key | Value | Purpose |
|---|---|---|---|
| `tokens` | token id | JSON `TokenRecord` | Metadata of issued tokens (never secret material). |
| `revocations` | token id | revoked-at Unix seconds | The revocation list. |
| `checkpoints` | checkpoint id | JSON recovery blob | Rollback data written before `apply`. |
| `meta` | key | string | Singletons such as the audit head hash. |

Values are stored as JSON strings, so the schema can evolve additively without a
binary migration. `redb` gives ACID single-file transactions.

### `TokenRecord`

A serializable description of an issued token (metadata only — no token
material): `id`, `capabilities`, `denied`, `max_risk`, `confirm_above`,
`scope_any`/`scope_servers`, `created_unix`, `expires_unix`, optional `label`. It
uses `steward-core` types (`TokenId`, `CapabilitySet`, `RiskLevel`, ...), so it
maps cleanly onto a `steward_policy::TokenGrant` at authorization time.

### API (sketch)

```rust
struct StateStore { /* redb::Database */ }
impl StateStore {
    fn open(path: &Path) -> Result<Self, StateError>;
    fn open_in_memory() -> Result<Self, StateError>;   // for tests

    fn put_token(&self, record: &TokenRecord) -> Result<()>;
    fn get_token(&self, id: &TokenId) -> Result<Option<TokenRecord>>;
    fn list_tokens(&self) -> Result<Vec<TokenRecord>>;

    fn revoke(&self, id: &TokenId, at_unix: i64) -> Result<()>;
    fn is_revoked(&self, id: &TokenId) -> Result<bool>;

    fn put_checkpoint(&self, id: &str, recovery_json: &str) -> Result<()>;
    fn get_checkpoint(&self, id: &str) -> Result<Option<String>>;
    fn delete_checkpoint(&self, id: &str) -> Result<()>;

    fn set_audit_head(&self, head: &str) -> Result<()>;
    fn audit_head(&self) -> Result<String>;            // "" if unset
}
```

All writes are single transactions; reads use a read transaction.

### Errors

`StateError` wraps the `redb` error families (`Database`, `Transaction`,
`Table`, `Storage`, `Commit`) and `serde_json` serialization, via `thiserror`.

## Security considerations

- **No secrets in clear.** `TokenRecord` holds metadata only; token material and
  application secrets live in `steward-secrets`, never here.
- **Revocation is authoritative.** `is_revoked` is checked at authorization time
  in addition to expiry, so a stolen token can be cut off before it expires.
- **Crash safety.** `redb` transactions are ACID, so a crash mid-write cannot
  corrupt the store or the audit head.

## Alternatives considered

- **SQLite (`rusqlite`).** Pulls in a C library; violates ADR-0001.
- **`sled`.** Pure Rust, but `redb` has a simpler transactional model and a
  stable on-disk format better suited to a single-file runtime store.
- **A bespoke append-only file.** Reinvents transactions and indexing for no
  benefit.

## Test plan

- Round-trip a `TokenRecord` (put/get/list).
- Revoke and observe `is_revoked` flip; unknown ids are not revoked.
- Checkpoint put/get/delete.
- Audit head defaults to empty, persists across a reopen of the same file.

## Exit criteria

- Compiles for both musl targets; `clippy -D warnings` clean; tests pass.
- `cargo-deny` still passes (redb and its tree are OSI-licensed, no OpenSSL).
