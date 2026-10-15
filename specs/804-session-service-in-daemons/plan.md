# Implementation Plan: Session service settings in the Daemons manager

**Branch**: `feat/804_move-the-session-service-settings-container` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/804-session-service-in-daemons/spec.md`

## Summary

Feature 491 already gives every daemon its own `SandboxProfile` (`ContainerSettings.profile`) and a
form that saves it, but the form exposes only the image reference. The Session service page edits
the *legacy global block* `settings.daemon` and mirrors it into one registry entry with
`Settings::set_single_daemon`. This feature moves the page's fields into the Daemons form, saves them
with the form's own save (registry is the single source), drops `SettingsSection::Daemon`, and
retargets the hint texts, the rail badge and the user guide. Fields, defaults and validation do not
change; the field/validation code moves, it is not rewritten.

## Technical Context

**Language/Version**: Rust (workspace edition), iced 0.14 (existing).
**Primary Dependencies**: none new. `micold-core` (`daemons`, `sandbox`, `settings`), `micold-client`.
**Storage**: `settings.json` via `micold-core::settings` (local, Principle IV). No schema change;
no `settings_version` bump (additive/unchanged fields only, feature 491 R5).
**Testing**: core unit tests (`mise run test-core`), client integration tests in
`crates/micold-client/tests/`, geometry gate tests, quickstart visual pass.
**Target Platform**: Linux, macOS, Windows (Principle VI): no platform branch added.
**Project Type**: desktop app, two crates.
**Constraints**: restart-on-save and the unshared-sign-in notice behave as before (FR-004, FR-001).
**Scale/Scope**: ~8 files touched; ~700 lines moved, ~150 new.

## Constitution Check

| Principle | Status | Note |
|---|---|---|
| I Test-first | Pass | Tests listed in Test strategy are written before the move; existing page tests are moved, not dropped. |
| II Multi-session | Pass | Per-daemon edit leaves other daemons and sessions untouched (FR-003). |
| III Worktree integration | Pass | No worktree behaviour touched. |
| IV Local-first storage | Pass | `settings.json` only; nothing remote. |
| V Rust + iced | Pass | Existing widgets. |
| VI Cross-platform parity | Pass | One code path; available runtimes still per platform. |
| VII Documentation | Pass | `docs/daemon.md` and the user guide updated (FR-009); FR-006 texts. |
| VIII Reusable UI components | Pass | Reuses `Select`, `TextField`, `Checkbox`, `page`/`group`/`note`/`caution`; no new component. |

Complexity Tracking: no violations.

## Design

### D1. The form owns the profile (FR-001, FR-003, FR-010)

`features/daemons.rs::Draft` already holds `profile: SandboxProfile`. Add the typed-text mirrors the
page used (`image_path`, `cpus`, `memory_mib`, `pids`, `storage_mib`) beside it, and Msgs for runtime,
image kind, reference (exists as `ImageEdited`), archive, credential toggle, network, four limits and
survive-logout. `runtime_of` (container branch) parses limits into `profile.budget` and the archive
into `profile.image`, reusing `SettingsDraft::budget()`'s parsing: extract that parsing from
`features/settings.rs` into a free function over the four strings (single copy, used by both until
the page is deleted, then by the form only). A refusal returns `FormError { field }` with the field
id of the limit or image field, shown beside it (FR-008). The existing running-daemon confirmation
(`Pending::Edit`) already triggers on any runtime change, which now includes the profile (FR-004,
FR-010).

"Where sessions run" is the form's existing *Runs in* select (Container/Host); the page's separate
placement select and its `ConfirmPlacementDialog` path go with the page: the per-daemon
confirmation replaces them.

### D2. The view is moved, not rewritten (FR-001, FR-006)

`ui/settings/daemon.rs` helpers (`limits`, `network_warning`, `unshared_sign_in_notice`,
`sharing_summary`, `survival_support`, option tables, notices) move to a new
`ui/settings/container_fields.rs`, rewritten to take a small `ContainerView<'_>` (profile, text
mirrors, capabilities, unshared sign-in, placement) instead of `&SettingsDraft`.
`ui/settings/daemons.rs::form` appends them under the container branch. The SSH branch shows none
(FR-011). A host daemon shows the survive-logout note via `survival_support(HostProcess)` and the
container groups inert (Edge Cases).

Capabilities and `unshared_sign_in` are machine facts seeded from `app.sandbox` today into
`SettingsDraft.daemon`; they are read from `app.sandbox` when the form is drawn instead, for the
daemon that is the running local container (the legacy default). Other daemons show limits editable
("unknown means editable", existing rule).

### D3. Registry is authoritative; legacy block follows (FR-005)

Nothing edits `settings.daemon` after the move. `SettingsDraft.daemon` keeps round-tripping it
untouched so a settings save never rewrites the registry entry (the `changed_value` guard in
`persist.rs` then never fires). When the form saves the daemon the legacy block mirrors (legacy
default, else first), the block is updated in step so the rollback copy does not disagree:
new `Settings::sync_legacy_daemon` in core (inverse of `set_single_daemon`): a Container entry sets
`placement = LocalSandbox` and `sandbox = profile`; a Host entry sets `placement = HostProcess` and
keeps the sandbox profile; an SSH entry leaves the block untouched. Unit-tested for all three. Migration is untouched:
feature 491's R5 already seeds only id 1; a daemon added later starts from `SandboxProfile::default()`
(`begin_add` already does).

Boot-time placement (`Placement::resolve(settings.daemon...)`, `apply_placement`) keeps reading the
legacy block, now kept in step by D3, so the shell's single-service path is unchanged and out of
scope.

### D4. Removal of the section and pointers (FR-002, FR-006, FR-007)

- `SettingsSection::Daemon` removed from the enum, `ALL`, `label`, `icon`; `Icon::SessionService`
  stays (the overflow diagnostics menu may use it; grep at implementation). No section name is
  persisted or linked (`SettingsSection` is not serialised), so the "remembered section" edge case
  has nothing to map; `FieldError.section` loses the variant with it.
- Settings draft fields only the page used (`DaemonDraft` text mirrors, `Msg::PlacementChanged`…
  `StorageLimitChanged`, `pending_placement`, `ConfirmPlacementDialog`, `placement_note`) are
  removed once unreferenced; `validate()` no longer parses them.
- The rail badge: `draft.shares_credentials()` becomes a NEW `ViewModel::shares_credentials: bool`
  in `features::daemons::views`, true when any registry entry's container profile shares a
  credential or the open form's draft does (so ticking a share shows the badge before save, as the
  page did). `settings_view::view` reads it from its existing `daemons` argument and attaches the
  badge to the `Daemons` row (FR-007).
- Texts: `features/sandbox.rs:134`, `micold-core/src/sandbox/runtime.rs:378,395,404,411` (the
  limit-hint, runtime, image, port hints) become "Settings → Daemons"; `docs/daemon.md` (lines
  ~408, 430, 444, 477) and the user guide (grep `Session service` in `docs/`) updated.
  `ui/toolbar.rs` "Session service diagnostics" is a different thing (the service's diagnostics
  action) and is kept.

## Requirement map

| Req | Where |
|---|---|
| FR-001, FR-010 | D1, D2 (`features/daemons.rs`, `ui/settings/daemons.rs`, `container_fields.rs`) |
| FR-002 | D4 (`features/settings.rs` enum, `ui/settings_view.rs`) |
| FR-003 | D1 (per-entry profile), form drafts are per daemon |
| FR-004 | D1 (`Pending::Edit` confirmation), existing `confirm` path |
| FR-005 | D3 (no migration change; `begin_add` default) |
| FR-006, FR-009 | D4 texts and docs |
| FR-007 | D4 badge |
| FR-008 | D1 `FormError.field` |
| FR-011 | D2 (SSH branch shows none); D1 stored profile of an SSH entry not touched |

## Test strategy

| Layer | Covers |
|---|---|
| core unit (`micold-core`) | `sync_legacy_daemon`; hint texts name Daemons (FR-006); `begin_add` default profile |
| `features::daemons` unit/integration (`tests/features_daemons*.rs`) | profile fields round trip through `save`; validation field ids; two daemons independent (FR-003); remove discards, rename keeps; confirmation on running daemon |
| client UI tests (`tests/features_settings.rs`, `settings_sections.rs`, `settings_rail_motion.rs`, `a_field_note_shares_its_fields_column.rs`, `sandbox_limits.rs`) | moved to the Daemons form; rail has no Session service; badge from registry; gate lists `daemon.*` as chosen in the Daemons form |
| client tests moved or deleted with the placement path | `saving_a_placement_moves_the_daemon.rs`, `settings_reports_the_live_placement.rs`, `upgrade_single_daemon.rs` (FR-005 carry-over kept, retargeted to the registry; asserts a 491-migrated file shows the legacy values in entry 1's profile), `settings_draft_tracks_the_live_theme.rs`, `daemons_feature.rs` (extended); tests naming `SettingsSection::Daemon` (`features_settings.rs`, `settings_rail_motion.rs`, `a_field_note_shares_its_fields_column.rs`, `sandbox_limits.rs`) are retargeted to the Daemons form or `Daemons`; FR-009 and FR-006 doc/text claims are checked by a grep test over `docs/` and the hint strings; new test: a settings save with unchanged `daemon` leaves the registry untouched after a form save synced the block (D3) |
| geometry gates | field-note column, form layout |
| quickstart §B visual pass | the long form in Daemons scrolls and fits |

## Project Structure

```text
specs/804-session-service-in-daemons/   spec.md plan.md research.md data-model.md quickstart.md
crates/micold-core/src/settings.rs      sync_legacy_daemon
crates/micold-core/src/sandbox/runtime.rs  hint texts
crates/micold-client/src/features/{daemons,settings,sandbox}.rs
crates/micold-client/src/ui/settings/{daemons,container_fields,daemon(deleted)}.rs
crates/micold-client/src/ui/settings_view.rs
docs/daemon.md, user guide
```

No contracts directory: no external interface changes (no wire, CLI or file-format change).
