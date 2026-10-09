# Quickstart: #488

## Part A — automated
1. `mise run test-core` — provider fixtures, bind rule, store round-trip, protocol version.
2. `mise run gate` — full gate. 3. `mise run image && mise run test-sandbox` — both CLIs report a version in the image.

## Part B — visual pass (`visual-pass` skill)
With stand-in `codex` on `PATH` and no `opencode`: open the new-session chooser; Codex selectable,
OpenCode shown unavailable naming `opencode`; Settings default lists the available providers and its note names the missing ones with the reason (a chooser note needs two or more available CLIs, so also put a second stand-in such as `claude` on `PATH`); start the
Codex session, row labelled `codex`, badge `Unknown`.

## Part C — real CLIs (when installed)
Start, quit and restart a session of each; confirm it resumes its own conversation and the label
appears after a first turn; record results in `contracts/*.md` (V8, V10, V14, V15).
