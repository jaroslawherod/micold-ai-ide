# Quickstart: Keyboard Map for Reviewing Changes (#1274)

How to show the feature works. Rules: [contracts/keys.md](./contracts/keys.md),
[contracts/jump-to.md](./contracts/jump-to.md); shapes: [data-model.md](./data-model.md).

## Prerequisites

- M1: nothing beyond the workspace (`mise trust` once in a fresh worktree).
- M3, M4: #1269, #1270, #1271 and #1273 merged. M5: #1127 merged.

## §A — Automated

| Milestone | Command | Expect |
|---|---|---|
| M1 | `mise run test-core` | `review::keys`, `review::navigate`, `review::jump`, `review::diff::header_rows` and `quick_switch` tests green; 968's existing `quick_switch` tests unchanged and green |
| M2 | `mise run gate` | `review::keys` (incl. `single_keys` off, `sheet_rows`, `key_hint`) and `settings` (`review_single_keys` default, missing key, round trip) tests green; `tests/settings_issue_mapping.rs` lists the new field |
| M3–M5 | `mise run gate` | `tests/keymap.rs`, `tests/review_keys.rs` (new), `tests/quick_switch.rs`, `tests/overlay_registry.rs`, docs test, layout snapshot and its gates green |

## §B — Recorded pass on Linux (visual-pass, 1280×800, light and dark)

Fixture: an entry with `a.rs`, `b.rs`, `c.rs` (two hunks each), `d.bin` (binary), two files marked
reviewed, and comments on `a.rs:3`, `a.rs:20`, `c.rs:7` plus one outdated comment.

| # | Do | See |
|---|---|---|
| B1 | Review tab shown, `a.rs` selected; `j`, `j`, `k` | `b.rs`, `c.rs`, `b.rs` selected in turn, row in view, diff from the top |
| B2 | `n`, `n`, `n` | first hunk, second hunk of `b.rs` marked current at the top; then `c.rs` first hunk (`d.bin` passed over) |
| B3 | `p` on the first hunk of a file | previous file with hunks, its last hunk current |
| B4 | the current-hunk mark | visible in both themes (FR-005) |
| B5 | `v` twice | mark set, count +1; mark cleared, count −1 |
| B6 | `g` repeatedly | unreviewed files in file order, wrapping; all marked → "All files reviewed" |
| B7 | `]` ×4, `[` | each card in turn, `c.rs` selected for the third; outdated one opens the drawer; wrap; `[` goes back |
| B8 | `c` with picked lines / with a current hunk / with neither | composer on the picked lines / on the hunk's changed lines / note "Pick lines or move to a hunk (n) to comment" |
| B9 | in the composer type `j`, then Ctrl+Shift+Enter | `j` in the text; saved drafts sent, composer open with its text |
| B10 | Ctrl+Shift+Enter with two drafts, outside the composer | sent as **Send review (2)** sends |
| B11 | `?`; `j`; Escape; overflow → **Keyboard shortcuts**; `?` | sheet lists all of `REVIEW_KEYS` in the platform's form; `j` does nothing; closes; opens; closes |
| B12 | hover previous/next hunk, **Mark reviewed**, **Send review** | tooltips name `p`/`n`, `v`, the chord |
| B13 | focus the tree filter and type `jk` | text in the filter, selection unchanged |
| B14 | (M5) jump-to field: empty query; `tests`; a comment word; choose each | review files and comments after locations/sessions, before projects; file selected; card shown |
| B15 | layout snapshot states `review-current-hunk`, `review-shortcut-sheet` and `review-shortcut-sheet-single-keys-off` | recorded and reviewed (FR-021) |
| B16 | Settings → Appearance: clear **Single-key shortcuts in the Review tab**, Save; in the Review tab press `j`, `v`; hover **Mark reviewed**; restart | (M3) `j` and `v` do nothing, the tooltip names no key, still off after restart; (M4) also `?` does nothing, Ctrl+Shift+Enter with a draft sends, overflow → **Keyboard shortcuts** lists only the two chords and "Single-key shortcuts are off (Settings → Appearance)" |

## §C — macOS and Windows

- macOS: Cmd+Shift+Enter sends; Ctrl+Shift+Enter does not; sheet and tooltips show `Cmd+…`.
- Windows with a German layout: `[`, `]` (AltGr) and `?` (Shift+ß) act; Ctrl+J and Alt+J do nothing.
- Both: a focused terminal receives every one of the review keys unchanged.

### §C results (2026-10-26, close unit)

No macOS or Windows machine was available for a manual run; this container is Linux only. Each
check is recorded from the CI suite instead: PR #1460's run
([40280111614](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/40280111614)) passed
**build + test** on `macos-latest` and `windows-latest` (as on `ubuntu-latest`; read from the PR's
check runs, 2026-10-26), and these are the
tests that carry each check there.

| Check | Carried by | Manual run |
|---|---|---|
| macOS: Cmd+Shift+Enter sends, Ctrl+Shift+Enter does not | `keymap.rs::the_primary_modifier_is_cmd_on_macos_and_ctrl_elsewhere` (its `cfg!(target_os = "macos")` arm runs on the macOS job: logo is primary, Ctrl is not); core `the_send_chord_needs_primary_and_shift_and_nothing_else`; `review_keys.rs::the_send_chord_sends_as_send_review_does` (its `chord()` is Cmd+Shift+Enter on macOS) | not run |
| macOS: sheet and tooltips show `Cmd+…` | core `labels_give_the_linux_windows_and_the_macos_form`; `review_keys.rs::with_the_switch_off_the_send_chord_still_sends_and_is_named` and `with_the_switch_off_the_sheet_lists_only_the_chords` pick the macOS label on the macOS job | not run |
| Windows, German layout: `[` `]` with AltGr and `?` with Shift+ß act | core `ctrl_alt_or_logo_alone_refuses_a_single_key_but_altgr_does_not` (Ctrl+Alt without logo, as Windows reports AltGr, with `[` acts) and `shift_is_part_of_the_character_not_a_modifier` (`?` with Shift acts: the key is the produced character, whatever the layout); `keymap.rs::a_review_press_carries_the_produced_text_and_every_modifier` | not run: no German layout in CI |
| Windows: Ctrl+J and Alt+J do nothing | core `ctrl_alt_or_logo_alone_refuses_a_single_key_but_altgr_does_not` | not run |
| Both: a focused terminal receives every review key unchanged | `review_keys.rs::a_focused_terminal_keeps_every_review_key` (added at close: no review key resolves while the terminal has the keyboard; the Review tab and the terminal never share the screen, `State::terminal_focused`); `keymap.rs::plain_ctrl_brackets_still_reach_the_terminal` | not run |

Open: a manual pass on a Mac and on Windows with a German layout. The CI evidence covers the
decision rules; it does not cover the OS delivering AltGr or Shift+ß as those modifiers and text.
