# Data Model: Worktree session guard

All in `micold-core::worktree_guard` unless noted. No persisted state except the setting.

- **SessionBinding** (enum): `Worktree(PathBuf)` | `Default(PathBuf project root)` | `Unknown`. Own worktree (spec). `Unknown` => fail open.
- **GuardInput**: `binding: SessionBinding`, `project_root: PathBuf`, `worktrees: Vec<PathBuf>` (every location git lists, root included), `session_cwd: PathBuf`, `tool: ToolCall`.
- **ToolCall** (enum): `Edit{file_path}` (Edit/Write/MultiEdit/NotebookEdit) | `Bash{command}` | `Other`. Parsed from the PreToolUse `tool_name`/`tool_input`.
- **Allowlist**: shared build dir `<root>/target-shared`; worktrees whose dir name starts with `autopilot-prefetch`.
- **GuardVerdict** (enum): `Allow` | `AllowUnknown{reason}` | `Deny{message, target}`. Invalid combinations (deny with no target) unrepresentable.
- **GuardState** (daemon, per session): `Off` | `On{binding, project}`; frozen at launch (rules R7).
- **PathFacts** (trait): `canonicalize_existing_prefix`, `case_insensitive(path)`; fake impl in tests.
- **DaemonSettings.worktree_guard_enabled** (`micold-core::protocol::messages`): bool, default true; edit request field `Option<bool>`.

Transitions: launch => GuardState fixed; settings toggle => only later launches.
