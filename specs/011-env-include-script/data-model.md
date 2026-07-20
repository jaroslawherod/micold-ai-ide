# Phase 1 Data Model: Environment-Include Script

Derived from spec.md's Key Entities, resolved by research.md's R4/R8. Two shapes: one persisted
(Settings fields), one in-memory-only (the resolved snapshot on `App`).

## `Settings` additions (persisted — `src/settings.rs`)

Three new flat fields, sibling to the existing `theme: ThemePreference` and
`scrollback_lines: usize` (research R8 — no nested sub-struct):

| Field | Type | Default | Constraint |
|---|---|---|---|
| `env_include_enabled` | `bool` | `true` | none — any bool is valid |
| `env_include_script_path` | `String` | `default_env_include_path(home).to_string_lossy()` (R7) | empty string is treated as disabled (spec Edge Cases), not rejected |
| `env_include_timeout_secs` | `u64` | `10` | clamped on load to `MIN_ENV_INCLUDE_TIMEOUT_SECS` (`1`) `..= MAX_ENV_INCLUDE_TIMEOUT_SECS` (`60`), mirroring `clamp_scrollback`'s existing shape |

All three are `#[serde(default = "...")]` on both `Settings` and `StoredSettings`, so a
pre-existing settings file that predates this feature loads with the feature ON and the default
path/timeout (FR-004) — the same missing-field-defaults contract `scrollback_lines` established.
`SETTINGS_VERSION` moves `2 → 3` (doc-comment bookkeeping only, per R8).

**Validation rules** (mirrors `clamp_scrollback`):
- `clamp_env_include_timeout(secs: u64) -> u64` clamps to `1..=60` seconds
  (`MIN_ENV_INCLUDE_TIMEOUT_SECS = 1`, `MAX_ENV_INCLUDE_TIMEOUT_SECS = 60`); out-of-range persisted
  values are silently clamped on load, exactly like `an_out_of_range_persisted_value_is_clamped_on_load`
  already asserts for scrollback.
- The path field has no format validation — any string is accepted; whether it resolves to a
  usable script is discovered only at resolution time (FR-011's "missing/fails/no diff" all
  degrade gracefully, never a Settings-save-time error).

**State transitions**: None — these are plain persisted scalars, changed only via `SettingsSaved`
(validated) and read at `boot()`/`SettingsOpened` (seeding the draft), matching `scrollback_lines`'s
existing lifecycle exactly.

## `EnvIncludeOutcome` (in-memory only — `src/env_include.rs`)

```rust
pub enum EnvIncludeOutcome {
    /// The feature is off, or the configured path is empty/blank (spec Edge Cases).
    Disabled,
    /// Resolved successfully; `vars` (carried alongside, see EnvIncludeSnapshot below) reflects
    /// what the script changed relative to the clean baseline.
    Success,
    /// The configured path did not exist at resolution time (Rust-side stat, before any
    /// subprocess is spawned — research R1).
    MissingScript,
    /// The script ran but its last command / an explicit `exit` produced a non-zero status.
    NonZeroExit { code: i32, diagnostic: String },
    /// Sourcing did not complete within the configured timeout; the subprocess was killed.
    TimedOut { diagnostic: String },
}
```

`diagnostic` (present on both failure variants that can have partial output — `NonZeroExit` and
`TimedOut`) is the script's own captured combined stdout+stderr from that attempt (research R1),
per the clarified FR-013 tradeoff (full diagnostic detail, deliberately accepting the
secret-exposure risk). `MissingScript` carries no diagnostic — there is nothing to have printed,
since no subprocess was ever spawned for it.

**Why an enum, not a `bool succeeded` + `Option<String> error`**: makes "failed but has no
category" or "succeeded but also has a diagnostic" unrepresentable, satisfying Principle V's
type-level invalid-state guarantee mirrored from `ShellLifecycle`'s feature-010 precedent.

## `EnvIncludeSnapshot` (in-memory only, one shared instance on `App` — `src/main.rs`)

```rust
struct EnvIncludeSnapshot {
    /// Variables captured from the last successful (or partially-successful, if the spec's
    /// "no usable diff" edge case applies) resolution attempt. Empty when Disabled/MissingScript/
    /// never-yet-resolved. Never persisted (FR-008).
    vars: Vec<(String, String)>,
    /// The most recent resolution attempt's outcome, shown by settings_form.rs (FR-013).
    outcome: EnvIncludeOutcome,
}
```

- **Cardinality**: exactly one instance, shared by every session (spec Key Entities — "a single
  shared snapshot is used by every session"), not keyed per `SessionId`. This is the one
  deliberate departure from this codebase's usual per-session state shape (`SessionTerminals`,
  `ShellLifecycle`) — it is app-level configuration, not session state, so it does not implicate
  Principle II's session-isolation guarantee (research plan.md Constitution Check, Principle II).
- **Lifecycle**: constructed once in `boot()` (R5); replaced wholesale (not mutated field-by-field)
  by `refresh_env_include(app)` on the two refresh triggers (R5); dropped when the app exits.
- **Relationship to spawn call sites**: `launch_spec()` (`src/main.rs`, for `claude`) and
  `ensure_attached_process`'s `TerminalMode::Regular` branch (for the shell) both build their
  `env: Vec<(String, String)>` as `snapshot.vars.iter().cloned().chain(once(("TERM".into(),
  "xterm-256color".into()))).collect()` — appending the hardcoded `TERM` pair *last* so it always
  wins on key collision (FR-009) regardless of iteration order, without needing an explicit filter
  step.

## Relationships

```text
Settings (persisted)                    App (in-memory, gui-only)
├── env_include_enabled ─────┐          ├── env_include: EnvIncludeSnapshot
├── env_include_script_path ─┼─ read at ├──   ├── vars: Vec<(String,String)>
└── env_include_timeout_secs ┘  boot()/ │      └── outcome: EnvIncludeOutcome
                                refresh └── (existing) scrollback_lines, terminals, ...
                                            │
                                            ├── launch_spec() (claude)         ─┐ both merge
                                            └── ensure_attached_process         ─┘ snapshot.vars
                                                (TerminalMode::Regular, shell)     + hardcoded TERM
```

No new relationship to `Session`, `Workspace`, or any persisted worktree/project entity — this
feature is orthogonal to session/worktree identity (plan.md Constitution Check, Principle III).
