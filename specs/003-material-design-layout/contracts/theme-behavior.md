# Contract: Theme Resolution & Live-Update Behavior

Defines how the app turns a `ThemePreference` + the OS scheme into the rendered `ColorScheme`,
and how live changes propagate. The resolution function is pure (`theme::resolve`) and fully
unit-tested.

## Resolution truth table (`resolve(pref, system) -> ColorScheme`)

| `ThemePreference` | `SystemScheme` | Result `ColorScheme` | Requirement      |
|-------------------|----------------|----------------------|------------------|
| `Light`           | *(any)*        | `Light`              | FR-007           |
| `Dark`            | *(any)*        | `Dark`               | FR-007           |
| `FollowSystem`    | `Light`        | `Light`              | FR-005           |
| `FollowSystem`    | `Dark`         | `Dark`               | FR-005           |
| `FollowSystem`    | `Unspecified`  | `Light`              | FR-018 (fallback)|

## Startup (FR-005, FR-009, FR-019, SC-002)

1. Binary loads `Settings` from `settings.json` (missing/corrupt → `FollowSystem`).
2. Binary calls `dark_light::detect()` once, maps `Mode → SystemScheme`, seeds
   `state.system_scheme`.
3. First render's `.theme()` closure calls `state.color_scheme()` → the correct theme with no
   flash of the wrong scheme.

## Live OS change while `FollowSystem` (FR-006, SC-003)

1. A `Subscription` polls `dark_light::detect()` on a sub-second (~500 ms) interval, mapping to `SystemScheme`.
2. It emits `SystemThemeChanged(scheme)` **only when the value differs** from the last emission
   (no-op otherwise → no render churn, no flicker).
3. The reducer updates `state.system_scheme`; the next frame's `.theme()` closure reflects it.
4. The subscription runs regardless of preference, but a fixed `Light`/`Dark` preference makes
   `resolve` ignore `system_scheme`, so overrides are unaffected by OS changes.
5. A **transient** `dark_light::detect()` failure (e.g. a timeout under CPU load) is distinct
   from a successful `Ok(Mode::Default)` reading: it is folded through `theme::observe_system_scheme`,
   which keeps the last-known `system_scheme` rather than falling through to the FR-018 fallback
   for that poll cycle (FR-021; BUG-001).

## User override (FR-007, FR-008, FR-009, SC-004)

1. The theme menu emits `ThemePreferenceChanged(pref)`.
2. The reducer sets `state.theme_pref`; the next frame re-themes immediately.
3. The binary persists the new `Settings` (I/O boundary). On next launch, startup restores it.
4. Selecting `FollowSystem` resumes tracking `system_scheme` live.

## Non-behavior

- No message here opens/closes any overlay; the modal state machine and `on_escape` are untouched.
- `SystemThemeChanged` is never persisted (transient OS state, not a user choice).
- Timing target: a switch (override or OS change) is reflected within 1 second (SC-003), bounded
  by the ~500 ms poll interval for OS changes and immediate for user overrides.
