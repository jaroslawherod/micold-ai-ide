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
