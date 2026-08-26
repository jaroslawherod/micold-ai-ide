# 010 T029 — the quickstart's 11 GUI steps, run for the first time

**Date**: 2026-08-21
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own `micold-ai-ide` + `micold-daemon`, built in one invocation and copied
out of the shared target directory **inside** the build lock (`~/vp83/bin`, 2026-08-20 21:03). The
newest commit touching `crates/` is `d28a0c6` (2026-08-19), so the pinned pair is this branch.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp83`, scratch `XDG_DATA_HOME`/`XDG_CONFIG_HOME`,
`CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1` (see step 9). `claude` v2.1.238. Everything started here
was stopped by PID afterwards.
**Fixture**: `/home/jaro/.aaa-vp83d/r10` — a fresh `git init -b main`, one commit (`07c3e24`), **no
worktrees** and no `.claude` directory, at a real (not symlinked) path. The quickstart's step 1 asks
for exactly that.

## Result

| # | Step | Result |
|---|------|--------|
| 1 | Open a project with no worktrees | **PASS** — opened through the app's own picker |
| 2 | Default entry present immediately, visually distinct | **PASS** — `step2-default-entry.png` |
| 3 | Start a session from Default; nothing appears under `.claude/worktrees/` | **PASS** |
| 4 | `pwd` reports the project root | **PASS** — `step4-pwd-project-root.png` |
| 5 | Coexistence with a worktree session; closing one does not affect the other | **PASS** |
| 6 | Two Default sessions, both independently usable (US3 — this is **T024**) | **PASS** — `step5-6-coexistence.png` |
| 7 | Location tooltips (FR-010) | **PASS** — `step7-location-tooltips.png` |
| 8 | Tag filters never hide Default | **PASS** — `step8-filter-never-hides-default.png` |
| 9 | Restart persistence | **PASS** — `step9-restart-resumed.png` |
| 10 | Worktree create/rename/delete unaffected (FR-008) | **PASS** — `step10-default-unaffected-by-worktree-flows.png` |
| 11 | Project root becomes unavailable (**T027**'s step) | **FAIL** → [BUG-001](../bugs/BUG-001.md) — `step11-stuck-starting.png` |
| — | Visual/asset check: Default icon in both themes | **PASS** — `default-icon-both-themes.png` |
| — | Documentation check (Principle VII) | **PASS** |

## Step 2 — the Default entry with zero worktrees

The sidebar shows one `Default` row carrying an expand glyph, a **house** icon and an
always-visible `+`, above "No worktrees yet. Add one to get started." No git or branch
iconography, no tofu boxes. Its `+` tooltip reads **"Start a new session in the project root"**.

## Steps 3 and 4 — the session really is in the root

Pressing `+` started `claude` and the terminal rendered it live. Confirmed at process level rather
than from the frame:

```
633176  /home/jaro/.aaa-vp83d/r10
        claude --session-id 4fe43bcc-… --settings …/hooks/4fe43bcc….json
```

and on disk, `r10` still contained only `.git` and `a.txt` — **no `.claude` directory was created
at all**, so FR-002's "zero git worktree mutations" holds in the strongest form the fixture allows.

Step 4 asks for the same fact from inside the terminal. Switching the session to **Regular
Terminal** from the status bar's mode control (tooltip: "AI CLI — switch to Regular Terminal"):

```
$ pwd; git rev-parse --show-toplevel; git worktree list
/home/jaro/.aaa-vp83d/r10
/home/jaro/.aaa-vp83d/r10
/home/jaro/.aaa-vp83d/r10 07c3e24 [main]
```

One worktree entry, the root itself, and no `.claude/worktrees/...` anywhere.

## Steps 5 and 6 — coexistence, and two Default sessions

A worktree (`feat/side-work`) created through the app's own dialog, with a session started on it.
All three sessions were then listed at once — two under `Default`, one under `Side work` — each
under the right entry, each backed by its own process:

```
633176 claude  …/r10                                   (Default #1)
634849 claude  …/r10                                   (Default #2)
634322 claude  …/r10/.claude/worktrees/feat-side-work  (Side work)
```

Both Default sessions are independently usable: switching to the first and running
`echo SESSION-ONE-ALIVE` produced its output with its own scrollback intact, while the second
showed a live `claude` banner. That is **T024**, in full.

Closing the worktree session removed only its row and only its process (`634322` gone; the other
two untouched) — step 5's closing clause.

The state file agrees, and is the shape the contract asks for: `"worktree_dir": null` for the two
Default sessions, `"worktree_dir": "feat-side-work"` for the third.

## Step 7 — the location tooltips

Hovering `Default` shows **"Project root"**; hovering the worktree row shows
**`.claude/worktrees/feat-side-work`** — the path relative to the project, exactly as FR-010 words
it. Stacked in `step7-location-tooltips.png`.

## Step 8 — the filter can never hide Default

With two tagged worktrees (`feat`, `fix`) and only **`fix`** active, the `feat` worktree is hidden
and **Default remains, with both its sessions**. Tag filters are also never *cleared* by anything
this feature does; activating one and then creating a worktree left the filter in place (the new
`fix` worktree was correctly hidden by the active `feat` filter until it was switched).

## Step 9 — restart persistence, and the trap under it

**PASS, at the second attempt**, and the first attempt is worth recording because it looks exactly
like broken persistence.

Quitting the client **and** the daemon and relaunching restored nothing: the two Default sessions
came back `"archived": true`. That is FR-020 empty-session pruning doing its job — neither session
had a recorded `claude` conversation, because a `claude` spawned from inside another Claude Code
session inherits `CLAUDE_CODE_CHILD_SESSION` and writes no transcript. The same trap is recorded
for 005 V8. The tombstones are visible afterwards as
`~/.claude/projects/-home-jaro--aaa-vp83d-r10/<id>.archived` — 005 BUG-003's durable markers.

Re-run with a session that had actually said something ("Reply with exactly: PONG"), a full
client + daemon restart restored it under the **Default** entry, titled from its own conversation
(**"Pong reply"**), and resuming it replayed that conversation in the pane with the banner still
reporting `~/.aaa-vp83d/r10`. Restored *and* resumable, against the project root.

## Step 10 — the existing worktree flows are untouched

Create, rename and delete were each run with two Default sessions open:

| Flow | Observed |
|---|---|
| create | `fix/timeout-thing` — dialog previews directory and branch, both created as previewed |
| rename | right-click → Rename → "Renamed Alpha"; sidebar label only. `git worktree list` and `git branch --list` byte-identical afterwards — the dialog's own promise ("Changes only the name shown in the sidebar — not the branch or folder") |
| delete | trash → confirmation naming the directory, with "Also delete the branch" checked; worktree, directory and branch all gone |

Throughout, the Default row and both its sessions were unaffected. FR-008 holds.

## Step 11 — the failure

Renaming the project root away while the app runs and then pressing Default's `+` produces a
session row that sits at **`starting…` indefinitely** (observed 80 s) with no error anywhere and no
process ever spawned.

A worktree session whose own directory has been renamed away behaves **identically**. So step 11's
comparison clause — "consistent with how a worktree session behaves" — passes; its substantive
clause — "not a crash or a **silently-stuck session**" — fails, for both.

Root cause is in the shared start path, not in anything this feature added:
`spawn_session_start` (`crates/micold-daemon/src/server.rs:1487`) logs a start failure with
`tracing::warn!` and then replies `OperationOk`/`SessionCreated` anyway. Filed as
[BUG-001](../bugs/BUG-001.md), which also notes that the client-spawned daemon's stderr goes to
`/dev/null`, so even the warning lands nowhere.

The fixture was restored afterwards; `git worktree list` and `git branch --list` are what they were.

## Visual/asset check

The Default entry's house icon renders correctly in **both** themes — no tofu box, in dark or in
light (`default-icon-both-themes.png`; theme cycled from the overflow menu, which also persisted
across the step 9 restart). `cargo test icons_font` is T028's territory and already passing.

## Documentation check (Principle VII gate)

`docs/user-guide/worktrees-and-sessions.md` carries a dedicated section, **"The 'Default' entry:
sessions without a worktree"** (lines 120–148), covering what it is, that it is not a worktree and
has its own house icon, that multiple concurrent sessions are supported, that the shared checkout
makes their working-tree changes mutually visible, that tag filters never hide it, and the location
tooltip. `docs/user-guide/icons.md` lists the `Project root` icon against the Default entry. PASS.

## What this run did not cover

- **macOS and Windows.** Linux only; nothing here is platform-specific and T028 confirmed no
  platform-gated code was added, but parity is unobserved.
- **Unmounting** the project root, as opposed to renaming it. A rename is what step 11 suggests
  first and it was enough to produce the failure; an unmount would exercise the same path.
- The **stuck session's recovery** — whether closing the stuck row and restoring the directory
  returns the app to a good state. Out of scope for the step, and BUG-001 is about the missing
  signal, not about cleanup.

## Harness artifacts (not app defects)

1. **The worktree context menu is anchored, not at the cursor.** It opens at a fixed
   `SIDEBAR_MENU_ANCHOR = (24, 96)` (`ui/mod.rs:74`), near the top of the sidebar — nowhere near
   the row that was right-clicked. Two right-clicks were recorded as "no menu appeared" purely
   because the screenshot was cropped to the row. `WorktreeMenuToggled` is a *toggle*, so those two
   presses also cancelled each other.
2. **Rows collapse on their own.** A row that is not the selected one renders collapsed, hiding its
   session chips; a session appearing to vanish is usually its row folding. Same artifact recorded
   in 005's evidence.
3. **Switching a session's mode keeps both processes.** Toggling AI CLI → Regular Terminal leaves
   the `claude` process alive alongside the new shell, and toggling back re-attaches to *that same*
   process (no new PID, scrollback intact) rather than spawning another. It looks like a leaked
   process and is not one.
