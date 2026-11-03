# Quickstart

## Part A: automated
1. `mise run test-core` (visibility and settings tests).
2. `scripts/build-lock.sh cargo test -p micold-client --test transcript_entry_view --test transcript_selection_parity` and the reducer/persist tests by name.
3. `mise run gate` before pushing.

## Part B: visual pass (showcase or client)
1. Open a structured session whose transcript has a session start, a raw event, an orphan result, an ok footer and an error footer between messages.
2. Toggle off (default): only messages and the error footer remain, plus "N events hidden" with a Show button; no JSON is visible.
3. Click Show (or the header toggle): every low-signal entry is one summary line, in order. Expand a raw one: original text appears, scrolls if long. Collapse it.
4. Restart the app: the toggle is as left. Resume a saved session: same behaviour.
5. Select across the transcript with the toggle off and copy: no hidden text in the clipboard.
