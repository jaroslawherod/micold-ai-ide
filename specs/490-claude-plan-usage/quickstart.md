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

## Part B results (2026-10-11)

Run on Xvfb with iced's software renderer (tiny-skia; no Vulkan here), driven by XTEST, with a
private data and runtime directory. There was no Claude account on the machine. A real Claude
session was started from the app, and readings were fed to its relay file with Part A's
`micold-daemon status-line` command. Screenshots are in [visual-pass/](visual-pass/).

| Step | Result | Evidence |
|---|---|---|
| B1 Shown | **pass** (relay), **for the user** (real account, `/usage` comparison SC-002) | The app bar showed `42% · 00:06`. The tooltip read `5-hour: 55%, resets 00:06` / `At or above 50%: 5-hour` / `Updated 23:06` (`app-bar-tooltip.png`). |
| B2 Warning | **pass** | At 50 with 55–60% the glyph became the triangle and the glyph and label turned the error colour. A 42% reading returned the normal look with no new session. Typing 49 and pressing Save was refused with "Enter a whole number from 50 to 100" (`app-bar-normal-warning-dark-off.png`, `settings-plan-usage-light-dark.png`). |
| B3 Off | **pass** (app side), **for the user** (Claude Code's own line) | Turning the switch off and saving hid the indicator at once. A new session's `hooks/<uuid>.json` had no `statusLine`, and the running session's relay file was removed. |
| B4 User status line kept | **for the user** | Needs Claude Code drawing its own status line with a real account, and a Windows machine with and without Git Bash. |
| B5 Unavailable | **pass** (no reading), **for the user** (offline, API-key login) | No indicator showed before the first reading. Neither log had a plan-usage or status line at WARN or ERROR. |
| B6 Themes | **pass** | The showcase `UsageIndicator` in light and dark (`usage-indicator-light-dark.png`), and the real app bar and Settings in light and dark. |

The relay's first post, about 3 s after the session started, did not show. Every later post did,
with and without proxy variables in the environment, and that first miss could not be reproduced.

CI (T048): on #779's head (`bc59b039`), the `build + test` jobs ran green on ubuntu-latest,
macos-latest and windows-latest, and so did `package + smoke (windows-11-arm)`. The real-runtime
sandbox job failed once on an image-import timing bound in `micold-core` that this feature does
not touch, and passed on its one re-run.
