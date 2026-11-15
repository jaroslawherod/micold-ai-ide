<!-- claude-projects-attribution: {"github_login":"jaroslawherod"} -->
_Requested by **jaro** · [project thread](https://claude.ai/code/project/chan_01RyHdfvmnDGyHasLedRFLeQ?thread=cmsg_01RyHdfvmnDGyHasLedRFLeQQnpAUEn5Ury9Y2VWpxkzG6)_

## Milestone M3 of 6 — sidebar user groups (specs/1786-sidebar-user-groups)

**Deliverable**: a worktree or run group menu offers "Move to group" (groups, Ungrouped, New group…) and the row moves, by keyboard too.
**Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test main sidebar_group_actions`
**Tasks**: T024–T028 · **Satisfies**: US2 scenarios 1–4; FR-005, FR-010 (Move to group), FR-016 (menu); SC-002, SC-006

- Core: `SidebarLayout::move_item` (remove before insert, index adjusted and clamped, `IsDefault`/`UnknownItem`/`UnknownGroup` refusals), with `adopt`, `holds`, `list_of`.
- Client: `Msg::Move`, `MoveTarget`, `move_menu_items`, `NamingTarget::NewAndMove`; "Move to <group>", "Move to Ungrouped" and "Move to new group…" in the worktree menu, the Overview card menu (the keyboard path: Menu key, arrows, Enter) and the run group menu. No group yet: only "Move to new group…", which creates the group and moves the row; a refused name keeps the dialog and moves nothing.
- Decision D7: a header's count is the rows shown; `n of m` while a tag filter hides members, `empty` only for a group with no member.
- User guide: "Move to group" in `docs/user-guide/worktrees-and-sessions.md`.

## Agent review
- Review A (code-review, high): 8 findings, 5 fixed (gone worktree counted as hidden, choosing the row's own list reordered it, New group over an open dialog closed it, `listed` ignored filters, adopt rule copied twice), 3 declined (unbounded menu height, unheld row to Ungrouped, header-fits gate pose belongs to M6).
- Review B (conformance): pending, run by the orchestrator after this PR opens.
- Visual pass: pending, run by the orchestrator.

## Local gate
Local gate: see the last comment; `scripts/gate.sh` result is recorded below when it finished. Four micold-core permission tests fail only as root (settings_refuses_save_over_failed_read, settings_write_is_logged, two in worktree_leftovers) and are unrelated.

Refs #1786

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvP59qpaoUKPRLrQia3
