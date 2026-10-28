# Visual pass M5 (T031): quickstart Part B steps 4, 5, 7b

2026-10-28, commit 09d3654e6, debug build of `micold-ai-ide` and `micold-daemon` from the same build
(client attached to daemon). Xvfb 1600x1400 + lavapipe (wgpu Vulkan), driven by xdotool, against a
real authenticated `claude` 2.1.312 in a scratch git repo. Models: claude-sonnet-5-5, then
claude-haiku-5-5 (via `/model haiku`). Light theme first, then Dark (Settings > Appearance).

| Check | Light | Dark | Crop |
|---|---|---|---|
| `/effort high`/`HIGH`, `Max`, `default`, `auto` + Enter: trigger updates, marker "Reasoning effort: X from here on", draft cleared, no user turn | PASS | PASS | 01, 11 |
| `/effort` alone opens the effort picker (Default marked current), draft cleared | PASS | PASS | 02, 13 |
| `/eff`: exactly one `/effort` row; unfiltered `/` lists it once under **In this app** (headers are hidden while filtering, by design: `PaletteLine` docs) | PASS | PASS | 03, 04, 12 |
| `/effort extreme`: "Use /effort with one of: default, low, medium, high, xhigh, max", draft kept | PASS | PASS | 05, 12 |
| Busy turn, `/effort low`/`LOW` sent mid-turn: trigger changes at once, draft cleared, turn finishes, marker after its Done | PASS | PASS | 06, 15 |
| Level the model does not offer | not run | not run | — |
| 7b guide paragraph "`/effort` in the message box" matches | PASS | — | — |

Session turn counter read 4 turns for 4 prompts: no `/effort` text reached Claude Code.

**Not run:** the unoffered-level refusal. The real CLI reported all five levels for both
sonnet-5-5 and haiku-5-5 (picker in haiku shows Default, low … max), so no model here offers fewer.
Covered by unit tests only (`effort.rs`, `features_command_palette.rs`).

**Finding (not in the pass criteria): stale refusal notice.** After `/effort extreme` is refused,
the notice stays under the transcript after the draft is cleared, after `/effort` opens the picker,
and after a corrected `/effort high` succeeds (trigger reads "Effort: high" with "Use /effort with
one of: …" still above it; crop 14). It clears only when a real message is sent:
`Composers::take_send` is the only path that resets `notice`; `send_intent`'s `EffortPicker` /
`SetEffort` arms do not. The guide does not say the notice persists.

Cannot judge here: smoothness or mid-animation frames (software rasteriser, debug build).
