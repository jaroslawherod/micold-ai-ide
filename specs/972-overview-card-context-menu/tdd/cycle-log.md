# TDD cycle log: 972 (M1–M3)

The feature has no `tdd/test-list.md` (as 482, 837 and 946): tasks.md's test tasks are the list,
and each row names the task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001, T002 | `Icon::Open` is `login` (U+EA77), in `Icon::ALL` (66), distinct, in the shipped font | `cargo test -p micold-client --test icons --test icons_font`: `error[E0599]: no variant, associated function, or constant named `Open` found for enum `Icon`` | `icons` 5 passed, `icons_font` 10 passed |
| T003 | `location_menu_items` extracted from the `worktree_menu` closure | structural, behaviour-preserving | `worktree_menu_pull_request`, `features_sidebar`, `overview`, `features_worktree` green unchanged (10, 85, 44, 3 passed) |
| T004, T005 (red only) | reducer: open/replace/dismiss/entry/scroll/in-place/navigating/Open; composition: *Open* + row items, wrapped messages, settle | `cargo test -p micold-client --test overview_card_menu`: did not compile, `unresolved imports ... CardMenu, MenuOrigin, OverviewCardMenu`, `unresolved import ... card_menu_items`, `no variant named CardMenuOpened`, `no field card_menu` | pending (T009, T010) |
