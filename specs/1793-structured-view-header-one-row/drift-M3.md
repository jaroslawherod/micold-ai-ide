# Plan drift, M3

- crates/micold-client/src/{app.rs,features/header_bar.rs,features/session.rs}: `header_bar::State::follow` also keys on the active session's shell count, so choosing a `+` entry closes the menu without a cross-feature outcome (the registration-cost and isolation gates forbid one); `State::active_shell_count` supplies it.
- crates/micold-client/src/overlay/registry.rs: the `+` menu is a registered popover (architecture guard overlay_registry).
- crates/micold-client/src/ui/terminal.rs: the strip's switch offer helpers lost their only caller with the `Open in terminal` slot and are removed with their tests.
- tests/support/covered_states.rs, tests/layout_snapshot.rs, tests/fixtures/layout_snapshot.txt, tests/gates/*, tests/popover_displacement.rs: the always-present `+` menu surface adds two layers, so dialog layer paths re-point and the snapshot is regenerated; strip anchors changed (status, split; one touch-checked control).
- tests/terminal_tabs.rs, tests/terminal_focus.rs, tests/features_*: the `+` is now the last strip member and presses toggle the menu.
- docs/user-guide/terminal-panes.md: the removed icon and switch slot.
