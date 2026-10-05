# Contract: client/daemon messages

- `ClientMsg::AttachDiscover { req, project }` returns `DaemonMsg::AttachReport { req, report }` (the `DiscoveryReport` of data-model.md). Blocking work runs in `spawn_blocking` off the lock, in the style of `refresh_worktrees_off_runtime`. Read-only.
- `ClientMsg::AttachApply { req, project, targets: Vec<AttachItem> }` where `AttachItem` is `Worktree { dir_name }` or `Session { id }`; answers `OperationOk` with `results: Vec<{ item, AttachOutcome }>`. Under one catalog lock: validate each target against the live worktree cache (so a stale client list cannot attach a non-worktree), claim each worktree, adopt each session as an idle `Session::restored` at its location (never started: FR-008, `AiCli` mode), persist once, broadcast `CatalogChanged` once. A `Session` item whose worktree is unattached attaches that worktree first.
- Resume of a listed session is `Session` attach followed by the existing `SessionStart`; a second start while `Starting/Running/Restarting` is refused with "already running" (FR-016).
- Both messages are added to `protocol_roundtrip.rs` and the pinned schema hash.
