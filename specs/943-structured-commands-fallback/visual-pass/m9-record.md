# M9 visual pass — quickstart Part B 3 and 6–9 (T065)

- **Date**: 2026-10-24
- **Where**: cloud container, Xvfb `1600x1400x24`, `ICED_BACKEND=tiny-skia`, driven with xdotool;
  client and daemon pinned from one debug build of this branch (daemon log: `client attached to
  daemon`). A real `claude` 2.1.310, a scratch git project holding this repository's `.specify/` and
  the `speckit-specify`, `speckit-autopilot`, `speckit-plan` and `speckit-clarify` skills, a
  **Claude Code (structured)** session started from the Default row's chevron. Light first, then
  dark (`"theme": "dark"` in the private data home's `settings.json`, the same session resumed).
- **Harness caveat** (as M8): tiny-skia under Xvfb can show a frame one input behind, and with no
  window manager keys reach the window only after `windowfocus`; a typed `/mcp` that arrived late
  once doubled into `/mcp/mcp`, which went to Claude as a message (the documented behaviour for a
  name that is no command). Every judged frame was captured after a resize and several seconds.

| Step | Check | Verdict | Evidence |
|---|---|---|---|
| B3 | `/speckit-specify <text>` from the composer runs to its first permission request, answered in #942's view (Allow / Deny / Always allow); Deny ends the turn with Claude's question | PASS (SC-007) | `m9-b3-speckit-specify-permission-light.png` |
| B3 | `/speckit-autopilot <text>`: its chip, then its first permission request in the same view | PASS (SC-007) | `m9-b3-speckit-autopilot-permission-light.png` |
| B6 | **Open in terminal** in the transcript's header | PASS (both themes) | `m9-b6-header-open-in-terminal-{light,dark}.png` |
| B6 | Sending `/mcp`: nothing reaches Claude; a card says why, **Open in terminal** / **Not now**, and what confirming does | PASS (both themes) | `m9-b6-mcp-offer-{light,dark}.png` |
| B6 | Confirming: the session restarts as a terminal session (its tab reads `claude`); above the terminal "1 queued message waits for the structured view" and **Open in structured view** | PASS (both themes) | `m9-b6-terminal-held-queue-line-{light,dark}.png` |
| B6 | `/mcp` typed into the terminal | **FAIL → fixed**, then NOT SEEN | Run 1: the daemon refused the switch, *The command of a switch to the terminal must be one that needs the terminal*: the client sent `mcp`, the daemon types and checks `/mcp` (below). After the fix the switch went through, and the daemon declined to type the command because Claude Code first asked to trust the scratch folder (`the switch's command was not typed … would first ask whether to trust this folder`, 938's first-prompt rule): `m9-b6-terminal-trust-prompt-light.png`. Trusting a folder in the shared Claude Code config was left to a human |
| B6 | **Open in structured view**: the transcript as before, a **Continued in the terminal** marker saying the turns are in the terminal's scrollback, the queue still on hold with **Send queued** | PASS (both themes) | `m9-b6-back-marker-queue-held-{light,dark}.png` |
| B6 | A draft typed before **Open in terminal** (header, no command) is in the composer after the round trip | PASS | `m9-b6-draft-kept-after-round-trip-light.png` |
| B6 | Same conversation: after two round trips Claude names the keeper of the story it was writing before the switches | PASS | `m9-b6-same-conversation-light.png` |
| B7 | Switching while a permission request waits, and while a turn runs: the dialog names the work (*A permission request is waiting; switching now withdraws it*, *A turn is in progress; switching now ends it*); Cancel changes nothing | PASS (both themes) | `m9-b7-confirm-switch-during-request-light.png`, `m9-b7-confirm-switch-during-turn-dark.png` |
| B7 | Two windows: a switch in one, the other (read-only, the project open elsewhere) follows to the terminal with the same line and button | PASS | `m9-b7-two-windows-follow-light.png` |
| B8 | A Copilot session offers no switch | NOT RUN live (no `copilot` here) | `features_session` `the_session_menu_offers_the_other_kind_for_claude_only`, `a_claude_session_offers_the_other_kind_and_other_clis_offer_nothing` |
| B8 | A sandboxed session switches inside its sandbox | NOT RUN (no container runtime: `docker` has no daemon here) | M6's `structured_switch` runs the switch through the same start path; CI's sandbox job |
| B9 | macOS and Windows | NOT RUN (Linux container) | the change adds no `cfg` arm; CI builds and tests all three |
| — | The `/login` card after an authentication failure | NOT RUN live (needs a signed-out CLI) | drawn only while `offers_login` (M7's tests); review A M9 F1/F2 fixes |

## Bug found and fixed

A switch's command never reached the terminal: `SwitchRequest.command` holds the command's name
(`mcp`), and the shell sent it as is, while the daemon types `SessionSwitchKind.command` into the
terminal verbatim and accepts only what `classify_draft` calls terminal-only, which needs the `/`.
`SwitchRequest::wire_command` now sends `/mcp`. Red/green: `features_session`
`switching::a_switchs_command_goes_out_as_the_terminal_types_it` (cycle log, M9).

## Seen, not M9's (follow-ups)

- The command of a switch is not typed when Claude Code first asks to trust the folder; the daemon
  logs why, the window says nothing.
- Pressing Enter on the terminal Claude Code's first-run theme screen during this pass stored a
  theme in the container's own Claude Code config (`~/.claude.json`).
