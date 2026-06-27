# Safety Model

This repository is built around one rule: a tool should answer an operational
question without creating a new way to send mail or corrupt list state.

## Defaults

- Read-only tools are enabled by default.
- Guarded writes are disabled by default.
- Queue-control writes are separately disabled by default.
- Private audience exports require an explicit private artifact root.
- Tool output is redacted and aggregate wherever raw recipient or credential
  data might appear.

## Blocked Operations

The MCP server intentionally does not provide tools for:

- sending;
- scheduling;
- cron triggering;
- imports;
- generic raw contact exports;
- contact delete/edit operations;
- unsubscribe or resubscribe mutation;
- suppression mutation;
- settings save;
- SMTP credential changes;
- bounce setting changes;
- DNS or provider mutation.

## Admin HTML Allowlist

Legacy Interspire admin pages are brittle. The HTML adapter admits only known
paths:

- lists and list edit pages;
- selected settings tabs;
- users and user edit pages;
- newsletter manage and edit pages;
- schedule and stats pages.

Extra query parameters, duplicate query keys, path escapes, cross-origin URLs,
and send/import/export/contact mutation paths are blocked before HTTP requests
are made.

## Guarded Queue Controls

Queue control has two phases.

Preview:

- reads the Schedule page;
- finds cancel/delete links inside bounded table rows;
- validates that each link is a Schedule-page cancel/delete route with a
  numeric identifier;
- returns a deterministic plan id, redacted row summary, action, and route
  fingerprint.

Apply:

- requires `INTERSPIRE_GUARDED_WRITES=1`;
- requires `INTERSPIRE_QUEUE_WRITE_CONTROLS=1`;
- requires the exact plan id and action from preview;
- re-reads the Schedule page before apply;
- applies only the matching cancel/delete route;
- re-reads the Schedule page after apply;
- returns before/after counts and evidence.

Queue apply does not authorize sending and does not mutate lists, contacts,
suppression state, settings, SMTP, provider configuration, or DNS.

## Private Audience Artifacts

Audience hygiene exports can contain raw recipient addresses. They must be
written outside the repository under an explicitly approved private root:

```bash
export INTERSPIRE_AUDIENCE_HYGIENE_ROOTS=/secure/private
```

The output directory must be an absolute subdirectory under one of those roots.
Repository paths, relative paths, dot components, symlinks, root directories,
and unresolved escapes are rejected.

MCP output reports aggregate counts, warnings, file paths, sizes, and SHA-256
hashes only. The private files themselves must not be committed or pasted into
issue trackers, tickets, or chat.
