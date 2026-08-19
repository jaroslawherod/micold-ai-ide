# Cross-feature writes: the catalogue (T067)

**Feature**: 021 | **Satisfies**: FR-020, FR-021 | **Converts**: nothing — T067a does that

Every cross-feature write `crates/micold-client/tests/feature_write_isolation.rs` finds today,
with a proposed outcome for each. The guard's `ALLOWED` table is the machine-readable copy and the
one that fails a build; this file is the argument behind it.

**42 writes across 27 operations.** One has already been converted (T066:
`worktree::loaded` → `expanded`, now `Outcome::WorktreesReplaced`), which is why the guard starts
from 43 in T066's commit and 42 here.

## Read this by cause, not by row

The single most useful fact in the table below is that **most of these are not a feature reaching
into a neighbour at all**. They are a *root helper* that writes across features, attributed by the
guard to whichever feature called it. `clear_for_dialog` alone accounts for 8 of the 42.

That changes what T067a is. Converting 42 writes one at a time would mean 42 commits mostly
re-deriving the same decision. Converting **7 causes** is the real work, and each conversion
retires a group.

| # | Cause | Writes | Proposed outcome |
|---|---|---|---|
| A | `State::clear_for_dialog` — a dialog opening clears the focus slot and closes every popover | 8 | `DialogOpened` |
| B | `State::focus_terminal` — a terminal taking the keyboard clears the focused field | 5 | `TerminalFocused` |
| C | The popover mutual-exclusion rule (features 009, 015) | 12 | **not uniform — see below** |
| D | `Workspace::forget` — forgetting a project drops what three features hold against its path | 4 | `ProjectForgotten(PathBuf)` |
| E | `State::push_notification` | 2 | `NotificationRaised` — **exists** (T065) |
| F | The reveal: displaying a session opens the row holding it | 4 | `SessionRevealed(SessionId)` |
| G | worktree_form creates; worktree owns the list | 4 | `WorktreeCreated` / `WorktreeCreateFailed` |
| — | `session::switch_active` → `workspace.active` | 1 | see note below |
| — | `sidebar::toggle_location` → `reveal_suppressed_for` | 1 | see note below |
| — | `session::restore_after_activation` → `show_agent_worktrees` | 1 | see note below |

### A — `clear_for_dialog` (8 writes, `focused_field`)

`help::about_opened`, `project::forget_requested`, `project::rename_started`,
`session::remove_requested`, `settings::opened`, `worktree::delete_requested`,
`worktree::rename_started`, `worktree_form::opened`.

Opening a dialog clears the focus slot, because the widget tree that reported focus is being torn
down and will never report losing it (feature 006 BUG-003). **T063 is what made this answerable**:
while `focused_field` was `root`-owned the question "is writing state nobody owns a violation?" had
no answer. `features/window.rs` owns it now, so these are ordinary cross-feature writes and
`DialogOpened` is the outcome — applied by the root to the window feature, and to the overlay
registry for the popover-closing half.

### B — `focus_terminal` (5 writes, `focused_field`)

`session::mode_toggled`, `session::selected`, `session::shell_instance_close_requested`,
`session::shell_instance_selected`, `session::started`.

Putting a terminal in front of the user gives it the keyboard, which clears whatever field held it
(FR-011). Same slot as A, different trigger, so it is a separate outcome rather than a merge.

**T059 recorded an open question here that is still open**: is `focus_terminal` a session operation
sitting in the wrong file? If it moves into `features/session.rs`, group B may collapse into "the
session feature emits `TerminalFocused`" with no other change. T067a should settle that *before*
converting B, because the answer changes what the conversion looks like.

### C — popover mutual exclusion (12 writes)

`help::menu_toggled`, `project::menu_toggled`, `project::switcher_toggled`,
`sidebar::filter_menu_toggled`, `worktree::menu_toggled`.

**Correction (T067a-2 attempted this and stopped).** This section first claimed the rule was
uniform — "one `PopoverOpened(SurfaceId)` applied by the root to every *other* registered popover
retires all 12". **It is not uniform, and a frozen test proves it.** Reading the five toggles:

| Toggle | Closes |
|---|---|
| `help::menu_toggled` | switcher, sidebar filter, project menu |
| `project::switcher_toggled` | help, sidebar filter, project menu |
| `sidebar::filter_menu_toggled` | help, switcher, project menu |
| `project::menu_toggled` | help, sidebar filter, **worktree menu** — *not* the switcher |
| `worktree::menu_toggled` | **project menu only** |

The project context menu is drawn from a row *inside* the open switcher panel, so the switcher
stays open behind it — stated in `State::project_menu_open`'s own doc and pinned by
`tests/switcher_forget_menu.rs::right_click_opens_the_menu_and_the_switcher_stays_open_behind_it`.
A uniform "close every other popover" would fail that test, which is FR-027's freeze doing its job.

So this is **two rules, not one**: the three *toolbar* popovers are mutually exclusive with each
other; a *context menu* displaces the toolbar popovers and other context menus, with the
switcher-behind-its-own-menu exception.

**The shape is therefore a design decision, not a mechanical conversion.** The cleanest option is
to put the exclusion in the registry beside `DismissalRules`, where a surface declares what it
displaces — then `PopoverOpened(id)` asks the registry and the exception is data on a surface
rather than a special case in the root. That changes the registry's vocabulary, which is more than
a burn-down step should decide on its own. Left for an explicit call.

### D — `Workspace::forget` (4 writes)

`project::forget_confirmed` → `workspace.foreground_by_project`, `workspace.included_worktrees`,
`workspace.sessions`, `workspace.worktree_names`.

Forgetting a project drops everything held against its path, and three features hold something. The
write is *one call* in `micold-core`; the four members it reaches are what make it four rows. The
clearest case in the catalogue: `ProjectForgotten(path)` with session and worktree each answering
for their own.

### E — `push_notification` (2 writes, `notify`)

`session::arm_notice`, `project::open_refused`. The contract already named this one and **T065
built it**: `NotificationRaised`, listed under "emitted by: any feature". Convert first — the
vocabulary is done, so this is a pure call-site change and the cheapest proof T067a works.

### F — the reveal (4 writes)

`session::set_current_session` → `default_expanded`, `expanded`, `pending_reveal_scroll`;
`sidebar::toggle_location` → `reveal_suppressed_for`.

Displaying a session opens the row that holds it, commits the outgoing one, drops a stale
suppression and arms a scroll. `SessionRevealed(SessionId)` covers the first three. The fourth is
the mirror in the other direction — collapsing a row cancels a suppression a session close armed —
and may want its own variant; grouped here because the two are one conversation.

**Care needed**: `tests/current_session_writers.rs` holds that every writer of `active_session`
goes through `set_current_session`. Converting F must not give a second path a way in.

### G — the form creates, the worktree feature owns the list (4 writes)

`worktree_form::created` → `worktrees`, `worktree_error`; `worktree_form::create_failed` →
`worktree_error`; `worktree_form::opened` → `worktree_error`.

`worktree_form` is a separate feature precisely because its lifecycle is independent (FR-003, and
T064 gave it its own message type), but what it creates lands in `worktree`'s list and its failures
land in `worktree`'s error slot. `WorktreeCreated(Worktree)` / `WorktreeCreateFailed(String)`.

### The three singletons

* **`session::switch_active` → `workspace.active`** (via `Workspace::activate`). Switching the
  active project *is* a project operation; the session feature calls it because the switch is what
  its own step 1 and step 3 bracket. Possibly not a violation so much as a function in the wrong
  module — decide before converting, as with B.
* **`sidebar::toggle_location` → `reveal_suppressed_for`** — see F.
* **`session::restore_after_activation` → `show_agent_worktrees`** (feature 014, FR-010e): arriving
  in a project must not carry the previous one's reveal of agent worktrees. A sidebar fact reset
  from the session's activation path; folds naturally into whatever `ProjectActivated` shape D and
  F settle on.

## What T067a should do with this

1. **E first** — the outcome exists, so it is a call-site change and proves the pipeline.
2. ~~**C next** — largest group, existing mechanism, no open questions.~~ **Wrong: C has an open question of its own**; see the correction in its section.
3. **D, G** — clean causes, new variants, no ambiguity.
4. **A** — needs `DialogOpened` designed, but T063 removed the blocker.
5. **B and the `switch_active` singleton — decide the "wrong file?" question first.** Converting a
   misplaced function into an outcome would enshrine the misplacement.

Each conversion deletes its rows from `ALLOWED`, and
`the_allowlist_names_only_live_violations` fails if a row stops being a violation without being
deleted — so a conversion that forgets to update the list is caught by the same test that permitted
it.
