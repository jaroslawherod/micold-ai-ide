# Tasks: Session service settings in the Daemons manager

**Input**: `specs/804-session-service-in-daemons/` (spec.md, plan.md, research.md, data-model.md, quickstart.md)

**Tests**: mandatory (Constitution I); each phase lists its failing tests before the code.
**Documentation**: each story's user-guide task is in that story's phase (Constitution VII).

Paths: `C` = `crates/micold-core/src`, `K` = `crates/micold-client/src`, `T` = `crates/micold-client/tests`.

## Phase 1: Foundational

- [x] T001 Test first: `Settings::sync_legacy_daemon` for Container (placement LocalSandbox + profile), Host (HostProcess, profile kept) and SSH (block untouched) in `C/settings.rs` tests
- [x] T002 Implement `Settings::sync_legacy_daemon` in `C/settings.rs` (inverse of `set_single_daemon`)
- [x] T003 Test first: `parse_budget(cpus, memory_mib, pids, storage_mib) -> Result<ResourceBudget, (LimitField, String)>` keeps today's parsing and errors, in `K/features/settings.rs` tests
- [x] T004 Extract `parse_budget` from `SettingsDraft::budget()` in `K/features/settings.rs` as a free function used by the page and the form (single copy)

## Phase 2: User Story 1 - Container settings in the daemon form (P1) 🎯 MVP

**Goal**: Settings > Daemons edits every Session service field per daemon, saved by the form's own save.
**Independent test**: edit a daemon's runtime and a limit, save, see the confirmation and restart.

- [x] T005 [US1] Test first: profile fields (runtime, image kind/ref/archive, credential shares, network, four limits, survive-logout) round trip through `save`; `FormError.field` ids `cpus`, `memory`, `pids`, `storage`, `image_path` in `T/daemons_feature.rs`
- [x] T006 [US1] Test first: two daemons edit independently; unsaved drafts of two forms kept apart; remove discards, rename keeps; running daemon asks confirmation (FR-003, FR-004) in `T/daemons_feature.rs`
- [x] T007 [US1] Test first: SSH entry shows no container fields and an unedited SSH entry keeps its stored profile; container to SSH save drops it (FR-011) in `T/daemons_feature.rs`
- [ ] T007a [US1] Test first: a settings save with unchanged `daemon` leaves the registry untouched after a form save synced the block (D3) in `T/daemons_feature.rs`
- [x] T008 [US1] Add `image_path`, `cpus`, `memory_mib`, `pids`, `storage_mib` text mirrors, the Msgs for each field, and `runtime_of` parsing via `parse_budget` to `Draft` in `K/features/daemons.rs`; refusals return `FormError { field }`
- [x] T009 [US1] Move the field helpers (`limits`, `network_warning`, `unshared_sign_in_notice`, `sharing_summary`, `survival_support`, option tables) from `K/ui/settings/daemon.rs` to new `K/ui/settings/container_fields.rs` taking `ContainerView<'_>`; register in `K/ui/settings/mod.rs`
- [x] T010 [US1] Append the container groups under the container branch of `form` in `K/ui/settings/daemons.rs`, read capabilities and unshared sign-in from `app.sandbox`; host daemon shows only the survive-logout note, SSH none
- [x] T011 [US1] Call `sync_legacy_daemon` from the form save of the legacy-mirrored daemon in `K/features/daemons.rs` (covered by T007a)
- [x] T012 [US1] Run the geometry gates (field-note column, form layout) for the longer form and fix overflow in `K/ui/settings/daemons.rs`
- [x] T013 [US1] User guide: describe container runtime, image, shares, network, limits and survive-logout in the daemon form in `docs/user-guide/sandboxed-daemon.md`

**Checkpoint**: both homes edit the same registry entry; the page is removed in US2.

## Phase 3: User Story 2 - The standalone section is gone (P1)

**Goal**: no Session service rail entry, hint texts and badge retargeted.
**Independent test**: rail lists no Session service; hint names Settings > Daemons; badge from any section.

- [x] T014 [US2] Test first: rail has no Session service entry; badge shows when any daemon (or the open form) shares a credential, none otherwise, from any section (FR-002, FR-007) in `T/settings_sections.rs`, `T/settings_rail_motion.rs`
- [x] T015 [US2] Test first: limit, runtime, image and port hints name Settings > Daemons and no user-facing string names Session service (FR-006): `C/sandbox/runtime.rs` tests, `T/sandbox_limits.rs`
- [x] T016 [US2] Add `ViewModel::shares_credentials` in `K/features/daemons.rs` views and read it for the `Daemons` rail row in `K/ui/settings_view.rs`
- [x] T017 [US2] Retarget hint texts to "Settings → Daemons" in `K/features/sandbox.rs` and `C/sandbox/runtime.rs`
- [x] T019 [US2] Retarget or delete tests naming `SettingsSection::Daemon` or the placement path: `T/features_settings.rs`, `T/settings_rail_motion.rs`, `T/a_field_note_shares_its_fields_column.rs`, `T/sandbox_limits.rs`, `T/saving_a_placement_moves_the_daemon.rs`, `T/settings_reports_the_live_placement.rs`, `T/settings_draft_tracks_the_live_theme.rs`, `T/overlay_registry.rs`, `T/popover_displacement.rs` (done before T018 so the tree compiles)
- [x] T018 [US2] Remove `SettingsSection::Daemon` (enum, `ALL`, `label`, `icon`, `FieldError.section`), delete `K/ui/settings/daemon.rs`, drop `ConfirmPlacementDialog`, `pending_placement`, `placement_note` and the page-only Msgs and `DaemonDraft` mirrors in `K/features/settings.rs`, `K/ui/settings_view.rs`; remove `mod daemon` in `K/ui/settings/mod.rs`; delete `K/ui/confirm_placement.rs` with its `mod` line in `K/ui/mod.rs` and its entry in `K/overlay/registry.rs` (check `K/ui/confirm_daemon_removal.rs` only borrows from it)
- [x] T021 [US2] Grep test that `docs/` and the hint strings have no stale "Session service" reference (FR-009, SC-004), beside the doc-text tests in `T/`
- [x] T020 [US2] User guide and daemon doc: drop the Session service section and update references in `docs/user-guide/settings.md`, `docs/user-guide/worktrees-and-sessions.md`, `docs/user-guide/install-macos.md`, `docs/install.md`, `docs/README.md`, `docs/daemon.md`

## Phase 4: User Story 3 - Existing settings carry over (P2)

**Goal**: a pre-change settings file shows the same values in the migrated daemon.

- [x] T022 [US3] Test first, then retarget: a 491-migrated file shows legacy placement, runtime, shares, limits and survive-logout in entry 1's profile, no other daemon seeded, a later daemon starts at defaults, in `T/upgrade_single_daemon.rs`
- [x] T023 [US3] User guide: state that existing values moved to the migrated daemon in `docs/user-guide/sandboxed-daemon.md`

## Phase 5: Polish

- [ ] T024 Quickstart §B visual pass: the long Daemons form scrolls and fits; record the result in `specs/804-session-service-in-daemons/quickstart.md`

## Phase 6: User Story 4 - Collapsible container sections (P3, amendment M4, #890)

**Goal**: the daemon form's container settings sit in titled collapsible sections from a new shared Material component.
**Independent test**: open a container daemon, toggle sections by pointer and keyboard, trigger a limit error in a closed Limits section and see it open.

- [x] T025 [US4] Test first: `AccordionSection` shows its title, a chevron for open/closed, reveals its body only when open, emits `on_toggle` on press and on Space/Enter with focus, draws the focus ring, in both themes, in `K/ui/material/accordion_section.rs` tests (patterns: `K/ui/material/accordion.rs`, `keyboard_focus.rs`, `style_states.rs`)
- [x] T026 [US4] Implement `AccordionSection` (builder `new(title, body, roles).open().on_toggle()`, `From` into `Element`) composing `Accordion`, in `K/ui/material/accordion_section.rs`; register in `K/ui/material/mod.rs`
- [x] T027 [US4] Add the component showcase entry (closed, open, live toggle) in `K/showcase/catalogue.rs` and `K/showcase/sections/surfaces.rs`
- [x] T028 [US4] Test first: section toggle changes only the open set; default is Container only; a section holding a `FormError` field is open; the draft and saved settings are untouched by toggling (FR-012, FR-014, FR-015) in `T/daemons_feature.rs`
- [x] T029 [US4] Add the open-section set and `DaemonsMsg::SectionToggled` to `K/features/daemons.rs`, with the pure "open or has error" rule
- [x] T030 [US4] Split `container_fields::groups` into five section bodies and wrap each in `AccordionSection` in `K/ui/settings/container_fields.rs`, `K/ui/settings/daemons.rs`; keep focus-tracking ids
- [x] T031 [US4] Run the geometry gates `T/daemons_section_geometry.rs` and `T/a_field_note_shares_its_fields_column.rs` with the sections and fix overflow in `K/ui/settings/daemons.rs`
- [x] T032 [US4] User guide: describe the collapsible sections in the daemon form in `docs/user-guide/sandboxed-daemon.md`

**Checkpoint**: the form is shorter at first open; nothing hidden by a closed section can fail silently.

## Dependencies

Phase 1 before all. US1 before US2 (the form must carry the fields before the page goes). US3 after US2. US4 after US3 (T025–T027 are independent of the form; T030 needs T026 and T029).
T001, T003 parallel; T005–T007 parallel.

## Implementation Strategy

MVP is US1: the form edits every field while the page still exists (interim two homes over one
registry entry). US2 then removes the page and retargets pointers. T024 has no code: left to the
close unit; its visual pass also covers the M4 sections.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Container settings in the Daemons form 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: Settings > Daemons edits a daemon's container runtime, image, shares, network and limits and saves them with the form's own save and restart confirmation.
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001, FR-003, FR-004, FR-008, FR-010, FR-011
- **Verify**: `cargo test -p micold-client --test daemons_feature` and `mise run test-core`
- **Depends on**: —
- **Tier**: full

### M2 — Session service section removed

- **Tasks**: T014–T021
- **Deliverable**: the settings rail has no Session service entry; hints name Settings > Daemons; the Daemons rail row carries the Sharing badge; docs match.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-002, FR-006, FR-007, FR-009; SC-002, SC-004
- **Verify**: `cargo test -p micold-client`
- **Depends on**: M1
- **Tier**: full

### M3 — Existing settings carry over

- **Tasks**: T022–T023
- **Deliverable**: a settings file from before the change shows the same values in the migrated daemon's form.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-005; SC-003
- **Verify**: `cargo test -p micold-client --test upgrade_single_daemon`
- **Depends on**: M2
- **Tier**: light

### M4 — Collapsible container sections

- **Tasks**: T025–T032
- **Deliverable**: the daemon form's container settings sit in five titled collapsible sections (Container open, rest closed) built from a new reusable `AccordionSection` Material component, shown in the component showcase; a field error opens its section.
- **Satisfies**: US4 acceptance scenarios 1–5; FR-012, FR-013, FR-014, FR-015; SC-005
- **Verify**: `cargo test -p micold-client --test daemons_feature` and `cargo test -p micold-client accordion_section`, `--test daemons_section_geometry` and `--test a_field_note_shares_its_fields_column`
- **Depends on**: M3
- **Tier**: full
