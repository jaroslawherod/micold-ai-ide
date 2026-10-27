# Cycle log — M1 (one-line header)

Honest note: the gates and the first implementation were written in one pass, so there is no commit
that shows a gate failing against the old header. The red runs below are the failing runs observed
while the layout was being fixed; each failed for the stated reason and went green as stated.

| Behaviour | Red (observed) | Green |
|---|---|---|
| `the_header_is_one_line_with_a_compact_changes_button` (FR-001, FR-003, SC-003) | Changes button 28 dp against chips 15.6 dp: the first assertion ("no taller than the chips") was unreachable; the assertion became "no taller than the title line" (user-accepted Q2) | green in `layout_snapshot` after the change |
| `the_title_shortens_before_the_chips_and_the_button` (FR-002, H10') | Flat row at 640 px: Changes laid out 134.8 wide against 251.5 asked while the title kept 5.7 px; and with the button first the chips went to 0 wide | green with the H10' reading chosen by the user (Q1): label cut after the title |
| Existing fit gates (H10) at 640 px | button 10.1 and 134.8 wide against 251.5 and 227.6 asked | green with H10' |

Final: `cargo test -p micold-client --test layout_snapshot`: 152 passed.
