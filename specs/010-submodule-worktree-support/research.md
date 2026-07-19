# Research: Git Submodule Support for Worktree Creation

## R1 — Detecting whether the new worktree has submodules

**Decision**: After `worktree_add_new_branch` succeeds, ask the `Git` boundary whether the
newly created worktree directory contains a `.gitmodules` file (`has_submodules(worktree_path)`
on the `Git` trait, implemented in `GitCli` as a file check on the checked-out tree). If absent,
creation completes exactly as today (FR-003). If present, proceed to R2.

**Rationale**: `git worktree add -b <branch> <path> HEAD` already checks out the tree at HEAD
into `path`, so `.gitmodules` — a normal tracked file — is present in the new worktree the
moment `worktree add` succeeds, with no extra git invocation needed to find out. Checking the
*new worktree's* copy (not the source repo's) is also the only correct source of truth: the
worktree may be created from a branch/commit whose `.gitmodules` differs from the source
checkout. Routing the check through the `Git` trait (rather than a raw `fs::exists` call inside
`worktree.rs`) preserves `worktree.rs`'s existing "no direct subprocess/fs" invariant, so
`create_worktree`'s orchestration stays unit-testable against `FakeGit` (Constitution
Principle I), matching how `target_exists` is already threaded in as a boundary-supplied fact
rather than computed inline.

**Alternatives considered**: Running `git config --file .gitmodules --get-regexp path` up front
against the *source* repo before creating the worktree — rejected: extra subprocess call for
information a plain file check on the already-created worktree gives for free, and it would
check the wrong tree if HEAD differs from what gets checked out.

## R2 — Fetching submodules (including nested ones)

**Decision**: Add `submodule_update_init_recursive(worktree_path)` to the `Git` trait,
implemented as `git -C <worktree_path> submodule update --init --recursive`. Run it as a
second, separate step after `worktree_add_new_branch` succeeds (not as a flag on `worktree
add` itself).

**Rationale**: `git submodule update --init --recursive` is the single command that covers the
full requirement in one call: it registers each submodule from `.gitmodules` (`--init`), clones
it if not already present, checks it out at the commit the superproject records, and recurses
into submodules-of-submodules (`--recursive`) — directly satisfying FR-002's nested-submodule
requirement. `git worktree add` has no submodule-recursion flag of its own (that exists on `git
clone`, not `worktree add`), so a second, explicit command is required regardless. Keeping it as
a distinct step (rather than trying to fold it into one shell invocation) also keeps the
existing `create_worktree` orchestration's failure/rollback boundary exactly where it already
is: after a `Git` trait call returns `io::Result`.

**Alternatives considered**: `git clone --recurse-submodules` semantics via a fresh clone instead
of a worktree — rejected outright, it abandons the whole point of `git worktree` (shared object
store, no full extra clone) that the app already relies on (research R7, feature 005).

## R3 — Failure handling integrates with the existing rollback plan

**Decision**: A failure from `submodule_update_init_recursive` is treated exactly like a failure
from `worktree_add_new_branch` today: `create_worktree` runs the existing `rollback_plan()`
(`worktree_remove(force)` → `worktree_prune` → `branch_delete` → caller removes the directory)
and returns an error. The error message surfaced to the user is git's own stderr text from the
failed `submodule update` invocation, unmodified — git's own messages already name the failing
submodule path and the underlying cause (e.g. "Unable to fetch in submodule path 'libs/foo'",
"Authentication failed", "fatal: reference is not a tree").

**Rationale**: This is the behavior the user chose (clarification, spec FR-005): any submodule
fetch failure rolls the whole worktree creation back to a clean pre-creation state, with no
"partially usable" worktree left behind. Reusing the *same* rollback plan the codebase already
has for `worktree_add_new_branch` failures — rather than inventing a second, submodule-specific
cleanup path — keeps `CreateError` and the rollback ordering singular and already-tested.
Surfacing git's raw stderr (rather than building a custom failure-category classifier) satisfies
FR-006's "which submodule and why" requirement with zero new parsing logic to maintain; git's
own error text is already specific per failure kind (network vs. auth vs. missing commit).

**Alternatives considered**: A bespoke enum classifying failures into
`Network`/`Auth`/`UnreachableCommit` — rejected as unnecessary complexity: git's stderr already
distinguishes these in the message text, the spec only requires the *user* to be able to tell
what happened (FR-006/SC-003), not the app to branch on the category programmatically, and a
hand-rolled classifier would be a fragile, high-maintenance parser of git's evolving message
text for no behavioral gain.

## R4 — Keeping the UI responsive while submodules fetch (progress feedback)

**Decision**: Move worktree creation off the synchronous `update()` path it uses today (`main.rs`
`Message::AddWorktreeSubmitted` currently calls `create()` — which runs `git worktree add`
directly — inline, before returning `Task::none()`) onto `iced::Task::perform`, following the
existing precedent in `main.rs` where `Task::perform(async move { scan(dir) }, ...)` wraps a
blocking, synchronous function for the folder-listing scan. `AddWorktreeSubmitted` will:
1. Validate the form and immediately dispatch a new `WorktreeCreateStarted` message so the form
   can show a "Creating worktree…" state right away (covers the fast, non-submodule path too).
2. Return `Task::perform(async move { create(&repo, &names) }, ...)`, mapping `Ok`/`Err` to the
   existing `WorktreeCreated` / `WorktreeCreateFailed` messages.

Because `.gitmodules` detection happens *inside* `create()` (R1), a submodule-free repository's
task resolves just as fast as today — the async wrapper adds no latency, only removes blocking.

**Rationale**: `create()` — worktree add plus, now, submodule fetch — is a blocking subprocess
call that can legitimately take from milliseconds (no submodules) to minutes (many/large
submodules over a slow network). Running it inline inside `update()`, as the code does today,
would freeze the whole iced UI thread for that entire duration — acceptable when the operation
was always sub-second, not acceptable once it can be network-bound. `Task::perform` is the
established, already-used mechanism in this codebase for exactly this shape of problem, and the
`gui` feature already pulls in `iced`'s `tokio` feature for async support (used by the OS-theme
poll and PTY streaming, per feature 006), so no new dependency is introduced. A `Creating
worktree…` text state satisfies SC-002 ("user can tell fetching is underway within 1 second")
without needing byte-level progress — this project has no existing spinner/progress-bar widget
(`terminal.rs` notes a live activity indicator is an unbuilt follow-up), so a text label matches
the one existing precedent for an in-progress UI state (`SelectorStatus::Loading` → "Loading…"
in `project_selector.rs`) rather than introducing a new shared component for one call site.

**Alternatives considered**: A dedicated progress-reporting channel/subscription streaming
per-submodule fetch progress (à la the PTY output streaming pattern, research R4 of feature
006) — rejected as disproportionate: the spec's success criterion is that fetching *reads as
in-progress*, not that the user sees a live submodule-by-submodule progress bar; a single
"Creating worktree…" → done/error transition meets that bar with far less new surface area.

## R5 — Cross-platform behavior

**Decision**: No platform-specific code. `submodule_update_init_recursive` shells out to the
same `git` binary via `std::process::Command` that every other `Git` trait method already uses
(research R7, feature 005), and `git submodule` ships as part of core git on every platform the
app targets — no additional binary, package, or OS branch is introduced.

**Rationale**: Directly satisfies Constitution Principle VI (Cross-Platform Parity) the same way
the existing worktree/branch commands already do; CI already builds and tests on Linux, macOS,
and Windows, so this feature rides the existing gate with no new platform-specific test lane.

## R6 — Residual git-internals state after a rollback (accepted limitation)

**Decision**: Rely on the existing `rollback_plan()` (`worktree remove --force` → `worktree
prune` → `branch -D`) exactly as-is; do not add a submodule-specific `git submodule deinit`
step to rollback.

**Rationale**: `git worktree remove --force` deletes the worktree's working directory (including
any partially-fetched submodule content within it) and deregisters the worktree; that is the
same cleanup a user would get running the equivalent commands by hand. Git may retain the
fetched submodule's own repository under the main repo's internal storage after such a removal
— a known characteristic of how git stores submodule data, identical to what would happen if a
user manually ran `git submodule update --init` inside a worktree and then removed that worktree
from a terminal. Adding a bespoke `submodule deinit --all --force` step to chase this down is
extra failure-prone surface (deinit itself can fail or partially apply) for state that is inert
disk usage, not a correctness or data-loss issue, and nothing in the spec's success criteria
requires reclaiming it.

**Alternatives considered**: Explicit `git submodule deinit --all --force` before `worktree
remove` in the rollback path — rejected per above; can be revisited later as a standalone
cleanup enhancement if it proves to matter in practice.

## R7 — Documentation placement

**Decision**: Extend the existing "Creating a worktree" section of
`docs/user-guide/worktrees-and-sessions.md` with a short paragraph describing automatic
submodule fetching and what the user sees on failure, rather than adding a new doc page.

**Rationale**: Principle VII requires user-facing changes to ship with user-guide docs in the
same change. This feature is a behavioral addition to an existing, already-documented flow
("Creating a worktree"), not a new surface, so it belongs in that section rather than a new file
— consistent with how other worktree-creation details are already documented there.
