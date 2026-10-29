# Visual pass M1 (#1602): self-targeted line in the agent-confirm dialog

- Result: PASS. Quickstart Part B step 2, the "asking session ends" line only. The changed-file and unpushed-commit lists are not covered here.
- Route: the host has no Xvfb, xdotool, or sway. The full window `ui::view_with` was rendered headless with tiny-skia, using the reference fonts and the light scheme, through a scratch test (since deleted) built on `tests/support/layout.rs`. Entrance settled for `SETTLE_FRAMES`.
- Prompt: "planner" deletes worktree "w1" with stop_sessions and delete_branch; `self_targeted` true in `visual-pass-m1.png`, false in `visual-pass-m1-baseline.png`.
- Seen: "The session that asked ends after you allow this." appears as its own Body line under the muted 60-second text and above Allow/Deny. It uses the dark on-surface color, so it has full contrast. Nothing is clipped or overlapping, and the dialog grows 18 px.
- Baseline: without `self_targeted`, the line is absent and the layout is otherwise identical. The dark scheme was not rendered.
