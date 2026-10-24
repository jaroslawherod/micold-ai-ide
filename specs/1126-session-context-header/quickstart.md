# Quickstart: Worktree context header and session tabs above the terminal (#1126)

## Part A: automated

| Command | Proves |
|---|---|
| `mise run test-core` | H1–H6: title, state, branch chip, pull request chip, Changes label, summary; `StatusPhrase::of_status` agrees with `of` (FR-002–FR-006) |
| `scripts/build-lock.sh cargo test -p micold-client --lib -- session_strip changes::tests cdk::keyed session_area` | S1–S6: members, order, marked, stopped, status, presses, offsets; `HeaderCounts` transitions; keyed slots; `changes_press` (FR-007–FR-011) |
| `scripts/build-lock.sh cargo test -p micold-client --test features_changes --test session_area_stability --test terminal_tabs --test tab_reveal --test terminal_focus --test keyboard --test navigation` (and the lib's `app_chord_tests`) | H7 follow step; S8 stability; reveal on every marking press; pane chords and focus unchanged (FR-011, FR-012, FR-014, SC-002) |
| `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot --test layout_text_overflow` | the session-area covered states match the regenerated fixture; overflow, containment, `strip_controls_hold_their_size`, `strip_controls_anchor_the_trailing_edge`, `tab_children_fit`, `session_area_fits` pass (FR-001, FR-010, FR-013, SC-004, SC-005) |
| `scripts/gate.sh` | the whole merge gate |

Regenerate the fixture only once the session area is done:
`UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`,
then read the fixture diff: only the session-area states may change.

## Part B: visual pass (the `visual-pass` skill)

Prerequisite: the client on a private display (`ICED_BACKEND=tiny-skia` on Xvfb in the cloud), a
project with a worktree created from an issue, a branch with an open pull request whose checks fail,
two sessions in it (main and a helper), two shell instances in the main one; light, then dark.

| # | Do | Expect |
|---|---|---|
| B1 | Select the worktree's main session | top to bottom: header (issue title; chips: state with icon and word, `fix/…` short branch, `PR #n · Open · checks failing`; Changes with counts at the right), the strip (`claude · main` underlined, its shells `1` `2`, "+", the helper's tab, then status, New session and the split buttons at the right), the terminal; nothing under it (US1.1, US2.1, SC-005) |
| B2 | Press the helper's tab; press a shell tab; press the main tab | the helper is shown and the header is unchanged; the shell's terminal; the AI conversation (US2.2, US2.7, Edge *several sessions*) |
| B3 | Press "+", then **New session** and cancel, then confirm | a third shell tab; the confirmation opens; cancel changes nothing; confirm adds a helper tab after the others, selected and shown (US2.3) |
| B4 | Press split side by side, then split stacked | the focused pane splits; each pane keeps its own header; header and strip stay once above the panes (US2.4, Edge *several panes*) |
| B5 | Exit the AI process (`/exit`) | stopped mark on its tab, status *idle* and **restart** beside it; restart runs it again (US2.5, US2.6) |
| B6 | Press Changes; press it again | the Changes view under the header, button shown pressed; back to the terminal (US3.2, US3.5) |
| B7 | Edit a file in the worktree | the counts change within a few seconds (FR-014) |
| B8 | Open the Default entry's session | title *Default*, its state chip, no pull request chip (US1.4) |
| B9 | Resize the window to its minimum (640×480) | nothing overlaps or leaves the window; title and chips end in `…` with tooltips; Changes and splits whole; the strip scrolls with a faded edge (SC-004, FR-013) |
| B10 | Open 12 tabs at 1280×800 | the strip scrolls; New session and splits stay put; the faded edge takes the accent when the marked tab is beyond it (US2.8) |
| B11 | Run every chord of `terminal-panes.md#keyboard-shortcuts` with the terminal focused | each acts as before (US4.1, SC-002) |
| B12 | Repeat B1, B9 in dark | the same, all text legible |

Record screenshots under `specs/1126-session-context-header/visual-pass/` and the results in
`visual-pass.md`.
