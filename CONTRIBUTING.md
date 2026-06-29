# Contributing

Contributions are welcome when they preserve the core safety contract: curated
Interspire operational tools, redacted output, and no broad admin escape
hatches.

## Before You Open A Pull Request

Run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

For dependency changes, also run:

```bash
./scripts/dependency_governance_check.sh
```

## Tool Changes

Every new tool or argument should include:

- a clear operator question it answers;
- output contract tests;
- schema snapshot review;
- redaction tests for any sensitive input or HTML-derived text;
- safety tests for blocked paths or failure modes.

Avoid adding generic XML/API/HTML escape hatches unless the curated tools have
proved insufficient and the new surface has an explicit safety model.

## Snapshot Changes

When the tool surface intentionally changes:

```bash
MCP_TOOLKIT_UPDATE_TOOL_SNAPSHOTS=1 cargo test tool_schema_snapshot_contract_is_stable
```

Review the `spec/tool_schema_snapshot.v1.json` diff before committing.
