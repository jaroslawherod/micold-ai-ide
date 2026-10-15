# Research: Session service settings in the Daemons manager

## R1. Where the values live
**Decision**: the registry entry's `ContainerSettings.profile` (feature 491) is authoritative; the
legacy `settings.daemon` block is kept for rollback and synced from the entry on a form save.
**Rejected**: keeping the global block as the source and having the form edit it (two homes, cannot
be per daemon, FR-003); deleting the legacy block (loses the one-release rollback promised by 491 R5).

## R2. Which save
**Decision**: the Daemons form's own `Saved` with its running-daemon confirmation (spec FR-010).
**Rejected**: the page-level Save (the section it belongs to is gone and the registry is not saved
by it); a per-section autosave (changes the restart semantics).

## R3. View code
**Decision**: move the page's pure helpers to `container_fields.rs`, parameterised by a small view
struct, keeping their tests. **Rejected**: render the old view with a fake `SettingsDraft` per daemon
(keeps the coupling that blocks per-daemon editing); rewriting the helpers (risks FR-004g/BUG-008
regressions the tests pin).

## R4. Host/SSH daemons and stored container settings
**Finding**: `DaemonRuntime::Host` and `::Ssh` hold no profile; only `Container` does.
**Decision**: container settings are kept while the form is open (switching Runs in back and forth
keeps the draft); a *saved* Host/SSH daemon stores none, and an unedited SSH/Host entry is untouched.
Spec edge case and FR-011 reworded to match. **Rejected**: adding an inert profile to Host/Ssh
(registry schema change, out of scope of "only location changes").

## R5. Remembered section
**Finding**: `SettingsSection` is not serialised and nothing links it by name, so there is nothing
to map. **Rejected**: keeping a hidden section.

## R6. Legacy boot path
**Decision**: untouched; it reads the synced legacy block. **Rejected**: moving boot placement to the
registry here (separate refactor, no user-visible gain).
