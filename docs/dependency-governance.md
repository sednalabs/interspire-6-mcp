# Dependency Governance

This document defines dependency selection and upgrade policy for
`interspire-mcp`.

## Goal

Keep the server secure, maintainable, and release-friendly by preferring
well-maintained crates with clear operational risk signals.

## Go/No-Go Criteria

All new direct crates and major upgrades must meet every hard gate below.

1. `security`: No unresolved RustSec advisory for the selected version.
2. `license`: License is allowlisted by `deny.toml`.
3. `source`: Registry source is trusted. Public git dependencies must be
   explicitly allowlisted.
4. `maintenance`: Evidence of active maintenance.
5. `adoption/reputation`: Evidence the crate is broadly used or maintained by
   a trusted project.
6. `fit`: Clear justification that existing dependencies or the standard
   library cannot solve the need with lower risk.

If a hard gate fails, the dependency change is a no-go unless an explicit,
time-bounded exception is approved and documented.

## Required Evidence

Every dependency change should include a policy note in the associated pull
request:

```text
Dependency change note
- crate: <name> <old -> new>
- change type: <new | upgrade | removal>
- purpose: <why needed>
- alternatives considered: <stdlib/existing crates/other crates>
- maintenance evidence: <release recency + repo activity>
- adoption/reputation evidence: <reverse-deps/downloads/known users or maintainer org>
- security status: <cargo deny + cargo audit result>
- license status: <allowlisted license(s)>
- startup impact: <expected effect on cold start/steady state>
- rollback plan: <how to revert safely>
- exception (if any): <risk accepted, owner, expiry date>
```

## Enforcement

Install the local tool set:

```bash
cargo install --locked cargo-deny cargo-audit cargo-outdated
```

Run:

```bash
./scripts/dependency_governance_check.sh
```

The script enforces:

- advisory, license, ban, and source policy via `cargo-deny`;
- RustSec vulnerability checks via `cargo-audit`;
- direct dependency stale-risk reporting via `cargo-outdated`.

Outdated direct dependencies are report-only by default. To make them blocking:

```bash
STRICT_OUTDATED=1 ./scripts/dependency_governance_check.sh
```

`cargo outdated` is allowed to warn rather than fail in report-only mode because
its solver can be stricter than the locked build graph when git dependencies
pin older shared crates. Treat those warnings as triage input, not a green light
to ignore stale dependencies.

## Current Exceptions

- `RUSTSEC-2025-0057` (`fxhash`): maintenance-status advisory inherited through
  `scraper -> selectors`. This is not a known vulnerability, but it is still
  dependency debt. Keep the exception visible in both `deny.toml` and
  `scripts/dependency_governance_check.sh`, and revisit it when replacing the
  HTML parsing stack or when `selectors` moves away from `fxhash`.

## Current Direct Git Dependencies

`mcp-toolkit`, `mcp-toolkit-core`, `mcp-toolkit-observability`,
`mcp-toolkit-policy-core`, and `mcp-toolkit-testing` are pinned to
`sednalabs/mcp-toolkit-rs` commit
`358f5da30f898451fe8b9a3acf0ae2237a88a1dd`.

Dependency change note
- crate: MCP Toolkit crates, public git pin
- change type: upgrade
- purpose: use the shared sensitive-read posture, Apps SDK metadata helper,
  policy-core decision helper, and redaction helper instead of implementing
  those generic MCP safety primitives locally.
- alternatives considered: local Interspire-only policy code, or waiting for a
  published toolkit crate release. Local-only code would duplicate generic MCP
  safety behaviour, and waiting would block the public Interspire release.
- maintenance evidence: the dependency is maintained in the Sedna Labs MCP
  Toolkit repository and the pinned commit is covered by the upstream
  sensitive-read primitive pull request.
- adoption/reputation evidence: the toolkit is the shared Sedna Labs MCP
  foundation used by this server and sibling MCP implementations.
- security status: `./scripts/dependency_governance_check.sh` must pass before
  release; advisory exceptions remain documented above.
- license status: toolkit crates are public Sedna Labs crates governed by the
  repository license and this repository's `deny.toml` source/license policy.
- startup impact: no expected runtime startup regression; helpers are policy,
  metadata, and redaction code on existing call paths.
- rollback plan: revert the sensitive-read integration and return to the
  previous redacted-only readback surface, or move the git pin to the first
  released toolkit version that contains the same primitives.
- exception: temporary public git pin until the toolkit branch lands and a
  release tag or crate publication is available.
