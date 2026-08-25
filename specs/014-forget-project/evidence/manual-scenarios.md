# 014 — the manual scenarios (quickstart A–D), run for the first time

**Date**: 2026-08-20
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own client + daemon, one invocation, copied inside the build lock, verified
by `strings` to come from this checkout.
**Setup**: throwaway git repos under the session scratchpad; `XDG_DATA_HOME` pointed at a scratch
dir, so the catalog read and written throughout is this run's own.

## Scenario A — forget a non-active project — **PASS**

| Step | Observed |
|---|---|
| 1 | Two projects listed under Known projects; `scratch-repo` active (check marker), `scratch-repo-b` not. |
| 2 | Forget on B opened `Forget "scratch-repo-b"?`, stating what is discarded and that **nothing on disk is deleted**. **No** "will stop N sessions" line — B had none (`a2-no-session-line.png`). |
| 3 | Cancel closed the modal; B still listed (FR-004). |
| 4 | Forget → Forget removed B; A remained, still active. |
| 5 | On disk: B's folder, its `.git`, and its commit are all intact. |
| 6 | The catalog on disk no longer lists B, so no relaunch can bring it back (FR-007). |

## Scenario B — forget the active project with running sessions — **PASS**

Two AI sessions started in the active project, both confirmed live as `claude` processes under this
run's `XDG_RUNTIME_DIR`. The switcher independently agreed, showing **"2 running"** on the row.

| Step | Observed |
|---|---|
| 2 | The modal read **"This will stop 2 running sessions."** (FR-002a) — `b2-two-running-sessions.png` |
| 3 | Forget removed the project; **no** active working space; both `claude` processes gone, **no orphans** (FR-010); the repo, its worktrees and its branches all still on disk (FR-006) |
| 4 | It was the only project, and the shell fell back to the first-run empty state — "No project open / Open a folder to set it as your working space" (FR-009) — `b4-empty-state.png` |

Reached via feature 015's route rather than the Known-projects button: right-click the row in the
top-bar switcher → **Forget project** → 014's confirmation. That exercises both features at once and
is recorded for 015 below.

## Scenario C — forget an unavailable project — **PASS**

A catalog entry pointing at a path that does not exist. The row renders with a warning marker and
its primary action reads **"Unavailable"**, disabled; **Rename** and **Forget** stay enabled
(`c-unavailable-row.png`). Forget → Forget removed it, and the catalog on disk no longer lists it
(FR-011).

## Scenario D — re-opening a forgotten folder — **PASS**

Steps 1–3 were run on **2026-08-21**; step 4 had already passed on 2026-08-20. Same harness, a fresh
scratch `XDG_DATA_HOME`, and a fixture at a **real** path (`/home/jaro/.aaa-vp83d/myrepo`, since a
symlinked project path misclassifies every worktree — see
[002 BUG-002](../../002-project-workspace-management/bugs/BUG-002.md)).

The step 1 fixture is the one the scenario actually asks for — a project carrying *all three* kinds
of remembered state, each set through the UI rather than seeded:

| What | How | Persisted as |
|---|---|---|
| a custom project name | Known projects → Rename → "Renamed Fixture" | `projects.json` `display_name` |
| a worktree-name override | right-click the worktree row → Rename → "Custom Alpha Name" | `projects/ee1fe55f45c66a12.json` `worktree_display_names: {"feat-alpha": "Custom Alpha Name"}` |
| a session record | **+** on that worktree row; a real `claude` process attached | the same file's `sessions[]`, plus `last_session` |

| Step | Observed |
|---|---|
| 1 | Forget from the switcher's context menu. The modal read **"This will stop 1 running session."** — the singular form; scenario B recorded the plural (`d-forget-one-session.png`). Confirming emptied the catalog to `{"projects": [], "last_active": null}` and **deleted** the per-project state file; no `claude` process survived. |
| 2 | Re-opened the *same* folder through the selector — Open a project → `/home/jaro` → `.aaa-vp83d` → `myrepo` → **Open this folder**. |
| 3 | **PASS on all three.** The entry came back as **`myrepo`**, the folder name — not "Renamed Fixture" — in the title, the top-bar chip and the Known-projects row. The worktree came back as **`Alpha`**, its default, not "Custom Alpha Name". The sidebar shows **no session**. The recreated state file is `{"sessions": [], "worktree_display_names": {}}` — same filename, because the key is derived from the same path, and empty. |
| 4 | (2026-08-20) After forgetting, `.../micold-ai-ide/projects/` is empty — nothing for a reload or session-reconciliation to resurrect (FR-005/FR-012). |

`d-reopen-defaults.png` is the before/after: red-bordered is the project before forgetting — chip
"Renamed Fixture", worktree "Custom Alpha Name", a live session — and blue-bordered is the same
folder re-opened, showing "myrepo" and "Alpha" with no session row.

**FR-012 also holds here**: Known projects lists the re-opened folder **once**. The forget-then-open
round trip produced a fresh entry, not a second one.

## Also confirmed here: feature 015, Scenario A

015's own tasks were complete, but its manual procedure had never been recorded either, and
Scenario B above ran straight through it:

1. Project switcher opened from the top bar. ✅
2. Right-click on a project row → a context menu with **Forget project** and a trash icon. ✅ (US1 AS1)
3. The switcher panel stayed **visible behind** the menu. ✅ (FR-009)
4. Choosing it closed the menu and opened 014's confirmation, naming that project. ✅ (US1 AS2)
5. Forget removed it from the switcher and Known projects; the catalog on disk agrees. ✅ (US1 AS3)
6. The folder and its git repository are unchanged. ✅

**One harness caveat worth recording**, because it looked like a defect for three attempts: the
menu anchors at `self.cursor`, which is fed by a `CursorMoved` subscription that only runs while
the switcher is open. A synthetic `mousemove` immediately followed by a right-press produced **no
menu at all** — the press arrived before any motion event did. Moving the pointer in a few steps
first, then pressing, opens it every time. That is the harness, not the application; a human moving
a real pointer cannot hit it.
