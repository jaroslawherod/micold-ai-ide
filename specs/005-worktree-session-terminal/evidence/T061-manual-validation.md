# 005 T061 — the quickstart validation scenarios V1–V10, run for the first time

**Date**: 2026-08-21
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own `micold-ai-ide` + `micold-daemon`, built in one invocation and copied
out of the shared target directory **inside** the build lock (`~/vp83/bin`, 2026-08-20 21:03). The
newest commit touching `crates/` is `d28a0c6` (2026-08-19), so the pinned pair is this branch.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp83`, scratch `XDG_DATA_HOME`/`XDG_CONFIG_HOME`. Everything
started here was stopped by PID afterwards.
**`claude`**: v2.1.238 — above the v2.1.210 floor the quickstart's prerequisites name, so
`--session-id` is supported and the discovery fallback is not what was exercised.
**Fixture**: `/home/jaro/.aaa-vp83d/w5repo`, a throwaway repo at a **real** path (a symlinked
project path misclassifies every worktree — [002 BUG-002](../../002-project-workspace-management/bugs/BUG-002.md)),
two commits, and the three worktrees the scenarios below create.

## Result

| # | Scenario | Result |
|---|----------|--------|
| V1 | Open a project, git-only | **PASS** — a git repo opens and lists its worktrees; a non-git directory is refused with a message and nothing opens. SC-001 measured at **0.26–0.36 s** against a 3 s budget. |
| V2 | Browse worktrees & sessions | **PASS** — see below |
| V3 | Create a worktree via the form | **PASS** but for the forced-git-failure clause — see below |
| V4 | Start a session & embedded terminal | **PASS** — see below |
| V5 | Concurrency & switching | **PASS** — `v5-session-swap.png` |
| V6 | Close a session | **PASS** — see below |
| V7 | Crash auto-restart with guard | **restart PASS, guard FAIL** → [BUG-004](../bugs/BUG-004.md) |
| V8 | Persistence, restart & resume | **PASS** — `v8-resume-after-restart.png` |
| V9 | Invalid/missing worktrees | **PASS** — `v9-missing-vs-ok.png` |
| V10 | Reusable components & theming | **PASS on Linux** — `v10-tree-both-themes.png` |

## V2 — browse worktrees & sessions

The sidebar is a tree: worktree rows at the top level, session chips beneath. Expanding and
collapsing works from the row's leading folder glyph, independently per row (`v10-tree-both-themes.png`
shows `Default` and one `Login page` collapsed while another is expanded). A collapsed worktree
hides its session chips — worth knowing, because a still-running session on a collapsed row looks
for a moment like a session that vanished. Its process is untouched; expanding brings the chip back.

An empty project shows "No worktrees yet. Add one to get started." with the add affordance beside
it — recorded under [008 T056](../../008-worktree-sidebar-refinement/evidence/T056-delete-with-running-session.md),
which reached that state by deleting the last worktree.

Both themes: `v10-tree-both-themes.png`, dark left, light right, identical geometry.

## V3 — create a worktree via the form

Every clause ran through the app's own New worktree dialog, and each result was confirmed against
`git -C <repo> worktree list` / `git branch`, not just against the sidebar:

| Input | Result |
|---|---|
| `feat` + `ABC-123` + `Login page` | branch `feat/abc-123_login-page`, dir `.claude/worktrees/feat-abc-123_login-page`, row "Login page" with an `ABC-123` tag |
| `feat` + `#123` + `Login page` | dir `.claude/worktrees/feat-123_login-page`, tagged `#123` — BUG-003's numeric reference survives |
| delete, then re-create from the **existing branch** picker | same directory, same `ABC-123` tag — BUG-003's branch-carried boundary round-trips |
| empty ticket | `chore/cleanup`, segment omitted |
| duplicate dir/branch | blocked with a clear message, and `git worktree list` unchanged — nothing was half-created (FR-009) |

The derived `dir`/`branch` preview (FR-008a) updates as the fields are typed.

SC-002 (create within 30 s) measured at **0.51 s**.

**Not run**: the forced-git-failure rollback (FR-006b). Provoking a genuine git failure at the right
step is not reachable from the UI; it is covered by `FakeGit`'s primed-failure test.

One thing that looks like a defect and is not: after picking a branch in the Existing-branch tab the
search field renders empty. [021 FR-014a](../../021-branch-typeahead-search/spec.md) requires exactly
that — the field holds only search text, never the selected branch's name, which lives in the preview.

## V4 — start a session & embedded terminal

- The session's process is `claude --session-id <uuid>` with **cwd = the worktree**, read from
  `/proc/<pid>/cmdline` and `/proc/<pid>/cwd` — not inferred from a fake.
- The uuid is the app's own: `/status` inside the running session reported
  `Session ID: 1a24f871-00a5-4e53-bad3-6a1326eaa4a9`, byte-identical to the record in
  `projects/eb1c74d0e085f7f7.json`. The CLI agrees the app told it which session it is.
- Typing reaches the process and output renders: "Reply with exactly: PONG" → `PONG`
  (`v8-resume-after-restart.png`).
- **Session start is disabled on an unavailable worktree** — see V9.

**SC-004 — PASS with a large margin.** Frame-by-frame from the click that starts a session
(`v4-sc004-session-start.png`, red = the last frame without it, blue = the first frame with it):
at **+1.14 s** the pane is still the project overview; at the next capture, **≈+1.25 s**, the
`claude` banner, the worktree cwd and the prompt are all rendered and the status reads `running`.
Against a 5 s budget. (An earlier attempt sampled only the first 1.24 s and caught nothing — the
budget is fine, the capture span was too short.)

## V6 — close a session

Closing a session from its chip's `×`:

| Claim | Observed |
|---|---|
| the process is terminated | the session's `claude` PID is gone within ~2 s; no orphan |
| it leaves the sidebar | the chip is gone; its worktree row stays, now with no session |
| the record | **kept, flagged `"archived": true`** |
| the durable marker | `~/.claude/projects/<slug>/<session-id>.archived` written (FR-020c) |
| the rest | the other two sessions and their processes untouched |

The quickstart's V6 line — "session + persisted record removed" — is the **pre-BUG-003** wording;
FR-015a was rewritten on 2026-07-23 to keep the record as an invisible tombstone precisely so
reconciliation cannot resurrect a closed session. The behaviour above matches the amended FR, not
the stale line. The quickstart is corrected in this change.

Close also works **during** a restart loop: it was the only thing that stopped V7's.

## V7 — crash auto-restart, and the guard that never fires

**FR-022 passes.** `kill -9` on a session's `claude` is followed immediately by an automatic
relaunch, no user action.

**FR-022a fails.** Each relaunch failed the same way and a new PID appeared about once a second,
for over four minutes, without ever reaching `Failed`. The status readout oscillated between
`restarting…` and `running` (`v46-restart-loop.png`). The cause is that the crash-loop counter is
reset by liveness, not by time: the daemon's 250 ms supervision tick marks any still-alive session
`Running`, which zeroes `attempts`. Filed as [BUG-004](../bugs/BUG-004.md) with the mechanism, the
PID trace, and why the core FSM's own tests pass.

`v46-restart-loop.png` also shows a smaller thing: while restarting, the session chip renders with
**no label at all**. The name comes back on a successful resume (V8's chip reads "Reply with PONG",
FR-011a's provider-supplied title).

## V8 — persistence, restart & resume

| Claim | Observed |
|---|---|
| close the app → processes stop | both client and daemon stopped by PID; no `claude` survived |
| records persist | the session stayed in `projects/eb1c74d0e085f7f7.json`, not archived |
| reopen → the session is back in the sidebar | yes, chip titled **"Reply with PONG"** from the provider |
| resumed via `--resume` | `claude --resume 1a24f871-… ` with cwd = the worktree, read from `/proc` |
| no scrollback replay | exactly one banner and one copy of the conversation — the resumed CLI's own redraw, not the app replaying its buffer (`v8-resume-after-restart.png`) |

**One harness fact worth recording, because it looked like a V8 failure for a whole cycle.** A
session with no recorded `claude` conversation is *supposed* to disappear across a restart — FR-020
(005/BUG-001): "empty sessions MUST NOT be persisted… pruned when loading". This harness's `claude`
inherits `CLAUDE_CODE_CHILD_SESSION` from the agent running it, so transcript saving is **off** and
every session is an empty session. Relaunching the app with
`CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1` and sending one real message is what makes V8 testable at
all; without it the correct behaviour and the failure look identical.

Also confirmed in passing: the app derives the provider's per-directory slug exactly as `claude`
does (`feat-abc-123_login-page` → `…-feat-abc-123-login-page`), so the archived markers and the
transcripts land in the same directory. A mismatch there would silently break both reconciliation
and the tombstones.

**Not run**: the project-close half ("switch/close the active project → processes stop, records
persist"). 008 FR-001 amended the switch case — sessions now keep running on switch — and the close
case is the same code path as the app shutdown covered above.

## V9 — invalid/missing worktrees

`rm -rf`'d a worktree directory behind the app's back (`git worktree list` then reports it
`prunable`) and reopened the project:

- the row is **shown, not hidden**, with its name and folder glyph in the error colour and a
  `missing` chip beside the type/ticket chips;
- hovering it offers **only** the delete action — the `+` that starts a session is absent, where an
  adjacent healthy row has both (`v9-missing-vs-ok.png`, red = missing, blue = healthy).

FR-018a asks for start to be disabled; the shipped behaviour is stronger — the affordance is not
rendered at all.

## V10 — reusable components & theming

`v10-tree-both-themes.png` is the same sidebar in dark and light at identical geometry. The
`TreeView` rows, the expand glyphs, the `IconButton` actions (`+`, `×`, trash), the tag chips, the
active-session chip and the `missing` error colouring all resolve correctly in both. The theme is
the cycling `Theme:` overflow item (Auto → Light → Dark), not a submenu.

**Linux only.** "and on all target platforms" is not reachable from here, and CI runs no GUI
walkthrough on any platform.

## What this run did not cover

| Claim | Why |
|---|---|
| V3's forced-git-failure rollback | not provokable from the UI; covered by `FakeGit::fail_next_*` |
| V8's project-close half | superseded by 008 FR-001 for switch; same path as shutdown for close |
| V10's macOS/Windows parity | no such machine here |
| T058 — redraw coalescing ≤1/frame, scrollback cap under flood | lavapipe is a software rasteriser; a frame-pacing number measured on it says nothing about a user's GPU. Recorded as unmeasured, not passed. |

## Harness artifacts (not app defects)

1. **`xdotool click 1` is too fast for several controls.** `mousedown`, dwell ~200 ms, `mouseup`
   works every time. Same family as the artifacts recorded in 014-forget-project and
   014-hide-agent-worktrees.
2. **A collapsed worktree row hides a running session**, which reads as "the session disappeared"
   until the row is expanded. Recorded under V2 above; both PIDs were alive throughout.
3. **Transcript saving is off for any `claude` this harness starts** (see V8). It changes what
   "correct" looks like for persistence, pruning and resume — the single most misleading thing in
   this run.
