# Contributing to Gravital-Steward

Thank you for your interest. This project is built with strict, documentation-
driven governance so that a security-critical runtime stays auditable and
trustworthy. Please read this before opening a pull request.

## Golden rules

1. **Documentation first.** A crate or feature begins with an accepted RFC (and
   any ADRs it needs). Code that depends on an unmade decision is not merged.
   See [`docs/rfc`](docs/rfc) and [`docs/adr`](docs/adr).
2. **Phases are sequential.** Phase N+1 does not start until every exit criterion
   of Phase N is green in CI. See [`ROADMAP.md`](ROADMAP.md).
3. **Idempotency and reversibility are mandatory.** Every effectful operation
   implements the full `inspect → plan → validate → apply → verify → rollback`
   lifecycle (ADR-0003).
4. **Security by construction.** Best practices are encoded as executable rules,
   not comments.
5. **No non-open-source dependencies.** Every dependency must carry an OSI-
   approved license (prefer MIT/Apache/BSD) and must not pull in OpenSSL or
   unnecessary C. `cargo-deny` enforces this.

## Workflow

1. Find or open a **GitHub issue** describing the work. All commitments, debt and
   milestones live in issues — not in scattered files in the repo.
2. For new crates/features, open an **RFC PR** first and get it accepted.
3. Implement on a topic branch. Keep commits focused and meaningful.
4. Open a pull request linking the issue and RFC.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/) in **English**:

```
<type>(<scope>): <imperative summary>

<body explaining what and why, not how>
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`,
`chore`. Scope is usually the crate, e.g. `feat(core): …`, `docs(adr): …`.

The PR description is generated automatically from the commits on the branch
(see `.github/workflows/pr-description.yml`), so well-formed commit messages
produce good PR documentation for free.

## Quality gates (must pass in CI)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --target x86_64-unknown-linux-musl
cargo deny check        # licenses, bans, advisories
```

- Every public item carries a doc comment (`missing_docs` is a warning that CI
  treats as an error via `-D warnings`).
- New behavior ships with tests. `Medium+` operations ship with an idempotency
  test and a rollback test.
- `unsafe` is forbidden; any exception requires its own ADR.

## Language and attribution

- All code, comments, documentation, issues and commits are in **English**.
- Keep the repository understandable to any contributor: clear names, small
  modules, documented invariants.

## License of contributions

By contributing you agree that your contributions are licensed under the license
of the crate or document you modify: `AGPL-3.0-or-later` for core runtime crates,
`Apache-2.0` for the protocol/SDK crates and documentation. See
[ADR-0002](docs/adr/ADR-0002-licensing.md).
