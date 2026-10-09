# Data Model: #488

No new persisted entity; additive fields only.

| Entity | Change | Compatibility |
|---|---|---|
| `AiCli` (`micold-core/src/session.rs`) | variants `Codex`, `OpenCode`; `ALL: [AiCli; 5]` | wire-visible ⇒ `PROTOCOL_VERSION` 38→39 (FR-016) |
| `StoredAiCli` (`store.rs`) | variants `Codex`, `OpenCode` | `#[serde(default)]` enum unchanged for old files (FR-002) |
| Binding file | `<config_dir>/micold-bindings/<session-uuid>` (string id) and `<session-uuid>.archived` — app-owned, written once at bind | additive file; no store schema change, none on the wire |
| `AiCliProvider` trait | required methods (no defaults, per the trait's FR-021 rule; all five providers and `FakeAiCliProvider` implement them): `identity()`, `launch_args_in(config_dir, id, mode)`, `new_conversations(config_dir, cwd, since)`, `bind(config_dir, id, &ConversationRef)`, `sandbox_auth_file(home)` | AppAssigned providers return the old values, so claude/copilot/pi are byte-identical (FR-015, SC-007) |
| `ConversationRef` | `{ id: String, cwd: PathBuf, created: SystemTime }` | new, core-internal |
| `FolderTrust` | arm `CodexProjects` (reads `config.toml` `trust_level`) | new arm |

State: a binding file is created once (bind) and never rewritten; candidates exclude ids already
bound by any binding file under the same `micold-bindings/`.
