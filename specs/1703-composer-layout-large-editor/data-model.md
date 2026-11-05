# Data model

## Settings.composer_options_open (persisted)
- `bool`, default `false`; serde-defaulted so older or unreadable files read collapsed.
- Written only by the toggle and auto-expand; never by the settings form save.

## ComposerLayout (client state, `features/composer.rs`)
- `options_open: bool`: initialised from Settings at boot; toggled by `OptionsToggled`; set true by a model/effort/permission refusal. Every change persists.

## LargeEditor (client state, `features/large_editor.rs`)
- `Option<LargeEditor { session: SessionId, content: text_area Content }>`; `None` = closed.
- Transitions: `Requested(session)` (only when none open and text area focused) -> Some with the session's draft text, cursor at end; `Done` -> write text to `session`'s draft only (also if ended), close; `Cancelled` -> close, draft unchanged.
- Attachments are never read or written.
