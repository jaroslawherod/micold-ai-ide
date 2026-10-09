# Research: 487 drop and paste paths

Code facts were located by search. Two spikes open tasks.md, before their dependent tests: (S1) confirm `TerminalBytes` reaches the PTY of a non-focused pane, or add a pane-addressed variant; (S2) confirm pane rectangles are obtainable for hit-testing.
## R1 Drop events
- **Decision**: subscribe to iced `window::Event::FileDropped(PathBuf)` (one event per file, same frame batch) next to `listen_with(window_focus_message)` (`shell/subscriptions.rs:149`); coalesce events arriving in one update turn into one `FilesDropped(Vec<PathBuf>)`. Target = pane under the pointer: **new** app-level state `pointer: Option<Point>` fed by a `listen_with` `CursorMoved` subscription (no window-level pointer state exists today; only per-widget `CursorMoved` handlers), plus the terminal area's pane rectangles published by the split layout (`split_view.rs`) for hit-testing.
- **Why**: no drop handling exists; iced gives no cursor in the event, so the pointer position is the only way to satisfy FR-005.
- **Rejected**: dropping onto the focused pane (violates FR-005); a per-widget drop (iced has no widget-level file drop).
- **Risk**: coalescing order = event order; verified by a client test. If pointer tracking is stale on some OS, fall back to the pane under the window centre and note it in the notice-free path (task spike).

## R2 Quoting
- **Decision**: `ShellKind {Posix, Bash, Fish, PowerShell, Cmd}` (Bash covers bash and zsh) derived from the terminal's shell command (`terminal.rs:62` `default_shell_command`, basename match; unknown → Posix on unix, Cmd on Windows). A sandboxed session is always Posix/Bash (the container shell), whatever the host. Posix: single quotes, `'` → `'\''`. Fish: single quotes, `\\` and `\'` escaped. PowerShell: single quotes, `'` doubled. Cmd: double quotes; `"` and `%`/`^`/`!`-class and newline are not representable inside → `Unrepresentable` (refused with the file named). Non-UTF-8 names: Posix keeps bytes (unix `OsStrExt`) via `$'\xNN'` fallback only for Bash, otherwise `Unrepresentable`; Windows lone surrogates → `Unrepresentable`.
- **Why**: spec edge cases say quote natively or refuse, never garble or translate.
- **Rejected**: `shlex`/`shell-escape` crates (POSIX only; no PowerShell/cmd/fish); backslash-escaping (breaks on newline and non-ASCII).

## R3 Clipboard image
- **Decision**: `arboard` `Clipboard::get_image()` (RGBA) on a blocking task, `png` encode, only when `iced::clipboard::read()` returned no text and the displayed terminal is an AI session. Enable arboard's `wayland-data-control`.
- **Why**: only maintained cross-platform image clipboard read; keeps text-wins (spec clarification) because text is read first.
- **Rejected**: external tools per OS; extending iced (out of scope); reading images from `text/uri-list` (that is the drop path).

## R4 Where images go
- **Decision**: `<worktree>/.micold-pasted/<session-id>/<unix-nanos>-<seq>.png`; fallback (no worktree = Default session, not writable, read-only/network FS per a probe write) `<data_dir>/pasted/<session-id>/`. Sandboxed fallback uses `MountSet.state` host dir (`sandbox/mod.rs:488`) because it is already a container mount (`shared_locations`, :749), so nothing needs remounting.
- **Why**: the worktree is already visible to a sandbox through its project mount and to the agent; a per-session dir makes cleanup one `remove_dir_all`.
- **Rejected**: system temp (not mounted; cleanup by OS only); adding a new mount (needs container restart, `mounts_changed` flow).

## R5 Hiding images from VCS (FR-008)
- **Decision**: append `/.micold-pasted/` once to the repository's `info/exclude`, found with `git rev-parse --git-path info/exclude` run in the worktree (worktrees share the common dir's file). Idempotent; failure to append is non-fatal and falls back to the temp dir.
- **Why**: no tracked file changes (assumption in spec). Note: nothing in the workspace writes `info/exclude` today (confirmed by search).
- **Also**: write `*` into `.micold-pasted/.gitignore` when the dir is created, so status stays clean even if the exclude append fails.
- **Rejected**: editing the project's tracked `.gitignore`.

## R6 Sandbox translate and refuse
- **Decision**: `plan_insertion` canonicalises each path (symlinks resolve, missing file: canonicalise the parent and re-join), checks it lies under a `MountSet.projects[].host`, then `pathmap::map_for(host, windows_host)`. Outside → `Refusal::OutsideProjects{path}`. Note a project registered after the container started is not mounted (`daemon_sync.rs:649`); `plan_insertion` takes the mounted list so such a path is refused with the reason "not mounted yet".
- **Rejected**: inserting host paths and letting the agent fail; guessing a `/mnt/host` path.

## R7 Insertion bytes and cleanup
- **Decision**: build text `p1 p2` (no leading or trailing space; existing input untouched, FR-004), wrap with `keymap::paste_bytes(text, bracketed)` into the existing `TerminalBytes` message (honours the Running write-gate).
- Cleanup: daemon `delete_session` (`state.rs:3295`) removes `<worktree>/.micold-pasted/<id>` and `<data>/pasted/<id>`; at start, remove any `pasted/<id>` whose id is not a live session and sweep known worktrees' `.micold-pasted/*`.
- **Rejected**: client-side cleanup (the daemon outlives clients and owns session lifetime).
