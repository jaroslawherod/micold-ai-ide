# Contract: the rest-delay tooltip

**Modules**: `crates/micold-core/src/tooltip.rs` (NEW), `crates/micold-client/src/ui/cdk/tooltip.rs`,
`ui/cdk/motion.rs`, `ui/material/mod.rs`, `ui/material/line_clamp.rs` (NEW) ·
**Research**: R7–R9, R11, R12 · **Data model**: [§5–6](../data-model.md)

## 1. The rule: `micold_core::tooltip::RestTimer`

```rust
pub const REST_TOLERANCE: f32 = 4.0;                    // logical pixels
pub enum RestTimer { Away, Waiting { anchor: (f32, f32), since: Instant }, Open, Spent }
pub struct Rest { pub open: bool, pub wake_at: Option<Instant> }

impl RestTimer {
    pub fn observe(&mut self, cursor: Option<(f32, f32)>, now: Instant, delay: Duration) -> Rest;
    pub fn press(&mut self);
    pub fn reset(&mut self);
}
```

The transition table is [data-model §5](../data-model.md). What `tooltip_rest.rs` holds:

| Behaviour | Requirement |
|---|---|
| Still for `delay` → open; at `delay − 1 ms` → not open | FR-015, SC-003 |
| A move of more than 4.0 px before opening restarts the wait from that instant | FR-016 |
| Moves of at most 4.0 px from the anchor do not restart it, however many | FR-016 |
| Moving every 100 ms by 10 px for 10 s never opens | SC-004 |
| Once open, any movement over the trigger keeps it open | spec Assumptions |
| Cursor off the trigger → closed, and the next entry waits the full delay | FR-017 |
| `press()` → closed, and stays closed until the cursor has left | FR-017 |
| `reset()` → closed, and waits the full delay again | FR-017 |
| `wake_at` is `Some(since + delay)` only while waiting | FR-018 |

## 2. The widget: rest mode of `cdk::tooltip`

`cdk::tooltip::Tooltip` gains `after_rest(Duration)` and `subject(u64)` (NEW, chainable). Without
`after_rest` the widget behaves as today (all twenty existing call sites).

In rest mode its state holds a `RestTimer` and the last subject. On each event:

| Event | Action |
|---|---|
| `Mouse(CursorMoved)`, `Mouse(CursorEntered/Left)` | `observe(cursor over the trigger?, Instant::now(), delay)` |
| `Window(RedrawRequested(now))` | `observe(cursor over the trigger?, now, delay)`: this is where a still cursor crosses the deadline, and where a row that moved under a still cursor (scroll, a narrowed list) is seen against its new bounds |
| `Mouse(ButtonPressed)` over the trigger | `press()`; the event is **not** captured, so the trigger still receives the click |
| the subject differs from the stored one | `reset()`, store the new subject |

After each: if `Rest.open` changed, the widget invalidates its layout so the overlay appears or
goes; if `Rest.wake_at` is `Some(t)`, it calls `motion::wake_at(shell, t)`.

"Cursor over the trigger" is `cursor.position_over(trigger bounds)`, so a row hidden by the
list's clip, or covered by another overlay, is not hovered. Each row is its own widget with its
own state: moving to another row is `Away` for the first and a fresh `Waiting` for the second
(FR-016, "another row always starts the 3 seconds again").

`cdk/` holds no appearance (`cdk_no_appearance.rs`): the delay and the subject are behaviour.

## 3. Waking once: `cdk::motion::wake_at` (FR-018)

```rust
pub fn wake_at<M>(shell: &mut Shell<'_, M>, at: Instant);   // NEW
// body: shell.request_redraw_at(window::RedrawRequest::At(at));
```

`tests/idle_requests_no_frames.rs` is extended:

- **Source half.** In `src/ui/`, frame requests exist only in `cdk/motion.rs`, and there exactly
  two: the `animating()`-guarded `request_redraw` in `Progress`, and the `request_redraw_at` in
  `wake_at`. `wake_at` has exactly one caller outside `motion.rs`: `cdk/tooltip.rs`.
- **Behaviour half.** A rest-mode tooltip driven with a cursor at rest asks for exactly one
  redraw, at `since + delay`, and for none once open. Away, spent, and a tooltip without
  `after_rest` ask for none.

## 4. Subject (FR-017)

Widget state is kept by position in the tree. When the list narrows, a different issue arrives at
a position whose state may be `Open` or part-way through `Waiting`. `subject(key)` makes the
identity explicit: a changed key resets the timer, so an open tooltip closes and the 3 seconds
start again for the row now under the cursor. A tooltip without a subject never resets this way.

## 5. At most `n` lines: `clamp_to_lines` and `Tooltip::max_lines`

```rust
// micold_core::tooltip
pub fn clamp_to_lines<'a>(text: &'a str, max_lines: usize, lines_of: impl Fn(&str) -> usize) -> Cow<'a, str>;
```

Rules: [data-model §6](../data-model.md). `tooltip_clamp.rs` drives it with a fake measure
(`lines_of = ceil(chars / 40)`):

| Input | Output |
|---|---|
| 100 characters, 3 lines | unchanged, borrowed, no `…` |
| 120 characters exactly (3 lines) | unchanged |
| 500 characters of words | ≤ 3 lines by the measure, ends in `…`, cut after a whole word |
| 500 characters without spaces | ≤ 3 lines, ends in `…` |
| a text ending in `…` that overflows | ends in one `…` |
| any text, `max_lines = 3` | `lines_of(output) <= 3` (property over generated lengths) |

`material::Tooltip::max_lines(usize)` (NEW, chainable) builds the label with
`material::line_clamp::LineClamped` (NEW, modelled on `material/ellipsized.rs`): at layout time it
measures a `Caption` paragraph at `TOOLTIP_MAX_WIDTH` less the panel's padding with
`Wrapping::WordOrGlyph`, takes `lines = ceil(height / line height)`, calls `clamp_to_lines`, and
caches the result for that text and width. Without `max_lines` the label is built as today.

## 6. `material::Tooltip` (FR-028)

```rust
impl<'a, M: 'a> Tooltip<'a, M> {
    pub fn after_rest(self, delay: Duration) -> Self;   // NEW
    pub fn max_lines(self, lines: usize) -> Self;       // NEW
    pub fn subject(self, key: u64) -> Self;             // NEW
}
```

- `material_builder_api.rs` holds the three as chainable builder methods.
- The showcase's `Tooltip` entry (`showcase/sections/floating.rs`) gains an instance with
  `after_rest(3 s)` and `max_lines(3)` over a long text, captioned so a reader knows to hold the
  cursor still.
- `docs/development/component-library.md` describes the rest mode, the line limit, the subject
  and the second frame door, in the milestone that adds them.

## 7. Relation to spec 036

036 adds a show delay counted from pointer entry. It is a different builder method on the same
component. This contract neither uses nor forbids it; a tooltip uses one or the other.
