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
