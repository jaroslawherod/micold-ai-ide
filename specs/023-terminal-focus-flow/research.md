# Phase 0 Research: Natural Terminal Focus Flow

Everything below was read out of the current tree and the vendored `iced` 0.14 sources. Nothing here
is inferred from the symptom alone — the two-press bug in particular had a plausible-sounding
explanation that the code contradicts, and it is recorded so nobody re-derives it.

## R1 — Why one press is swallowed (the reported bug's actual mechanism)

**Finding.** The press is not eaten by the focus release. It is eaten by a **widget-tree diff**: the
terminal's bottom bar removes a child when focus is released, every sibling after it shifts one
index left, and iced's positional state diff drops the `is_pressed` flag the button's release
depends on.

The chain, each link verified:

1. `TerminalPane::update` (`ui/material/terminal_pane.rs`) publishes `Message::TerminalFocusReleased`
   on any left press while `self.focused` and the cursor is **not** over its bounds — deliberately
   *without* `capture_event`, so "the click still reaches whatever is under it". That comment is
   correct, and it is why the naive explanation ("the release swallows the click") is wrong.
2. `iced_widget::button` publishes `on_press` on **`ButtonReleased`**, and only
   `if state.is_pressed` — a flag set on `ButtonPressed` and stored in that button's node of the
   widget tree (`iced_widget-0.14.2/src/button.rs`).
3. Between the press and the release, the published `TerminalFocusReleased` is applied and `view()`
   re-runs. In `ui/terminal.rs`, the bar pushes the release-focus `IconButton` **only**
   `if state.terminal_focused` — so it vanishes, and the mode toggle, the "+" instance button and
   the instance tabs, all pushed after it, each move one index earlier in the row.
4. `Tree::diff_children` zips old and new children **by position** and reuses state when the tag
   matches (`iced_core-0.14.0/src/widget/tree.rs`). The pressed control is now diffed against its
   left neighbour's node: `is_pressed` is either replaced by that neighbour's `false` or reset.
5. `ButtonReleased` arrives, `is_pressed` is false, nothing is published. The press is gone.
6. The second press works because focus is already released, so the bar no longer changes shape
   mid-click.

**Decision.** Fix it twice, at both altitudes.

- The rule fix (which the spec requires anyway): a press on a control that types nothing must not
  change focus at all (FR-005), so the bar never changes shape mid-press. Delete the click-outside
  release from `TerminalPane::update`.
- The structural fix: the bottom bar's **child list must not depend on focus**. Push the
  release-focus affordance unconditionally and gate only its `on_press`, so the row's shape is
  stable whatever focus does. Guarded by `tests/terminal_bar_stability.rs`.

**Rationale for doing both.** The rule fix removes today's trigger; the structural fix removes the
class. A focus-conditional child in a row of pressable siblings is a trap that any future feature can
step in again, and it fails silently — a swallowed press looks like a slow app, not like a bug.

**Alternatives considered.**

- *Publish the release on `ButtonReleased` instead of `ButtonPressed`.* Moves the tree churn after
  the click resolves, so the press survives. Rejected: it leaves the trap armed for anything else
  that re-shapes the bar, and it makes focus lag the pointer by the length of a click.
- *`capture_event` on the outside press.* Would make the double press mandatory rather than
  accidental. Rejected outright.
- *Keyed widget identity (`iced::widget::keyed`) for the bar.* Would survive a shifting sibling
  properly. Rejected as disproportionate: the bar has one focus-conditional child, and deleting that
  conditionality is smaller than adopting keyed rows for one row.

## R2 — Where the "does this control take the keyboard?" classification lives

The requirements checklist flagged FR-004 vs FR-005 ("accepts typed input" vs not) as the
classification the whole one-press rule rests on, and asked the plan to name where it lives so it
cannot drift.

**Decision.** It does not live anywhere, because it is **observed rather than classified**.

- Controls that type already report themselves. `material::FilledField` runs an `operate` pass over
  its control after the children have handled the event and publishes `on_focus_change(bool)` when
  the answer changes; `ui/focus.rs`'s `TrackFocus::track_focus` binds that to
  `Message::FieldFocusChanged(FieldId, bool)`, and `State.focused_field` holds it. All seven live
  text fields are wired. This landed on `main` as BUG-003's fix, for the visual affordance — this
  feature is its second consumer.
- Menus and dialogs are already state, and since feature 024 they are already *enumerated*: the
  overlay registry's `open_dialog(&State)` / `open_popovers(&State)` answer "is a surface open"
  without anyone naming a flag. That list is 024's FR-009 — one line per surface, and the only such
  list — so this feature reads it instead of writing a second one.
- Controls that do **not** type need no entry anywhere, because after R1's deletion a press on one
  of them does not touch focus. There is no list to forget to add to.

**Rationale.** A list of "focus-taking controls" would have to be extended by every future feature
that adds a field, and nothing would fail when it wasn't. Deriving the answer from state that the
components themselves report cannot drift: a new text field wired with `track_focus` participates
automatically, and one that isn't wired is visibly broken (it draws permanently at rest — BUG-003's
own symptom), so the failure is loud.

**Alternatives considered.**

- *A `focus_effect(&Message) -> Effect` classifier over all of `Message`.* Rejected: `Message` has
  131 variants; an exhaustive second match would be a compile-time guarantee bought with a
  maintenance surface larger than the feature.
- *An app-level focus registry every control registers with.* Rejected: iced already owns focus for
  the widgets that have it; a parallel registry would need reconciling with the real one.

## R3 — The keyboard holder: stored, or derived?

**Decision.** Derived. `State.terminal_focused: bool` is replaced by `State.terminal_released: bool`
plus:

```rust
pub fn terminal_focused(&self) -> bool {
    self.active_session.is_some()
        && !self.terminal_released
        && self.focused_field.is_none()
        && !self.any_surface_takes_keyboard()
}
```

**Rationale.** Nearly every requirement in the "automatic focus" and "bounds" groups becomes a
consequence instead of a rule someone must remember:

| Requirement | How derivation satisfies it |
|---|---|
| FR-009 default holder | The predicate *is* the default: true unless something says otherwise |
| FR-010 return when a transient finishes | The dialog closes, the registry reports nothing open, the predicate is true again — no restore stack |
| FR-012 / FR-016 session gone | `active_session.is_none()` ⇒ false, with nothing to clear |
| FR-012a launch | `Default::default()` has `terminal_released: false`, so a restored session is focused |
| FR-013–FR-015 window return | Nothing mutates on blur, so nothing needs restoring — the "suspended holder" has no runtime existence |
| FR-017 / FR-018 bounds | The registry's answer and `focused_field` are terms of the predicate, so the terminal *cannot* hold the keyboard alongside them |
| FR-019 output changes nothing | Output touches none of the four terms |
| FR-020 one holder, displayed only | `active_session` is the only session the predicate can be true for |

It also removes the FR-008a hazard at the source: the two `Task::done(Message::TerminalFocused)`
re-assertions in `main.rs` exist to win a race against a release published by the same click. With
the release deleted, there is no race, no follow-up message, and no frame in which focus is
momentarily elsewhere.

**Alternatives considered.**

- *Stored `enum KeyboardHolder { Terminal, Field, Overlay, App }` plus a `suspended: Option<Holder>`
  for window blur.* This is what the spec's Key Entities read like on a first pass. Rejected: it
  duplicates facts the overlay registry and `focused_field` already hold, and every duplicate is a chance for
  the two to disagree — precisely the failure mode `terminal_focused` has today.
- *Keep the bool and fix the assignments.* Rejected: seven write sites is what made project switch,
  mode toggle and instance switch each miss a case; the eighth would too.

**Consequence to accept.** `terminal_focused` stops being a field, so integration tests that build
`State { terminal_focused: true, ..Default::default() }` must set `terminal_released` (or drive the
message) instead. That is a mechanical edit in `tests/terminal_focus.rs`, and it is the point: the
state you can set is the decision the user makes, not the answer the application derives.

## R4 — `terminal_context_menu` is pane furniture, not a menu

FR-004 says a menu that opens on a press holds the keyboard; FR-007 says a press within the pane's
own furniture — "its scrollbar, status bar, and context menu" — leaves the terminal holding it. The
terminal's right-click menu is both.

**Decision.** FR-007 wins: `terminal_context_menu` is **not** a term of the predicate. It is drawn
inside the pane, its two items are Copy and Paste (the pane's own actions), and taking the keyboard
away to open it would mean a right-click stops the user typing.

**Consequence recorded, not fixed here.** Because the terminal keeps the keyboard while that menu is
open, `ui::subscription` still returns `Subscription::none()`, so Escape does not dismiss it — it
goes to the process, as every other key does. That matches the pane's existing behaviour, is
consistent with FR-022, and is out of scope; dismissal is by pressing elsewhere or choosing an item,
as today.

## R5 — The press that grants focus (FR-008b)

`TerminalPane::update` publishes `Message::TerminalFocused` on a press over its bounds when
`!self.focused`, then falls through to selection/mouse-report handling — but that handling asks
`press_routing(self.focused, …)`, and `self.focused` is the flag from the **view that is already on
screen**, i.e. `false`. So the granting press never reports to a mouse-aware program: in a TUI the
first press does nothing and the user presses again.

**Decision.** Compute the press's own answer: `let focused_now = self.focused || grants_focus;`
where `grants_focus` is true exactly for the left press over bounds that published
`TerminalFocused`, and route the press on `focused_now`. `press_routing` is already pure and
unit-tested; it gains no branch, only a truer argument.

**Rationale.** Same principle as everything else here: one press, one outcome. The write gate is
untouched, so a press still cannot reach a process that is not `Running`.

## R6 — Verification approach

- Pure logic — the predicate, the navigation set, press routing — is unit-tested in
  `crates/micold-client/tests/`, written failing first (Principle I).
- The `src/ui/` edits are thin wiring covered by Principle I's GUI exception, validated by
  `quickstart.md` Part B. That part is run headlessly with the repo's `visual-pass` skill (Xvfb +
  xdotool + screenshot), not by asking a human — the pass has to *see* the focus ring and the
  swallowed-press behaviour, and both are visible.
- The exception's precondition gets its own gate: `tests/terminal_bar_stability.rs` reads
  `src/ui/terminal.rs` and fails if the bottom bar's construction branches on terminal focus. This
  is the shape `tests/showcase_glue.rs` established when constitution 1.5.0 widened the exception —
  a precondition nobody checks is a precondition nobody keeps.
