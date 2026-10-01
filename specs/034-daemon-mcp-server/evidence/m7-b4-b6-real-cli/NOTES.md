# M7 quickstart B4, B5, B6 with the real Claude Code CLI

Date 2026-10-01. `claude --version`: 2.1.287 (Claude Code), model Haiku 4.5 (ANTHROPIC_MODEL=haiku honoured).

How it was run: two Xvfb displays (:71, :72) + lavapipe (software Vulkan), not a real display. Client and daemon are the
pinned pair from the scratchpad `pin/` dir, copied to a private dir. Private XDG_DATA_HOME/CONFIG/RUNTIME/STATE dirs; both
clients attached to one daemon (window 2 is the read-only "already open in another window" one). The daemon's env had a
private CLAUDE_CONFIG_DIR (copy of credentials, settings and a `.claude.json` with the throwaway project trusted); no trust or
onboarding question appeared in any session. The private copy was deleted afterwards. Throwaway git repo `q` (one commit,
`main`) seeded into the private `projects.json`. Worktrees were created through the app's New worktree dialog (type + name),
which names them `<type>-<name>`. Sessions created from the sidebar "+": S1 = Claude Code in `chore-b` (caller; the app titled
it "Delete worktree feat-x" after the first prompt), S2 = Claude Code in `feat-y` (titled "Pong reply" after B5.B). The real
agent made every tool call except B4.E (curl, marked). Crops at 1x unless noted; palettes reduced to keep each PNG small.
Light theme was used from B6 onward (the theme change needed a client restart; daemon and sessions stayed).

## B6 Settings rows: PASS
b6-dark-environment.png, b6-dark-select-menu.png, b6-light-environment.png, b6-light-select-menu.png. Toggle "Let AI sessions
manage worktrees and sessions" and select "Let agents read and type into other sessions" (default Auto) have the same field
shape, floated label and supporting text as "Default AI CLI". Nothing clipped or overlapping in either theme. Menu lists
Auto / Confirm each send / Off. The open menu covers the second supporting-text line (overlay).

## B4 confirmation dialog
- A. PASS. b4a-dialog-window1-top-window2-bottom.png, b4a-both-windows.png (50% scale of both). Both windows show: "Delete
  worktree feat-x" asks to delete worktree "feat-x" and its branch, stopping its sessions; Allow, Deny; 60 s note. Calling
  session is named by its AI title (not "New session").
- B. PASS. Allow in window 1. A screenshot ~1.5 s later (not saved) showed window 2 without the dialog and without the `feat-x`
  row; git worktree list lost it. b4b-both-windows-after-allow.png is 25 s later (both windows, no `feat-x`). Agent printed
  `{"branch_deleted":true,"leftovers":[],"removed":"feat-x"}` (b4d-after-allow-agent-result.png is the D result; B's text is in
  the terminal history, not separately cropped).
- C. PASS. Deny. Agent result line: `error: refused_by_policy: declined by the user` (b4c-deny-agent-result.png); `feat-z` stayed
  (git worktree list; sidebar). Window 2 dialog gone.
- D. PASS (second attempt, see Deviations). Dialog open at ask+2 s, Allow clicked at ask+57 s (about 55 s with the dialog
  open). Agent's MCP call returned the success result (`Worked for 58s`), `feat-z` deleted
  (b4d-after-allow-agent-result.png; daemon-tool-calls.txt: `outcome=ok`).
- E. PASS (curl, not the agent). Both client processes quit, daemon kept. curl `tools/call delete_worktree feat-y` with S1's
  binding file: `needs_confirmation: no application window is open to confirm it` in 0.001 s (b4e-curl-no-window.txt);
  worktree stayed. Window reopened afterwards.

## B5 cross-session (agent made all calls)
- A. PASS. b5a-read-s2-output-no-dialog.png: plain text of S2's screen (banner, `Try "fix typecheck errors"` prompt), no dialog.
- B. PASS. b5b-s2-one-submission-pong.png: S2 shows ONE user turn with both lines ("Reply with the word PONG." / "Then stop."),
  one answer "PONG". No dialog (agent returned `{}`).
- C. PASS. b5c-setting-confirm-each-send.png (saved without restart). b5c-confirm-dialog-no-text.png: "asks to type into session
  "Pong reply"", text not shown. Allow: "Reply with OK." arrived, S2 answered OK (b5c-allow-s2-received-ok.png). Deny:
  `error: refused_by_policy: declined by the user` (b5c-deny-agent-result.png), no NOPE in S2 (b5c-s2-after-deny-no-nope.png).
  A read at this setting returned S2's text with no dialog (b5c-read-at-confirm-no-dialog.png).
- D. PASS. b5d-setting-off.png; b5d-off-read-and-send-refused.png: both `error: refused_by_policy: reading and typing into other
  sessions is turned off in Settings ("Let agents read and type into other sessions")`; no dialog; S2 untouched
  (b5d-s2-untouched.png).
- E. PASS. b5e-setting-auto.png; b5e-auto-send-no-dialog.png (`{}`); b5e-s2-auto-again.png (AUTO-AGAIN answered).

Also saved: daemon-tool-calls.txt (daemon log lines of tool calls: op, target, outcome).

## Deviations
- The worktree for the caller is `chore-b`, not `b` (dialog names it `<type>-<name>`). Targets `feat-x/y/z` match.
- Worktrees made with `git worktree add` before launch were NOT listed in the sidebar; the first project was abandoned and a
  fresh repo's worktrees were created through the dialog (the sidebar apparently lists only included/app-created worktrees).
  A stray Claude session in the project root of the first repo was closed.
- B4.D first two attempts failed through my timing: the dialog expires 60 s after the request and I clicked at about 75 s and
  then missed the dialog while polling; both ended `needs_confirmation: no answer in an application window within 60 seconds`
  in the agent (expected behaviour at 60 s). The third attempt is the recorded one.
- B4.B timing "within ~1 s" was checked by a screenshot ~1.5 s after the click, not saved; saved shots are later.
- Window 2's mirrored terminal lagged behind window 1's (stale text in one screenshot); the dialog state was correct.
- The B5.B prompt text was typed by the agent as one tool call; one-submission is read from S2's single turn.

## Not run
Click-outside-as-Deny, Escape, copilot sessions, animation, real GPU, B1 steps 1-6, the 60 s timeout of a send at Confirm
(covered in M6 stub pass; here the B4 timeout was seen only for delete_worktree).
