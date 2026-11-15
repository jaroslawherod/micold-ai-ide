# Quickstart: validating worktree parent

Prerequisite: a built daemon and a project with the micold tools connected (`docs/user-guide/agent-tools.md`).

**Part A: tools** (each step is an automated test in M1)
1. From a session in `default`, `create_worktree` twice; create a third by hand in the app. `list_worktrees` shows `parent: "default"` on the first two only.
2. From a session in worktree `W`, `create_worktree` without `parent`: row shows `parent: "W"`; with `parent: "default"`: shows `default`; with `parent: "nope"`: refused, no worktree.
3. `list_worktrees` with `parent: "W"` returns only `W`'s direct children.
4. Rename `W`, restart the daemon: children still report `W`'s directory name and `known`.
5. Delete `W`: children report `parent_state: "removed"`. Recreate a worktree named like `W`: it has no children.
6. `list_sessions`: a session in a child worktree carries the parent fields.

**Part B: sidebar** (M2; `visual-pass` skill)
1. Parent label appears on child rows only, with the parent's display name; renaming the parent updates it.
2. A removed parent shows as removed. The parent filter narrows to one parent and composes with the issue filter.

Visual pass recorded 2026-11-15 (Xvfb + lavapipe, 1280 px wide, debug build of this branch, fixture repo with parents `feat-1900_alpha-parent` and `feat-1902_beta-parent`, a `default` child, an orphan child of a removed parent, and a hand-made worktree). B1 PASS: only child rows carry a parent chip and parent rows have none (`b1-parent-label-light.png`); after giving the parent the display name "Alpha Renamed" both children show it (`b1-renamed-parent-light.png`, `b1-renamed-parent-dark.png`). B2 PASS: the orphan shows `chore-gone-parent (removed)` and its own filter chip lists only it (`b2-filter-popover-dark.png`, `b2-removed-parent-filter-dark.png`); `parent:Alpha Renamed` narrows to its two direct children (`b2-parent-filter-light.png`) and adding `issue` leaves only the issue child (`b2-parent-and-issue-filter-light.png`). Screenshots are in `/mnt/project-files/issue-1859/`. Not covered: transitions. The rename was applied through the state file and a restart, not the rename dialog.

**Part C: supervisor** (M3; fixture shell tests)
1. A fixture list with runs of two parents: the supervisor rule selects only its own and refuses actions on the other.
