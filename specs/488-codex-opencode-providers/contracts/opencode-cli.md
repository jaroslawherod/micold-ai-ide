# Contract: OpenCode (`opencode`)

- Command `opencode` (Windows `opencode.exe`/`.cmd`).
- Fresh: `opencode`. Resume (bound id): `opencode --session <id>`. Never `--continue`.
- Store read through `opencode session list --format json` (cwd/project filter) and
  `opencode export <id>`; 2 s timeout, session PATH, best-effort (V10 field names assumed).
  Candidate = listed session, `directory` == cwd, created after spawn, not bound elsewhere.
- Label: session title / first user message from the export; none on failure.
- Binding + archive marker: `~/.local/share/opencode/micold-bindings/<session-uuid>[.archived]` (OpenCode's own store is a database, never written).
- Sign-in: `~/.local/share/opencode/auth.json`, shared read-only into the sandbox.
- Activity `None`; tool server `Unsupported{reason}`; readiness `OutputSettled`; trust `NeverAsks`.
