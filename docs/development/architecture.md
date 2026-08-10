# Client architecture

How the iced client is organised, and where to put things.

> **Status**: written incrementally as feature 021 lands. Sections marked _(Tier N — pending)_
> describe work not yet merged; the rest describes the codebase as it stands.

## Tier structure

The client is being moved onto The Elm Architecture in four tiers, each landing on its own. The
order is not arbitrary: every tier needs the one before it.

| Tier | What it establishes | State |
|---|---|---|
| 1 | **Feature modules** — one module per feature, holding its types together with the functions over them | landed |
| 2 | **Overlay registry** — floating surfaces register themselves instead of being enumerated in a match | landed |
| 3 | **Reducer modules + outcomes** — per-feature reducers, and cross-feature effects expressed as returned outcomes rather than direct writes | pending |
| — | **Shell split** — `main.rs` divided along the same seams, with capabilities assembled at boot | pending |

Tier 1 is the foundation: without per-feature boundaries there is nothing for the overlay registry
to register into, and nothing for a per-feature reducer to be a reducer *of*.

## Where a feature lives

**One module per feature, under `crates/micold-client/src/features/`.** A feature's types live
there together with the functions over them. There is no parallel `state.rs` / `update.rs` /
`view.rs` split — a type and the operations on it stay in one file.

| Feature | Module |
|---|---|
| Daemon connection | `features/connection.rs` |
| Notifications | `features/notifications.rs` |
| Project switching, its context menu, rename | `features/project.rs` |
| Sessions, foreground, terminal selection | `features/session.rs` |
| Settings form | `features/settings.rs` |
| Sidebar rows, tag filters, tree projections | `features/sidebar.rs` |
| Worktree visibility, naming, tags, rename | `features/worktree.rs` |
| Worktree-creation form | `features/worktree_form.rs` |
| Help menu and the About dialog | `features/help.rs` |
| Overlays | `overlay/mod.rs` + `overlay/registry.rs` — the surface type, and the one place surfaces are named |

Views are **not** in these modules. They live in `crate::ui`, beside the feature they draw rather
than inside it, because they need the rendering framework and feature modules must not.

### Two rules, and why they are checked rather than trusted

**Feature modules name no rendering framework in code.** `tests/features_are_render_free.rs` reads
the source and fails on the mention; comments are exempt. This is what lets application state live
in the client crate rather than the render-free core — the modules could sit in the core, and the
only reason they do not is that being in the client is more convenient for code that the shell
drives. That argument holds exactly as long as the property does, so it is a test and not a
convention.

**Group by feature, not by name or by neighbourhood.** Three helpers called `worktree_tree`,
`filtered_worktree_tree` and `available_tag_filters` live in `features/sidebar.rs`, not
`features/worktree.rs`: they return `WorktreeNode` and `TagFilter`, read `sidebar_filters`, and
build sidebar rows. `SelectKind` lives in `features/session.rs` rather than `features/project.rs`
despite having sat between two project types in the old file. Both placements were decided by what
the code is *about*, and both went the other way in the original task list — grouping by name or by
line range is the specific failure this structure exists to prevent.

The worktree-creation form is its own module rather than part of `features/worktree.rs`. It is the
one feature whose intermediate state nothing else reads, which is also why it was extracted first.

### Answering "where does this feature live?"

Name one module from the table. All of them can be answered that way now. Overlays were the
holdout through Tier 1 — `Overlay` and `ClosingOverlay` were enumerated in `app.rs`, which is not a
module anything lives *in* — and Tier 2 is what fixed it: each surface is described in the feature
module that owns it, and `overlay/` holds only the shared type and the registration list.

If a feature needs two modules, that is the signal something is misfiled — with one current
exception, recorded rather than hidden: the Settings form's validation still lives in `main.rs`'s
`Message::SettingsSaved` arm, because it is reducer code returning a `Task`. It joins
`features/settings.rs` in Tier 3.

### What is still in `app.rs`

`State`, `Message` and `on_escape`. `Overlay` and `ClosingOverlay` were here through Tier 1 and are
gone as of Tier 2. Tier 1 moved the feature types out; the state root and the message vocabulary are
Tier 3's to split. Because the transitional re-exports are gone, a `crate::app::` import is now an
honest measure of how much monolith remains.

Some feature modules still carry `impl State` blocks. That is expected in Tier 1 and not a
boundary violation: `State` is one struct until Tier 3 splits it, and Rust resolves inherent methods
on the type rather than the module, so moving them changed no call site. What it does mean is that
those features cannot yet be tested without building a `State`, and their isolation tests say so
rather than asserting something weaker to look cleaner.

### Visibility widening is a signal, not a cost of doing business

Three helpers went from private to `pub(crate)` to cross a module boundary: `rematch_branches` and
`reset_branch_search` (worktree form), `worktree_tags` (worktree, read by the sidebar), and
`session_mut` (session, called by seven reducer arms). Each is noted at its definition with the task
that returns it to private. A helper that has to widen is telling you the boundary does not yet fall
where the code assumes it does — Tier 3 is where most of them are answered, because the callers
doing the reaching are reducer arms that have not moved yet.

## Adding a floating surface

A floating surface is anything the window stacks over its content: a dialog, a panel popover, a
context menu. Adding one costs **its own module, and one registration line** — that is what Tier 2
exists to make true, and the steps below are the whole of it. A dialog also needs a view, which
lives in `crate::ui` beside the feature, exactly as every other view in the client already does.

(The snackbar floats in a band of its own and is not registered. It has no state anyone opens and
nothing dismisses it but its own timer, so there is nothing for a registration to say; `ui::view`
pushes it directly from `state.notify`.)

### 1. Describe the surface where the feature lives

In the feature module that owns it, a marker type implementing two traits:

```rust
pub struct HelpMenu;

impl FloatingSurface for HelpMenu {
    fn id(&self) -> SurfaceId { SurfaceId::new("help_menu") }
    fn layer(&self) -> Layer { Layer::Popover }
    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Popover).cancelled_by(Message::HelpMenuToggled)
    }
}

impl Registered for HelpMenu {
    fn open_in(state: &State) -> Option<Self> { state.help_menu_open.then_some(HelpMenu) }
}
```

Four facts, and nothing else:

- **`id`** — a `&'static str` name, not an enum variant, because an enum is the central list Tier 2
  removed. It is never shown to the user; it keys the exit animation and names the surface when a
  guard fails.
- **`layer`** — which band it belongs to, from `micold_core::overlay::Layer`: `Popover`,
  `ContextMenu`, `Dialog`, `Snackbar`, bottom to top. The band decides stacking and priority.
  Registration order decides nothing, and `registration_order_does_not_decide_anything` proves it by
  running every state through a reversed list.
- **`dismissal`** — a chainable builder ending in the message that cancels the surface. It *decides*
  nothing: which triggers close which kind of surface is `micold_core::overlay::dismisses`, and
  `DismissalRules` forwards every question to it. What it adds is the part the core cannot know —
  this surface's cancel message. `.protecting_input()` marks a dialog non-dismissible, for one
  holding input an accidental close would destroy.
- **`open_in`** — how to tell, from the state, that this surface is open. A popover reads its own
  flag; a dialog reads the state it draws from, which since Tier 2 *is* what says it is open (there
  is no separate slot to keep in step).

### 2. If it is a dialog, write its view in `crate::ui`

```rust
// crates/micold-client/src/ui/rename.rs — declared `pub(crate) mod rename;`
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.rename_draft.as_ref().map(|draft| modal(draft, scheme, state.focused_field))
}
```

Every dialog wrapper has that exact signature — the registration line stores it as a function
pointer, so they have to. Take `env_include_outcome` whether or not you need it; only the Settings
form does.

The two halves live in different modules on purpose: a feature module may not name the rendering
framework (`tests/features_are_render_free.rs` reads the source and fails on the mention), and views
belong beside the feature in `crate::ui`. `None` means the surface is open but the live state it
draws is absent — nothing is drawn, rather than an empty dialog.

**Popovers register no view.** A panel popover's panel is pushed by `ui::view` whether or not it is
open, because the panel owns its own fade and has to outlive the flag that opened it; a context
menu is pushed only while open, since it is anchored at a cursor position that only exists then.
Either way the drawing comes from the feature's own field and not from the registry, and
`a_popover_is_not_drawn_from_the_registry` holds that line — a popover given a registered dialog
view would be drawn a second time, inside the modal band.

### 3. Add one line to the registry

In `overlay/registry.rs`, inside `register!`:

```rust
crate::features::help::HelpMenu,                                        // a popover
crate::features::project::RenameProjectDialog => crate::ui::rename::dialog,  // a dialog
```

A type name, and for a dialog the view that draws it. This is the only list, and a macro rather than
a plain array so the line can be a type name and nothing else — no closure to get subtly wrong, no
place to tuck in a per-surface special case.

### 4. Open it

A popover: set its field. A dialog: `state.clear_for_dialog()` **first**, then set up the state the
dialog draws from. The order matters — `clear_for_dialog` closes whatever is already floating,
including any open dialog, so running it afterwards closes the one you just prepared.

That call is also where the "one dialog at a time" invariant lives. It was a type guarantee until
Tier 2 — the `Overlay` enum was one slot — and is now a mechanism, held by `one_dialog_at_a_time`
and `the_reducer_opens_a_dialog_through_that_mechanism` in `tests/overlay_registry.rs`.

### What you do *not* do

No match arm to extend, anywhere. Escape, scrim clicks, scroll-beneath dismissal, stacking order,
"opening a dialog closes the popovers", and the exit-animation snapshot are all rules over the
registry. Six central matches used to have to hear about a new surface; there are none.

### The guards, and what each would catch

| Guard | Catches |
|---|---|
| `overlay_registration.rs` | a popover-shaped `State` field with no registration — the one that opens and cannot be closed, since it is drawn from its own field and only the registry closes it |
| `overlay_registry.rs` | dispatch: each surface's identity and cancellation, a dialog registered without a view or with the *wrong* view, two dialogs open at once |
| `overlay_builder_api.rs` | a surface configured by a public field or a `&mut self` setter instead of the builder (Principle VIII) |
| `overlay_dismissal_rules.rs` | dismissal decided locally rather than derived from the core rule |
| `features_are_render_free.rs` | a feature module naming the rendering framework |
| `one_overlay_implementation.rs` | a second floating-surface primitive |

An unregistered **dialog** is the one failure with no guard of its own, and the honest reason is
that it needs none: a dialog is drawn only *through* its registration, so an unregistered one is
simply not drawn — it fails the first time anyone opens it, rather than trapping the user behind a
surface with no exit.

## Adding a capability

_(Shell split — pending: fill when capabilities are assembled at boot, per task T057.)_

## Reading and writing across features

_(Tier 3 — pending: fill when outcomes land, per task T068. Covers why guard tests hold this line
rather than the type system.)_
