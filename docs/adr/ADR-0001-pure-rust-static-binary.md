# ADR-0001: Pure Rust, single static `musl` binary

- Status: Accepted
- Date: 2026-06-15
- Deciders: Angel Nereira

## Context

Gravital-Steward is a runtime that an AI coding agent installs on a raw Linux
host to operate applications, databases and web services. Its installation must
be trivial (`curl | sh`), its footprint small, and its behavior reproducible.
The runtime holds privileged operations and signs an audit log, so its supply
chain and memory safety matter directly to the security of every host it runs
on.

Forces at play:

- **Frictionless install.** Requiring Node, Python or a JVM on the target host
  defeats the "one command on a raw VM" promise and enlarges the attack surface.
- **Memory safety.** The runtime parses untrusted input (manifests, tokens) and
  performs privileged actions. Memory-unsafe languages add a class of
  vulnerabilities we cannot afford.
- **Reproducibility.** The same input must produce the same plan and the same
  resulting binary, for both auditability and trust.
- **No surprise C dependencies.** Dynamically linking `glibc` or pulling in
  OpenSSL complicates static builds and supply-chain review.

## Decision

The entire runtime is written in **pure Rust** and ships as a **single,
statically linked binary** targeting `*-unknown-linux-musl`
(`x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`).

Concretely:

- TLS is provided by **`rustls`**, never OpenSSL. `openssl`, `openssl-sys` and
  `native-tls` are banned in `deny.toml`.
- Embedded state uses **`redb`** (pure Rust), not an SQLite C library.
- `unsafe_code` is **forbidden** at the workspace lint level; any exception must
  be justified in its own ADR.
- The release profile strips symbols, enables LTO, and aborts on panic to keep
  the binary small and predictable.

## Alternatives considered

- **Go.** Single static binary and good ergonomics, but a garbage-collected
  runtime, weaker compile-time guarantees, and no equivalent of Rust's
  type-level "make invalid states unrepresentable" for the operation contract.
- **Node/Python with a packaged runtime.** Fails the frictionless-install and
  footprint requirements and broadens the supply chain enormously.
- **Dynamically linked `glibc` Rust build.** Simpler to build but reintroduces a
  host dependency and undermines the "no dependencies on the host" guarantee.

## Consequences

- Positive: trivial install, minimal footprint (target < ~20–30 MB), strong
  memory and type safety, clean supply chain, reproducible builds.
- Positive: `cargo-deny` in CI can mechanically enforce the no-OpenSSL,
  OSI-license-only rules.
- Negative: some ecosystem crates assume `glibc` or wrap C libraries and cannot
  be used; we must prefer pure-Rust alternatives or write integrations directly
  (e.g. `zbus` for systemd instead of shelling out).
- Negative: `musl` can have performance differences (allocator) we must measure
  for hot paths in later phases.
