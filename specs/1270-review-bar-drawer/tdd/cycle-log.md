# TDD cycle log: 1270 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `review::{bar, drawer}` modules and the two test files | structural | compiles |
| T002, T003 | `review_bar::{HEIGHT, SEND_WIDTH}`, `comments_drawer::WIDTH` in `anatomy::ALL` | `cargo test -p micold-core --test tokens_anatomy`: `error[E0433]: cannot find review_bar in anatomy` (4), `cannot find comments_drawer in anatomy` (2) | `tokens_anatomy` 22 passed; client `anatomy_call_sites` 9 passed with the three recorded as not built (ledger D13) |
| T004, T006 | `ScopeCounts::of` per scope, `is_empty`, `reported_or_sent`; `counts_text` in each `CountsForm` | `cargo test -p micold-core --test review_bar`: did not compile, `unresolved imports micold_core::review::bar::{counts_text, …, ScopeCounts, SendButton}` | `review_bar` 17 passed |
| T005, T007 | `send_button` labels, 999+ cap, Sending…, not sendable, `primary == enabled`; `error_text` | same red run (`send_button`, `error_text`, `SendButton` unresolved) | `review_bar` 17 passed |
| T017, T018 | `overflow`: Clear sent on sent/applied/answered, Discard drafts on drafts without a send, per scope | same red run (`overflow`, `Overflow` unresolved) | `review_bar` 17 passed |
| T023, T025 | `DrawerTab::{of, label, empty_text}`, `START_HINT`, `tab_counts`, SC-006 property over 4 statuses × 2 scopes, `rows_of` order, `width` | `cargo test -p micold-core --test review_drawer`: did not compile, `unresolved imports micold_core::review::drawer::{newly_reported, rows_of, tab_counts, width, Drawer, DrawerEvent, DrawerTab, SendWatch, START_HINT}` | `review_drawer` 23 passed |
| T024, T026 | `Drawer` default, `apply` for Toggle/Close/Escape/SelectTab/Scrolled/SendStarted/SendFailed/Reported, `newly_reported` | same red run | `review_drawer` 23 passed |
| T044, T045 | `card_preview`: ≤ 4 lines whole, wrapped lines counted, > 4 cut with `…`, 4,000-char cap sets `cut`, control characters via `display()` | `cargo test -p micold-core --lib review::reply`: `error[E0425]: cannot find function card_preview` (6), `cannot find value CARD_LINES` | `review::reply` 7 passed |

## M2

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T046, T047 | `LineClamped::when_cut`: element under a cut text, no height and not drawn when whole | `cargo test -p micold-material --lib line_clamp`: `error[E0599]: no method named when_cut found for struct LineClamped<M>` (3) | `line_clamp` 13 passed |
| T008, T009 | `ReviewBar`: longest fitting `CountsForm` at three widths, trailing group bounds identical across labels/counts/error/drawer, widest label fits `SEND_WIDTH`, `HEIGHT`, × and controls emit | `cargo test -p micold-client --lib --no-run`: `cannot find type ReviewBar` (7), `SendButton`, `width_of`, `ROLE`; then `the_widest_label_fits_the_send_width`: `left: 176.0 right: 169.0` | `SEND_WIDTH = 169`; `review_bar` 7 passed |
| T027, T028 | `SideSheet`: `min(360, half)` at the right edge, title/×/content inside, × emits, press on the panel captured | same red run: `cannot find type SideSheet` (2) | `side_sheet` 4 passed |
| T029, T030 | `ReviewPanelRow` whole-reply mode: wrapped whole reply, Show all only when shortened, Mark applied / Answer… only on a Sent row with handlers | same red run: `no method whole_reply`, `on_mark_applied`; `no field whole_reply / show_all on Parts` | `review_panel_row` 8 passed |
| T048, T049 | Card reply through `LineClamped` (4 lines), Show all when cut or shortened, none for a short reply | `a_reply_over_four_lines_is_cut_there_and_offers_show_all` failed (`reply.content.ends_with('…')`, review_comment.rs:437) before the card change | `review_comment` 13 passed; `ui::material::` 613 passed |
| T010, T031, T050 | showcase `review_bar`, `side_sheet`, card and row poses | structural | `showcase_completeness` 46, `showcase_captions` 7, `material_boundary` 6 passed |
| T052 | layout snapshot | — | `layout_snapshot` 112 passed unchanged: no covered state shows a card with a long reply, so no re-record |

## M3

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T011, T012 | `review_bar`: none on a session tab, shown scope's counts, `send_button`/`overflow` from core, files browser keeps the scope; `SendFailed` sets the error on its entry's view (shown or hidden), next send or `SendErrorDismissed` clears it | `cargo test -p micold-client --test features_changes --no-run`: 32 errors, `unresolved imports … review_bar, ReviewBarModel, ReviewDrawer, ComposerPlace, drawer_rows, drawer_counts`, `no variant SendFailed / SendErrorDismissed` | `features_changes` 151 passed |
| T013, T014 | a failed `ReviewSend` answer reaches `Msg::SendFailed` of its entry and keeps the error snackbar; a delivered one keeps the success snackbar | `PendingOp::ReviewSend` had no `project`/`worktree_dir` fields (the module did not compile) | `shell::daemon_sync::tests::review_send` 2 passed (3 with the row below) |
| T019, T021 | `OverflowToggled`/`OverflowDismissed`; Clear sent and Discard drafts… close the menu | same red run: `no variant OverflowToggled` (5), `OverflowDismissed` (3), `no field overflow_open` (4) | `features_changes` 151 passed |
| T020 | `review_overflow` is a registered `Popover`; outside click, Esc and scroll beneath send `OverflowDismissed`; a dialog over it takes Esc first | `cargo test --test overlay_registry --no-run`: 3 errors (`overflow_open`, `OverflowDismissed`) | `overlay_registry` 25 passed; `popover_displacement` 4 passed with the `review_overflow` row |
| T032, T034 | `Msg::Drawer` / `DrawerScrolled` on the shown view's drawer; drawer and `ComposerPlace::Drawer` text survive switching and closing, dropped with the tab; `review_bar.drawer_open` follows | same red run: `no variant Drawer` (13), `DrawerScrolled` (2), `DrawerEdit`, `DrawerAnswer`, `no field drawer`, `no field place` | `features_changes` 151 passed |
| T033, T035 | Esc: overflow menu, discard dialog, Show all dialog first; then the drawer; closed drawer → `None`; the Overview before the drawer | `cargo test --test overlay_dispatch_ordering --no-run`: 5 errors (`no variant Drawer`, `no field drawer`) | `overlay_dispatch_ordering` passed |
| T036, T037 | `drawer_rows(state, tab)` order and fields (`markable`, whole reply to `CARD_CHARS`, `has_full_reply` past it), `drawer_counts`; `DrawerRowSelected` jump and reason; a push drops a gone focus and its Show all dialog | same red run: `cannot find type DrawerRow`, `no variant DrawerRowSelected` (3) | `features_changes` 151 passed; `features_pr_changes` 81 passed (`drawer_groups`) |
| T038, T039 | `SendPressed` starts the watch with the sent drafts; first report opens on Replies once; another window's send, a hidden tab and a failed send open nothing | same red run (`drawer` field, `SendWatch` reached through it) | `features_changes` 151 passed |
| T040 | user guide "The review panel": closed at first, Esc, tabs, auto-open | — (docs) | — |
| Review A F1 | a send that never left (no daemon) fails in the bar and drops the watch | not run red: added with the fix of review A M3 F1, which found the gap by reading (`send_op` returned nothing for an unroutable send) | `review_send::a_send_with_no_daemon_fails_in_the_bar_and_drops_the_watch` passed in the full gate at 5358a2b7 |

## M4

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T015, T041 | `gates/review_bar_fixed.rs`: the bar `HEIGHT` high directly above the status bar at the tab's width; Send review's bounds equal across 0/99 drafts, sending, an error and the drawer open (SC-001); the diff pane's bounds and painted lines equal with the drawer open (SC-002); the diff area spans the body beside the list, at least the old diff + review column (SC-004); the drawer `drawer::width` wide at the area's right edge, down to the bar, its left edge right of the area's middle (FR-010, FR-017) | the gate run against M3's `ui/changes.rs` and `ui/mod.rs` (everything else of M4 in place): `review_bar_fixed` 6 failed, `the Review tab lays out the bar at [0, 0, 1, 1, 2, 2]`, `… the diff area at [0, 0, 1, 1, 2, 1, 1]` (no bar, no stacked diff area) | `review_bar_fixed` 6 passed after T016, T042; the first green run found the drawer 24 px short of the bar (the column's gap), fixed by moving the gap into the header's padding |
| T016, T022, T042, T043 | Changes header without Review (n)/Send review; `ReviewBar` wired (Send, ×, ⋮, Comments); overflow `MenuOverlay` hung from the ⋮ (`overflow_trigger_right`, unit-tested against the laid-out ⋮); `SideSheet` stacked over the diff area with the Drafts/Replies `TabStrip`, rows (whole reply on Replies, `DrawerEdit`/`DrawerAnswer` composer in the row), empty texts and `START_HINT` | as above; `the_overflow_trigger_ends_where_the_menu_hangs_from` written with `overflow_trigger_right` | `layout_snapshot` 132 passed; `review_bar` unit tests passed |
| T053, T054 | covered states `review-bar`, `drawer-drafts`, `drawer-replies`, `overflow` | `the_layout_matches_the_committed_fixture` failed (`session-area-review-shown` changed, four new states unrecorded); `containment` flagged the drawer's rows overhanging their `Scrollable` (recorded as `DRAWER_ROWS_CONTENT`) | re-recorded: only `session-area-review-shown`, `session-area-review-sessionless`, `status-bar-changes` (all show a Review tab) changed, four states added |
| T055 | user guide: *Sending a review* (the bar), *The comments drawer*, *Clearing and discarding* (overflow, what is shown) | — (docs) | — |
| T056 | visual pass 1280×800, light and dark | — | `visual-pass-m4.md` |
| Review A F1 | a drawer composer is drawn in its row only while that row is shown (drawer open on its tab), on its card otherwise (`composer_in_drawer`) | `cargo test --test features_changes --no-run`: `cannot find function composer_in_drawer in module changes` | `features_changes` 153 passed |
| Review B F1 | the drawer's list scrolls back to its tab's offset whenever it comes into view (opened, tab switched, Review tab shown again): `Effect::ScrollDrawer` → `scroll_to(DRAWER_SCROLL_ID)` | same red run: `no variant named ScrollDrawer found for enum Effect` | `features_changes` 153 passed (two 1269 tests now expect the scroll as the only effect of re-showing a review whose drawer was left open) |
| Review A r2 F1 | a row's Edit / Answer… jumps to the comment first, so its card exists for the composer while the drawer is closed | `edit_or_answer_in_a_drawer_row_selects_the_comments_file` failed: `Edit jumped`, left `Some(RelPath("b.rs"))` | `features_changes` 154 passed |
