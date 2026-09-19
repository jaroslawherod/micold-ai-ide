# First-turn label fixtures (feature 032)

Synthetic conversation records for `crates/micold-core/tests/first_turn_label.rs` and the
provider tests beside it (contract `specs/032-untitled-session-labels/contracts/first-turn-label.md`,
clauses C3 and C4).

- `claude/` holds `.jsonl` transcripts shaped on `claude` 2.1.2xx records.
- `copilot/` holds `events.jsonl` / `workspace.yaml` shapes from Copilot 1.0.10–1.0.83 (feature 032, M2).

Every record here was written by hand to reproduce one record *shape* (field names, nesting, the
tags `claude` wraps command and injected text in). None is a copy of a real conversation, and no
real user transcript may be committed here. A first turn past the 1 MiB read bound is built at test
run time rather than stored, so no multi-megabyte file lives in the repository.
