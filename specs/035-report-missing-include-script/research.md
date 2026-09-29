# Research: Report a Missing Environment-Include Script

Every Technical Context item was known from the code, so nothing is marked NEEDS
CLARIFICATION. Each decision below records what was chosen and what was rejected.

## R1. What "readable file" is checked with

**Decision**: `std::fs::metadata(path)` (follows symlinks), then `std::fs::File::open(path)` for
reading, then drop the handle. Results:

- `metadata` fails with `ErrorKind::NotFound` → *not found*
- any other `metadata` error, or `open` error → *not a readable file*
- `metadata` reports anything other than a regular file → *not a readable file*
- otherwise → *present*

**Rationale**: FR-001 defines "readable" "as the operating system reports it for the current
user". Opening the file is the only portable way to ask that. Permission bits do not work on
Windows and miss ACLs on Unix. Opening never executes anything (FR-003). An immediate error other
than absence (a parent directory's permissions, say) becomes *not a readable file*, as the Edge
Cases require, and is never treated as present.

**Alternatives rejected**:

- `Path::exists()`, which 011's resolver uses. It says a directory exists, and it collapses
  every error to `false`, so a permissions error would read as "not found".
- `Path::is_file()`. It cannot tell *not found* from *not readable*, and it cannot see read
  permission.
- Mode bits (`metadata.permissions()`). Not portable, and they ignore ACLs and the effective user.

## R2. `~`, relative and blank paths

**Decision**: classify from the stored string before any probe:

1. Blank (`trim().is_empty()`): no result at all (`None`), matching 011's `snapshot_for`
   short-circuit (FR-011).
2. The string is `~`, or starts with `~/` or `~\` (a string test, the same on every OS, since
   `Path::components` would read `~\env.ps1` as one component on Unix): *not found*, marked `tilde`.
   There is no probe. Resolution would not expand it, and the path as written is relative to
   whichever directory the resolver happens to use, so no probe answer would be meaningful. The
   notice says `~` is not expanded and a full path is needed (Edge Cases).
3. `!Path::is_absolute()`: *relative (not checked)*. There is no probe (Edge Cases, FR-001).
4. Absolute: probe (R1).

**Rationale**: the order is the spec's. `~` comes before "relative" because the spec gives `~` its
own not-found wording, while a `~/…` path is also relative to `std::path`. `Path::is_absolute` is
std's own platform abstraction (`C:\…` and `\\server\share\…` are absolute on Windows, `/…` on
Unix), so the core does not branch on the OS itself (Principle VI).

**Alternatives rejected**:

- Probing `~/…` literally: the answer is about a directory named `~` relative to the client's
  working directory, which says nothing useful.
- Expanding `~` in the check only: the check would then disagree with resolution (spec: the check
  takes the path literally, "as resolution takes it").
- Probing a relative path against `default_resolution_cwd`: the spec forbids it. The answer
  would be true for one directory and shown in every window.

## R3. Where the check runs, and the 2 s bound

**Decision**: core `check_bounded(probe: Arc<dyn ScriptPathProbe + Send + Sync>, path, bound)`
runs the probe on a dedicated `std::thread` and waits on an `mpsc` channel with
`recv_timeout(bound)`. It returns *could not be checked* on timeout. The client calls it inside
`tokio::task::spawn_blocking` from a `Task::perform`, the pattern `shell/links.rs` and
`shell/service_control.rs` already use, and the result comes back as an ordinary message. The bound
is a constant, `SCRIPT_PATH_CHECK_BOUND = 2 s` (FR-006).

**Rationale**: the bound is a rule, so it belongs in tested core code, and a fake probe that
blocks can test it with no iced runtime. A hung `stat` on a stalled NFS mount or unreachable UNC
share cannot be cancelled on any OS. The detached thread is left to finish or hang on its own,
which costs one parked thread per hung check and never blocks the UI or a session. The client runs
on the machine where 011's own client-side resolution runs (the host), so the check and 011's
Settings note describe the same filesystem.

**Alternatives rejected**:

- `tokio::time::timeout` around `spawn_blocking` in the shell. It works, but puts the rule in
  untestable glue.
- A synchronous check in `on_settings_opened`: it would block the UI thread on a hung mount,
  which FR-006 forbids.
- Checking in the daemon over a new protocol message. It adds a protocol change and a
  round-trip for a local `stat` the client can do. The daemon's own reporting gap is #454, out
  of scope.

## R4. When the check runs

**Decision**: three triggers, all in the shell:

- `on_settings_opened`, origin `Opened` (FR-009 "each time Settings is shown")
- `apply_save`, after the write and 011's refresh, origin `Saved` (FR-009 "after every save",
  FR-004)
- the `DaemonMsg::SettingsChanged` arm, only while Settings is open (the draft is `Some`), origin
  `Opened`. Another window's save changes the stored path, and this window's page must not keep
  showing the old path's answer (Principle II).

It never runs from any session-launch or terminal-restart path (FR-006, SC-004).

**Rationale**: these are the named events. A per-frame or timer re-check would do a filesystem
call per redraw and still not be "each time shown". The `SettingsChanged` echo of this window's own
save arrives after the save has closed Settings, so it starts no check. If Settings is open again
by then, the extra check only refreshes the page. R5 keeps the save's notification separate from
which result is displayed.

**Alternatives rejected**:

- A file watcher (`notify` crate). A new dependency and a live watcher for a page that is
  mostly closed. FR-009 only asks for "each time shown".
- Checking on every draft edit: 011's contract gives the path field no validation while typing
  (Clarifications).

## R5. Stale results and the notification

**Decision**: the settings feature state holds `script_check: ScriptCheck`, which is either
`Idle`, `Pending { seq }` or `Done(CheckedScriptPath)`. It also holds a monotonically increasing
`script_check_seq: u64` (the latest check started) and `script_check_save_seq: Option<u64>` (the
latest check a save started). The reducer applies `Msg::ScriptPathChecked { seq, origin, result }`
in two independent steps:

1. **Display**: the result replaces `script_check` only when `seq == script_check_seq`. An older
   result is dropped.
2. **Notification**: when `origin == Saved`, `Some(seq) == script_check_save_seq`, and the state
   is *not found* or *not a readable file*, `update` returns the FR-004 notification as
   `features::notifications::info(..)`, an `Outcome::NotificationRaised`. That is the T067a rule:
   a feature reducer never writes `state.notifications` itself. It then clears
   `script_check_save_seq`.

**Rationale**: the reducer is render-free and tested (Constitution I), and `features::settings`
already owns this state (028 contract S1). The notification rule stays out of the shell, and it reaches the queue as an outcome, the way
`features/notifications.rs` requires of a feature reducer (`feature_write_isolation.rs`). The two
steps are separate so that a later `Opened` check cannot swallow a save's notification. That later
check might be the user reopening Settings at once, or another window's `SettingsChanged` while
Settings is open. Each save notifies at most once, and only the newest save notifies, so two quick
saves do not post two notices for a path that changed in between.

**Alternatives rejected**:

- Keying results by path alone: two checks of the same path at different moments (file
  created in between) could land out of order and show the older answer.
- Posting the notification from the shell: it would put a business rule in glue.

## R6. Notification level and wording

**Decision**: `NoticeLevel::Info`. Wording: `The environment-include script was
not found: <path>` or `… is not a readable file: <path>`. For a `~` path, add `(~ is not expanded;
use a full path)`.

There is no "Settings saved." prefix. If the write itself failed, `apply_save` has already
posted "Couldn't save your settings", and the path notice must not contradict it.

**Rationale**: `NoticeLevel::Error` means "an action the user asked for could not be completed"
(`features/notifications.rs`). The save did complete (Clarifications: "Save anyway"). The words
name the path and the problem (FR-004).

**Alternatives rejected**:

- Error level: misstates the save as failed.
- A new warning level: the notification surface has two levels by design (FR-032c of 027). Adding
  one is out of scope.

## R7. How the page words it, and how it agrees with 011's note (FR-002, FR-005, FR-014)

**Decision**: a pure `script_path_notice(check, last_outcome) -> Vec<NoticeLine>` in
`features/settings.rs`, beside `missing_cli_notice`, computes every line on the page below the
timeout field. That covers the path indication and 011's failure note (the page's current `failure()`
moves into it). Rules:

- 011's `MissingScript` caution now reads `Script not found: <path>`, the same words as the path
  indication.
- When the path indication already says *not found* for that path, 011's `MissingScript` line
  is not repeated. One caution states it, followed by the on/off note.
- With the feature on, `MissingScript` and a *present* check replace both with FR-014's caution
  and note.
- *Not a readable file*, *relative* and *could not be checked* show their line first, then 011's
  failure note unchanged (Edge Cases: "both notes").

The full table is in contracts/settings-indication.md.

**Rationale**: FR-005 requires the wording to agree and the report not to vanish when the feature
is switched off. One function computing both makes disagreement a test failure rather than a
review finding, and it takes the rendering's only branching out of the GUI glue (Constitution I).

**Alternatives rejected**:

- Two independent blocks (new indication + 011's untouched "Script not found"): the page would
  say "not found" twice with the feature on, in two wordings.
- Replacing 011's note entirely: FR-005 says it "MUST remain". It still carries the non-zero-exit
  and timeout diagnostics.

## R8. Which "enabled" the notice describes

**Decision**: the stored flag at the time the check started. It is captured into the check
(`CheckedScriptPath.enabled`), not read from the draft checkbox.

**Rationale**: the indication is about the stored configuration (FR-001 "stored script path",
FR-002 "whether the feature is currently on or off"). The draft may be mid-edit. Any save or
`SettingsChanged` starts a new check with the new flag.

**Alternatives rejected**: the draft checkbox. Ticking it without saving would change what the page
claims sessions get.

## R9. The capability

**Decision**: `trait ScriptPathProbe { fn probe(&self, path: &Path) -> ProbeAnswer }` in core, with
`StdScriptPathProbe` (R1) and `FakeScriptPathProbe` (a scripted answer or a block-until-dropped
gate, and a call count). Held as `Arc<dyn ScriptPathProbe + Send + Sync>` in
`shell::capabilities::Capabilities`, constructed only in `Capabilities::real()`. There is a
`#[cfg(test)] with_script_path_probe(...)` narrowing like `with_link_opener`.

**Rationale**: 021's rule that concrete implementations are chosen in exactly one place, enforced
by `tests/no_concrete_implementations.rs` and `tests/service_capability_fakes.rs`. Those guards
only see ports listed in `tests/inventory/mod.rs` `PORTS`, so the port is registered there and its
fake is added to the guard's `known` list. The fake's call count is what proves "a launch does no
check" (SC-004), in the same way `FakeEnvIncludeResolver::calls` proves "disabled spawns nothing".

**Alternatives rejected**: calling `std::fs` directly from the shell. Untestable, and it breaks the
single-assembly-point guard.
