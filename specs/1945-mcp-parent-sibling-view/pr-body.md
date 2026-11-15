## Design: MCP parent/sibling view (specs/1945-mcp-parent-sibling-view)

Spec, clarifications, plan, research, contracts and tasks for #1945: a session sees its parent, parent worktree, siblings and children (unparented worktrees count as children of `default` for visibility), gets a one-call `neighbourhood` tool, can message its parent with `send_parent_input`, and the parent link is recorded before the child session starts. Daemon and MCP only; no GUI.

Milestones: M1 visibility (T001-T005), M2 tools (T006-T014), M3 record-before-start and docs (T015-T020).

## Agent review
- Spec: 2 rounds, clean. Plan: 2 rounds, clean. Tasks: round 1 CHANGES (1 MAJOR, 3 MINOR, fixed); round 2 (scoped) clean.

## Local gate
Specs-only: `scripts/check-criteria-observables.sh` passed.

Refs #1945

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvKNynWdKUiNRXgAUic
