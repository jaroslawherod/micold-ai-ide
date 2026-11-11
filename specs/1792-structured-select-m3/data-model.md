# Data model: 1792

No new persisted or core data. Existing: `composers.options_open: bool` (persisted as `composer_options_open`), `settings.reduce_motion: bool`.

- **Select trigger** (view-only): inputs label, value text, open, disabled, hover/focus/pressed from widget state, `Roles`. States: rest, hover, focus, pressed, disabled, open. Owned by `ui/material`; no state beyond what iced widget state keeps.
- **Expand track** (view-only): per `expand` wrapper instance, held in the composer view's widget tree: progress 0..1 toward `options_open`, duration from a motion token (zero under reduce motion). Created with the composer view of a session, dropped with it. One session's track never reads another's.
- **Transition**: closed to opening on toggle; opening to closing on a toggle mid-flight, from the current progress; any state to its target at once when reduce motion is on.
