# Quickstart: validating Structured View — Select and Copy Transcript Text (#1550)

No prerequisite feature is unmerged (research R13). Test files marked *(new)* are created by the
milestones in [plan.md](plan.md) *Delivery*.

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Selection model | `cargo test -p micold-core --test structured_selection` *(new)* and `--lib structured::selection` | S-1–S-5, S-11 of [contracts/selection-model.md](contracts/selection-model.md): order, exact text, shown markdown text (no markers, link text, tab-separated cells), grapheme boundaries on CJK/emoji/combining fixtures, stability under appended and grown entries, word/line granularity |
| Shown text helpers | `cargo test -p micold-core --lib markdown structured::transcript::summary` | `shown_text`, `list_marker`, `work_value` (uncut, multi-line, `None` for tools without a work field) |
| Window state | `cargo test -p micold-client --test features_transcript_selection` *(new)* | S-6–S-10, S-12: clearing rules, isolation across two sessions and two windows, no clipboard outcome on release or click, select-all, copy emits `ClipboardWriteChecked` with the selection text and a failed read-back sets the pane notice |
| Copy keys | `cargo test -p micold-client --test features_composer` | K-1–K-11 of [contracts/keys-menu-copy.md](contracts/keys-menu-copy.md) with `mac` both ways (SC-002) |
| Copy controls | `cargo test -p micold-client --test features_transcript_rich` | C-1–C-6: each target's clipboard text equals the held value byte for byte, streaming value so far, empty value, confirmation only on the activated control and window (SC-004) |
| View parity | `cargo test -p micold-client --test transcript_selection_parity` *(new)* | every entry kind through `RICH` and `PLAIN`: the drawn `(TextPath, text)` list equals `units()` (SC-001) |
| Shared components | `cargo test -p micold-material` and `cargo test -p micold-client --test showcase_completeness --test material_builder_api` | T-1–T-6, A-1–A-5 of [contracts/selectable-text.md](contracts/selectable-text.md) by simulated events; builders and showcase entries; unselected drawing pixel-equal to the plain leaf |
| Contrast | `cargo test -p micold-client --lib ui::material::composition_contrast` and `cargo test -p micold-core --test tokens_contrast` | `on_primary_container` on `primary_container` ≥ 4.5:1 in both schemes, composed on every transcript surface (FR-020, SC-006) |
| Budget | `cargo test --release -p micold-client --test transcript_open_budget` | the selection variant: select-all + copy at the event bound < 1 s; one drag step < 100 ms (SC-003) |
| Everything | `scripts/gate.sh` | green |

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` on `PATH`; the dev client with a structured session. Run on
Linux and, from the CI-built clients, macOS and Windows (Principle VI).

1. Ask for a reply with a heading, bold text, a link, a bulleted list, a table and a `rust` code
   block, then have Claude run `ls` and `cat` a file. Drag from the middle of the reply's first
   paragraph, through the Bash card, into the next reply: the highlight follows the pointer across
   entries; press Ctrl+C (Cmd+C) and paste into an editor: the text as shown, one line break between
   entries, no `**`, no link target, cells tab-separated, the folded card's header only (US1, FR-010,
   FR-011).
2. Drag below the bottom edge and hold: the transcript scrolls and the selection grows (FR-003).
3. Click once: the selection clears and the clipboard still holds step 1's text (FR-004, FR-012).
4. Double-click a word, triple-click a wrapped prose line and a code line; Ctrl+A in the transcript;
   right-click a selection and choose Copy; right-click with nothing selected: Copy is disabled
   (US4).
5. Start a long turn; select text and press Ctrl+C: copied, turn still running; click to clear and
   press Ctrl+C: interrupted (US2, SC-002). Select, click into the composer, press Ctrl+C: the
   composer's rule applies; right-click the transcript → Copy still copies the selection (FR-019a).
6. Copy controls: a Bash card's header (full command), an unfolded card's input and result, an Edit
   diff's path; each by pointer and by Tab + Enter; each shows "Copied" only on itself (US3).
7. Select a span of a partial reply while it streams: it stays highlighted and unchanged (SC-005).
8. Light and dark: selections on prose, code surface, diff lines and a failed card are clearly
   visible and readable (FR-020, SC-006). Select CJK and emoji text: no broken characters.
9. Two panes on two sessions, and two windows on one session: each keeps its own selection; a copy
   key acts only on the focused pane (Principle II).

Record the pass with the `visual-pass` skill into `specs/1550-transcript-text-selection/visual-pass/`.
