# Autopilot ledger — #804 session-service-in-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Move the Session service settings (container runtime, limits) into the Daemons manager (#804); labels: enhancement, flow:feature, effort:low
- **Kind**: feature
- **Effort**: low
- **Issue**: #804
- **Worktree branch**: feat/804_move-the-session-service-settings-container
- **Started**: 2026-10-15
- **Phase**: milestone M2 implement
- **Next step**: continue M2 per Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #869 | Design | merged | 4b96bf2b526ca9b42e7dd9993d4b437016f49f18 |
| #882 | M1 | merged | ebf69d4be76163577384bf9fc921ff66cb8eb957 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Container settings in the Daemons form | #882 | merged |
| M2 | T014–T021 | full | Session service section removed | #… | implementing |
| M3 | T022–T023 | light | Existing settings carry over | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b13a0044e288eb4d69133c3183da0ba44d895f63:1bf62326e8f9f29b3ef9df261c3ddba110f6ae54 | CHANGES: 4 MAJOR (fixed; F2 routed to clarify) |
| Spec | 2 | 23e1dba8160d1800dfdd6d8915d4550176991274:1bf62326e8f9f29b3ef9df261c3ddba110f6ae54 | CLEAN (1 MINOR: remote-daemon marker has no FR; for clarify) |
| Plan | 1 | e8b9da12b0d78a98730b2678305343a44c3e1daa:506df1d556f5f8a14ab742264002c269ec5851fa | CHANGES: 2 MAJOR (badge source, budget error type) fixed; 3 MINOR fixed |
| Plan | 2 | 2ae1b02bed2b8ad66a62b54a31fda380948a8911:d0659308fae210beb1fdf3989b8f0b8d68845619 | CLEAN (1 MINOR: name parse_budget in plan D1, in data-model) |
| Review B M1 | 1 | ef3356982b2d:e016b011a15f | CHANGES: 1 MAJOR declined (no red evidence), 1 MINOR fixed |

## Declined review findings

- Review A M1 (code-review high), fixed: form save now refreshes `sandbox_boot` (the in-memory legacy block is page state, left to M2); removal no longer re-targets the legacy block; `ViewModel.legacy_default` follows `sandbox_daemon`. Declined: capabilities not re-probed when the draft's runtime changes (same as the settings page, a probe needs a restart); retained archive path refused after switching image source (spec'd UnusedArchivePath); restart prompt on any profile change (FR-004 restart-on-save); Draft/ContainerView duplication and `groups()` page bool (taste, MINOR); stale open settings_draft baseline (page removed in M2).

- Review B M1 F1 (MAJOR, no red-before-green evidence for T005-T007): cannot be produced after the fact; git history is one WIP commit. Recorded honestly in tdd/cycle-log.md. F2 fixed (T010 wording). F3 covered by the cycle log.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M2 unit hit 150k mid-implementation (branch reset to origin/main at ebf69d4b, WIP committed, NOT pushed, no PR, nothing compiled yet beyond lib check which was clean). Done in src: `SettingsSection::Daemon` + `daemon.rs` page + `confirm_placement.rs` + `ConfirmPlacementDialog`/`pending_placement`/`PendingPlacementChange`/`placement_step`/`placement_note`/`placement_move_consequence` + page-only Msgs and reducers removed from `features/settings.rs`; persist.rs `on_settings_saved` now goes straight to `apply_save` (apply_placement, on_placement_change_confirmed removed); `container_fields.rs` lost `Surface` (Form only; `groups(view, errors, ...)`, `limits(view)`); `ViewModel.shares_credentials` field added in `features/daemons.rs::views` (stored container daemons, form draft replaces its own daemon's stored one) and read by the Daemons rail row in `ui/settings_view.rs`; `ui/mod.rs` re-exports `container_fields::{limits as sandbox_limits, network_warning, ContainerView, LimitControl, NO_OUTBOUND_WARNING}`.
Tests already edited: main_tests.rs (placement tests deleted, helper `keep_running_in_the_draft`), overlay_registry.rs (17 dialogs, 36 states), popover_displacement.rs (DIALOGS 19), material_boundary.rs, settings_sections.rs (CHOSEN_ELSEWHERE gained daemon.* entries; page merge removed; stray-daemon test replaced), features_settings.rs; deleted tests/saving_a_placement_moves_the_daemon.rs, tests/settings_reports_the_live_placement.rs. sed already changed `SettingsSection::Daemon` -> `Daemons` in settings_rail_motion.rs, a_field_note_shares_its_fields_column.rs, sandbox_limits.rs.
Next: (1) `scripts/build-lock.sh cargo check -p micold-client --all-targets --keep-going` (run detached; another worktree held the lock), fix errors. (2) Still to retarget: tests/sandbox_limits.rs (was SettingsDraft + `ui::sandbox_limits(&d)`; now build a `ContainerView`, field ids are `FieldId::Daemon*Limit`, drop the `SettingsSection::Daemon` error-section test or use Daemons; add T015 hint tests: no string names "Session service"), tests/a_field_note_shares_its_fields_column.rs (image note now only in the open Daemons form: open `daemons` draft via `State.daemon_list`, or drop `IMAGE_SUPPORTING` cases), tests/settings_rail_motion.rs BADGED (Daemons badge now from `daemons.1.shares_credentials`, so the test must put a sharing daemon/form draft in `State`), tests/settings_sections.rs fix remaining; features_settings.rs other tests using removed Msgs; tests/daemons_feature.rs comment. (3) T014 tests: rail lacks "Session service", badge shows/hides from any section (unit-test `views().shares_credentials` in daemons_feature.rs + rail test). (4) T015/T017: change hint "Settings \u{203a} Session service \u{203a} Limits" in `features/sandbox.rs:134` and `core sandbox/runtime.rs` hints (limit, runtime, image, port) to "Settings \u{203a} Daemons"; also leftover "Session service" strings: ui/toolbar.rs:48 ("Session service diagnostics"), showcase/sections/surfaces.rs:471, ui/material/section_list.rs:1104 (test data, decide), icons.rs `Icon::SessionService` (unused now? remove or keep), FieldId::Settings{Cpu,Memory,Pid,StorageLimit,ImageReference,ImagePath,Credential,SurviveLogout} are now page-dead (budget() errors still carry them) - decide. (5) T021 grep test + T020 docs (docs/user-guide/settings.md, worktrees-and-sessions.md, install-macos.md, docs/install.md, docs/README.md, docs/daemon.md; README.md/CHANGELOG ignore CHANGELOG). (6) tick T014-T021 in specs/804-session-service-in-daemons/tasks.md, scoped gate, review A (high) + B, visual pass (rail changed), full gate, PR "feat(804): remove the Session service section (#804)" body ends `Refs #804`.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

- Tasks review round 1: 2 MAJOR (T018/T009 paths) fixed, 3 MINOR fixed; not re-reviewed (path/order fixes only).
