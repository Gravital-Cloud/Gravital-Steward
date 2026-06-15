# RFC-0003: `steward-audit` — append-only, hash-chained, signed log

- Status: Accepted
- Date: 2026-06-15
- Author: Angel Nereira
- Phase: 1 — Security core + minimal MCP
- Tracking issue: #2

## Summary

`steward-audit` records every action in an append-only log whose integrity is
cryptographically verifiable: each entry is BLAKE3 hash-chained to the previous
one and Ed25519-signed by the runtime key. An independent `verify` recomputes
the chain and validates every signature, so any tampering is detectable without
trusting the producer.

## Motivation

The blueprint (§5 layer 5, §17) and the reference operations guide both require
that *every automated intervention be auditable and immutable*, suitable for
export to a SIEM for government/banking compliance. A plain log is not enough:
we must be able to prove the record was not altered after the fact.

## Design

### Event shape

The wire shape is `steward_proto::audit::AuditEvent` (pinned in the Apache
protocol crate): `audit_id`, `timestamp`, `actor`, `token_id`, `capabilities`,
`operation`, `risk`, `outcome`, `rollback_available`, `prev_hash`.

### Hash chain

For entry *i*:

```
canonical_i = serde_json::to_vec(event_i)          // deterministic field order
entry_hash_i = BLAKE3(canonical_i)                 // hex-encoded
event_i.prev_hash = entry_hash_{i-1}               // "" for the genesis entry
```

Because `prev_hash` is a field of the event, it is covered by `entry_hash`, so
the chain is tamper-evident: changing any earlier event changes its hash and
breaks every subsequent `prev_hash`.

### Signature

```
signature_i = Ed25519_sign(signing_key, entry_hash_i_bytes)
```

Each appended record is a `SignedEnvelope { event, entry_hash, signature }`,
where `entry_hash` and `signature` are hex-encoded.

### API

```rust
struct AuditLog { /* signing key + current head hash */ }
impl AuditLog {
    fn new(signing_key: SigningKey) -> Self;     // head = genesis ("")
    fn append(&mut self, draft: AuditDraft) -> SignedEnvelope;
    fn head(&self) -> &str;                       // current entry_hash
}

fn verify(chain: &[SignedEnvelope], key: &VerifyingKey) -> Result<(), VerifyError>;
```

`AuditDraft` carries the caller-supplied fields; `append` fills `prev_hash`,
computes `entry_hash`, signs it, advances the head, and returns the envelope.
Persistence (writing envelopes to disk / `steward-state`) is intentionally **out
of scope** here: this crate owns the cryptography and the chain invariants, not
storage. The head hash can be persisted and restored via `new_with_head`.

### Errors (`VerifyError`)

`HashMismatch { index }`, `BrokenChain { index }`, `BadSignature { index }`,
`Serialization`.

## Security considerations

- **Key management is external.** The crate accepts a `SigningKey`; generation,
  storage and rotation belong to the runtime (and a KMS in Phase 5). Tests use a
  fixed seed for determinism.
- **Deterministic serialization.** `AuditEvent` contains no maps, so
  `serde_json` field order is stable and hashing is reproducible.
- **Independent verification.** `verify` needs only the public key and the
  chain, enabling third-party / SIEM-side validation.
- Pure Rust crypto (`blake3`, `ed25519-dalek`); no OpenSSL (ADR-0001).

## Alternatives considered

- **Merkle tree instead of a hash chain.** More complex; a linear chain is
  sufficient for an append-only audit log and simpler to verify and stream.
- **Signing the canonical event instead of the hash.** Equivalent, but signing
  the fixed-size hash keeps signature input uniform and cheap.
- **Putting signature fields on the proto `AuditEvent`.** Rejected: keeps the
  protocol event transport-neutral; the signature envelope is an audit-crate
  concern.

## Test plan

- A multi-entry chain verifies successfully end to end.
- Tampering with an event payload → `HashMismatch`.
- Rewriting a `prev_hash` link → `BrokenChain`.
- A signature from the wrong key → `BadSignature`.
- Genesis entry has an empty `prev_hash`; `head()` advances on each append.

## Exit criteria

- Compiles for both musl targets; `clippy -D warnings` clean; tests pass.
- `verify` accepts a valid chain and rejects each tampering mode with the
  correct `VerifyError`.
