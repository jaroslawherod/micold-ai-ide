# Contract: Settings-Modal UI (Environment-Include Section)

**Module**: `src/ui/settings_form.rs`, `src/app.rs` (Message/SettingsDraft), `src/main.rs`
(seeding + save handling).

## `SettingsDraft` additions (`src/app.rs`)

```rust
pub struct SettingsDraft {
    pub scrollback_lines: String,             // existing, unchanged
    pub env_include_enabled: bool,            // NEW
    pub env_include_script_path: String,      // NEW
    pub env_include_timeout: String,          // NEW — text field, parsed/validated on save
    pub error: Option<String>,                // existing, shared by all fields' validation
}
```

## New `Message` variants (`src/app.rs`)

- `SettingsEnvIncludeEnabledToggled(bool)` — flips `draft.env_include_enabled`.
- `SettingsEnvIncludePathChanged(String)` — sets `draft.env_include_script_path` verbatim (no
  validation while typing, same as the path field having no format validation at all —
  `data-model.md`).
- `SettingsEnvIncludeTimeoutChanged(String)` — sets `draft.env_include_timeout` verbatim (parsed
  as `u64` and clamped only on `SettingsSaved`, mirroring `scrollback_lines`'s existing
  text-field-then-parse-on-save pattern).

## Seeding (`Message::SettingsOpened` handler, `src/main.rs`)

On open, in addition to the existing `draft.scrollback_lines = app.scrollback_lines.to_string()`:
```rust
draft.env_include_enabled = app.settings_env_include_enabled;
draft.env_include_script_path = app.settings_env_include_path.clone();
draft.env_include_timeout = app.settings_env_include_timeout_secs.to_string();
```
(exact `App` field names are a task-level choice; the shape is: the three current persisted
values, read the same way `scrollback_lines` already is.)

## Save (`Message::SettingsSaved` handler, `src/main.rs`)

Extends the existing scrollback-parse-and-validate block:
1. Parse `draft.env_include_timeout` as `u64`; on parse failure, set `draft.error` and keep the
   overlay open (same failure UX as an invalid scrollback value) — **do not** silently substitute
   a default, so the user's typo is visibly rejected rather than swapped out.
2. On success: write `env_include_enabled`, `env_include_script_path`, and the clamped timeout
   into the persisted `Settings` (alongside `theme`/`scrollback_lines`, same `store.save(...)`
   call).
3. Call `refresh_env_include(app)` (research R5) — this re-resolves immediately so the failure
   diagnostic (if any) reflects the just-saved configuration the next time Settings is viewed,
   without requiring an app restart or a session restart.

## Rendering (`src/ui/settings_form.rs::modal`)

Order, top to bottom, inside the existing `fields` column:
1. `text("Settings")` (headline, unchanged)
2. Existing scrollback label + `text_input` (unchanged, unmoved)
3. **NEW grouped block**, visually separated from the scrollback field by the existing
   `spacing::MD` the column already applies between children (FR-015 — "grouped... visually
   distinct" is satisfied by this ordering/clustering, not a new bordered container):
   - `text("Environment include")` (label, mirrors the scrollback section's own label style)
   - `checkbox("Enabled", draft.env_include_enabled).on_toggle(Message::SettingsEnvIncludeEnabledToggled)`
     styled via a new `style::checkbox(r)` helper (research R9)
   - `text_input("Script path", &draft.env_include_script_path).on_input(Message::SettingsEnvIncludePathChanged)`
   - `text_input("Timeout (seconds)", &draft.env_include_timeout).on_input(Message::SettingsEnvIncludeTimeoutChanged)`
4. **NEW, conditional**: if `app.env_include.outcome` (passed into `modal()` as an added parameter,
   or read from `draft`/`core` — task-level wiring choice) is a failure variant, render a read-only
   block: the failure category as a short label ("Script not found" / "Exited with an error" /
   "Timed out") followed by the diagnostic text in a scrollable/monospace-ish text block. Rendered
   only on failure — nothing is shown here on `Success`/`Disabled` (SC-006).
5. Existing error text (unchanged — shared by scrollback *and* the new timeout field's parse
   failure, same as today's single `draft.error` slot).
6. Existing Save/Cancel `row` (unchanged).

## Interaction contract (from spec User Story 2/3 acceptance scenarios)

- Toggling `Enabled` off and saving stops the script from being sourced on the next refresh —
  `app.env_include` becomes `(vec![], EnvIncludeOutcome::Disabled)` without invoking
  `env_include::resolve()` at all (research contract — `resolve()` is never called when disabled).
- Changing the path or timeout and saving takes effect on the very next refresh (immediate, via
  step 3 above) — not just on the next app run.
- The failure block (step 4) is the sole mechanism satisfying FR-012/FR-013/SC-006 — no toast,
  banner, or other new notification surface is introduced (spec Assumptions).
