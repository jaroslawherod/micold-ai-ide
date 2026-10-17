# TDD cycle log: 946 M1

The feature has no `tdd/test-list.md` (as 482 and 837): tasks.md's test tasks are the list, and each
row names the task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `review::anchor`, `review::reply` declared | structural, no behaviour | compiles |
| T002, T003 (core half) | `CommentState::{Draft, Sent, Reported}` store shape; `status()`; `reference()`; `CommentRef::parse`; `fresh_id`; `Move::new`; `clear_sent` keeps Drafts only; Reported refuses `set_text`/`delete` | `cargo test -p micold-core --lib review::comment` against `todo!()` stubs: 6 failed (`status_is_exactly_one_of_four`, `a_reference_is_c_…`, `a_reference_parses_…`, `fresh_id_skips_…`, `a_move_keeps_…`: `not yet implemented`; 482's `a_pending_comment_has_the_persisted_json_shape`: `{"pending":null}` expected, `{"draft":null}` written). The store-shape, `clear_sent` and Reported-refusal tests passed at declaration: the serde reprs, the `Reported` arm the compiler demanded in `pending_index`, and 482's `retain(Draft)` already gave them (not red-first) | `cargo test -p micold-core --lib review::`: 98 passed. 482's JSON-shape test renamed and updated to `{"draft":null}`: the spec changes the stored name (data-model § store), not a weakened assertion |
