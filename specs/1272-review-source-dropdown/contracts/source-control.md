# Contract: the review source control (#1272)

Three interfaces, one per layer. Types are in [../data-model.md](../data-model.md).

## §1 Core: `micold_core::review::source_menu`

```rust
pub fn rows<'a>(input: &MenuInput<'a>) -> Vec<Row<'a>>;
pub fn stops(rows: &[Row<'_>]) -> Vec<(Stop, bool)>;
pub fn label<'a>(input: &MenuInput<'a>) -> Label<'a>;
impl Label<'_> { pub fn full(&self) -> String; }
pub fn tooltip(base_line: Option<&str>, label: &Label<'_>) -> String;
impl Stop { pub fn keeps_open(self) -> bool; }
```

Pure: no I/O, no clock (the stale flag arrives in `PrRows::List`), no iced. Every function is total:
any `MenuInput` yields rows, stops and a label, and none panics or reports an error (FR-005).

## §2 Client reducer: `features::changes`

- `source_menu_input(view: &OpenView, enabled: bool, now: u64) -> MenuInput<'_>` — the only bridge
  from `OpenView` to §1.
- Messages per data-model §9. `SourceMenuPicked` with no highlight, or on a disabled stop, does
  nothing. Every pick of a source or Files goes through the existing `SourceChosen` /
  `FilesPaneToggled` arms, so 1003 and 1150 behaviour (diff read, kept selections, last pick wins,
  PR-disappears fallback, per-entry memory, tree and browsed-file re-read, Back cleared) has one
  implementation (FR-006, FR-007).
- No new `Effect`: the control causes only the reads the existing messages already cause.

## §3 Shared component: `ui::material::MenuSelect` (NEW)

Builder form (Principle VIII):

```rust
MenuSelect::new(fixed_label, rows, open, on_toggle, on_dismiss, roles)
    .title(Option<&str>)                 // the elastic, ellipsized part of the label (R5)
    .tooltip(String)
    .keyboard(highlight, stops, highlighted_enabled)
    .on_move(|Direction| M)
    .on_pick(M)
    .max_list_height(f32)
    .into()                              // Element<'a, M>

MenuSelectRow::option(content, chosen, on_pick)
MenuSelectRow::option_with_chips(content, chosen, on_pick, Vec<ToggleChip<M>>, note: Option<&str>)
MenuSelectRow::heading(text) / ::note(text)
```

- Composes `cdk::picker::Picker` (floating, not `in_flow`), `cdk::tooltip::Tooltip`, `Ellipsized`,
  `ToggleChip`, and the `material::picker` list surface and row states; the view passes
  `PullRequestRow` elements as pull-request row content.
- The trigger is a button: press, `Enter` or `Space` emits `on_toggle`. The list opens over the
  body below the trigger and never pushes layout; it scrolls inside itself above
  `max_list_height`.
- A press on a chip emits the chip's message and leaves openness to the caller; a press outside the
  list, `Esc` or `Tab` emits `on_dismiss`.
- Light and dark themes through `Roles`; identical on Linux, macOS and Windows (no platform branch).
- Showcase: closed, closed with a long title, open with every `PrRows` state, open on the Default
  entry (Committed unavailable with its note).
