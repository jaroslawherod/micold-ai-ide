# Research: Daemon settings overlay

Each decision lists rejected alternatives.

## R1. Host the form on the existing floating-surface registry

**Decision**: add `DaemonFormDialog` and `DaemonDetailsDialog` as registered `Layer::Dialog` surfaces
(`overlay/registry.rs` `register!`), drawn through the `DialogView` fn pointer, like
`ConfirmDaemonRemovalDialog` / `BindDaemonDialog` already are. Open state is read from
`daemon_list.draft` / `daemon_list.details`. Escape (`app::on_escape`) and the scrim click
(`ui/mod.rs` `Modal::on_dismiss(on_escape(state))`) then work and are identical by construction.

**Why**: the spec assumes the overlay registry gives scrim, Escape routing and modality; it does
(verified: `ui/mod.rs` ~L725-760). Zero new dismissal code.

**Inputs of the view**: `DialogView` receives only `(&State, ColorScheme, &EnvIncludeOutcome)`, so `ui/daemon_form_dialog.rs` rebuilds what `settings_view.rs` passes today from `State`: `MachineFacts.draft` from the settings draft's `daemon`, `legacy_default` from the daemon view model, `missing_cli` from `missing_cli_notice(availability)`, and `focused` from `state.window.focused_field`. `form`, `details_block` and `missing_cli_notice` become `pub(crate)`.

**Rejected**: (a) a bespoke overlay in the Settings view, which forks the dismissal logic and breaks
`tests/one_overlay_implementation.rs`; (b) keeping the form inline with a dimmed page, which is not modal (FR-006).

## R2. Overlay open iff the draft exists

**Decision**: no new flag. `daemon_list.draft.is_some()` is the form overlay; `draft.editing`
distinguishes edit from add. Details gets one `details: Option<DaemonId>` replacing
`details_shown: BTreeSet<DaemonId>` and `Msg::DetailsToggled` becomes `DetailsOpened(id)` /
`DetailsClosed`.

**Why**: the draft already exists, is cleared on save/cancel, and cannot be out of step with a flag
that does not exist (Principle V). Modal means at most one details target.

**Rejected**: a second `form_open: bool` (two states that can disagree); keeping the set (allows several
details open, meaningless for a modal).

## R3. A new shared `ScrollingDialog` builder

**Decision**: add `ui/material/scrolling_dialog.rs`: `ScrollingDialog::new(title, body, actions, roles)`
with `.width(f32)`, producing a `Surface` of `SurfaceKind::Dialog` whose body is a `scrollable`
and whose action row is outside the scroll area (pinned), height limited to the window. Reuses
`dialog::fields`/`actions` spacing and the §7.4 figures.

**Why**: `dialog::confirm` takes a short fields column and is not scrollable, and the daemon form is
many sections tall (FR-011). Principle VIII wants a shared builder-style primitive, not a one-off.
The body uses the existing `material::scrollable` style. Both new dialogs use it (Details with a Close action).

**Rejected**: wrapping `dialog::confirm` in a feature-local `scrollable` (one-off, and its action
row would scroll away); a free function with positional parameters (violates VIII builder rule).

## R4. Confirmation inside the form dialog

**Decision**: `Pending::Edit` (set by `save_checked`) is shown by the form dialog: the actions row
is replaced by the confirmation sentence and its two buttons (existing `RemovalConfirmed` /
`RemovalCancelled`). `State::form_confirmation()` returns the prompt only for `Pending::Edit`;
`ConfirmDaemonRemovalDialog::open_in` uses a `standalone_prompt()` that excludes it (so there is never
a second modal). `DaemonFormDialog` implements `FloatingSurface` by hand (not `floating_surface!`)
with a `confirming` field: `dismissal` is `RemovalCancelled` when confirming, `DraftDiscarded`
otherwise. `cancel` already clears `pending` and keeps the draft (edits kept).

**Rejected**: leaving the separate confirm dialog stacked over the form (the spec says in-dialog, not a second modal);
a single fixed `cancelled_by` (Escape would discard the edits the user wanted to keep).

## R5. Focus: trap and return

**Decision**: (a) Return: record an `Opener` (`Add` or `Row(DaemonId, Action)`) in `daemons::State`
on open; after a close message, `shell/daemons.rs` issues `iced::widget::operation::focus(id)` for
the opener's widget id. Buttons get an optional `.id(..)` (new builder method on the material `Button`)
(`TakesTheKeyboard` wrapper makes them focusable). (b) Trap: first task of implementation is a
failing test and probe of whether `focus_next`/`focus_previous` (`Message::FocusMoved`) leave
the overlay layer; if they walk the page behind, scope the operation to the overlay container id.

**Why**: iced has no built-in dialog focus management; the codebase already issues focus tasks from `main.rs`/`shell`.

**Rejected**: no focus return (violates FR-009); a global focus-restore stack (over-general for one dialog).

## R6. Cross-window registry change

**Decision**: nothing in the reducer replaces `draft` on `DaemonRegistryChanged` (verify by test).
On Save, `save_checked` for an edit whose id is no longer in `model.registry` returns a `FormError`
shown in the dialog ("This daemon no longer exists"), storing nothing; otherwise last save wins as today.

`DaemonDetailsDialog::open_in` returns `None` when its daemon is no longer in the registry (replacing the `details_shown.retain` pruning in `RemovalConfirmed`), so no empty modal can stay open; a test covers it.

**Rejected**: refreshing the draft from the registry (loses user edits, FR-014); silently re-adding the daemon.

## R7. System-raised prompts while an overlay is open

**Decision**: `show_next_drift` only opens a prompt when `pending.is_none()` **and** no form/details overlay is open;
closing the overlay (save, discard, details closed) calls `show_next_drift`. Queued entries keep
their order and are never applied to the draft.

**Rejected**: showing the drift prompt over the form (a second modal that would overwrite `pending`, and `Pending::Edit` with it).

## R8. Unchanged on purpose

Validation, per-kind sections (890), counted confirmation wording, removal flow, bind picker, the
Settings page save. Out of scope: #1091 accordions.
