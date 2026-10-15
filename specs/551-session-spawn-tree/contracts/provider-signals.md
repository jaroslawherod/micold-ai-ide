# Contract: provider origin readers

```text
fn reported_origin(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<Uuid>
```

- Pure read of the provider's own store; no write, no network.
- Returns `Some` only when the provider's own record names the session that started this one.
  Never derived from a title, a path, a prefix or a timestamp (FR-002).
- Default implementation returns `None`.
- A reader is merged only with a fixture in `crates/micold-core/tests/fixtures/<provider>/` and a
  test that the same fixture without the field yields `None`. The fixture is a recording of the
  provider's real output where one can be made; otherwise it is built from the provider's own
  authoritative schema or source and its first line or file name says `synthetic` (user-agreed
  2026-10-13, so a provider that reports is not blocked on a recording).
- The signal table in research.md R2 is mirrored by the user guide table (FR-014); a test fails
  when an `AiCli` variant has no row.

## Spike result (T025, 2026-10-12)

No provider has a real recording yet. Decided 2026-10-13: a provider whose signal is confirmed from its own schema or source gets a reader (M3) with a synthetic fixture; the rest are gaps.

- Claude Code: gap. No parent field found for `claude --bg` sessions.
- OpenCode: reader in M3, synthetic fixture. `session.parent_id` column exists in `opencode.db` (nullable text); no recorded
  sessions on the spike host. Reader would read `SELECT parent_id FROM session WHERE id = ?`.
- Codex: reader in M3, synthetic fixture (not observable here). Source-confirmed shape: rollout line 1 `type: "session_meta"`,
  `payload.source.subagent.thread_spawn.parent_thread_id` (UUID string); also optional
  `payload.parent_thread_id`. A recorded rollout replaces the synthetic fixture when one exists.
- Copilot CLI, Pi: gap (see research.md R2).
