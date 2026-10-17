# Data model: Required-field markers in forms (937)

All state here is runtime-only view state on an open draft: nothing is persisted, nothing crosses
the wire (Principle IV unaffected).

## `micold_core::required` (new module, iced-free)

### `FieldRequirement`

```text
enum FieldRequirement { Optional, Required, Missing }
```

| Value | Marker | Error state + message |
|---|---|---|
| `Optional` | no | no |
| `Required` | yes | no |
| `Missing` | yes | yes (unless the field's own error is set, which wins — FR-010) |

### `Requirement`

What a form says about one field now: `{ required: bool, empty: bool }`. `empty` uses the trim rule
of the validator it mirrors (research R5). Built by `Requirement::text(required, &str)` (trimmed)
and `Requirement::choice(required, Option<&T>)`.

### `RequiredMarks<K: Ord + Copy>`

| Field | Type | Meaning |
|---|---|---|
| `left` | `BTreeSet<K>` | fields the user has left after focusing them (blur, R2) |
| `attempted` | `bool` | a submit was attempted on this draft |

| Operation | Effect |
|---|---|
| `default()` | fresh form: nothing left, no attempt (FR-006 "never on a fresh form") |
| `leave(k)` | records a blur of `k`; also how a one-field form's submit marks its field |
| `attempt()` | records a submit attempt |
| `state(k, Requirement) -> FieldRequirement` | `Optional` if not required; `Missing` if required, empty and (`attempted` or `k ∈ left`); else `Required` |
| `legend(iter of Requirement) -> bool` | whether the form shows the legend: any visible field required (FR-004) |

Invariants: `state` is a pure function of the marks and the requirement, so typing a value moves
`Missing → Required` in the same update with no other state touched (FR-007); a field that stops
being required reads `Optional` whatever the marks say (edge case *stops being required*).

Constants: `LEGEND = "* required"`, `MESSAGE = "This field is required"`, `MARKER = "*"`.

## Drafts that gain marks

Each gains `pub required: RequiredMarks<FieldId>` (default on open, dropped with the draft) and a
`required_fields(&self) -> Vec<(FieldId, Requirement)>` listing its visible fields in display order.

| Draft | File | Fields listed (R5) | Submit attempt in |
|---|---|---|---|
| `WorktreeForm` | `features/worktree_form.rs` | `AddWorktreeType`, `AddWorktreeName` (new/issue); `AddWorktreeBranch` (existing) | `submitted` |
| `ParallelDialog` | `features/runs.rs` | `RunPrompt`, `RunType`, `RunName`, `RunBaseBranch` | `confirmed` |
| `RenameDraft` | `features/project.rs` | `RenameProjectName` | `rename_confirmed` |
| `WorktreeRenameDraft` | `features/worktree.rs` | `RenameWorktreeName` | `rename_confirmed` |
| `SettingsDraft` | `features/settings.rs` | per section: `SettingsScrollback`; `SettingsEnvIncludeTimeout`, `SettingsLongTaskThreshold`, `SettingsPlanUsageWarn` (shown); `IssueMappingLabel(i)`; `SettingsJiraSite/Key/Email/Token` (conditional) | `saved` |
| daemons `Draft` | `features/daemons.rs` | `DaemonName`; `DaemonSshHost` (SSH); `DaemonContainerName`, `DaemonImage`, `DaemonImagePath` (imported file) | `Saved` arm |
| daemons `State` | `features/daemons.rs` | `repository_marks: RequiredMarks<FieldId>` over `DaemonRepository(id)` | `add_repository` → `leave(DaemonRepository(id))` |

`SettingsDraft::required_fields` takes a `SettingsSection` so each section page asks only for its
own fields (legend per section). Removing or moving a GitHub mapping entry clears the
`IssueMappingLabel(_)` marks, since the indices shift.

## `FieldId` changes (`features/window.rs`)

New: `AddWorktreeType`, `AddWorktreeBranch`, `RunType`, `RunBaseBranch`. Changed:
`DaemonRepository` → `DaemonRepository(DaemonId)`.

## Messages and outcomes

- `features::window::field_focus_changed` returns `Outcome::FieldBlurred(FieldId)` when `focused`
  is false (in addition to today's `focused_field` update).
- The root applies `Outcome::FieldBlurred(id)` through `features::required::blurred(state, id)`,
  which matches `id` to the one draft that owns it and calls `leave(id)`; an id whose draft is not
  open changes nothing.
