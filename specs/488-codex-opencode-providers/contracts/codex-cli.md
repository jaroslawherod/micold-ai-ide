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

## T001 probe (2026-10-09, `@openai/codex` 0.162.0, linux-x64, no sign-in, no network to the model)

- Pin: `@openai/codex@0.162.0`. `codex --help`: `Usage: codex [OPTIONS] [PROMPT]`; `resume`, `fork`, `exec`, `-c key=value` confirmed.
- V8 (first-turn line shape), V13 (activity), V14 (per-launch MCP), V15 (trust prompt): **not probed** — need a signed-in session; every fallback stays in force.
- R8: no environment switch for the update check or telemetry found (`codex doctor` reports `updates` from config only). `launch_env()` stays empty.
- Store: `$CODEX_HOME` else `~/.codex` (`codex doctor` honours it). Platform paths unprobed beyond Linux.
