# Quickstart

## Part A (automated)
- `mise run test-core` (settings round trip).
- `scripts/build-lock.sh cargo test -p micold-client features_composer` and the new large-editor, persist, startup and selector-geometry tests.
- `mise run gate` before pushing.

## Part B (visual pass, `visual-pass` skill)
1. Fresh settings: composer shows one control row, settings icon in collapsed look, no options row.
2. Click the icon: row with Model, Effort, Permission; icon tonal. Toggle off. Restart: state restored.
3. Hover indicator in Ready/Busy/Waiting: tooltip names the state; no status text.
4. Hover/focus/press/open/disable each selector: identical treatments; model list has a filter first row.
5. Permission Bypass: warning marker on the settings icon.
6. Type two lines, press Ctrl+Shift+M: editor opens at end; Done and Esc behave per spec; open-editor icon hidden for single-line unfocused draft.
7. Narrow the window: selectors wrap/shrink to 140 dp without overlap.

## Part B results (T032, 2026-11-07, `visual-pass`, private headless sway, real Claude CLI)

Evidence in `evidence/t032-*.png`; earlier captures in `evidence/m2-*`, `evidence/m3-*`, `visual/`.

| Step | Result | Notes |
|---|---|---|
| 1 | PARTIAL | One control row in the #1764 order (activity icon, text, settings, attach, Send; D8); no options row on a fresh start. The `t032-*` captures predate the activity-icon alignment fix (circle ~16 px low); the fix is pinned by a centre-line test, not re-captured. |
| 2 | PASS | Settings icon opens Model/Effort/Permissions and turns tonal; `composer_options_open` persisted true/false; restored after restart. `t032-options-420.png`, `t032-options-520.png`. |
| 3 | PARTIAL | Ready and Busy ("Working") tooltips seen, no status text (`t032-busy-tooltip.png`). Waiting not reached: the installed CLI runs in mode `auto` and never asks for permission; pinned by view tests (T014). |
| 4 | PASS | One selector style; permission list `t032-permissions-list.png`; model list with filter row `evidence/m3-model-list.png`. Hover/focus/press/disabled treatments are pinned by the anatomy tests. |
| 5 | PASS after restart | Bypass warning on the settings icon, banner and tooltip (`t032-bypass-marker.png`). A live switch is not applied because the CLI reports mode `auto` (CLI behaviour, not this feature). |
| 6 | PARTIAL | Expand icon opens the large editor with Cancel and Done; Done keeps the text; icon hidden for a single-line unfocused draft (`t032-large-editor.png`). Ctrl+Shift+M and Esc not drivable (wtype sent no special keys); pinned by the M1 tests and `visual/results.md` (Esc, Ctrl+Enter, Cancel captured in M1). |
| 7 | PASS | Options row after the wrap fix: 420 px window shows three lines, 520 px shows Model, then Effort and Permissions side by side; all at least 140 dp, no overlap. Closes the carried-over Review B MAJOR. |

Finding fixed in this close PR: the activity circle sat about 16 px lower than the attach and settings icons (bottom-aligned row, indicator 24 px against padded `IconButton`s). The indicator is now centred in the 48 dp touch target of its neighbours.
