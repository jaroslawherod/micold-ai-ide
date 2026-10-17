# Research: Required-field markers in forms (937)

Each entry: **Decision**, **Rationale**, **Alternatives considered**. Paths are relative to
`crates/micold-client/src/` unless they start with `crates/`.

## R1 — Where the requirement is decided, and where it is drawn

**Decision**: Split it in three. (1) Each form's render-free feature module says, per field, whether
the field is required now and whether it is empty (`required_fields()` on the draft, beside the
validation it mirrors). (2) A generic, iced-free `micold_core::required` module keeps which fields
the user has left and whether a submit was attempted (`RequiredMarks<K>`), and turns the two into a
`FieldRequirement` (`Optional | Required | Missing`). (3) The material components take that value
through one builder, `.required(FieldRequirement)`, and draw the marker, the error state and the
message themselves; `RequiredLegend` draws the explanation.

**Rationale**: FR-008 and SC-006 put the drawing in the shared components; Principle I puts the
decision logic in tested render-free code; FR-002 ties "required" to each form's own validation,
which already lives in the feature modules (`features/worktree_form.rs` `preview()`,
`features/runs.rs` `ParallelDialog::validate`, `features/settings.rs` `SettingsDraft::validate`,
`features/daemons.rs` `runtime_of`/`form_error`, `micold_core::project::RenameError`). Putting the
blur/attempt bookkeeping in core makes it one tested type instead of six copies (duplication budget).

**Alternatives considered**: a `required: bool` plus a separate `required_error: bool` on the
builder — two flags that can disagree (marker off, error on), which Principle V rules out. Deriving
"required" from the validators' first error — they stop at the first failure, and the spec's edge
case wants every empty required field marked together.

## R2 — What "blur" means for each kind of control (open item)

**Decision**:

| Control | Blur is | Reported by |
|---|---|---|
| `TextField` | the input loses the keyboard | existing `on_focus_change(false)` → `WindowMsg::FieldFocusChanged(id, false)` |
| `Select` | the select stops being *engaged* — it neither holds the keyboard nor has its list open — after having been engaged, without a choice in between | new `Select::on_blur(M)`, published from `Widget::update` |
| `Typeahead` (the existing-branch picker) | its list is dismissed (Escape, outside press, Tab away) | existing `on_dismiss` → `FormMsg::BranchDismissed` |

For `Select` the edge is detected in `update` against an `engaged` bit kept in `SelectState`
(`focused || open`), because focus taken away by the focus *operation* (`unfocus`, no shell in hand)
cannot publish; it is published on the next event the select sees. A pick is not a blur: picking
fills the field, so there is nothing to report.

**Rationale**: a select is "focused" by opening it or by Tab reaching it; the user has *left* it when
it closes without a choice or the keyboard moves on. That is the select analogue of a text field's
blur, and it is what Material's guidance on validating "when the user leaves the field" means for a
menu. The select already tracks both bits (`select.rs` `focused`, `open`), so no new state source.

**Alternatives considered**: treat selects as submit-only — leaves FR-006's blur rule unmet for half
the required fields (Type, Base branch). Report every close — a pick would then flash the error for
one frame before the value lands.

## R3 — One message for every blur

**Decision**: every blur reaches the root as `WindowMsg::FieldFocusChanged(id, false)` (the select
and typeahead join through `TrackFocus`/an explicit arm), and `features::window::field_focus_changed`
returns `Outcome::FieldBlurred(id)`; the root routes it to the open draft that owns `id`
(`features::required::blurred`, one `match` on `FieldId` groups). New `FieldId` variants:
`AddWorktreeType`, `AddWorktreeBranch`, `RunType`, `RunBaseBranch`; `DaemonRepository` becomes
`DaemonRepository(DaemonId)` so each remote daemon's repository step is its own field.

**Rationale**: `FieldId` is already the app's single field namespace (`features/window.rs:108`).
Recording the blur in the owning draft keeps each form's marks in its own draft (Principle II): a
blur never touches another form. The blur is recorded even when the guard in `field_focus_changed`
ignores it for `focused_field`, because "the user left this field" is true either way.

**Alternatives considered**: a global `blurred: BTreeSet<FieldId>` on the window — shared across
drafts, so reopening a form would inherit marks from the last one (Principle II). Per-form messages
(`FormMsg::NameBlurred`, …) — one variant per field across six forms.

## R4 — Rename dialog files (open item from the spec review)

**Decision**: confirmed. `ui/rename_dialog.rs` is the one shared view (`RenameSpec`); `ui/rename.rs`
(project) and `ui/worktree_rename.rs` (worktree) only fill a `RenameSpec`. Both drafts' validation
is `micold_core::project::RenameError { Empty, Whitespace }`. The change is in `rename_dialog.rs`:
the free-standing error `Text` under the field (`rename_dialog.rs:48`) becomes the field's own
required error, and the dialog gains the legend.

**Rationale**: read in full; no other rename view exists (`overlay/registry.rs:305,335`).

## R5 — Inventory of forms and required fields (FR-009)

**Decision**: these are the required fields FR-002 picks out, each traced to the rule that refuses
it empty. "When shown" fields count only while visible.

| Form | View file | Required field (condition) | Refused by |
|---|---|---|---|
| Add worktree, new/issue source | `ui/worktree_form.rs` | Type | `NamingError::NoType` |
| | | Name (blank after trim) | `NamingError::EmptyNameAfterSlug` |
| Add worktree, existing source | | Branch (no branch picked) | `preview()` → `EmptyNameAfterSlug` |
| Add worktree, all sources | | — Ticket, Daemon (defaulted), AI CLI (always a choice), First prompt: optional | |
| Run in parallel | `ui/parallel_dialog.rs` | Prompt, Type, Name, Base branch | `Invalid::{EmptyPrompt, NoType, EmptyName, NoBaseBranch}` |
| | | — Ticket, per-run Provider (always set): optional | |
| Rename project / worktree | `ui/rename_dialog.rs` | Project name / Worktree name | `RenameError::{Empty, Whitespace}` |
| Settings › Terminal | `ui/settings/terminal.rs` | Scrollback lines | `scrollback()` parse |
| Settings › Environment | `ui/settings/environment.rs` | Timeout; Long-task threshold; Warn at (when shown) | `timeout()`, `long_task_threshold()`, `WarnPercent::parse` |
| | | — Script path, Default AI CLI: optional | |
| Settings › GitHub | `ui/settings/github.rs` | each mapping entry's Label | `MappingErrorKind::Blank` |
| Settings › Jira | `ui/settings/jira.rs` | Site, Project key, Email (while enabled and any of the four holds input or a connection is stored); API token (same, and no token stored) | `jira_action()` / `JiraConnection::parse`, `ApiToken::parse` |
| Settings › Daemons, add/edit form | `ui/settings/daemons.rs` | Name | `RegistryError::BlankName` |
| | | SSH: Host | `SshTarget::new` (`host.is_empty()`) |
| | | — SSH User, Port, Key file; container Port (free port picked): optional | |
| Daemon container fields | `ui/settings/container_fields.rs` | Container name | `RegistryError::InvalidField("container_name")` |
| | | Image reference | `ImageRef::parse` → `ImageRefError::Empty` |
| | | Image file (source = imported file) | `ImageSourceProblem::MissingArchivePath` |
| | | — limits, runtime, image source, network: optional or always set | |
| Settings › Daemons, repository step | `ui/settings/daemons.rs` `repository_step` | Repository on the host | `RemotePathError::Empty` |
| Settings › Appearance | `ui/settings/appearance.rs` | none (no legend) | |

**Rationale**: each row was read from the validator named. Where a validator trims, the
requirement's "empty" trims too (edge case *Whitespace only*).

**Alternatives considered**: marking the four Jira fields required whenever Jira is enabled — the
form accepts all four empty (`!j.has_input()` → `Keep`), so FR-002 forbids it.

## R6 — Precedence, wording and position

**Decision**:

- Marker: the label text followed by `*` with no space (`"Name*"`), Material's own spelling, drawn
  in the label's role and colour so it floats and rests with the label.
- Legend: `"* required"` (`micold_core::required::LEGEND`), `TypeRole::Caption`, muted, the first
  child of the form's fields column, above its first field. Settings: once per section page, at the
  top of the section's content.
- Message: `"This field is required"` (`micold_core::required::MESSAGE`), which says the problem in
  words (FR-005).
- Precedence: a field's own error (`.error(Some(..))`) wins over the required message whenever both
  are present (FR-010); the forms produce their own errors only for non-empty values, and the
  emptiness errors they have today (`NoType`, `EmptyName`, `Empty`, "Enter a whole number…" on an
  empty field, `MappingErrorKind::Blank`, "A daemon needs a name.", …) stop being shown as form
  lines or field errors: an empty required field shows the required message instead.

**Rationale**: one constant per string makes SC-003 and SC-006 a source scan.

**Alternatives considered**: "Required" alone as the message — terse and reads as a label, not a
sentence.

## R7 — Submit buttons (FR-012)

**Decision**: Create (`ui/worktree_form.rs` `default_actions`) is enabled while the form is editing
and not prompting (`can_submit()` loses its `preview().is_ok()` clause, renamed `can_press()`), and
Start runs (`ui/parallel_dialog.rs` `actions`) is always enabled. Their handlers (`submitted`,
`confirmed`) call `marks.attempt()` and refuse, as they already do, on an invalid form. The settings
Save, the daemon form's Save and Rename were never disabled; their handlers gain `attempt()` (the
repository step's Add marks its one field).

**Rationale**: D2 (orchestrator default A). The shell already re-checks validity before it creates
(`main.rs:740`), so enabling the button cannot submit an invalid form.

If jaro answers **B** to D2: keep `on_press_maybe`, drop FR-012 and this entry; the required error
then appears only on blur.

## R8 — Height of the tallest forms

**Decision**: the legend adds one caption line plus `spacing::SM` to every form with a required
field. The add-worktree form's comments state that the tallest forms keep their height because the
panel-placement gate measures them (`tests/layout_snapshot.rs`, `tests/gates/`). The legend is
accepted as a deliberate height change: re-record the affected layout snapshots in the same task,
and keep the "no new rows" rule for everything else.

**Rationale**: SC-003 requires the legend; it cannot share a row with a field.

## R9 — Tests by layer

**Decision**:

- `crates/micold-core/src/required.rs` unit tests: `RequiredMarks` (fresh → nothing shown; blur →
  that field only; attempt → every required empty field; value → cleared without losing marks;
  not required → `Optional` even after attempt).
- `crates/micold-client/tests/features_*.rs`: each draft's `required_fields()` against its
  validator (a field is `required && empty` exactly when the validator refuses it for emptiness),
  and the blur/attempt routing (`features::required::blurred`).
- In-crate anatomy tests (`ui/material/text_field_anatomy.rs`, `select_anatomy.rs`,
  `form_field_anatomy.rs`): the marker is in the label text, the message replaces the supporting
  text, an own error wins.
- `crates/micold-client/tests/required_call_sites.rs` (new, modelled on
  `tests/field_focus_call_sites.rs`): no file under `src/ui/` other than `material/` contains the
  legend or message literal, and every inventory form calls `RequiredLegend` (SC-006).
- Quickstart Part B (visual pass): marker, legend, error state in light, dark and greyscale.
