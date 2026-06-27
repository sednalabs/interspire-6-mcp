# AGENTS.md - interspire-6-mcp

This repository contains a public Rust MCP server for Interspire Email Marketer
6.2.3. It follows the curated stdio intent-server pattern from
`sednalabs/mcp-toolkit-rs`.

## Engineering Rules

- Keep the MCP surface small and operator-shaped. Prefer focused intent tools
  over generic XML, SQL, HTTP, or admin escape hatches.
- Read-only tools are the default. Mutating tools require an explicit safety
  design, runtime enablement, preview/apply semantics, redacted output, and
  post-apply readback.
- Do not add send, schedule, cron-trigger, import, raw contact export,
  unsubscribe/resubscribe, suppression mutation, SMTP password, provider, DNS,
  or generic admin URL tools.
- Fixtures must be synthetic or redacted. Never commit credentials, cookies,
  raw recipient exports, saved admin HTML from a live system, provider payloads,
  private headers, or local operator files.
- Public docs and examples must use placeholder hosts and paths.
- Keep dependencies shallow and consistent with `mcp-toolkit-rs`.

## Required Checks

Run focused checks before committing behavior changes:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

For dependency changes, also run:

```bash
./scripts/dependency_governance_check.sh
```

## Documentation

Update README and docs when behavior, tool contracts, configuration, security
posture, or workflow expectations change. Documentation quality is part of the
release contract for this public repository.
