# Requests for Comments (RFCs)

An RFC is the design document for a crate or a feature. Under the project's
documentation-driven governance, **no code for a crate or feature is merged
before its RFC is accepted.** Where an ADR records a single cross-cutting
decision, an RFC describes the design of a concrete unit of work end to end.

## Workflow

1. Copy [`RFC-0000-template.md`](RFC-0000-template.md) to `RFC-NNNN-short-title.md`.
2. Open it as a pull request with status **Draft**.
3. Discuss and revise until the design and its exit criteria are agreed.
4. Mark it **Accepted** and begin implementation. The implementation PR links
   back to the RFC.

## Index

| RFC | Title | Status |
|---|---|---|
| [0000](RFC-0000-template.md) | Template | N/A |
| [0001](RFC-0001-steward-core.md) | `steward-core`: domain types and the Operation contract | Accepted |
