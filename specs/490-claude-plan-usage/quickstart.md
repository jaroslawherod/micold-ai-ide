# Quickstart: Show Claude Plan Usage and the Next Limit Reset

## Part A — automated

Run the merge gate (`mise run gate`). The feature's tests by layer are listed in plan.md, *Test
strategy*. Fast loop for the core logic: `mise run test-core`.

Relay smoke test without Claude Code (Linux/macOS, a running daemon with the switch on and one
Claude session started by the app):

```sh
f=$(ls ~/.local/share/micold-ai-ide/hooks/*.status.json | head -1)
echo '{"rate_limits":{"five_hour":{"used_percentage":42,"resets_at":'$(( $(date +%s) + 3600 ))'}}}' \
  | micold-daemon status-line "$f"; echo "exit=$?"
```

Expected: the user's own status line output (or nothing) and `exit=0`; the app bar shows `42% ·
<time an hour from now>` within 10 s.

## Part B — manual visual pass (needs eyes at a display, or the `visual-pass` skill)

Prerequisites: a Claude Pro or Max account signed in to Claude Code; the app built from this
branch.

1. **Shown** (US1 s1–s2, SC-001). Start a Claude session in the app, send one prompt. The app bar
   shows the usage glyph and `NN% · HH:MM`; hovering lists **5-hour** and **Weekly** with
   percentages and resets, and `Updated HH:MM`. Compare with `/usage` in the session (SC-002).
2. **Warning** (US2, FR-010). Settings → Environment → Claude plan usage: set the threshold to 50
   (or below the current percentage) and save. The glyph becomes the warning triangle in the error
   colour; the tooltip names the window. Set it back: the normal look returns without a new prompt.
3. **Off** (US3 s5, FR-002). Turn the switch off and save: the indicator disappears in every open
   window. Start a new Claude session: its status line is the user's own (or none) and
   `~/.local/share/micold-ai-ide/hooks/<uuid>.json` has no `statusLine`.
4. **User status line kept** (US3 s8, FR-020). With a `statusLine` in `~/.claude/settings.json`,
   start a session with the switch on: Claude Code shows the same status line as with it off. On
   Windows, repeat once with Git Bash installed and once without (research R3, R4).
5. **Unavailable** (US3 s1, s4, s7, SC-003). Offline, or with an API-key login: no indicator, no
   notice, and `grep -i "plan.usage\|status" <daemon log>` shows nothing at WARN or ERROR.
6. **Themes**. Steps 1–2 in light and dark; the showcase's `UsageIndicator` entry in both.
