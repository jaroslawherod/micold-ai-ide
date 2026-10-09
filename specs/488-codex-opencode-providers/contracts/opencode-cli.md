# Contract: OpenCode (`opencode`)

- Command `opencode` (Windows `opencode.exe`/`.cmd`).
- Fresh: `opencode`. Resume (bound id): `opencode --session <id>`. Never `--continue`.
- Store read through `opencode session list --format json` (cwd/project filter) and
  `opencode export <id>`; 2 s timeout, output capped at 256 KiB, session PATH, best-effort (V10 field names assumed).
  Candidate = listed session, `directory` == cwd, created after spawn, not bound elsewhere.
- Label: session title / first user message from the export; none on failure.
- Binding + archive marker: `~/.local/share/opencode/micold-bindings/<session-uuid>[.archived]` (OpenCode's own store is a database, never written).
- Sign-in: `~/.local/share/opencode/auth.json`, shared read-only into the sandbox.
- Activity `None`; tool server `Unsupported{reason}`; readiness `OutputSettled`; trust `NeverAsks`.

## T001 probe (2026-10-09, `opencode-ai` 1.18.35, linux-x64, no sign-in)

- Pin: `opencode-ai@1.18.35`. `opencode --help`: `-c/--continue`, `-s/--session <id>`, `--fork`, `--prompt`, `[project]` positional confirmed.
- V5: no flag chooses a new session's id (`--session` continues an existing one). Bind after start stands.
- V10: `opencode session list --format json` and `opencode export [sessionID]` exist (`--format table|json`, `-n/--max-count`); with an empty store the list printed nothing, exit 0. **Field names not probed** (no sessions without a model); V10 stays ASSUMED with its fallback.
- Paths (`opencode debug paths`): data `~/.local/share/opencode`, config `~/.config/opencode`, state `~/.local/state/opencode`.
- R8: `OPENCODE_DISABLE_AUTOUPDATE` exists in the binary; `launch_env()` sets `OPENCODE_DISABLE_AUTOUPDATE=1`. No telemetry switch found.
- V13, V14, V15: **not probed**; fallbacks stay.
