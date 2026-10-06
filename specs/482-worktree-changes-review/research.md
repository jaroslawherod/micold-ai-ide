# Research: Review a Worktree's Changes and Send Comments to Its Session

Feature 482. Each entry: decision, rationale, alternatives considered. Paths are this workspace's;
"new" marks what this feature adds.

## R1 — Who computes diffs: git, not a diff crate

**Decision**: Every list and diff comes from the user's `git` binary, run through the same
`no_window`/`local_only` wrappers `crates/micold-core/src/git.rs` (`GitCli`) already uses, and its
text output is parsed by pure functions in the new `crates/micold-core/src/review/` module. The
reads are:

| Purpose | Command (all with `-c core.quotepath=false --no-pager`, `--no-ext-diff --no-textconv --no-color`) |
|---|---|
| Default branch | `git symbolic-ref --quiet refs/remotes/origin/HEAD`, else `git rev-parse --verify --quiet refs/heads/main`, else `…/master` |
| Base commit | `git merge-base HEAD <default branch>` |
| Committed list | `git diff -z -M --numstat <base> HEAD` and `git diff -z -M --name-status --summary <base> HEAD` |
| Uncommitted list (tracked) | the same two against `HEAD` and the working tree (`git diff … HEAD`) |
| Both | the same two against `<base>` and the working tree (`git diff … <base>`) |
| Untracked | `git ls-files -z --others --exclude-standard` |
| One file's diff | `git diff -M -U3 <from> [HEAD] -- <path>` (`<old path>` added for a rename) |
| One untracked file | read from disk; the core synthesises an all-added diff |

**Rationale**: git already defines "committed", "uncommitted", renames, mode changes, binary
detection and the merge base exactly as the spec's Terms do; parsing its output keeps one source of
truth and adds no crate. "Both" as one diff from the base to the working tree gives FR-004's
"listed once with the combined change" for free. `GitCli` already shells out (feature 002 R7), so
this is the established I/O boundary; the parsers are pure and unit-tested on captured output, and
the commands are integration-tested against real temporary repositories (as
`crates/micold-core/tests/git_containment.rs` does).

**Alternatives**: `similar` 3.2 or `imara-diff` 0.2 over file contents read via `git show`: a second
diff algorithm that disagrees with `git diff` on hunks and renames, plus a new dependency, and
renames and modes would still need git. libgit2/gitoxide: rejected by feature 002 R7 for the whole
codebase; nothing here changes that.

## R2 — Where diffs are computed: the client, off the UI thread

**Decision**: The window that shows the Changes view runs the git reads itself, in
`tokio::task::spawn_blocking` behind `Task::perform`, as the pull request status reading does
(`crates/micold-client/src/shell/pr_status.rs`). A sequence number on each read drops a stale answer
(the `Phase::Reading { seq, again }` pattern of `features/pr_status.rs`).

**Rationale**: The diff is a per-window view of files on the host, which the client can read in
every placement (the sandboxed service sees only registered projects; the client runs on the host
regardless). Keeping it out of the daemon means no wire traffic for diff bodies (50,000-line diffs),
no per-window view state in the service, and nothing new for the service to throttle. FR-009's "no
blocking" holds because all git work and parsing happen in the blocking pool.

**Alternatives**: compute in the daemon and push results: large payloads through the postcard
channel, and the daemon would hold UI state (selected file, toggles) per window. Compute
synchronously in `update`: blocks redraws on any non-trivial repo (SC-003).

## R3 — Who owns comments: the daemon (single writer, all windows)

**Decision**: Review comments live in the daemon, persisted per project in a new file
`reviews/<project id>.json` beside the existing per-project state directory (`projects/`), written
atomically (temp + rename) by the daemon's catalog, the single writer (`catalog.rs`, invariant C1/C2).
Clients edit them through new `ClientMsg::ReviewEdit` / `ClientMsg::ReviewSend` messages and receive
`DaemonMsg::ReviewChanged` pushes for their attached project, on attach and after every change.

**Rationale**: FR-018 and the Isolation edge case need one authority: two windows see the same
comments, and a send in progress blocks the action in every window with each comment delivered at
most once. The daemon already is that authority for sessions and settings and already pushes per
project to attached clients (`state.rs`). FR-020's "survive a restart, discarded with the worktree"
is the daemon's existing persistence and worktree-delete path. A separate file keeps a large comment
list from slowing every catalog save and isolates a corrupt file to reviews only (store.rs BUG-001).

**Alternatives**: client-side file per project: two windows would race on writes and could both
send the same comments (violates FR-018). Embedding in the per-project state file: couples a
high-churn list to the session catalog's writes.

## R4 — How a prompt reaches a session

**Decision**: Reuse feature 034's delivery path. Move `deliver_first_prompt` and the trust check out
of `crates/micold-daemon/src/mcp/tools.rs` into `crates/micold-daemon/src/ops.rs` as shared
functions (`ops::type_submission`, `ops::create_session_with_prompt`) that both the tool server and
the new `crates/micold-daemon/src/review.rs` call. A running target receives
`micold_core::mcp::submission::encode_submission(prompt, bracketed)` on its primary PTY; with none
running, the daemon creates a session in that entry with `state.default_ai_cli()`, starts it
(`ops::start_session`, `LaunchMode::Fresh`), waits for readiness (`wait_ready_for_input`, bounded by
`first_prompt_bound`) and types the prompt.

**Rationale**: The spec's Assumption ("reuses the same path the app already uses to send user input
to a session"); one implementation of readiness, trust and bracketed paste.

**Refinements specific to review prompts**:

- **Bracketed paste required.** The review prompt is multi-line. Without bracketed paste each line
  break would submit separately, which breaks "exactly one prompt" (US2 s3). If the target terminal
  has not enabled `BRACKETED_PASTE`, nothing is typed and the send fails with an error; comments
  stay pending (FR-017).
- **Trust question.** As in 034, if `cli_would_ask_trust` holds nothing is typed and the send fails.
- **Delivered** means `write_input` returned `Ok` for the whole submission. That is the point where
  comments become sent (FR-017).

**Alternatives**: typing through the client's `SessionInput` path: the client would have to wait
for readiness of a session it just asked to create, and two windows could both type (FR-018).

## R5 — Which session gets the prompt

**Decision** (spec Clarifications): among the entry's sessions that are running (lifecycle
`Running`, primary PTY alive, not archived), the one most recently active; if none runs, start a new
one with the project's default AI CLI; never resume an ended session. "Most recently active" is a
new daemon-side, in-memory `last_active: Uptime` per session, set when the session starts, when a
client sends it input (`SessionInput`), and when its activity signal changes. The choice is the pure
`micold_core::review::target::pick_target`, ties broken by the larger session id order for
determinism.

**Rationale**: input and activity changes are the two things that mean "the user or the agent is
using this session"; both pass through the daemon already. In-memory is enough: after a daemon
restart no session is running until started, which sets it.

**Alternatives**: the session a window is viewing (`set_viewed`): per window, absent when the user
is in the Changes view, and two windows can disagree. Persisted last-input time: no requirement
needs it across restarts.

## R6 — At most once, and locking during a send

**Decision**: Per entry, the daemon holds `sending: Option<SendingSnapshot>` under the state mutex.
`ReviewSend` refuses with `ErrorKind::Busy` while it is set; otherwise it records the ids of the
pending comments, builds the prompt from exactly those, sets `sending`, and broadcasts. On success
only those ids become `Sent`; comments added during the send stay pending. While a comment is in a
send it cannot be edited or deleted (`ReviewEdit` refused, the client disables those actions).
`sending` is cleared on success and failure and is never persisted (a daemon restart mid-send
leaves comments pending, which is FR-017's safe side).

**Rationale**: FR-018 (unavailable in every window, at most once) and FR-017 (sent only once
delivered). Locking avoids a prompt that carried text A being recorded as sent with text B.

**Alternatives**: optimistic "mark sent, roll back on failure": a crash between the two leaves
comments sent that were never delivered.

## R7 — Base branch resolution

**Decision** (spec Clarifications, FR-003): the default branch is `origin/HEAD`'s target, else
local `main`, else local `master`; the base is `git merge-base HEAD <it>`. Nothing is persisted.
The view's header names it ("Compared with main at 1a2b3c4"). No default branch, or no merge base,
gives `Base::Unavailable(reason)`: committed changes are hidden with that reason (Edge Case "No
base found"). The Default entry never resolves a base: its kind is uncommitted-only by type
(`ReviewScope::RootUncommitted`), so the committed toggle cannot be represented for it
(Principle V).

**Alternatives**: the upstream of the worktree's branch: often unset for fresh worktrees and
points at the branch itself once pushed. Recording the branch the worktree was created from:
rejected by the clarification (nothing persisted, and attached/included worktrees have no record).

## R8 — Size limits and binary/undisplayable classification

**Decision**: classification is pure and happens before a diff body is parsed:

- binary: `--numstat` reports `-\t-`, or the diff body says `Binary files … differ`;
- undisplayable: either version is not valid UTF-8 (checked on the diff text; an untracked file is
  checked on its bytes, and a NUL byte counts as binary, as git's own heuristic does);
- large: added + removed lines from `--numstat` above 5,000, or either version above 2 MB
  (`git cat-file -s <base>:<path>` and the working file's metadata length, read in the same
  background task). A large file's diff is not requested until the user takes "Show diff"; the
  choice is per file and lasts while the view is open.

Constants: `review::limits::{MAX_CHANGED_LINES = 5_000, MAX_VERSION_BYTES = 2 * 1024 * 1024,
MAX_QUOTED_LINES = 50}`.

## R9 — File-change detection

**Decision**: while a Changes view is open, the window runs an iced `Subscription` (new
`crates/micold-client/src/shell/changes_watch.rs`) around a `notify::RecommendedWatcher` (crate
`notify` 8.2, already pinned in the workspace and used by `micold-daemon/src/event_log.rs`; new to
`micold-client`'s manifest). It watches the entry directory recursively and the git metadata that
changes on stage and commit (the worktree's own git dir: `HEAD`, `index`; the common dir's `refs/`
and `packed-refs`). Events are debounced 300 ms, then filtered by the pure
`micold_core::review::watch::relevant_paths` (drops `.git/objects`, `.git/logs`, lock files, and
for the Default entry everything under `.claude/worktrees/`), and the survivors by one
`git check-ignore -z --stdin` call to skip ignored paths. Any survivor triggers a refresh. A refresh in flight with another requested runs once more
after it (coalesced), so a build writing thousands of files costs at most two list reads.

**Rationale**: FR-010/SC-004 (2 s) with a 300 ms debounce plus one list read leaves ample margin.
notify's recommended watcher is inotify / FSEvents / ReadDirectoryChangesW: Principle VI's parity
behind one abstraction. Watching runs only while the view is open (spec Assumption).

**Alternatives**: polling `git status` every second: meets 2 s but costs a git run per second per
open view forever, and is not what the spec assumes. `notify-debouncer-mini` 0.7: a second crate for
a 20-line debounce the subscription already needs to own (coalescing with in-flight reads).

## R10 — Syntax colouring

**Decision**: enable iced's own `highlighter` feature (`iced_highlighter` 0.14, the iced project's
crate; it brings `two-face` 0.4 with syntect's pure-Rust `fancy-regex` engine, no C library). The
background load task highlights each side's lines in order (old side, new side) with
`iced::highlighter::Highlighter` per file extension (FR-007, spec Assumption) and stores per-line
spans as `(Range<usize>, Rgb)` in the client's render-free state, so rendering only paints spans.
Themes: `InspiredGitHub` in light, `Base16Ocean` in dark; an unknown extension gets no spans (plain
text). Spans are capped to the first 2,000 bytes of a line so a minified line cannot stall the task.

**Rationale**: Principle V (iced only) is met by the iced project's own highlighter; no other GUI
crate. Highlighting off the UI thread keeps SC-003.

**Licences**: iced_highlighter MIT, two-face MIT/Apache-2.0, syntect MIT, fancy-regex MIT.

**Alternatives**: syntect directly: the same engine without iced's `Highlight → Format` mapping.
tree-sitter: C grammars per language and a build-time C toolchain on all three platforms. No
colouring: fails FR-007.

## R11 — Rendering large lists and diffs without freezing

**Decision**: a new shared component `VirtualRows` (`crates/micold-client/src/ui/material/virtual_rows.rs`)
renders only the rows inside the viewport plus an overscan, inside the library `Scrollable`, with
fixed row height and spacer heights above and below; its windowing arithmetic is the pure
`visible_range(offset, viewport, row_height, len, overscan)`, unit-tested. Both the file list
(2,000 files, SC-003) and `DiffView` (50,000 lines) use it. Inline comment cards and the composer
are rows of a known, measured height (`comment_row_height(lines)`), so the arithmetic stays exact.

**Rationale**: per-frame work is bounded by the viewport, not the file. iced 0.14 has no built-in
virtual list.

**Alternatives**: `lazy` over the whole column: still lays out every row on the first frame and on
every change.

## R12 — Layout preference persistence

**Decision**: a service-owned setting `diff_layout: DiffLayout { Unified, SideBySide }` in
`micold_core::settings::Settings` (serde default `Unified`), changed through a new
`diff_layout: Option<DiffLayout>` field of `ClientMsg::SettingsSet` and pushed back by
`SettingsChanged`, exactly as `pr_status_enabled` (feature 040) is.

**Rationale**: FR-006 asks it to persist across files and restarts; settings are the existing
persisted, cross-window place for a user preference.

**Alternatives**: client-side `persist.rs` window state: per window, so two windows would disagree.

## R13 — Prompt format

**Decision**: the format in [contracts/review-prompt.md](contracts/review-prompt.md), built by the
pure `micold_core::review::prompt::build`. Grouped by file (paths sorted bytewise), comments in line
order: start line, then side `New` before `Old`, then creation time (a removed line's position
among new-side lines is not stable across refreshes, so it is not used). `\n` line breaks only,
paths with `/`, fences that out-length any backtick run in the quote. Over 50 quoted lines: first
line, an elision line naming how many were left out, last line.

**Rationale**: FR-015, SC-002, SC-006 (byte-identical everywhere: no platform-dependent path
separators, line endings or locale formatting).

## R14 — Outdated comments

**Decision**: outdated is derived, not stored: after every diff load, the pure
`ReviewComment::is_outdated(&SideLines)` compares the recorded quote with the lines now at the
recorded range on the recorded side (new side: the working file or the committed version under the
current toggles; old side: the base version). A file no longer in the list makes its comments
outdated. The flag is display-only; the prompt notes it (FR-013) because the client sends the ids it
judged outdated with `ReviewSend`.

**Rationale**: storing it would need every window to agree on when to write it; deriving gives the
same answer in every window from the same files.

**Alternatives**: re-anchoring comments to moved lines: out of scope (spec says the comment keeps
its quote and range).

## R15 — Where the view opens, and the text area

**Decision**: "Review changes" in the worktree row's right-click menu and the Default row's menu
(`ui/sidebar.rs`, features/sidebar.rs messages), opening the Changes view in the main area beside
the sidebar (`ui/mod.rs`, in place of the terminal pane), closed by its own close action or by
selecting a session. The comment composer needs a multi-line field the library lacks: a new shared
`TextArea` (`ui/material/text_area.rs`) over iced's `text_editor`, styled like `FilledField`, with a
builder API.

**Alternatives**: a separate window: the app has one window per project view, and a second would
duplicate the sidebar's context. A modal dialog for the composer: hides the code being commented.
