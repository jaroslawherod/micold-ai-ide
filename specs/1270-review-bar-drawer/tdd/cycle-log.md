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
