# Contract: Codex CLI (`codex`)

- Command `codex` (Windows: `codex.cmd` through the existing PATH resolver).
- Fresh: `codex` (no args). Resume (bound id): `codex resume <thread-id>`. Never `--last`/picker.
- Store: `$CODEX_HOME` else `~/.codex`; `sessions/YYYY/MM/DD/rollout-<ts>-<thread-id>.jsonl`.
  First line `session_meta` payload has `id`, `cwd`. Candidate = file with mtime ≥ spawn − 2 s and
  `cwd` == session cwd and id not bound elsewhere. Missing/unreadable/odd ⇒ no conversation.
- Label: first user turn from a ≤64 KiB prefix; none if absent (V8 assumed).
- Binding + archive marker: `<codex home>/micold-bindings/<session-uuid>[.archived]`, outside `sessions/`.
- Sign-in: `<codex home>/auth.json`, shared read-only into the sandbox.
- Activity `None`; tool server `Unsupported{reason}`; readiness `OutputSettled`; trust `CodexProjects`.
