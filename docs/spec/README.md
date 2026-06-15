# Protocol & operations specification

This directory is the normative specification of the runtime's external
contracts. It is the source of truth that the `steward-proto` types and the MCP
tool surface must conform to.

Planned documents (filled in as each phase lands its RFC):

- `operations.md` — the operation catalog (§9.1 of the blueprint): every public
  verb, its domain, risk level, required capabilities, input/plan/outcome shape.
- `errors.md` — the versioned error-code taxonomy (`PORT_IN_USE`,
  `CAPABILITY_DENIED`, `RISK_EXCEEDS_TOKEN`, ...). Mirrors `steward_proto::error`.
- `manifest.md` — the desired-state manifest schema (`steward/v1`). Mirrors
  `steward_proto::manifest`.
- `audit-event.md` — the audit event schema and the hash-chain/signature rules.
  Mirrors `steward_proto::audit`.
- `mcp.md` — the MCP transport binding (Streamable HTTP, JSON-RPC 2.0), the
  OAuth 2.1 resource-server requirements, and tool/resource exposure.

JSON Schemas are derived from the `steward-proto` types via `schemars`, so the
code and this specification cannot drift: a CI check regenerates and diffs the
schemas (planned for Phase 1).
