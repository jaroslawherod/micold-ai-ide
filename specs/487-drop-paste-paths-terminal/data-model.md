# Data Model: 487

All types are new, in `micold-core::path_insert`.

- **ShellKind** `{Posix, Bash, Fish, PowerShell, Cmd}`; a sandboxed target forces `Bash` — `detect(shell_command) -> ShellKind`.
- **InsertTarget** `{Host, Sandbox(&MountSet, mounted: &[String])}`.
- **InsertedPath** `{ original: PathBuf, shown: PathBuf /*host or container*/, text: String /*quoted*/ }`.
- **Refusal** `{ OutsideProjects{path}, NotMounted{path}, Unrepresentable{path, shell} }` — `message() -> String` names the file and reason (FR-011, SC-006).
- **InsertionPlan** `{ accepted: Vec<InsertedPath>, refused: Vec<Refusal> }` — `text() -> Option<String>` joins accepted with single spaces; none accepted → no insertion.
- **PastedLayout** `{ root: PathBuf, session: SessionId }` — `dir()`, `next_file(now, seq)` (unique name), `exclude_line()`, `orphans(roots, live_sessions) -> Vec<PathBuf>`.

Rules: a pasted image belongs to exactly one session (dir keyed by id); `orphans` returns only paths inside a `pasted` root, never a user-dropped path (SC-005). No state transitions; nothing persisted beyond the files.
