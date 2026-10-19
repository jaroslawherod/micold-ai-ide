# UI contract: Daemon overlays

## Messages (`features::daemons::Msg`)

| Message | Effect |
|---|---|
| `AddOpened`, `EditOpened(id)` | existing; now also open the form dialog and remember the opener |
| `Saved`, `DraftDiscarded` | existing; `Saved` success and `DraftDiscarded` close the dialog |
| `RemovalConfirmed`, `RemovalCancelled` | existing; for `Pending::Edit` are the in-dialog Yes / No |
| `DetailsOpened(DaemonId)`, `DetailsClosed` | new; replace `DetailsToggled(DaemonId)` |

## Registered surfaces (`Layer::Dialog`)

| Surface | Id | Open when | Cancelled by (Escape, scrim, Cancel/Close) |
|---|---|---|---|
| `DaemonFormDialog` | `daemon_form` | `daemon_list.draft.is_some()` | `DraftDiscarded`; while a `Pending::Edit` confirmation shows: `RemovalCancelled` |
| `DaemonDetailsDialog` | `daemon_details` | `daemon_list.details.is_some()` | `DetailsClosed` |

`ConfirmDaemonRemovalDialog` stays registered but is open only for prompts other than `Pending::Edit`.

## Layout

- Title (daemon name or "Add daemon"), scrolling body (the existing `form()` sections; for Details the caution, errors, log), pinned actions: form: `Save daemon` (filled) + `Cancel` (outlined); confirmation: the prompt's confirm/decline labels in place of those; details: `Close`.
- The Settings page shows no form and no details block; the row's `Details` button is no longer a toggle.
- Width bounded as `SurfaceKind::Dialog`; height bounded by the window; body scrolls.
