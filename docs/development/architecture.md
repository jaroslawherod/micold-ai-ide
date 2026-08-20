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
| — | **Shell split** — `main.rs` divided by external system, with capabilities assembled at boot | landed |

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
| Window size, pointer, which field holds the keyboard | `features/window.rs` |
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

If a feature needs two modules, that is the signal something is misfiled — with one exception,
recorded rather than hidden: the Settings form's validation lives in `shell/persist.rs::on_settings_saved`
rather than in `features/settings.rs`, because it parses a draft *and* returns a `Task` that writes
the file, and a feature module may do neither. **Tier 3 was named as where this would be fixed and
it was not**, which is worth saying plainly rather than leaving the promise standing: splitting the
parse from the write is a change to the settings feature, not to this architecture, and it wants its
own task. `features_settings.rs` says the same thing from the test side.

### What is still in `app.rs`

Two shared vocabularies and a routing table: `State`, `Message`, `State::update`, and the outcome
plumbing (`interpret`, `drain`, `on_escape`). `Overlay` and `ClosingOverlay` were here through
Tier 1 and are gone as of Tier 2; every reducer arm is gone as of Tier 3, and
`root_is_routing_only.rs` pins the number of arms that still *decide* anything at an exact **0**.

The file is 1,334 lines, of which the `Message` enum is 408 and the `State` struct 188 — so its
length measures the size of the application's vocabulary, not the root's logic. That is why it is
not split: FR-005 asks whether a file holds more than one feature, and this one holds none.

Some feature modules still carry `impl State` blocks. That is not a boundary violation: `State` is
one struct, and Rust resolves inherent methods on the type rather than the module, so moving them
would change no call site. What it does mean is that those features cannot be tested without
building a `State`, and their isolation tests say so rather than asserting something weaker to look
cleaner.

### Visibility widening is a signal, not a cost of doing business

Four helpers went from private to `pub(crate)` to cross a module boundary: `rematch_branches` and
`reset_branch_search` (worktree form), `worktree_tags` (worktree, read by the sidebar), and
`session_mut` (session, called by seven reducer arms). A helper that has to widen is telling you the
boundary does not fall where the code assumes it does.

**Tier 3 was expected to answer most of them and answered none** — all four are still `pub(crate)`,
and the reason is the same in each case: the reaching caller turned out to be a legitimate read
rather than a misplaced arm. `worktree_tags` is the clearest, and it is the single entry in
`ALLOWED_CROSS_FEATURE_NAMES` in `tests/feature_registration_cost.rs`: the sidebar renders a
worktree row's tags and does not get to decide what they are. A widening that survives the
restructuring it was blamed on is a widening that was never the restructuring's fault, and the
signal is still worth having — it just says "read across a boundary" here, not "misfiled".

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
crate::features::session::SessionContextMenu,                                // a popover
crate::features::project::RenameProjectDialog => crate::ui::rename::dialog,  // a dialog
crate::features::help::HelpMenu {                                            // ...that displaces
    displaces:
        crate::features::project::ProjectSwitcher,
        crate::features::sidebar::SidebarFilterPanel,
        crate::features::project::ProjectContextMenu,
},
```

A type name, and two optional clauses: for a dialog the view that draws it, and for a surface that
closes others when it opens, what it closes. This is the only list, and a macro rather than a plain
array so the line can be a type name and nothing else — no closure to get subtly wrong, no place to
tuck in a per-surface special case.

**Both clauses are here rather than on the surface, and for the same reason twice.** A view cannot
be named in a feature module because FR-006 forbids one naming the rendering framework. Displacement
cannot be declared there either: it is a fact about the *relation between* two surfaces, so it
belongs to neither of them, and `tests/surface_registration_cost.rs` holds that a surface may be
named only in its own module and here — a feature module saying what it displaces breaks that
guarantee once per surface it names.

There is no rule to derive displacement from, which is why it is declared. The three panel popovers
are mutually exclusive *and* each closes the project row menu; the row menu closes two of the three
and the worktree menu but deliberately **not** the switcher it was right-clicked in; the worktree
menu closes only the row menu; the session and terminal menus close nothing. `tests/popover_displacement.rs`
states all forty-two ordered pairs.

### 4. Open it

A popover: set its field. A dialog: `state.clear_for_dialog()` **first**, then set up the state the
dialog draws from. The order matters — `clear_for_dialog` closes whatever is already floating,
including any open dialog, so running it afterwards closes the one you just prepared.

That call is also where the "one dialog at a time" invariant lives. It was a type guarantee until
Tier 2 — the `Overlay` enum was one slot — and is now a mechanism, held by `one_dialog_at_a_time`
and `the_reducer_opens_a_dialog_through_that_mechanism` in `tests/overlay_registry.rs`.

### What you do *not* do

No match arm to extend, anywhere. Escape, scrim clicks, scroll-beneath dismissal, stacking order,
"opening a dialog closes the popovers", which popovers a new popover displaces, and the
exit-animation snapshot are all rules over the registry. Six central matches used to have to hear
about a new surface; there are none.

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

A **capability** is a narrow trait in `micold-core` naming one thing the application needs from
outside the process: run a git command, read the settings file, list a directory, ask the desktop
whether it prefers dark. The core declares what it needs; the shell supplies it. Everything between
those two — every feature module, every reducer arm, every view — sees only the trait.

Adding one costs **a trait and a fake in the core, one line in the port list, one line in
`Capabilities::real`, and an accessor.** The steps below are the whole of it.

### 1. Declare the trait in the core, beside what it is about

`micold-core/src/git.rs` holds `Git`; `env_include.rs` holds `EnvIncludeResolver`; `os_theme.rs`
holds `OsThemeProbe`. Not a `ports/` directory — a capability lives with the domain it serves, for
the same reason a feature's types live with the functions over them.

**Narrow, and the test is stated rather than judged.**
`specs/021-mvu-slice-architecture/contracts/service-capabilities.md`: *if a test must implement a
method it does not exercise merely to satisfy the trait, the capability is too wide and must be
split*. That is FR-016, and it is checked —
`no_test_is_forced_to_supply_an_operation_it_does_not_use` flags any port method in a test whose
parameters are all `_`-prefixed and whose body never mentions `self`, because such a method can
only return a constant.

It fired six times when it was written, and the answers went two different ways. `FolderScanner`
genuinely was too wide: every consumer that took it as a trait asked only `is_git_repo` and
`is_available`, while the one caller of `list_subdirs` reached for `StdFolderScanner` concretely —
so listing became `FolderBrowser`, a second capability the same type implements. `ProjectStore` was
**not** too wide; its only trait consumer exercises all three operations, and the stubs were
hand-rolled fakes written because no shared fake existed. Splitting it would have made the codebase
worse to score a check green. **A width failure is a question, not a verdict** — but the check is
never the thing that gets relaxed.

### 2. Write the fake next to it

```rust
/// A resolver that spawns nothing and remembers what it was asked (FR-019).
pub struct FakeEnvIncludeResolver { inner: RefCell<FakeResolverState> }
```

An ordinary `pub` item in the core, beside the capability. **Not behind a `cfg`, not in a separate
crate** — a fake any crate's tests can reach without configuration is worth more than the dead code
it costs, and `FakeGit` set that precedent long before this feature.

**Record what it was asked, not only what it answered.** `FakeEnvIncludeResolver::calls()` returns
every `(path, cwd, timeout)` in order, and that is what lets a test assert the *absence* of a call:
the env-include short-circuit's claim is not that the outcome is `Disabled`, it is that no
subprocess was spawned at all. An outcome can be right for the wrong reason. Where answers change
over time, script them — `FakeOsThemeProbe` consumes a list and repeats the last entry, so a test
says how many *distinct* answers it cares about rather than how many times something polls.

**The `Fake` prefix is load-bearing.** It is how the guards tell a double from a real
implementation, so a real one named `FakeSomething` would be waved through every check in
`no_concrete_implementations.rs`. `the_only_excluded_implementations_are_fakes` holds a list of the
nine that exist; add yours in the commit it appears in.

### 3. Add the port to `tests/inventory/mod.rs::PORTS`

```rust
pub const PORTS: &[&str] = &["Git", "ProjectStore", …, "EnvIncludeResolver", "OsThemeProbe"];
```

One list, read by both capability guards, because two scanners that happen to agree today is exactly
how a check keeps passing while its idea of the subject quietly diverges. Every gate that needs to
know what a capability is shares this definition rather than writing a second one — the same reason
`tests/inventory/` exists at all. Add the name **the moment the trait exists**: a capability the
guards do not know about is one FR-017 and FR-019 are not holding, and the omission is silent —
every check still passes, having looked at nothing.

### 4. Choose the real implementation in `Capabilities::real`, once

```rust
// crates/micold-client/src/shell/capabilities.rs
pub fn real() -> Self {
    Self {
        git: Arc::new(GitCli::new()),
        env_include: Arc::new(SubprocessResolver),
        …
    }
}

/// Sourcing the environment-include script.
pub fn env_include(&self) -> &dyn EnvIncludeResolver { &*self.env_include }
```

**This function is the single assembly point** (FR-018), and "once" is literal:
`each_implementation_is_chosen_in_exactly_one_place` counts occurrences of the type name across the
shell and fails at anything but exactly one. That is why `StdFolderScanner` — which implements both
folder capabilities — is built into a local and shared rather than named twice.

Return `&dyn Trait` from the accessor. Return an `Arc` only when a consumer genuinely cannot borrow
the application: `browser()` does, because the folder listing runs inside `Task::perform`'s
`async move`, which is `'static`. Wrap in `Option` only when the real implementation can fail to
*locate itself* — both stores resolve from a per-user data directory that a headless container may
not have. That is not an error the application reports; it runs without persistence, and the value
of the `Option` is that the question is asked once instead of at every call site.

Capabilities are supplied by dynamic dispatch on purpose (FR-019b). Every call here is already
gated behind disk, a subprocess or an OS query, none is reachable from the rendering path, and
threading generic parameters through to preserve static dispatch would make FR-018's single
assembly point harder to express for a cost that does not exist.

### 5. Use it from the shell module for its external system

| Module | The system it addresses |
|---|---|
| `shell/capabilities.rs` | none — it is the list of which implementation to use |
| `shell/startup.rs` | boot: the window, the assembly, the first frame |
| `shell/persist.rs` | the on-disk catalog and settings file |
| `shell/daemon_sync.rs` | a running session daemon, over the protocol |
| `shell/service_control.rs` | the session service as an OS process — pids, `loginctl` |
| `shell/subscriptions.rs` | the iced runtime and the OS events it carries |
| `shell/workspace.rs` | the user's filesystem and their git working copies |
| `shell/env_include.rs` | a subprocess running the user's own script |
| `shell/os_theme.rs` | the desktop's light/dark preference |
| `shell/clipboard.rs` | the system clipboard |

**One module per external system, and never per feature** (FR-019a). The question the split answers
is "what can a change to this one outside thing reach", so the module boundary follows the system,
not the caller: `default_resolution_cwd` is in `env_include.rs` because it exists to answer *which
directory to source in*, though its five call sites span boot and a settings save.

That rule cuts both ways. `daemon_sync` and `service_control` are two modules for one service,
because a daemon speaking the protocol and a daemon that must be killed by pid are different
systems — the second is used precisely *when* the first is unusable. The OS theme is split the same
way: `subscriptions` owns the clock that decides when to ask, `os_theme` owns the asking. And two
systems can share one module when they are one conversation — `workspace.rs` holds `FolderBrowser`
and `Git` together because the picker exists to find a repository and the same arm asks git what
worktrees it holds, so splitting it would put two halves of one decision in two files.

If two capabilities are one conversation, one module. If one capability is reached two ways, two.

### What you do *not* do

**Name a real implementation outside `shell/`.** The guard
`no_code_outside_the_shell_names_a_real_implementation` reads every client source and fails on the
mention — not the construction, the *mention*, because a file that names one has already decided
which is used. Comments are exempt.

**Add a `from_parts` constructor so a test can inject fakes.** One was written and deleted for want
of a caller. A seam nobody drives is not evidence that anything is substitutable; what the
capabilities actually bought is demonstrated where it is real, in `shell/persist.rs`'s
`boot_drops_a_session_the_provider_has_no_conversation_for`, a pruning rule that could not be tested
at all while it reached `ClaudeProvider` and the user's home directory directly. Add the constructor
when a caller needs it.

**Reach a capability from a feature module.** They cannot name a real implementation and must not
name the rendering framework either; a feature that needs something from outside says so in its
return value.

### The three ports that are not in `Capabilities`, and why each is not an oversight

`OsThemeProbe` is chosen at its call site. Its only consumer is `os_theme_poll`'s
`Subscription::map` closure, and iced panics on boot if a subscription's mapping closure captures
anything — a capturing closure has no stable identity, so the runtime restarts the underlying timer
every frame. A closure that cannot capture cannot be handed a capability. It is a real exception to
FR-018, recorded at `shell/capabilities.rs` rather than enforced anywhere, because the guard cannot
see it either way: that derivation reads `micold-core`, and `SystemThemeProbe` is defined in the
client — `dark-light` is a client dependency and the core deliberately has none on it.

`TerminalBackend` and `TerminalHandle` have only fakes in the workspace, so there is no real
implementation for an assembly point to choose. They predate this feature and FR-015 lists them as
already satisfactory.

### The one I/O concern that is not a capability at all

**The clipboard.** All three of its call sites return a deferred `Task` rather than a value, so a
synchronous port cannot wrap them without blocking — which is what FR-015a is for: where the GUI
framework makes a synchronous capability impossible, the concern becomes an explicit **effect
request** in the outcome vocabulary, and the shell interprets it.

```rust
// crates/micold-client/src/shell/clipboard.rs
pub fn interpret(outcome: Outcome) -> Task<Message> {
    match outcome {
        Outcome::ClipboardWrite(text) => iced::clipboard::write(text),
    }
}
```

The obligations do not weaken. FR-017 still applies (no feature reaches `iced::clipboard`), and so
does FR-019/SC-005 — the request must be assertable with zero real I/O, which is what
`tests/clipboard_request.rs` checks. `interpret` is the whole translation, one arm per variant and
no branch, and `the_shells_translation_decides_nothing` reads its body to keep it that way: a body
that grew an `if` would still compile and still pass every behaviour test.

The paste does **not** convert. Reads arrive back as an ordinary message exactly as before, and no
feature requests one — `clipboard::read` is a shell call, not an effect a reducer asks for.

### The guards, and what each would catch

| Guard | Catches |
|---|---|
| `no_concrete_implementations.rs` | a real implementation named outside `shell/`, or chosen in more or fewer than exactly one place |
| `service_capability_fakes.rs` | a capability with no fake, a fake no test constructs, a capability wider than its consumers |
| `clipboard_request.rs` | a feature reaching the framework's clipboard, and a shell translation that decides anything |
| `features_are_render_free.rs` | a feature module naming the rendering framework |

Both capability guards derive their subject rather than listing it — every `impl <Port> for <Type>`
in the core, minus the fakes. The argument for that is one line long: a hardcoded list of four was
already missing `ClaudeProvider` on the day it was written. Each file carries a vacuity test
holding the derivation to finding what it must, so a scan blinded by a reformatted `impl` header
fails loudly instead of passing everything.

## Reading and writing across features

A feature may **read** any other feature's data. It may not **write** it. That asymmetry is the
whole of the rule, and it is not arbitrary: a read cannot leave the app in a state its owner did not
choose, and a write can.

`State` is one struct, so nothing in the type system enforces this — every field is reachable from
every `&mut State`. What holds the line is `tests/feature_write_isolation.rs`, which scans the
feature modules, resolves what each operation writes (following calls to a fixed point), and fails
on any write whose field belongs to another feature.

### Why a guard test rather than the type system

Splitting `State` into per-feature structs would make the rule a compile error, and that is the
eventual destination. It is not where this codebase is, and a guard test buys the rule *now* at a
cost the split does not: it can carry an allowlist. `ALLOWED` names each pre-existing violation with
the task that will convert it, so the rule is enforced for new code while the backlog burns down
rather than blocking on a rewrite. `the_allowlist_names_only_live_violations` fails when an entry
stops being a violation, so the list cannot outlive what it permitted.

The list is empty now — 43 to zero over feature 021's Phase 6 — but the mechanism is what mattered:
a split `State` would have had to land in one commit, and this landed in fifteen, each of which
built and passed. What the empty list buys going forward is that a new cross-feature write is a
failing test rather than a line in a backlog.

### What a feature returns instead: `Outcome`

A feature that needs another's data changed returns a value saying so, and the root applies it:

```rust
pub fn loaded(state: &mut State, worktrees: Vec<Worktree>) -> Vec<Outcome> {
    state.set_worktrees(worktrees)          // its own data, written directly
}                                           // -> vec![Outcome::WorktreesReplaced(names)]
```

The root is the only interpreter (`app::interpret`), and `app::drain` applies outcomes to a fixed
point with a bound, so an outcome may raise further outcomes without the root knowing which
features are involved.

Mark these returns `#[must_use]`. A dropped `Vec<Outcome>` is a silent behaviour loss — the code
compiles and does less — and the attribute turns it into a compile error at every production call
site. It caught all thirteen when `switch_active` was converted.

### Three traps, each of which cost real time here

**A row count is not a violation count when the writer is code the guard cannot name.** The scan
attributes a write to the *feature operation* it appears in. When several features call one root
helper, the helper's write is reported against each caller, and the burn-down looks like N
violations when it is one misplaced function. Two of feature 021's largest clusters — eight rows and
five — were retired by *moving a function into the feature that owned its data*, with no outcome
written at all. Before designing an `Outcome`, ask where the code belongs; the answer is often that
the guard was pointing at callers.

**Ownership can be wrong, and a plausible name is how it stays wrong.** `worktree_error` reads like
the worktree feature's, and is the add-worktree modal's — `crate::ui::worktree_form` is its only
render site. Correcting `OWNERS` retired three rows that had been queued for conversion. When a
field's writers and its one reader disagree about who owns it, believe the reader.

**A test that calls a feature function directly stops exercising what moved.** Converting a write to
an outcome moves behaviour *out of* the function, so any test that calls it rather than going
through `State::update` silently tests less — and still compiles, because `#[must_use]` is satisfied
by an `assert!` that consumes the value and never applies it. Four files hit this in one task. Tests
that assert a moved consequence need a helper that drains the way the root does:

```rust
fn switch(st: &mut State, path: &str) -> bool {
    match st.switch_active(Path::new(path)) {
        Some(outcomes) => {
            drain(outcomes, |o| interpret(st, o));
            true
        }
        None => false,
    }
}
```

Tests that assert on *no* other feature's data need no such helper, and dropping the outcomes there
is correct rather than lazy — `features_session.rs` says so in its header and turned out to be
right.

### A core type's own operation is not a cross-feature write

`OWNERS` is keyed by *path*, not by field, because `Workspace` holds the project catalog, the
session lists and two worktree maps in one value — six members answering to three features. That
split is right for an ordinary write and wrong for `Workspace`'s own methods: `forget` clears
everything held against a project's path because that is its invariant, and reporting it as the
project feature writing session data would have three features each apply one clause of it, making
a half-applied forget expressible for the first time.

So a write a feature reaches **only** through a core method is exempt — the same principle that
always exempted a feature writing its own field: the code that performs the write owns what it
writes. The exemption is narrow in both directions. It does not cover a path the operation also
writes on a line of its own (the scan keeps two closures over the call graph to tell those apart),
and it is not silent: `CORE_MEDIATED` lists every one with the method that carries it, so reaching a
neighbour through some *other* core method costs a line and an argument rather than nothing.

### The guards, and what each would catch

| Guard | Catches |
|---|---|
| `no_feature_writes_another_features_data` | a feature writing a field it does not own, directly or through a call |
| `the_allowlist_names_only_live_violations` | an `ALLOWED` entry that no longer names a real write |
| `core_mediated_writes_are_inventoried` | a feature reaching a neighbour through an unlisted core method — the exemption's only silent path |
| `the_exemption_is_narrow` | a `CORE_MEDIATED` line that mediates nothing, or a write reported and exempted at once |
| `every_state_field_has_an_owner` | a new `State` field nobody claimed in `OWNERS` |
| `every_workspace_field_has_an_owner` | the same for `Workspace`'s members, which three features hold |
| `every_method_called_on_state_is_classified` | a method the scan cannot tell reads from writes |
| `the_scan_finds_the_operations_it_is_meant_to_read` | the scan going blind — a vacuity floor |

That last one exists because this guard has failed silently twice. Once it could not see free
functions at all (a reference it never peeled), reporting every feature clean while its own header
promised otherwise. Once it suppressed a violation whenever the operation also called a sibling
writing the same field — correct for an inherited write, wrong for one made on the spot as well,
and a suppressed row is indistinguishable from no row. **A green guard is evidence only in
proportion to what you have shown it can still fail on.** Probe it by reintroducing a write and
confirming it fires on the row you expect; for a single-assertion guard, distinctness lives in the
reported violation, not in the set of failing tests.
