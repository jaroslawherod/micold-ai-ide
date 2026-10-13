# 552 visual pass, quickstart §B

Date 2026-10-13. Private headless sway (pixman/lavapipe), build of HEAD 402d138a, dark scheme.

Driven: fixture git repo with worktrees under `.claude/worktrees/`: `agent-0123456789abcdef0`,
`agent-a1b2c3d4e5f607182`, `agent-fedcba98765432100`, `feat-normal`. Opened the project, sidebar and
"Attach existing worktrees" dialog.

Seen:
- Banner "Found 4 worktrees"; all four listed (display names "Agent <hex>" x3 and "Normal"), each with a Host badge only.
- No "Show agent worktrees" reveal chip anywhere in the sidebar; no `Agent` tag on any row.
- Header row (icons) then banner then Default row: no empty gap or spacing oddity.
- Attach dialog lists all four as ordinary radio rows (M3 item, informational).

Gaps: fixture had no `worktree-agent-<hex>` name; light scheme and settings screen not checked;
saved sidebar crop shows only the first two rows (banner + Default + 2 rows).

Verdict: PASS
