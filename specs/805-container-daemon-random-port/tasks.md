# Tasks: Container daemon without a port picks a random free port (805)

**Input**: spec.md, plan.md, research.md, data-model.md, quickstart.md. Tests first (Constitution I): each test task precedes the code it covers and is run red.

## Phase 1: Foundational (core picker)

- [ ] T001 [P] Test (red): `pick_free_port` skips taken and zero ports, returns the first usable one, fails with `NoFreePort` after 16 misses and on an `Err` from the source; `os_free_loopback_port` returns a non-zero port that binds; `DaemonRegistry::container_ports` lists every stored container port, in `crates/micold-core/tests/daemons_registry.rs`
- [ ] T002 Implement `PortPickError`, `pick_free_port`, `os_free_loopback_port`, `DaemonRegistry::container_ports` in `crates/micold-core/src/daemons.rs` (D1)

## Phase 2: User Story 1 - Empty port picks a free one (P1) 🎯 MVP

**Independent Test**: add a container daemon with an empty port, save, reopen edit: a port in 1..=65535 not used by another daemon.

- [ ] T003 [US1] Tests (red) in `crates/micold-client/tests/daemons_feature.rs`: add with empty port stores a port in 1..=65535 not another daemon's; reopen edit shows it; explicit port stored as given; `0` rejected with "Enter a port between 1 and 65535."; explicit duplicate rejected; Host/SSH daemon edited to Container with empty port gets a picked port; two empty adds with a source returning the same port twice get different ports; reducer with injected `taken_elsewhere` never picks that port; source always `Err` fails the save with the "No free port could be found; enter one yourself." port error and leaves the registry unchanged
- [ ] T004 [US1] Add `Model::next_port` (default `os_free_loopback_port`) and `Model::taken_elsewhere`; change `runtime_of(draft, registry, next_port)` to pick on an empty container port, mapping `NoFreePort` to the port `FormError`, and keeping the stored port when the port is empty while editing a container daemon with a stored port (FR-006, so T010 goes green), in `crates/micold-client/src/features/daemons.rs` (D2)
- [ ] T005 [US1] Add the `taken_elsewhere: Vec<u16>` parameter to `State::update_daemons` in `crates/micold-client/src/app.rs`, set both new `Model` fields at the two `Model` build sites, and pass `Vec::new()` at the internal `update_daemons` call in `app.rs`
- [ ] T006 [US1] In `crates/micold-client/src/shell/daemons.rs::update`, compute `taken_elsewhere` from the store's current registry (`app.caps.settings()` load, `.daemons.container_ports()`) for the form's save message and pass it to `update_daemons`
- [ ] T007 [P] [US1] Update the remaining callers and `Model` construction: `crates/micold-client/tests/support/daemons_fixture.rs` (both new fields, `Vec::new()`), `crates/micold-client/tests/a_field_note_shares_its_fields_column.rs` (`Vec::new()`)
- [ ] T008 [P] [US1] Port `TextField` placeholder says a free port is chosen when empty, in `crates/micold-client/src/ui/settings/daemons.rs` (copy only; `daemons_section_geometry.rs` stays green)
- [ ] T009 [US1] User guide: in `docs/user-guide/sandboxed-daemon.md` Settings > Daemons paragraph, say the port may be left empty, a free one is chosen and saved, it never changes by itself, and a taken port is reported at start (D4)

## Phase 3: User Story 2 - The port stays the same (P1)

- [ ] T010 [US2] Tests (red) in `crates/micold-client/tests/daemons_feature.rs`: edit with the port cleared keeps the stored port; edit without touching it keeps it; a start plan after save and restart carries the same port (`daemon_adopt_on_restart.rs` style)
- [ ] T011 [US2] Confirm T010 is green from T004's keep-stored-port branch; fix `runtime_of` in `crates/micold-client/src/features/daemons.rs` only if not (FR-006)

## Phase 4: User Story 3 - A taken port is reported (P2)

- [ ] T012 [US3] Test: add assertions to `crates/micold-core/tests/sandbox_runtime.rs` on the `PortUnavailable { port: 7727 }` message ("Port 7727 is already in use, so the sandbox has no control channel.") and the remedy ("Stop whatever holds the port, or choose another in Settings → Daemons."); the stored port staying unchanged needs no test beyond there being no write path at start (`shell/sandbox.rs` only reads it). No code change expected (D3). FR-008 is covered by the CI OS matrix running the same tests

## Dependencies

T001 → T002 → T003 → T004, T005, T007 (one compiling step: T004 adds the `Model` fields that T005 and T007 construct) → T006. T010 and T011 after T004. T012 independent.

## Implementation Strategy

One milestone: the stories share `runtime_of`, so US1 and US2 land together; US3 is verification only.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Container daemon picks a free port 🎯 MVP

- **Tasks**: T001–T012
- **Deliverable**: Saving a container daemon with an empty port stores a free loopback port that is shown on edit, kept on re-save and on restart, and a taken port is reported at start.
- **Satisfies**: US1 acceptance scenarios 1–3; US2 acceptance scenarios 1–2; US3 acceptance scenario 1; FR-001–FR-010; SC-001–SC-003
- **Verify**: `mise run test-core` and `cargo test -p micold-client --test daemons_feature` via `mise run gate`; quickstart Part B steps 1–5 (run and recorded in the close unit)
- **Depends on**: —
- **Tier**: full
