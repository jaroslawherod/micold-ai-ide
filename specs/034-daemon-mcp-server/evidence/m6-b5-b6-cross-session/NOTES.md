# M6 quickstart B5 + B6: cross-session access and its Settings row

Date 2026-10-01. Xvfb :96 + lavapipe (software Vulkan), not a real display. Client and daemon built from this worktree
(branch feat/daemon-should-expose-mcp-server-for-agent), copied to a private dir; private XDG data/runtime dirs; stub `claude`
(prints `STUB-READY-S2`, then `exec cat`, so typed input is echoed twice: tty echo + cat) first on PATH; throwaway git repo as
the project; two sessions S1 (caller) and S2 (target) in the default worktree, both created through the app (+ on Default).
Requests sent with curl to `POST /mcp` using S1's binding file. Crops at 1x unless named otherwise. The theme change needed
a client restart to take effect (daemon and sessions kept).

- A. PASS. a-dark-environment.png, a-light-environment.png, a-dark-select-menu.png (menu open). Toggle "Let AI sessions manage
  worktrees and sessions" and select "Let agents read and type into other sessions" (default Auto) legible in both themes; the
  select has the same field shape, floated label and supporting text as "Default AI CLI"; nothing clipped or overlapping.
  Menu lists exactly Auto / Confirm each send / Off. The open menu covers the second supporting-text line (overlay, expected).
- B. PASS. result-B-read.json (lines contain STUB-READY-S2, no escape bytes), result-B-send.json (`{}`), b-s2-terminal-auto.png:
  "first line / second line" shown once echoed by tty and once by cat, no dialog. With `cat` as stand-in, "one submission" is
  inferred (both lines arrived together as one block), not distinguishable from two quick submissions.
- C. PASS. c-confirm-dialog.png: "New session" asks to type into session "New session"; body "An AI session asked for this. Nothing
  changes unless you allow it; if nobody answers within 60 seconds, the request fails."; text sent is not shown; same shared
  Material dialog as M5 (light theme). Allow: result-C-allow.json `{}`, text in S2 (c-s2-after-allow.png). Deny:
  result-C-deny.json `refused_by_policy: declined by the user`, nothing new in S2 (c-s2-after-deny.png). read_session_output at
  this setting: result-C-read.json, returned in 0.06 s, no dialog.
- D. PASS. result-D-read.json and result-D-send.json both `refused_by_policy: reading and typing into other sessions is turned
  off in Settings ("Let agents read and type into other sessions")`; no dialog, S2 unchanged (d-s2-after-off.png).
- E. PASS. result-E-send.json `{}`, no dialog; result-E-read.json shows "auto again" in S2 (e-s2-after-auto-again.png).

Extra: extra-timeout-send-while-confirm-no-click.json: an unanswered send at Confirm each send returned `needs_confirmation:
no answer in an application window within 60 seconds` and typed nothing (the dialog was still on screen at the instant the
result arrived, gone 2 s later).

Notes: both sessions are labelled "New session" (the stub sets no AI title), so the dialog cannot be told apart by name here.
Not run: window-closed case for sends, click-outside, animation, real GPU, a real `claude` agent calling the tool.

Run on the tree of commit 5fd07a88, before the review A fixes. Those change nothing visible. Since them, a send into a session
whose CLI has no trust record for its folder is a `conflict`, so this setup would also need the project recorded as trusted in
the private home's `.claude.json`; that path is covered by `mcp_cross_session.rs`, not by this pass.
