# Visual pass: quickstart Part B step 6 (large editor), feature 1703

Real client (`micold-ai-ide` + daemon, same build, debug) on a private headless sway (1600x1200, lavapipe),
structured Claude session in a scratch git project. The showcase was not used: its large-editor entry is
a static dialog body (`Message::NoOp`), so it cannot show focus/hover, the chord or Done/Esc.
Keys by `wtype`, pointer by `vptr.py`. (wtype drops the first keystrokes of a burst, hence `ellocond` in the draft; not an app defect.)

| # | Observation | Result | Evidence |
|---|---|---|---|
| 1 | Empty draft, unfocused, unhovered: no open-editor icon | PASS | 01-unfocused-empty.png |
| 2 | Hovered: icon at text area top-right | PASS | 02-hovered-empty.png |
| 3 | Focused single line: icon shown | PASS | 03-focused-oneline.png |
| 4 | Single-line draft, unfocused, unhovered: icon hidden | PASS | 04-oneline-unfocused.png |
| 5 | Two lines, unfocused, unhovered: icon shown | PASS | 05-, 06-, 07-twoline-*.png (07 shows the icon) |
| 6 | Ctrl+Shift+M (cursor placed at start of text) opens the modal with the draft; typing lands at the end | PASS | 08-modal-open.png, 09-modal-typed-at-end.png |
| 7 | Done writes the edited text to the draft | PASS | 10-after-done.png |
| 8 | Icon click opens the same editor (hover state shown on icon) | PASS | 11-modal-via-icon-small.png, 12-after-esc.png |
| 9 | Esc closes and leaves the draft unchanged (typed "QQ" discarded) | PASS | 12-after-esc.png |
| 10 | Ctrl+Enter closes and commits ("CE" appended) | PASS | 13-after-ctrl-enter.png |
| 11 | Cancel closes and leaves the draft unchanged (typed "XX" discarded) | PASS | 14-after-cancel.png |
| 12 | `/shortcuts` sheet lists "Ctrl+Shift+M  Open large editor" in the Composer group (Linux form; the Cmd form is in `COMPOSER_KEYS`, not checkable on Linux) | PASS | 15-shortcut-sheet-small.png |

No defects found. Not checked: macOS Cmd forms, two-session isolation (AC5), empty-draft and already-open edge cases.
