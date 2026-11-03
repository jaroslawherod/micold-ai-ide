# Quickstart

## Part A: automated
1. `mise run test-core` (visibility and settings tests).
2. `scripts/build-lock.sh cargo test -p micold-client --test main -- transcript_entry_view transcript_selection_parity` and the reducer/persist tests by name.
3. `mise run gate` before pushing.

## Part B: visual pass (showcase or client)
1. Open a structured session whose transcript has a session start, a raw event, an orphan result, an ok footer and an error footer between messages.
2. Toggle off (default): only messages and the error footer remain, plus "N events hidden" with a Show button; no JSON is visible.
3. Click Show (or the header toggle): every low-signal entry is one summary line, in order. Expand a raw one: original text appears, scrolls if long. Collapse it.
4. Restart the app: the toggle is as left. Resume a saved session: same behaviour.
5. Select across the transcript with the toggle off and copy: no hidden text in the clipboard.

## Part B result (close unit, 2026-11-03)

Not run as a visual pass. The host has no Xvfb or xdotool; headless sway and grim exist, but the client cannot open a seeded transcript from the command line and the component showcase renders only `show_raw: true` samples (no toggle, no hidden notice). No screenshots were taken. Steps covered by automated tests instead:

1. Default hidden state, 2. hidden notice with Show button, 3. Show and expand: `transcript_entry_view` (including the new press tests for the header toggle and the Show button).
4. Restart and resume persistence: the persist and reducer tests in `main_tests.rs`.
5. Selection and copy with nothing hidden: `transcript_selection_parity`.
