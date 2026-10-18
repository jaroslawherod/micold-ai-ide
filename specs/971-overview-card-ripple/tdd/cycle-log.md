# TDD cycle log — M2 (keyboard ripple)

Tests and code were written in one pass; the red phase was proven afterwards by disabling the
feature (`if false && self.centred_on_keys` in `Ripple::update`, the behaviour of `origin/main`,
where no key starts a ripple) and re-running `cargo test -p micold-client --lib -- ripple overview_card`.

| Behaviour | Red (feature off) | Green |
|---|---|---|
| Enter/Space on a focused card ripples from the centre, every state (T016, SC-002) | `enter_and_space_on_the_focused_card_ripple_from_the_centre` FAILED | ok |
| The keyboard ripple fades to idle (T016, FR-004/005) | `the_keyboard_ripple_fades_to_idle` FAILED | ok |
| A captured activation key presses at the centre (T017) | `a_captured_activation_key_presses_at_the_centre` FAILED | ok |
| A repeated key does not restart it (T017, FR-005) | `a_repeated_key_does_not_restart_the_ripple` FAILED | ok |
| Unfocused card, no builder, uncaptured key, auto-repeat event start nothing (US2-3, T017) | pass (negative cases; guards by construction) | ok |

Red result: 4 failed, 45 passed. Green: 49 passed, 0 failed. FR-004's "only state layer" for the
keyboard ripple rests on M1's `beneath_a_ripple_the_surface_draws_no_layer_and_reports_the_heaviest`
and the card tests, since the keyboard ripple runs through the same `RippleState`, draw path and
`beneath_ripple` scope as the pointer one.
