# Contract: composer layout and large editor

## Messages (client)
- `Composer(OptionsToggled)`: flips `options_open`, persists.
- `Composer(LargeEditorRequested(session))`: from the chord or the icon button; ignored when an editor is open.
- `LargeEditor(Edited(action))`, `LargeEditor(Done)`, `LargeEditor(Cancelled)`.

## Keys
- Composer text area focused: Ctrl+Shift+M / Cmd+Shift+M -> request. In the editor: Ctrl/Cmd+Enter -> Done; Esc -> Cancel.

## Layout (top to bottom)
queue / hold / notice / [options row if open: Model, Effort, Permission] / Stop session + refusal note / chips / completion / input row: [activity indicator 24 dp | attach | text area (+ open-editor IconButton top-right) | settings IconButton 40 dp | Stop | Send] (#1764: settings swapped with the activity indicator and attach).

## Settings file
`composer_options_open` (bool, optional key, default false).
