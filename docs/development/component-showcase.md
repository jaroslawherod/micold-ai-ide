# The component showcase

Every component the shared library provides, on one page, in either colour scheme, with no setup.

```bash
mise run showcase
```

No daemon. No project. No git repository. No saved state. It opens, and the whole library is there.

## What it is for

Seeing what a component looks like used to mean launching the application, letting it spawn a session
daemon, opening a project, creating or selecting a worktree, and then finding a screen that happens to
use the component. If the component only appears in an error state, an empty state or a disconnected
banner, you first had to *produce that state*.

The showcase turns that into one command. Which matters most for the components you can otherwise
barely reach — the connection banner, the empty tree, the terminal pane — and for the checks that are
supposed to cover *every* interactive element. A row containing every interactive component in the
library is the only place "hover and pressed work everywhere" can honestly be confirmed in one pass.

It is a development tool. It is **never installed**: absent from the Debian asset list, absent from the
desktop entry, and `tests/packaging_excludes_showcase.rs` fails the build if either ever names it.

## What it is not

It is not a second implementation of anything. It composes components from `ui/material/` and
`ui/cdk/` exactly as a feature module does and supplies sample content; it holds no styling, no layout
rule and no interaction behaviour that belongs in the library. `tests/material_boundary.rs` scans
`src/showcase/` at the same zero budgets it holds the application's feature modules to, so a
hand-styled heading is a build failure rather than a review note.

That constraint is load-bearing. A gallery whose button was styled locally would be a gallery of
*near-misses*, and you would be comparing the showcase's button to the application's rather than
looking at one button.

## How it is put together

| File | What it holds |
|---|---|
| `src/showcase/catalogue.rs` | **The one list.** Every entry, every motion entry, every exemption. |
| `src/showcase/sections/*.rs` | The render functions the entries point at, grouped by kind. |
| `src/showcase/state.rs` | The render-free reducer. Every state transition, tested directly. |
| `src/showcase/samples.rs` | Fixed invented content, including the fabricated terminal grid. |
| `src/showcase/gallery.rs` | The view: the catalogue, traversed. |
| `src/showcase/main.rs` | The binary. `iced::application` and nothing else. |

The one thing worth understanding before you touch it: **each entry carries the function that renders
its own instances.**

```rust
pub render: for<'a> fn(&'a Showcase, Roles, usize) -> Element<'a, Message>,
```

The page *is* the catalogue traversed. So an entry cannot exist without something to show, and an
instance cannot appear without being declared. The obvious alternative — a `match` on component names
in the view — would have allowed exactly the gap the completeness check exists to close: an entry with
no arm renders nothing and still passes a name-only check.

## Adding a component to the gallery

1. Add an `Entry` to `COMPONENTS` in `catalogue.rs`, naming its module and type.
2. List its named variants, and any other posed state (`disabled`, `selected`, an empty state).
3. List what has to be exercised live, and set `interactive` to match — non-empty `live` if and only if
   `interactive`.
4. Write its `render` in the matching `sections/` file, from the real component and `samples`.
5. If it has no visible appearance of its own, add an `Exemption` with the reason instead.

You will not forget, because the build will not let you. Add a component to the library and skip this,
and `showcase_completeness` fails naming your component.

## What the checks are, and what each failure means

| Gate | Fails when |
|---|---|
| `showcase_completeness` | The library and the gallery disagree, in either direction (see below) |
| `showcase_captions` | An entry's `interactive` flag and its `live` list contradict each other |
| `showcase_isolation` | The showcase names the project store, settings, daemon, git, or the host theme |
| `showcase_determinism` | The gallery reads the clock, a random source, the environment or a file |
| `showcase_glue` | `gallery.rs` or `main.rs` grew a branch on showcase state |
| `showcase_state` | The reducer misbehaves |
| `packaging_excludes_showcase` | The Debian manifest or the desktop entry names the showcase |
| `material_boundary` | The showcase styled a widget, reached the style layer, or named a text size |
| `idle_requests_no_frames` | Anything outside `cdk/motion.rs` asks the runtime for a frame |

`showcase_completeness` is the one you will meet. Its nine rules:

- **C1** a library component with no entry and no exemption → add one.
- **C2** an entry naming a component that no longer exists → remove it.
- **C3** a library enum variant with no instance → pose it. Attribution is **library-wide**: a variant
  may be posed by an entry from any module. (`cdk/overlay.rs` declares `Anchor` and both of its
  components are exempted, so a module-scoped rule would be unsatisfiable there. The anchors are posed
  where they are actually visible — `Modal` is `Center`, `MenuOverlay` is `TopEnd`, `ContextMenu` is
  `Point`.)
- **C4** an entry naming a variant no enum declares → it is probably not a *library* variant. `Info`
  and `Error` are `app::NoticeLevel`; `Bottom` and `Left` are iced's `tooltip::Position`. Both belong
  in `posed`, not `variants`.
- **C5/C6** an animation with no motion entry, or a motion entry naming one that is gone.
- **C7** an exemption whose component vanished, or one with no reason.
- **C8** something both posed and exempted, or listed twice.
- **C9** an animation posed among the static components, or a static component in the motion section.

Plus four vacuity guards (**V1–V4**): the inventory must find the library, both layers must be present,
`animation.rs` must be where the motion category is read from, and the catalogue must not be empty. A
check that finds nothing has to fail, not pass — otherwise a relocation reads as a clean bill of health.

## One definition of "a component"

`tests/inventory/mod.rs` is the single scanner. `material_builder_api.rs` uses it to hold the library to
Principle VIII's builder shape; `showcase_completeness.rs` uses it to hold the gallery complete. Sharing
the code is deliberate — two scanners that happen to agree today drift silently, and the completeness
check would keep passing while its idea of the library diverged.

A **component** is a `pub struct` under `src/ui/material/` or `src/ui/cdk/` that either converts into
something (`From<Self> for …`) or is a documented terminal type. `MenuItem`, `ProjectRow` and `TreeItem`
are **records** — public fields, no conversion — so they are not components and need no entry. They are
visible on the page anyway, inside the menus, switcher and tree that consume them.

Two properties of the library make the keying matter, and both have their own tests:

- `material/surface.rs::Surface` and `cdk/overlay.rs::Surface` are **different components**. Keyed by
  name alone, each would satisfy the other's requirement.
- `material/animation.rs` declares `Fade` twice — the wrapper and its private widget-tree tag — so
  duplicates within a module collapse to one key.

## Motion, and why it is a separate category

The component definition recognises things that convert into an element, so it cannot see a helper
offered as a plain function. `fade`, `expand`, `scale` and `scrim` are free functions, and would have
fallen outside the check entirely — a whole category missing in silence, which is the failure the
two-way rule exists to prevent. So the motion category is enumerated deliberately: the free `pub fn`s
in `material/animation.rs`, held to the same two-way rule.

Each animation has a **Replay** control, because an animation you can only see by catching it once is
not reviewable, and a **Reverse** control, because Material exits are quicker than entrances and an
entry that could only be entered would hide half the specification.

There is no clock anywhere in the showcase. A replay is a *changed identity*: bumping a counter hands
the wrapper a different `restart_on(key)`, and the wrapper plays its own transition and asks for its own
frames. That is what keeps the page inert at rest.

## Recorded limits

Honesty about coverage is the point of the whole feature, so these are written down rather than left to
be discovered:

- **Three element-producing free functions are covered by no check** — `material::menu_panel`,
  `glyph::icon`, `glyph::icon_colored`. They are neither a `pub struct` nor animation helpers. Not
  invisible in practice (`Glyph` is a component; the popover panels are rendered by the overlay entries
  that use `menu_panel`), but no gate holds them. If that matters, it is a third category, added
  deliberately.
- **Density is dormant.** `Entry::density` is empty on every entry because nothing honours a density
  step yet — the scale is feature 018's, and this landed first. When 018 introduces the axis it adds a
  row per honouring component, and the rule that holds them belongs in that change.
- **Overlay layer ordering is not on the page.** `Showcase::open` is an `Option`, so two floating
  surfaces can never be open at once — which is what makes the deadlock edge case unrepresentable, and
  also means "a dialog is above a menu because it is a dialog" is not visible here. It is covered by
  `tests/overlay_stacking.rs`; do not conclude from the page that it is untested.
- **`TerminalPane` is not generic over its message type.** It emits `app::Message` directly, so only the
  application can compose it; the gallery maps its messages to a no-op. A message-type parameter would
  be a change to the library, and feature 020 was forbidden from touching the application's behaviour.
  Worth fixing whenever something else needs the pane outside the application.
- **The showcase's own appearance carries no cross-platform claim.** It must *compile* on Linux, macOS
  and Windows, which the workspace build enforces; nobody promises it looks identical on all three,
  because no user installs it. Parity of the components it displays belongs to the features that own
  them.

## What the gallery found

The first walkthrough over the page produced five findings in one pass — three in the library, two in
the gallery itself. That ratio is roughly what you should expect from it, and it is the argument for
landing this before feature 018 rather than after.

### In the library

**`Expand` overlaps the content it should be revealing.** It is the only animation wrapper that
animates its *layout* rather than its drawing: it reports a shrunken height to its parent while the
child node keeps its full height, relying on a draw-time clip to reveal it top-down. The clip does not
take effect, so everything below moves up into the vacated space and the full-height child paints over
it. The visible effect is that the reveal reads as "nothing happened", followed by overlapping text.

This is **not confined to the showcase**. `Accordion` is `expand(...)`, so the sidebar's tag-filter
panel behaves the same way in the application — it hides better only because its reveal is 90ms rather
than the gallery's 600ms, and because what sits below it is a worktree list rather than a labelled row.
Fixing it changes the application's motion, so it was out of scope for feature 020 (FR-019). It belongs
to 018's motion work or to a bugfix of its own.

**`Accordion` is only half of what its name implies, and is invisible outside the sidebar.** It has no
header, no twisty and nothing to press: the thing that opens it is a separate component
(`FilterTrigger`) that the call site pairs it with. It also renders through
`menu_panel(..., bordered: false)`, so an open panel is unoutlined — deliberate in the sidebar, where
the sidebar's own edge separates it, and illegible anywhere else. Posed alone it reads as a stray
paragraph. Both are naming and visual-system questions for 018.

**`cdk::overlay::Overlay`'s empty-set early return is a latent trap.** It returns its base untouched
when no surface is pushed and wraps it in a `stack` when one is, so a surface set that empties and
refills inserts and removes a level *above* the page — and iced reallocates the state of everything
beneath it, including scroll offsets. The application is safe only by accident: `ui::view` pushes its
overflow menu unconditionally, so its set is never empty. Any future call site that pushes
conditionally will hit this, and the symptom (a list silently jumping to the top when a dialog opens)
does not look like an overlay bug. Worth either a fix or a prominent comment.

### In the gallery, and fixed

Two, both mine, both found by the same walkthrough:

- **The page jumped to the top whenever a surface opened** — the overlay trap above, met by pushing
  conditionally. The gallery now pushes its menu panel unconditionally, as the application does.
- **`Accordion` was posed without the trigger that drives it**, so its reveal could not be exercised at
  all. It now shows closed, open, and paired with a `FilterTrigger`.

And two more from building it, before anyone looked at the page:

- **`Ellipsized` could not be constructed by a call site outside the library.** Its only constructor
  took a raw `f32`, and a feature module may not name a text size — so the one component whose entire
  job is text was the one component a feature module could not build at an ordinary role. It gained
  `Ellipsized::at_role`.
- **Four "variants" in the first draft of the catalogue were not library variants at all.** The
  completeness check refused them. Recorded above under C4.
