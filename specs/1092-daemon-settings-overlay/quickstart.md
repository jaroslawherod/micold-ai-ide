# Quickstart: validating the daemon overlay

Prerequisites: `mise trust`, then `mise run gate`.

## Part A: automated

1. `mise run test` (or `cargo test -p micold-client` through mise): `features_daemons`, `overlay_registry`,
   `popover_displacement`, `surface_registration_cost`, `daemons_section_geometry` and the new dialog tests pass.
2. Page-save independence: the dedicated test saves a daemon with a page-level edit pending and asserts the page's draft and its save-enabled state are unchanged (FR-007, SC-004).

## Part B: visual pass (`visual-pass` skill, real GUI)

1. Settings > Daemons: no form inline; Add daemon opens a dialog with a scrim; the page does not react to clicks.
2. Edit a daemon: fields filled; change the name; Save daemon: dialog closes, row shows the new name; page Save untouched (SC-001).
3. Edit then Cancel, Escape, outside click in turn: stored values unchanged on reopen (SC-002).
4. Empty the name, Save daemon: dialog stays, error beside the field.
5. Edit a running container daemon's runtime: Save daemon shows the confirmation inside the dialog; No (and Escape) keeps edits; Yes closes.
6. Details: dialog with errors/log, a new log line appears live; Close, Escape and outside click close it; the row has no Hide details.
7. Keyboard: Tab cycles inside the dialog; focus returns to the opening button on close.
8. Resize to the smallest window: form scrolls, Save daemon and Cancel stay visible (SC-005).
9. Light and dark theme: scrim, surface and action row look right.
