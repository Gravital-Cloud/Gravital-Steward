# RFC-0005: `steward-auth` — Biscuit capability tokens

- Status: Accepted
- Date: 2026-06-15
- Author: Angel Nereira
- Phase: 1 — Security core + minimal MCP
- Tracking issue: #2

## Summary

`steward-auth` issues, verifies and attenuates the runtime's capability tokens
using [Biscuit](https://www.biscuitsec.org/). A token is a signed, self-contained
bearer of capabilities that can be **attenuated offline** (restricted further
without contacting the server). Verification yields a `steward_policy::TokenGrant`
that the policy engine consumes to make the actual allow/deny decision.

## Motivation

The blueprint (§7 layer 1, §8) mandates capability tokens that are minimal by
default, attenuable offline, and explicitly deniable. Biscuit provides exactly
this: Ed25519-signed tokens whose authority block carries facts, with later
blocks able only to *add restrictions* (checks), never to widen authority. This
crate is the bridge between Biscuit tokens and the rest of the security core.

## Design

### What a token carries (authority block)

The authority block holds the grant as datalog facts plus an expiry check:

```text
token("tok_abc");
capability("server.inspect");      // repeated
denied("db.drop");                 // repeated
max_risk("medium");
confirm_above("high");
scope_any(true);                   // or one scope_server(...) per server
expires_unix(1750000000);
check if now($t), $t <= 1750000000;
```

Time is modeled as an **integer** `now(<unix_seconds>)` fact supplied by the
verifier and an integer `expires_unix`, so the whole grant is integer/string
datalog with no date-type handling. The check enforces expiry at authorization
time.

### API

```rust
struct TokenSpec { /* capabilities, denied, max_risk, confirm_above, scope, ttl */ }

struct AuthEngine { /* holds the root KeyPair */ }
impl AuthEngine {
    fn new(root: KeyPair) -> Self;
    fn public_key(&self) -> PublicKey;

    /// Issue a token; returns the base64 token plus a TokenRecord for the store.
    fn issue(&self, spec: &TokenSpec, now_unix: i64) -> Result<Issued, AuthError>;
}

/// Verify a token's signature and expiry, and extract its grant.
fn verify(token_b64: &str, root: &PublicKey, now_unix: i64)
    -> Result<Verified, AuthError>;

/// Attenuate a token by shortening its expiry. Offline; no server contact.
fn attenuate_expiry(token_b64: &str, not_after_unix: i64)
    -> Result<String, AuthError>;
```

- `Issued { token_b64, record: TokenRecord }`.
- `Verified { token_id, grant: TokenGrant, revocation_ids: Vec<Vec<u8>> }`.

Verification flow:
1. `Biscuit::from_base64(token, root)` — checks the Ed25519 signature.
2. Build an authorizer with `now(<now_unix>)` and `allow if true`, then
   `authorize()` — enforces the authority expiry check **and** any attenuation
   checks (e.g. a shortened expiry).
3. Query the **authority** facts to rebuild the `TokenGrant`. Because the
   authorizer trusts only authority (and authorizer) facts, an attenuation block
   can never add a capability — it can only restrict.
4. Expose `revocation_ids` so the caller can check them against the
   `steward-state` revocation list (revocation is enforced there, not here).

### Scope of attenuation in this iteration

This iteration implements **expiry-shortening** attenuation, which only
references the `now` fact the verifier always supplies. Capability- and
server-narrowing attenuation requires the full request context (operation,
server, risk) in the authorizer; that arrives when `steward-ops` integrates the
request path, and is tracked as follow-up work. Documented honestly rather than
half-built.

## Security considerations

- **Authority-only extraction.** Capabilities are read from the authority block;
  Biscuit's trust model prevents a later block from inflating them.
- **Offline attenuation.** Shortening expiry needs no server round-trip.
- **Revocation.** Enforced via `steward-state` using the token's revocation ids,
  in addition to expiry.
- **Key management is external.** The root `KeyPair` is provided by the runtime;
  tests use a deterministic key derived from fixed bytes.
- Pure-Rust crypto via Biscuit (Ed25519); no OpenSSL (ADR-0001). Biscuit is
  Apache-2.0.

## Alternatives considered

- **Plain signed JWT/PASETO.** No native offline attenuation or datalog policy.
- **Custom token format.** Reinvents a audited capability-token system for no
  benefit.
- **Date-typed expiry.** Rejected for this iteration in favor of integer `now`
  facts to keep extraction uniform and avoid date-format edge cases.

## Test plan

- Issue → verify round-trip yields the original grant (capabilities, denied,
  max_risk, confirm_above, scope, expires).
- An expired token (now > expires) fails verification.
- A token verified with the wrong public key fails.
- Expiry attenuation makes a token reject earlier than its original expiry while
  still verifying before the shortened deadline.
- The extracted grant maps onto `steward_policy::decide` (an end-to-end allow).

## Exit criteria

- Compiles for both musl targets; `clippy -D warnings` clean; tests pass.
- `cargo-deny` passes (Biscuit and its tree are OSI-licensed, no OpenSSL).
