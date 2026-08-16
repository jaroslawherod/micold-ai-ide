# BUG-002 — visual pass (T091)

**Date**: 2026-08-14
**Run by**: an agent, not a person at a display — Xvfb `:78` at 1600×1400, rendered by Mesa's
lavapipe (software Vulkan), driven with `xdotool`, captured with `import`. Per the repo's
`visual-pass` skill.
**Build**: this branch's own `micold-ai-ide` + `micold-daemon`, snapshotted to a private directory
before launching (see *What nearly invalidated this*).
**Isolation**: `HOME` and `XDG_RUNTIME_DIR` pointed at a scratch directory, so the app spawned its
own daemon on its own socket with its own catalog. The user's running `/usr/bin/micold-daemon` was
never contacted and is still up; everything this pass started was stopped by PID afterwards.

## Fixture

A scratch repository with 17 branches, one of which — `fix/olx-auth` — is checked out in a worktree
**outside** the app's own directory:

```
git worktree add -b fix/olx-auth /tmp/claude-1000/vp/outside/olx-auth
```

That is BUG-001's third holder shape, and the case BUG-002 is about.

## What was checked, and what was seen

### 1. A refusal must not cost the form (FR-034, FR-035, SC-009) — **PASS**

`bug002-list.png` establishes the geometry the defect needs: with the branch list open, the dialog
card ends well above the list, and the last two rows — `fix/olx-auth · in use outside this app` and
`master · in use by the project checkout` — are drawn over the window background, outside the card
entirely. This is not a contrived arrangement; it is what a 17-branch repository does at 1600×1400.

`bug002-after-press.png` is the same frame after a left press on `fix/olx-auth`. The form is still
open, the list is still open, nothing is selected, and no input changed. Before the fix this press
published `AddWorktreeCancelled` and the form vanished — proven separately by
`tests/add_worktree_form_survives_a_refusal.rs`, which was observed failing with the fix reverted.

The disabled rows are also visibly distinguishable from the enabled ones by tone, not only by the
absence of emphasis (021 FR-012b), and each carries its reason inline.

### 2. Including a worktree the app does not manage (FR-027–FR-033, SC-010, SC-011) — **PASS**

`bug002-blocked.png` — submitting a new-branch form that derives `fix/olx-auth` raises:

> 'fix/olx-auth' is already checked out in a worktree outside this app:
> /tmp/claude-1000/vp/outside/olx-auth.
> A branch can only be checked out in one place at a time. Include that worktree to work in it from
> here — nothing is moved or changed — or choose a different name.
>
> **[Include that worktree]** [OK]

Full path (BUG-001's fix, intact) plus the new action (FR-027). Reuse and overwrite are still not
offered.

`bug002-row.png` — pressing it adds a sidebar row, `Olx auth`, tagged **outside this app** (FR-029).
The form stays open on its explanation, because inclusion does not unblock the branch — git still
refuses the second checkout — and the user's inputs are untouched.

`bug002-hover.png` — hovering that row shows the full absolute path, and the row's action cluster
(new session, delete) is the ordinary one (FR-029: behaves like any other worktree).

`bug002-menu2.png` — its context menu carries **Stop showing** between Rename and Delete. Choosing it
removes the row; `git worktree list` in the fixture still reports the worktree, its `.git` file is
still there, and `fix/olx-auth` still exists (FR-030, FR-028, SC-011). Nothing was moved and no git
command ran.

`bug002-restart.png` — after re-including and restarting the whole app (client and daemon both
stopped), the row is back, still tagged. The persisted value was read from the project's own state
file:

```json
"included_worktrees": ["/tmp/claude-1000/vp/outside/olx-auth"]
```

A session started in the included worktree, with Claude Code running at
`/tmp/claude-1000/vp/outside/olx-auth` — the whole of SC-010, from a blocked branch to work in
progress, without leaving the app.

## What this pass does **not** answer

- **Mid-flight animation.** The list's entrance and the dialog's are 150–300 ms; a screenshot
  pipeline cannot reliably catch a chosen frame. Whether the transitions *look* right is unverified.
- **Frame pacing.** lavapipe is a software rasteriser; nothing here says anything about smoothness on
  a real GPU.
- **The name-collision case.** An included worktree whose folder name matches one of the app's own is
  covered by a unit test (`an_included_worktree_never_takes_a_name_the_app_is_already_using`), not by
  this pass — the fixture had no collision.
- **Light scheme.** Everything above was captured in the dark scheme only.

## What nearly invalidated this

The first launch showed the version-mismatch banner: the app spoke a schema hash the daemon did not.
The cause is worth recording, because it will catch the next person.

`target-shared/` is shared by every worktree in this repo, and cargo's *uplifted* binary
(`target-shared/debug/micold-daemon`) is a single name that the last builder wins. Another worktree
had built the daemon after mine, so `MICOLD_DAEMON_BIN` pointed at **that branch's** daemon while the
client was mine. `strings … | grep included_worktrees` returned 0 for the daemon and 6 for the client,
which is how it was caught.

The fix is to ask cargo where its artifact actually is
(`cargo build --message-format=json` → `.executable`) and copy both binaries somewhere private
*before* launching. A pass run against a mismatched pair proves nothing, and the banner is the only
warning you get.
