# M5 quickstart B4: confirmation dialog

Date 2026-10-01. Two Xvfb displays (:94, :95) + lavapipe (software Vulkan), not a real display. Client and daemon built
from this worktree (branch feat/daemon-should-expose-mcp-server-for-agent), copied to a private dir; private XDG data/runtime
dirs; stub `claude` (`exec sleep 100000`) first on PATH; throwaway git repo with worktrees `feat-b` (caller session) and
`feat-x` (target), both created through the app. Two client processes attached to one daemon (window 2 is the read-only
"already open in another window" one; it still shows the dialog). Requests sent with curl to `POST /mcp` using the
caller session's binding file. Crops at 1x.

- A. PASS. a-dark-both-windows.png (top = window 1, bottom = window 2), a-light-window1.png. Title: "New session" asks to
  delete worktree "feat-x" and its branch, stopping its sessions. Body line and 60 s note legible, nothing clipped,
  scrim behind, shared Material dialog (rounded surface, filled + outlined buttons). Light theme checked in window 1
  only (window 2 does not follow a theme change made in window 1 until restarted; not part of this feature).
- B. PASS. Allow in window 1: dialog gone in window 2 within 1 s, `feat-x` row gone in both
  (b-window1-after-allow.png, b-window2-after-allow.png); result-B-allow.json: `removed: feat-x`, `branch_deleted: true`.
- C. PASS. Deny: result-C-deny.json text is `refused_by_policy: declined by the user` (underscored, not "refused by policy");
  worktree stays; dialog gone in window 2 (c-window2-after-deny.png).
- D. PASS. Escape pressed in window 2: result-D-escape.json same as Deny; both windows clean
  (d-escape-both-windows-after.png, 1/4 scale, both windows side by side); worktree stays.
- E. PASS. About open in window 1 keeps About with no agent dialog (e-window1-about-open.png); window 2 shows the dialog
  (e-window2-agent-dialog.png); closing About in window 1 shows the agent dialog there (e-window1-after-about-closed.png).
- F. PASS. Both clients closed, daemon kept: curl returned in 0.5 s with `needs_confirmation: no application window is
  open to confirm it` (result-F-no-window.json); worktree stays.

Notes: button order is Allow then Deny (task text said Deny and Allow). The caller is named "New session" because the
stub session has no AI title. Not run: click-outside-as-Deny, the 60 s timeout, `stop_sessions: false` / branch-keeping wording,
animation, real GPU.
