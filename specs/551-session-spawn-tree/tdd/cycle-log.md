# TDD cycle log: 551 session spawn tree

Append only.

## Baseline

- Commit f602c37c. Suite not re-run at planning; the profile records green (5071 passed) at cdc473ab.

## M1 (2026-10-12)

- Tests written first for A5, A6, A7 and U1-U11 (core: origin.rs unit tests, store_roundtrip,
  protocol_roundtrip, mcp_tools_catalog; daemon: mcp_create_session, catalog_adoption,
  session_origin_removal), then the implementation of T003, T005, T007, T012-T016.
- Green: `mise run test-core` and the three daemon test targets. `schema_hash` pinned to protocol 41.
- Spike T025: no provider confirmed against a recorded fixture; every provider a gap (research R2).
- Refactor: none beyond sharing `validate_origin` between record and create paths.
- Red evidence: the behaviours were written in one WIP commit, so no per-behaviour red output was
  captured. Each M1 test references symbols absent on origin/main (`origin::validate_origin`,
  `StoredSession.origin`, `SessionSummary.origin`, `Catalog::record_origin`, the `create_session`
  `origin` argument), so against origin/main sources it fails to compile: red by absence, not by
  a recorded run. Not demonstrated further; `tdd-verify` in the close unit may re-run it.
- Spike T025 yields no fixture directory by design: every provider is a recorded gap, so its Verify
  reads "R2 marks each provider confirmed or gap".

- Correction after the rebase onto feature 776: `PROTOCOL_VERSION` is 42 (not 41) and `schema_hash` is pinned to it; `DaemonState::create_session` has no `origin` argument (FR-019 amended: the tool server records none), so the call sites keep their three arguments.
- Correction after the merge of main (feature 776's soft limit took 42): the origin bump is 42 to 43; `PROTOCOL_VERSION` is 43.
- Correction after the second merge of main (feature 552's agents took 43): the origin bump is 44 to 45 after 776 took 44; `PROTOCOL_VERSION` is 45.

## M2 (2026-10-16)

- Tests written with the code in one pass (U12 in origin.rs, U13 in tests/sidebar_spawn_tree.rs, U14 in layout_snapshot.rs); no per-behaviour red output captured, as in M1.
- Green: `cargo test -p micold-core --lib origin`, `-p micold-client --lib sidebar`, `--test sidebar_spawn_tree --test layout_snapshot --test features_sidebar`.
- The child marker is the word `agent` leading the row annotation (text, not a glyph), so row height is unchanged.

## M3 (2026-10-17)

- Tests written with the code in one pass (U15 in `provider_origin.rs`, U16 in `catalog_adoption.rs` and `provider_origin_discovery.rs`, U17 in `provider_origin_docs.rs`); no per-behaviour red output captured, as in M1 and M2.
- Green: `cargo test -p micold-core --test provider_origin --test provider_origin_docs`, `-p micold-daemon --test catalog_adoption --test provider_origin_discovery`.
- Codex and OpenCode readers run on synthetic fixtures built from the providers' schema or source (marked as such); Claude Code stays a documented gap.

## M4 (2026-10-18)

- Tests written with the code in one pass (U18 in `settings_roundtrip.rs`, U19 in `sidebar_spawn_tree.rs`); the settings test failed to compile before `folded_origins` existed. No per-behaviour red output captured, as in M1 to M3.
- Green: `cargo test -p micold-core --test settings_roundtrip`, `-p micold-client --test sidebar_spawn_tree --test features_sidebar`.
- U19's tests live in `sidebar_spawn_tree.rs` (the file M4's Verify runs), not `sidebar_tree.rs` as T033 first said.
- Fold toggling is a sidebar `Msg::ToggleOrigin`; the shell writes `Settings.folded_origins` after it (`persist_folded_origins`) and `startup.rs` reads it back. The set is never pruned.

## Close (2026-10-18)

Waiver (verification finding 1): no per-behaviour red output was captured in M1 to M4 (tests were written with the code, see each entry). Re-deriving a red per behaviour by stashing the implementation was not done; it would not change what the shipped tests prove. The gap is accepted and recorded here. Findings 2 and 5 (a surviving mutant each) were closed by new tests that fail under the mutant. Finding 3 is declined: the FR-007 action test exists as `a_child_row_acts_like_a_top_level_row` in `crates/micold-client/src/ui/sidebar.rs` (M2 review B round 1 added it).
