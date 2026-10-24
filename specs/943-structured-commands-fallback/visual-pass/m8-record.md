# M8 visual pass — quickstart Part B 1, 2, 4, 5 (T060)

- **Date**: 2026-10-24
- **Where**: cloud container, Xvfb `1600x1400x24`, `ICED_BACKEND=tiny-skia` (no lavapipe ICD here),
  driven with xdotool; client and daemon pinned from one debug build of this branch (daemon log:
  `client attached to daemon`). A real `claude` 2.1.310 (auth from the environment), a scratch git
  project with one project skill (`speckit-plan`) and one custom command (`hello`), a
  **Claude Code (structured)** session started from the Default row's chevron. Light first, then
  dark (`"theme": "dark"` in the private data home's `settings.json`, the same session resumed).
- **Harness caveat**: tiny-skia under Xvfb presents partial frames (scrolled lists drawn over black)
  and can show a frame one input behind. Every judged frame was captured after a window resize
  (forces a full redraw) and ~10 s of settling; a doubtful frame was re-captured. Two dark-run
  frames of the Enter-fill check stayed ambiguous under that lag, so Enter-fill is judged on the
  light run.

| Step | Check | Verdict | Evidence |
|---|---|---|---|
| B1 | `/` opens the palette above the composer, grouped **Skills** (project skill marked `(project)`), **Custom commands** (`/hello`), **Built-in commands**, **In this app** (`/model`), each with its description | PASS (both themes) | `m8-b1-palette-skills-group-{light,dark}.png`, `m8-b1-palette-groups-{light,dark}.png`, `m8-b1-palette-in-this-app-light.png` |
| B1 | `/login`, `/logout`, `/agents`, `/mcp` (and the CLI's own `/color`, `/focus`, `/reload-plugins`) read **opens in terminal** in words | PASS (both themes) | `m8-b1-palette-opens-in-terminal-{light,dark}.png` (`/config` is not in this CLI's reported list) |
| B1 | Before the session's first turn every skill is grouped under **Custom commands** | FINDING (follow-up) | `m8-b1-palette-open-light.png`: the CLI sends `init` (and its `skills`) only after the first message, so the kinds are unknown until then; after the first turn the groups are right |
| B2 | `sp` narrows (match emphasised); after `/speckit-p` `/speckit-plan` is selected; Tab fills `/speckit-plan ` | PASS (both themes) | `m8-b2-narrowed-sp-*`, `m8-b2-speckit-p-selected-*`, `m8-b2-tab-filled-*` |
| B2 | Enter on a palette row fills (`/hel` → `/hello`) and sends nothing; the next Enter sends | PASS (light) | `m8-b2-enter-fills-not-sends-light.png`; the sent command's chip, headed as typed with its description: `m8-b2-chip-card-light.png` |
| B4 | `/context`: its output behind **Command output**, drawn as markdown (heading, bold, tables), no markup | PASS (both themes) | `m8-b4-context-output-markdown-light.png`, `m8-b4-context-output-table-{light,dark}.png` |
| B4 | `/compact`: Working + Stop until done, then **Conversation compacted** with the summary folded | PASS | `m8-b4-compact-busy-light.png`, `m8-b4-compacted-marker-light.png`, `m8-b4-compacted-cleared-dark.png` |
| B4 | `/clear`: a *Conversation cleared* marker, earlier turns headed **Before the clear** | PASS (both themes) | `m8-b4-cleared-before-the-clear-light.png`, `m8-b4-compacted-cleared-dark.png` |
| B4 | Restart: Claude does not remember the cleared part | NOT RUN | needs a conversation fact to probe; FR-005a is covered by M5's daemon tests |
| B5 | The model control opens the picker: **Default**, the reported models (Sonnet, Fable, Opus, Haiku), the current one marked, a name field | PASS | `m8-b5-model-picker-light.png`, `m8-b5-picker-haiku-current-light.png` |
| B5 | Pick Haiku while idle: *Model changed to claude-haiku-5-5*, the control reads `claude-haiku-5-5 · your choice: haiku` | PASS | `m8-b5-haiku-picked-marker-light.png` |
| B5 | Pick during a turn: the turn finishes, then the change is marked | PASS | `m8-b5-picked-during-turn-dark.png` (Haiku turn running, Sonnet picked; marker after the turn's *Done*) |
| B5 | Type `no-such-model`: the CLI's reason, model unchanged | **FAIL → fixed** | run 1: the picker's field took no keys (see below); `/model no-such-model` showed *Model 'no-such-model' not found*, model kept (`m8-b5-no-such-model-refused-light.png`). After the fix: `m8-b5-picker-typed-name-dark.png`, `m8-b5-no-such-model-refused-dark.png` |

## Bug found and fixed

The model picker's typed-name field never received a key. Opening the picker focuses its field and
the composer's text area reports losing focus, so `focused_field` went empty and
`State::terminal_focused()` read true; `KeyboardElsewhere` then took the field's focus back on the
first key (the same mechanism as #941 T061 step 1). `terminal_focused()` now also yields while the
displayed session's model picker is open. Red/green: `layout_snapshot`
`composer_fits::the_open_model_picker_keeps_the_keyboard_from_the_pane` (cycle log, M8).

## Seen, not M8's (follow-ups)

- Before the first turn skills are grouped as custom commands (above).
- After `/compact` a second **Command output** card repeats the earlier `/context` output (the CLI's
  post-compact replay of a kept local-command output).
- A pick during a turn leaves a **Command output** card "Set model to 'sonnet' …" beside the marker.
- The collapsed **Command output** card's preview line shows the markdown's `##`.
- Raw `system` events (hook started/response, `post_turn_summary`, `active_goal`,
  `autocompact_state`, `rate_limit_event`) are drawn as *Raw event* rows — newer CLI events the
  transcript does not know (feature 938/940's mapping).
- After a client restart the earlier *Model 'no-such-model' not found* line was not redrawn.
