# Quickstart: 1792

## Part A: automated
- `scripts/build-lock.sh cargo test -p micold-client menu_select` and `... expand`, `... picker_parity`, `... composer` (targeted; `mise run gate` before pushing).
- Expect: trigger geometry equals `Select`'s in every state; menu parity; expand duration is the token and zero under reduce motion; two sessions independent.

## Part B: visual pass (skill `visual-pass`)
Prereq: client built, Xvfb or headless sway.
1. Showcase: `Select` and `MenuSelect` side by side, each state: rest, hover, focus, pressed, disabled, open. Judge equal look (SC-001, FR-009).
2. Client, structured session: press the settings icon. Row grows to full height over the standard duration (SC-002); press again, it collapses.
3. Toggle twice quickly: reverses with no jump (FR-007).
4. Open the model, effort and permission menus; compare with the showcase menu (US2).
5. Settings > Appearance > Reduce motion on: toggling the row switches in one step (FR-013).
6. Two sessions: expand in one; the other is unaffected (FR-008).
7. Narrow window: row wraps, stays reachable; light and dark theme.
Record the result in the feature's evidence file.
