# Security Policy

Gravital-Steward operates privileged actions on servers and signs an audit log.
Security is the core of the product, and we take vulnerability reports seriously.

## Reporting a vulnerability

**Do not open a public issue for a security vulnerability.**

Please report privately via GitHub's **"Report a vulnerability"** flow (Security
→ Advisories) on this repository, or by email to **contact@angelnereira.com**
with the subject line `SECURITY: <short summary>`.

Include, where possible:

- A description of the issue and its impact.
- The component/crate and version or commit affected.
- Steps to reproduce or a proof of concept.
- Any suggested remediation.

## What to expect

- **Acknowledgement** within 72 hours.
- An initial **assessment** and severity classification within 7 days.
- Coordinated disclosure: we will agree on a timeline with you and credit you in
  the advisory unless you prefer to remain anonymous.

## Supported versions

The project is in early development; security fixes target the `main` branch.
Once releases begin, this section will list supported versions.

## Security model

The runtime's security design is documented in
[`ARCHITECTURE.md`](ARCHITECTURE.md) (§5) and the ADRs. Key properties:

- Capability tokens (Biscuit) + OAuth 2.1 resource server; least privilege by
  default; fail closed.
- Predefined operations only — never a free shell.
- Blast-radius containment via cgroups v2 and namespaces.
- Secrets are never exposed to the agent in clear text.
- Append-only, hash-chained, signed audit log with independent verification.
- No OpenSSL or unnecessary C dependencies; `cargo-deny` enforces the supply
  chain in CI.

If you find a way to bypass any of these properties, that is a vulnerability —
please report it.
