# 008 T056 — deleting a worktree that has a running session

**Date**: 2026-08-20
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.

## Why this needed re-running

The 2026-07-20 attempt was recorded as *inconclusive* for a specific reason: the only instance
confirmed running at the time was the installed `/usr/bin` build of 2026-07-18, which predates
T054's and T055's fixes. A "no error banner" from that binary says nothing about this branch.

So the binary's provenance is the first thing established here, not an afterthought:

```
$ strings ~/vp83/bin/micold-ai-ide | grep -oE "micold-ai-ide/\.claude/worktrees/[a-z-]+" | sort -u
micold-ai-ide/.claude/worktrees/docs-cleanup-completed-features
```

That is this checkout. The client and daemon were built in **one** `cargo build` invocation and
copied to `~/vp83/bin` **inside** the build lock, so the pair is matched and neither is whatever
another worktree happened to leave in `target-shared` afterwards. The run was confirmed to attach
(`/tmp/vp83/micold/daemon.sock` live, no `refusing client` in the log).

T055's fix is present in the source the binary was built from — `micold-core/src/git.rs:235`
propagates the failure rather than discarding it with `let _ = …`.

## Setup

A throwaway repo at `scratchpad/scratch-repo`, opened as the only project. A worktree created
through the app's own New worktree dialog (type `feat`, name "login page two"), which produced
`.claude/worktrees/feat-login-page-two` on branch `feat/login-page-two`. A session started on that
worktree from its row's **+**, confirmed live and confirmed to be *in* the worktree:

```
claude 1231449 (cwd: scratch-repo/.claude/worktrees/feat-login-page-two)
```

## The delete

Hovering the worktree row reveals **+** and a trash icon (008's own refinement). Pressing the trash
opened the confirmation in `t056-confirm.png`:

> **Delete "Login page two"?**
> This permanently removes the worktree directory (.claude/worktrees/feat-login-page-two) and all
> of its sessions. This cannot be undone.
> ☑ Also delete the branch "feat/login-page-two"

## Result — PASS

`t056-after-delete.png` is the frame immediately after confirming.

| Claim | Observed |
|---|---|
| the sidebar updates | the worktree row is gone; the rail reads "No worktrees yet. Add one to get started." and the main view fell back to the project overview |
| **no error banner** | none anywhere in the frame — no notification surface, no snackbar, no inline error |
| the session's process is stopped | `pgrep -x claude` filtered to this run's `XDG_RUNTIME_DIR`: none remain. No orphan. |
| the worktree is gone from git | `git worktree list` no longer lists it |
| the branch is gone | `git branch --list` no longer has `feat/login-page-two` — the checked box was honoured |

The pre-existing worktree outside `.claude/worktrees/` (`scratch-wt-login`, on `feat/login-page`)
was untouched, as was `main`.

## What this run did not cover

The failure half of the checkpoint — "still loud on genuine failure" — is not reachable here
without provoking a real git failure (a locked worktree). It is covered by
`FakeGit::fail_next_remove` in `micold-core/src/git.rs:850` and the tests T051–T053 added for it.
