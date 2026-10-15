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
