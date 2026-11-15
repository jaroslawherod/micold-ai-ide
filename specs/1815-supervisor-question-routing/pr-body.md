## Close 1815 — supervisor-question-routing (specs/1815-supervisor-question-routing)

All four milestones are merged (#1901, #1907, #1911, #1916). This PR closes the spec.

- spec.md `**Status**` set to Closed.
- contracts/routing-cli.md: the `route target` row now states the M4 round-1 rules (list shape, `is_caller` fallback, `parent_state`, `default` via worktree lookup, live over failed).
- drift-M1.md records the plan-drift false positive (reference files from the design PR).
- Ledger: Phase `done`, Next step `handoff`, follow-up cleared.

plan-drift: M1 recorded in drift-M1.md, M2, M3, M4 clean. Converge skipped: Review B CLEAN for every milestone. No code or tests changed; CI is the gate.

Refs #1815

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvKE22c2oWVNFebVx1H
