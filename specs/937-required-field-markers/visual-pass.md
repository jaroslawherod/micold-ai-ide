# Visual pass — 937 required-field markers (2026-10-18)

Run by the close unit on the debug `micold-showcase` (tiny-skia, private Xvfb, built from the
final tree, gate green), driven with xdotool, screenshots read by eye. Partial, stated honestly.

| Check (quickstart Part B) | Result |
|---|---|
| B1 showcase: `RequiredLegend` entry shows `* required` (muted) | seen, light |
| B1/B3 showcase "Run in parallel" dialog pose, empty and invalid: `Prompt*` and `Name*` in error with "This field is required" in red, both at once; `Type*` and `Base branch*` marked and filled; `Ticket` unmarked with its "Optional" hint | seen, light |
| B1 greyscale (screenshot desaturated): marker, legend and message still read | seen (SC-005) |
| B1 dark theme; the field-anatomy entries (resting and floated `*`) | not run: the 60-ish-click scroll to them is unreliable under software rendering |
| B2 add worktree, B4 rename, B5 Settings pages and daemon form, B6 two windows | not driven in the live client (no input path for it in this container); covered by tests: `features_worktree_form`, `features_runs`, `features_settings`, `features_daemons`, `required_blur` (including the two-window and reopen cases), `required_call_sites`, `layout_snapshot` |

No defect found in what was seen.
