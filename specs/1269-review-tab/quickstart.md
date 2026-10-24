# Quickstart: Review as a per-worktree tab that keeps its state (#1269)

## Part A: automated

| Command | Proves |
|---|---|
| `mise run test-core` (cloud: `cargo test -p micold-core --all-targets`) | T1–T8: open or show, hide, ×, drop rules and the no-other-drop property, freshness and the changed mark, last-shown session, faces of the tab and the Changes button (FR-002, FR-003, FR-007–FR-012, FR-016, SC-002, SC-005) |
| `scripts/build-lock.sh cargo test -p micold-client --lib -- changes:: session_strip session_area app::` | U5 hidden reads (effect counts), U6 changed, U11 watch entries, U12 button, U13–U15 members, presses, widths (FR-006, FR-010, FR-011, FR-015, SC-003) |
| `scripts/build-lock.sh cargo test -p micold-client --test features_changes --test terminal_focus --test attention_view_report --test keyboard --test navigation --test idle_subscriptions --test session_area_stability` (and the lib's `app_chord_tests`) | U1 kept view whole across switches, U2–U4, U7–U10: switching, entry points, ×, removal, last session closed, focus, chords, attention (FR-004–FR-008, FR-013, FR-014, SC-001, SC-004) |
| `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot --test layout_text_overflow` | covered states `session-area-review-shown`, `-hidden`, `-sessionless` match the fixture; `session_area_fits`, `tab_children_fit`, `strip_controls_hold_their_size` with the Review member (FR-015, FR-017, SC-006) |
| `scripts/gate.sh` | the whole merge gate |

Regenerate the fixture only once the strip is done (M3), after rebasing on `origin/main`:
`UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`; only the
session-area states may change.

## Part B0: M2 check (no covered state changes in M2)

M2 adds no widget and changes no geometry the layout snapshot records (the Changes button keeps its
size; selected is a state layer). Its glue in `ui/session_area.rs` and `ui/mod.rs` is checked by
hand, light theme, 1280×800, with the M2 subset of Part B: B1 (without the tab: the review under the
header, Changes selected, tooltip *The review is shown*), B2–B4 (the way back is **Changes**), B7,
B8 (header only, no strip), B9. Record the result in the M2 PR body.

## Part B: visual pass (the `visual-pass` skill, M3)

Prerequisite: the client on a private display (`ICED_BACKEND=tiny-skia` on Xvfb in the cloud); a
project with worktree `W` (two sessions, main and a helper, one shell instance in main), worktree
`X` with one session, worktree `Z` with no session; light, then dark; window 1280×800.

| # | Do | Expect |
|---|---|---|
| B1 | Select `W`'s main session; press **Changes** | a **Review** tab appears last in the strip, before **New session**, and is shown, underlined; header and strip above the Changes view; Changes button drawn selected, tooltip *The review is shown* (US1.1, US2.3) |
| B2 | Select a file, scroll the diff, expand a hidden run, type a comment without saving, pick lines | as asked |
| B3 | Press the main session's tab, then the Review tab | the terminal; then the review exactly as left in B2 (US1.2, US1.3) |
| B4 | Press **Changes** while the Review tab is shown | nothing changes (US2.4) |
| B5 | Save two drafts; press the session tab; edit a file of `W` from the terminal | the tab reads *Review* with badge `2` and a dot; hover: *Review · 2 drafts / Changed since you looked* (US4.1, US4.3) |
| B6 | Press the Review tab | the edited file's new diff, the dot gone, the selected file and composer kept (US4.4) |
| B7 | Select `X`'s session; then **Review changes** on `W` in the sidebar | `X` shown with no Review tab; then `W` shown with its last-shown session current and its Review tab shown, kept (US1.4, US2.5) |
| B8 | **Review changes** on `Z` | `Z`'s own header; strip of only *Review* and **New session**; its review shown (US2.6) |
| B9 | Open and close the Overview; open and close Settings | the review returns unchanged (US3.4) |
| B10 | Press the Review tab's × | the tab leaves the strip; the session shown before it is shown; **Changes** again starts fresh, comments still there (US3.1, US3.2) |
| B11 | Resize to the minimum window (640×480) with the Review tab hidden, badge and dot showing; open 12 tabs at 1280×800 | label, badge, dot and × whole, nothing overlapping or clipped; the strip scrolls and showing the Review tab scrolls it into view (SC-006, edge case *Many tabs*) |
| B12 | Run every chord of `terminal-panes.md#keyboard-shortcuts` with a session tab shown, then with the Review tab shown | as before the feature; a session chord with Review shown shows that session and keeps the tab (US5.1, US5.2) |
| B13 | Repeat B1, B5, B8, B11 in dark | the same, all text legible |

Record screenshots under `specs/1269-review-tab/visual-pass/` and results in `visual-pass.md`.
