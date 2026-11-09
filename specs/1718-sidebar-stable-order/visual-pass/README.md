# Visual pass, feature 1718 (quickstart Part B steps 1 and 3), 2026-11-09

Surface: component showcase, StateIcon section (the five-state worktree rows built as the sidebar builds
them), `micold-showcase` built from HEAD 233ff1237, private headless sway + lavapipe, driven with `vptr.py`.
Not driven: the real client (step 2) and step 4 (arrow keys / reveal), which need a live daemon.
Dark theme: the showcase scheme button did not respond to the virtual pointer clicks, so the dark shots
come from a throwaway export of HEAD whose only change is the starting scheme (`ColorScheme::Dark`,
state.rs:211), built outside the worktree; the repository is untouched.

| Image | Theme | Shows | Result |
|---|---|---|---|
| light-states.png | light | the five states plus selected and tagged poses | PASS |
| dark-states.png | dark | same | PASS |
| light-hover.png | light | rest/hover pairs: Needs action, Done, Needs action selected | PASS |
| dark-hover.png | dark | same | PASS |

Checks (same verdict in both themes):
- Five distinct glyphs (exclamation, eye, play triangle, check circle, grid/equals): PASS
- Needs action and Needs attention tinted (red-family, action louder), Done tinted and quietest: PASS
- Working and Idle untinted: PASS
- Hover over a tinted row draws the state layer over the tint (tint stays visible, shifts tone): PASS
- Selected tinted rows (Needs action, Done) drop their tint and show the selection colour: PASS (by design)
- Row geometry identical across states (tint is colour only): PASS
