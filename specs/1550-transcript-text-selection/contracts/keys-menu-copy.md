# Contract: copy keys, context menu and copy controls

## Keys (`features/composer.rs::transcript_key(press, phase, selecting, mac) -> KeyIntent`)

Reached only for key presses no widget captured, while `transcript_keys` is subscribed (the active
session shows a transcript and its turn can be interrupted, or `focused_field ==
FieldId::Transcript(session)`). `selecting` = the session has a non-empty selection **and** the
transcript holds focus.

| ID | Platform | Press | selecting | busy | Intent |
|---|---|---|---|---|---|
| K-1 | Linux, Windows | Ctrl+C | yes | any | `CopySelection` |
| K-2 | Linux, Windows | Ctrl+Shift+C | yes | any | `CopySelection` |
| K-3 | macOS | Cmd+C | yes | any | `CopySelection` |
| K-4 | macOS | Ctrl+C | yes | any | `CopySelection` |
| K-5 | all | Ctrl+C | no | yes | `Interrupt` (#941 FR-012a, unchanged) |
| K-6 | all | Ctrl+C | no | no | `Edit` (nothing) |
| K-7 | all | Ctrl+Shift+C, Cmd+C | no | any | `Edit`; never `Interrupt` |
| K-8 | Linux, Windows / macOS | Ctrl+A / Cmd+A, transcript focused | any | any | `SelectAll` |
| K-9 | all | Escape | — | yes | `Interrupt` (unchanged) |

- **K-10 Composer focused**: the composer's `composer_key` decides (#941 FR-012a); the transcript
  selection is neither copied nor cleared. US2 scenarios 4 and 6.
- **K-11 Other focus**: with focus elsewhere, copy and select-all do nothing to the transcript.
  FR-019a.

## Context menu (`micold_material::ContextMenu`)

- **M-1** Opens at the right-press origin inside the transcript with **Copy** and **Select all**.
- **M-2** Copy is disabled (`message: None`) while the selection is empty, else copies it (S-12) and
  gives the transcript focus.
- **M-3** Opening or using the menu does not change or clear the selection.
- **M-4** Keyboard-operable; Escape or an outside click closes it (existing component behaviour).

## Copy controls (`TranscriptMsg::Copy { session, key, target }`)

- **C-1** `ToolValue` on the tool card header, shown iff `work_value` is `Some`; copies it uncut.
  FR-015.
- **C-2** `ToolInput` and `ToolResult` on an unfolded card (result only once one exists); copy the
  held text, the text so far while streaming. FR-016, US3 scenario 6.
- **C-3** `DiffPath` beside an inline diff's path. FR-017.
- **C-4** Every control is an `IconButton` with `TakesTheKeyboard`; Enter/Space copy as a press does;
  its state is `CopyState` per window keyed by `(session, key, target)`; only the activated control
  shows Copied/Failed, for `COPY_CONFIRM`. FR-018.
- **C-5** An empty value shows its control and copies the empty string. Edge Cases *Empty values*.
- **C-6** #940's code-block control is unchanged in look and behaviour (`Code(CodeId)` target).
