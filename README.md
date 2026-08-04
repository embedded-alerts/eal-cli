# eal-cli

flags-2-env operator CLI for Embedded Alerts health, listing, and WebSocket event watching.

**Product:** Embedded Alerts — Embedding-based alerting for semantically relevant new information.

Define semantic alert rules, ingest source documents, compare embeddings, rank matches, and deliver explainable notifications.

## Safety and production boundary

Similarity scores are ranking signals, not truth guarantees. Production ingestion must respect source terms, robots rules, privacy requirements, retention limits, and notification consent.

This repository is an executable bootstrap, not a production deployment. Before live
use, add authentication, tenant authorization, rate limits, durable migrations,
observability, backups, incident response, dependency review, and secret management.
## Examples

```bash
cargo run -- health
cargo run -- --api-url http://127.0.0.1:8080 list
cargo run -- watch
```

Precedence is `CLI > environment > schema default`. The CLI audits
`.cli-flags.toml`, rejects unknown options and parse errors, and crosses into typed
configuration once before network work.
