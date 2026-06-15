# Runbooks

Operational procedures for running and recovering a Gravital-Steward
deployment. Each runbook is a precise, tested sequence — never improvisation.

Every runbook follows the same structure (mirroring the reference operations
guide, §29):

- **Symptoms** — how you know you are in this situation.
- **Impact** — who and what is affected.
- **Safe diagnostics** — read-only commands to confirm the diagnosis.
- **Mitigation** — ordered steps to reduce impact first.
- **Risks** — what each step can break.
- **Validation** — how to confirm recovery.
- **Rollback** — how to undo the mitigation if it makes things worse.
- **Contacts** — who to escalate to.

Planned runbooks (added as the matching features land):

- Install the runtime on a fresh VM.
- Issue, list and revoke a capability token.
- Recover from a failed rollback.
- Verify the audit chain (`steward audit verify`).
- Rotate a leaked secret.
- Restore a database backup.
