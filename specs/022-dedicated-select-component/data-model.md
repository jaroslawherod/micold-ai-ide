# Phase 1 Data Model: Dedicated Select Component on a Shared Picker Base

Almost nothing here is application data. The feature's state is presentation state, and the point of
the design is that most of it lives inside widgets where no screen has to thread it. This document
records what exists, where it lives, and — for the two pieces that could plausibly have gone
elsewhere — why they live where they do.

---

## 1. The shared base — `micold_client::ui::cdk::picker` (renamed from `cdk::typeahead`)

Behaviour only. Names no colour, type, shape or duration (`cdk_no_appearance.rs`).

| Field | Type | Notes |
|---|---|---|
| `field` | `Element` | the trigger, already drawn — a text input for one picker, a pressable row for the other |
| `menu` | `Element` | the list, already drawn *and already wrapped in its transition* (§3) |
| `open` | `bool` | whether the caller wants the list showing |
| `gap` | `f32` | distance between field and list. Arrives from the caller because spacing is appearance — the existing precedent, unchanged |
| `exit` | `Duration` | **new.** How long the list keeps existing after `open` goes false. `Duration` rather than the `f32` first specified — `cdk::motion` already trades in durations, so `f32` would have converted twice. See §1.1 |
| `highlight` | `Option<usize>` | which row the keyboard is on |
| `rows` | `usize` | how many it can reach |
| `highlighted_enabled` | `bool` | whether the one it is on can be taken |
| `on_move` / `on_pick` / `on_dismiss` / `on_focus` | `Option<…>` | unchanged |

### 1.1 New widget-tree state: `Visibility`

| Field | Type | Notes |
|---|---|---|
| `progress` | `Progress` | 1.0 while open, falling to 0.0 across `exit` after it closes |

**Why the base holds this and not the material layer.** *Whether the list is on screen at all* decides
whether `overlay()` returns anything, and `overlay()` is the base's. The material layer's own `scale`
and `fade` tracks decide what it looks like on the way out; this one decides how long there is
anything to look at. Keeping the two in one place would mean either the base learning a duration's
meaning (forbidden) or the material layer deciding whether an overlay exists (not its job).

**Invariant**: `progress > 0` ⟺ `overlay()` returns `Some`. A list at rest holds no track and asks
for no frames — `idle_requests_no_frames.rs` is the gate.

---

## 2. The select — `micold_client::ui::material::Select`

### 2.1 Builder inputs

| Input | Type | Required | Notes |
|---|---|---|---|
| `options` | `&'a [T]` | yes | `T: Clone + ToString + PartialEq` — unchanged bounds, so `ConventionalType` still satisfies them with no changes of its own |
| `selected` | `Option<T>` | yes | the current choice |
| `on_selected` | `Fn(T) -> M` | yes | emitted when a choice is made |
| `roles` | `Roles` | yes | theming, per Principle VIII |
| `placeholder` | `String` | no | default `"Select…"` |
| `label` | `Option<String>` | no | rendered inside the container, above the value |
| `supporting` | `Option<String>` | no | beneath the container |
| `error` | `Option<String>` | no | replaces the supporting text and recolours the chrome |
| `disabled_options` | *(not added)* | — | FR-017 says an option **may** be presented unavailable. Deferred: no consumer needs it, and `Row::disabled()` is already there when one does. Recorded so the omission is a decision. |

**Removed**: `active(bool)`. Nothing supplies it because nothing has to — see §2.2. This is a
breaking change to the builder, and the two call sites that pass it (`showcase/sections/controls.rs`'s
`form_field` entry, and nothing else) are updated with it.

### 2.2 Widget-tree state: `SelectState`

| Field | Type | Notes |
|---|---|---|
| `open` | `bool` | whether the list is showing |
| `highlight` | `Option<usize>` | where the keyboard is |

**Why the component owns these** (`logical_state_ownership.rs`). The test's rule is: *would this value
still mean something with the screen switched off?* Neither would. Openness is not persisted, not
restorable, and means nothing to a headless reader; a keyboard highlight is the same. Both are the
category that test assigns to components, and both are already component-owned today — `pick_list`
holds them privately, which is exactly why `Select::active` could never be answered and why accepted
fidelity gap #3 exists.

**The asymmetry with the search picker is deliberate and is the interesting part of this design.** The
search picker's openness is caller-owned because its list content is a function of the query, which is
application state — `WorktreeForm.branch_query` is persisted in the form and drives what the rows even
are. A select has no such coupling: its options are fixed by the call site and its openness is
coupled to nothing. Forcing symmetry would reintroduce the gap it closes.

| | Openness held by | Because |
|---|---|---|
| Search picker | the screen (`WorktreeForm.branch_list_open`, `Showcase.typeahead_open`) | the list is derived from caller-held query state |
| Select | the widget | nothing outside it is coupled to whether it is open |

### 2.3 Rows

The select builds `material::typeahead::Row` values — `{ label, spans, enabled }` — one per option,
with `spans` empty and `enabled` true. No new type. `EmphasisedLabel` with no spans is a plain label,
so the same row renderer serves both pickers unmodified (research R6).

---

## 3. The shared presentation — `micold_client::ui::material::picker` (extracted from `material::typeahead`)

Pure functions over already-decided inputs. No state.

| Function | Takes | Returns |
|---|---|---|
| `row_element` | `Row`, highlighted, selected, press message, `Roles` | one list row — leading marker slot, label, state layer, ripple when pressable |
| `menu_element` | rows, highlight, selection, empty message, pick handler, `Roles` | the panel: `menu_panel` surface, capped at `MAX_ROWS_BEFORE_SCROLL`, scrolling beyond |
| `animated_menu` | the panel, `open`, `Roles` | **new.** the panel wrapped in its transition (§4) |

All three move out of `material/typeahead.rs` unchanged in behaviour. `ROW_ROLE`, `GAP` and
`MAX_ROWS_BEFORE_SCROLL` move with them, so both pickers read one definition of each.

---

## 4. The transition

Not a type — a composition of wrappers that already exist, recorded here because the numbers are the
contract.

| Property | Value | Source |
|---|---|---|
| Enter duration | `duration::SHORT_3` (150 ms) | §6.3, "menu fade in" |
| Enter curve | `STANDARD_DECELERATE` | §6.3, same row — and `Motion`'s default, so it is not restated |
| Exit duration | `duration::SHORT_2` (100 ms) | §6.3, "menu fade out" |
| Exit curve | `STANDARD_ACCELERATE` | §6.3, same row — also the default |
| Scale floor | `MIN_SCALE` (0.96) | `material/animation.rs`, existing |
| Reflow | none | `scale` transforms drawing only; `fade` forwards layout |

No token is added. No entry joins §6.3's table.

---

## 5. What is *not* state

Worth stating, because a reader may expect these and their absence is the design:

- **No message for opening or closing the select.** The application gained
  `AddWorktreeBranchFocused` / `Dismissed` for the search picker because that picker's openness is
  caller-held. The select's is not, so `app.rs` gains nothing and `Message` grows by zero variants.
  The one arm that exists today (`AddWorktreeTypeSelected`) is unchanged.
- **No persisted anything.** Principle IV is untouched: no new file, directory, key or network call.
- **No session or worktree state.** Principles II and III are untouched.
