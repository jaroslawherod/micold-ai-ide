# The component library

How the rendering layer is organised, and the one rule that keeps it that way.

## Two layers

Everything that decides an appearance lives in `crates/micold-client/src/ui/`, split in two:

**`ui/cdk/`** — behavior primitives. Things that *do* something without deciding how it looks: the
overlay that positions and stacks floating surfaces, the `Progress` track that advances an
animation and knows when to stop. A primitive here must not hard-code a colour, a corner radius or
a duration.

**`ui/material/`** — styled components. `Button`, `TextField`, `Modal`, `NavigationDrawer`,
`TreeView`, and the rest. These decide appearance, and they are the only place that may. The style
layer (`ui/material/style.rs`) is *internal to this directory* — nothing outside it can reach the
functions that turn a design token into a widget style.

Everything else under `ui/` — `sidebar.rs`, `toolbar.rs`, `shell.rs`, the dialogs — is a **feature
module**. Feature modules compose components. They do not style widgets.

## The rule

> A feature module composes components. It never reaches for the rendering stack's widgets to style
> them itself.

Concretely, a feature module may use layout primitives (`row!`, `column!`, `Space`, `stack!`) and
components from `material::`. It may not call `.style(...)` on an iced widget, construct a
`container::Style`, or import `ui::material::style`.

The reason is not tidiness. When a call site can style a widget directly, the design system becomes
advisory: two places that should look the same drift, and changing the look means finding every
place that decided it. Making the styling layer unreachable turns "use the component" from a
convention into the only thing that compiles.

### Why this is enforced by tests rather than review

Three gates run in CI. Each exists because the corresponding mistake is easy to make and invisible
in a diff:

| Gate | File | What it catches |
|---|---|---|
| Boundary | `tests/material_boundary.rs` | a feature module importing the style layer or styling a raw widget |
| Builder API | `tests/material_builder_api.rs` | a component that is not built the same way as every other one |
| Opacity ratchet | `tests/component_api_opacity.rs` | a public component signature exposing *how* it works |

The ratchet is worth understanding before you add a component. It scans every public signature in
the library for a short list of forbidden things — progress values, style closures, animator
handles, rendering-stack types — and holds a `REMAINING` list of known exceptions. It fails **both**
when a new leak appears *and* when the list names something already fixed. It is currently empty,
which is the state it is meant to stay in.

## Components own how they look; the application owns what is true

The dividing line:

> If the state would still matter with animation disabled, it is the application's. If it exists
> only to make a transition look right, it is the component's.

So `sidebar_hidden` is application state — it is written to disk and restored next run. How far the
drawer has slid is not; the drawer owns that, in the widget tree, where the renderer already keeps
per-instance state.

This has a consequence worth stating plainly: **there is no central animator**, no enumeration of
animated elements, and no animation clock. A component that animates holds a `cdk::motion::Progress`
in its widget-tree state and asks the runtime for the next frame only while it is moving. Two
instances cannot interfere, because neither can see the other. A removed component drops its state
with it — so "nothing animates after an element disappears" is structural rather than policed.

The corollary for callers: a component takes a **destination**, never a position.

```rust
// The caller says where it should be:
Modal::new(dialog, roles).shown(state.overlay != Overlay::None)

// Not how far along it currently is:
fade(dialog, progress)   // ← what this used to look like
```

## `UnreadMark`: one mark for three places

`UnreadMark` (`ui/material/unread_mark.rs`, contract `specs/039-session-attention-notifications/contracts/unread-mark.md`)
is a filled `primary` circle, optionally followed by a count and the word `unread`. It says "you
have not looked since this session last finished a turn"; it is not the `ActivityBadge`, which says
what the session is doing now and sits in a row's leading slot. The mark sits at the trailing edge.

No host draws it by hand. Each of the three places has a host API that builds it:

| Place | API | Draws |
|---|---|---|
| A session's row | `TreeItem::unread(bool)` | the mark at the trailing edge and an emphasised label |
| A switcher row | `MenuItem::trailing_mark: Option<usize>` | `● n unread`, muted, under the running count; `None` and `Some(0)` draw nothing |
| The switcher's button | `Button::trailing_mark(count, tooltip)` | `● n` inside the button, with a tooltip saying what the number counts; zero draws neither |

`.count(0)` renders nothing at all. The mark's fill comes from one function, `unread_mark::fill`,
so the contrast gate measures the colour that is drawn. The showcase has an entry for each place,
in both schemes.

## `Tooltip`: the rest mode and the line limit

`material::Tooltip` has three optional builder steps. Without them a tooltip behaves as it always
did: it opens as soon as the cursor is over its content.

| Step | Effect | Rule |
|---|---|---|
| `.after_rest(Duration)` | Opens only after the cursor has rested on the content for the delay. A move of more than 4 px (`REST_TOLERANCE`) restarts the wait; a press closes it until the cursor leaves. | `micold_core::tooltip::RestTimer` |
| `.max_lines(n)` | The label is cut to at most `n` `Caption` lines at `TOOLTIP_MAX_WIDTH` less padding, after a whole word, ending in an ellipsis. | `micold_core::tooltip::clamp_to_lines` |
| `.subject(u64)` | For a tooltip on a row a list reuses: a changed key closes an open tooltip and restarts the wait. | the widget's `diff` and `update` |

The wait needs a wake-up with no event to carry it, so `cdk::motion` has a second frame door beside
the guarded next-frame request: `wake_at(shell, Instant)`, the only `request_redraw_at` in
`src/ui/`. `tests/idle_requests_no_frames.rs` holds both: exactly one next-frame request behind
`animating()`, exactly one timed request that is the whole body of `wake_at`, and `wake_at` has one
listed caller, `ui/cdk/tooltip.rs`. A new caller goes into that test's `CALLERS` list with the reason
it stops asking.

The line limit is `ui/material/line_clamp.rs`, which shapes the paragraph and passes its measured
line count to `clamp_to_lines`; its tests are in-crate because `ui::material` is `pub(crate)`.

## Adding a component

1. Put it in `ui/material/`, one file per component, and register it in `mod.rs`.
2. Give it a builder: `Thing::new(required, args, roles)` plus chainable `.option(x)` steps, and a
   `From<Thing> for Element` at the end. The builder gate checks this.
3. Take design tokens (`Roles`, `spacing::*`, `type_scale::*`), never literal colours or sizes.
4. If it animates, own a `Progress` and take a destination. Do not add a parameter that carries
   progress in — the ratchet will fail, and it is right to.
5. If it needs to tell the application something, emit a message describing a **decision** ("the
   edge was dragged to x"), not a mechanism ("a drag started").
6. Add it to [the component showcase](component-showcase.md) — an `Entry` in
   `src/showcase/catalogue.rs` plus a render function in `src/showcase/sections/`. This is not
   optional and not a courtesy: `tests/showcase_completeness.rs` fails the build naming your component
   until you do, because a catalogue that silently omits things is worse than no catalogue at all.

Point 5 is the one most often got wrong. The resize handle used to emit `SidebarDragStarted` and
`SidebarDragEnded` so the application could mount a pointer-capture layer on its behalf. Owning the
drag itself let all of that be deleted; what remained was the single message that carried a
decision.

## Pickers: one foundation, two controls

A **picker** is a field with a list of choices anchored beneath it. There are two — `Typeahead` for
searching a long list, `Select` for choosing from a fixed one — and everything they have in common is
one place rather than two similar places.

| | Shared, in `material::picker` / `cdk::picker` | Each control's own |
|---|---|---|
| Look | the row, the panel it sits on, the grow-and-fade transition | the field: a search box, or a trigger with a chevron |
| Behaviour | anchoring, flipping, dismissal, the keyboard rule | where the answer to a key *goes* |
| Timings | `short_3` in, `short_2` out, §6.3's menu rows | — |

Neither control states a duration or a curve. If you are writing a third picker, you do not either:
call `picker::animated_menu` and pass `picker::EXIT` to the base, and it matches the other two by
construction. `src/ui/material/picker_parity.rs` builds both existing controls and compares their
lists rectangle for rectangle, so "they look the same" is measured rather than intended.

### Searching a long list: `Typeahead`

Any picker over more entries than fit on screen consumes `material::Typeahead` rather than building
a second one. It is the branch picker's control, but nothing about it is branch-shaped — a check
(`tests/typeahead_is_generic.rs`) fails the build if either half of it names a branch, a worktree or
git.

```rust
material::Typeahead::new(&self.query, rows, Message::QueryChanged, roles)
    .placeholder("Search branches…")
    .open(self.list_open)
    .highlighted(self.highlight)
    .selected(selected_index)
    .empty_message("No branches match that search.")
    .on_focus(Message::Focused)
    .on_move(Message::HighlightMoved)
    .on_dismiss(Message::Dismissed)
    .on_pick(|i| Message::Chosen(i))
    .into()
```

What it does **not** do is as important as what it does:

- **It does not match.** Rows arrive already matched, already ranked and already carrying the byte
  ranges to emphasise. The matcher is `micold_core::typeahead` — render-free, so the tiers, the
  ranking rules and the keyboard rule are all unit-tested rather than clicked at.
- **It holds no state.** Whether the list is open, where the keyboard is, and what is selected are
  all the caller's, passed in and echoed back as messages. That is what lets an open list with no
  rows exist at all — the state that shows the no-match message.
- **It knows nothing about your domain.** Its row is `{ label, spans, enabled }` and, optionally,
  a second line. Whatever explains an unavailable row must already be inside `label`, and the second
  line is plain text the caller composed; the component never learns what it is made of.

A row may carry that second line through `Row::details(text, spans)`. A row **with** details is two
wrapping lines: the label at the body role, the details under it at the caption role in the
secondary colour. Both wrap, a word wider than the row breaks inside the word, and nothing is
truncated; the row is as tall as its text needs, never shorter than a menu item, and the picked-row
marker sits beside the label's first line. A row **without** details is unchanged: fixed height, one
truncated line.

So a new picker's work is a mapping — your candidate type to a `Row`, and a row index back to your
candidate — plus the four messages. The branch picker's version of that mapping is about twenty
lines in `ui/worktree_form.rs`, and it is the only place branch vocabulary and component vocabulary
meet.

The behaviour half, `cdk::picker`, is the **only** module in the library allowed to write its own
`Widget::overlay()`, and `tests/one_overlay_implementation.rs` holds it to saying why: the list
anchors to the field's own on-screen bounds so it works inside a content-sized dialog, and it draws
rows that emphasise individual characters, which the rendering stack's own menu cannot. Both pickers
go through it, so adding a third adds no second mechanism.

### Choosing from a fixed list: `Select`

```rust
material::Select::new(&ConventionalType::ALL, self.chosen, Message::TypeChosen, roles)
    .label("Type")
    .placeholder("Select…")
    .into()
```

Shorter than the type-ahead's, and the difference is the interesting part: **there is no `.open()`,
no `.highlighted()`, and no dismiss or move message.** A select holds its own openness and its own
keyboard position, because nothing outside it is coupled to either — its options are fixed by the
call site, so there is no query for the list to be a function of. The type-ahead is the other way
round for exactly that reason: its rows *are* a function of caller-held state, so its openness is the
caller's too.

That asymmetry is deliberate and is what closed accepted fidelity gap #3. §7.7 wants the active
indicator thickened while the list is **open**, and the previous select — built on the rendering
stack's `pick_list` — reported its open state to its own style closure and to nobody else, so
`Select::active` had to be supplied by a caller and none was. Openness being the widget's own is what
lets the indicator answer from the control's own knowledge, and `active` left the builder with it.

So: if the list's contents depend on something the screen holds, the screen holds the openness too.
If they do not, the widget holds it. Do not make a new picker symmetric with the other one for
symmetry's sake — that is the mistake this arrangement exists to avoid.

### Keeping the highlighted row in view

When rows differ in height, a fixed scroll step cannot keep the highlight visible. `menu_element`
wraps the highlighted row, and only it, in a container carrying the widget `Id` `PICKER_HIGHLIGHT`.
`ui::picker_scroll::picker_highlight_into_view` is a `Task` that runs a two-pass widget operation,
like `focus::scroll_focused_into_view`: pass one finds that row and the innermost scrollable it is
inside, pass two scrolls only that scrollable, by `focus::delta_into_view` with a margin of 0 (the
focus operation uses its 16dp `MARGIN`). A row already wholly visible does not move the list, and a
row taller than the list is aligned to its top.

Only the issue picker chains it today, after `FormMsg::IssueHighlightMoved`
(`shell::issues::on_issue_highlight_moved`). The branch picker and `Select` have rows of one base
row height and are not wired. A caller with rows of differing height chains the task after its own
highlight-moved message.

## Switching between options: `ToggleChip`

```rust
material::ToggleChip::new("GitHub issue", Message::SourceChosen(Source::Issue), roles)
    .active(source == Source::Issue)
    .disabled(!github_available)
    .into()
```

`.disabled(true)` is a builder step like the rest, not a second chip type. A disabled chip emits no
press and does not ripple, because a ripple would report a press that never happens. It takes
Material's disabled treatment from `tokens::state`: its content at `DISABLED_CONTENT` (38%) and, when
active, its container at `DISABLED_CONTAINER` (12%). Say *why* it is disabled next to it. The
add-worktree form puts the reason in the caption under its source switch, because a greyed chip on
its own does not tell the user what to fix. The gallery poses it as `disabled`.

## Where the boundary genuinely bends

Two things sit outside the library on purpose, and both are documented at their call sites:

- **The terminal grid.** It draws cells from a VT model, not components, and is a feature module for
  that reason.
~~**Widget-attached dropdowns.** `Select` is built on iced's `pick_list`, whose overlay is
positioned by the rendering stack from the trigger's on-screen bounds.~~ **No longer an exception.**
It was one for a real reason — a dropdown inside a content-sized dialog has nothing window-level to
anchor against, and `pick_list` was the only thing that could position from the trigger's own bounds.
`cdk::picker` does that now and is ours to style, so the select is a component like any other and the
sanction that named it was **deleted** rather than left standing. The gate that holds the list to one
entry is what forced that: `tests/one_overlay_implementation.rs` fails while a sanction that no
longer applies is still listed.

So the list is one exception, not two. If you find yourself wanting a second, the gates will tell you
— and the honest move is to extend a component rather than add an entry to a `REMAINING` list.
