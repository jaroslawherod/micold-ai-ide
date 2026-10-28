# M2 visual pass: Add attachment button (#1464)

Build: micold-showcase at 29fff0102, private headless sway + lavapipe. Entry is now a pose group of `AttachmentChip`
("Add attachment, enabled/disabled"), not a separate `AddAttachmentButton` entry.

Result: PASS for what could be observed; focus pose and real composer row NOT verified (see below).

- m2-showcase-section.png: paperclip glyph in a round icon button; enabled is dark, disabled is grey. Distinct.
- m2-showcase-rest.png / m2-showcase-hover.png: hover adds a tinted circle state layer (466 px differ) and the tooltip "Add attachment" above it. Reads sensibly.
- Focus ring: not captured. Headless sway has no keyboard seat (capabilities 1 = pointer), so Tab/wtype reach nothing. Check by eye.
- Composer row at minimum width: not rendered (needs the full client with a live session). Check by eye or layout snapshot.
- Observation: in the showcase gallery the two Add attachment poses wrap onto different rows with the chips (gallery layout, not the composer).
