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
| 2 | **Overlay registry** — floating surfaces register themselves instead of being enumerated in a match | pending |
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
| Overlays | _still `app.rs`_ — Tier 2's registry is what gives this one a module |

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

Name one module from the table. Eight of the nine features the spec names can be answered that way
today; **overlays cannot**, and that is the honest state of Tier 1 rather than an oversight —
`Overlay` and `ClosingOverlay` are still enumerated in `app.rs`, and Tier 2 replaces that
enumeration with a registry.

If a feature needs two modules, that is the signal something is misfiled — with one current
exception, recorded rather than hidden: the Settings form's validation still lives in `main.rs`'s
`Message::SettingsSaved` arm, because it is reducer code returning a `Task`. It joins
`features/settings.rs` in Tier 3.

### What is still in `app.rs`

`State`, `Message`, `Overlay`, `ClosingOverlay` and `on_escape`. Tier 1 moved the feature types out;
the state root and the message vocabulary are Tier 3's to split. Because the transitional
re-exports are gone, a `crate::app::` import is now an honest measure of how much monolith remains.

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

_(Tier 2 — pending: fill when the overlay registry lands, per task T038.)_

## Adding a capability

_(Shell split — pending: fill when capabilities are assembled at boot, per task T057.)_

## Reading and writing across features

_(Tier 3 — pending: fill when outcomes land, per task T068. Covers why guard tests hold this line
rather than the type system.)_
