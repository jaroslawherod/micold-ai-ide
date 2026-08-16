# Feature Specification: Reuse or Overwrite an Existing Branch When Creating a Worktree

**Feature Branch**: `fix/support-existing-branches`

**Created**: 2026-07-26

**Status**: Draft

**Input**: User description: "Creation of worktree creates a new branch but if the branch already exists should ask user what to do. Overwrite the branch or reuse it. It's required e.g. to continue the work that was started outside of micold ide"

**Bugfix**: 2026-08-14 — [BUG-002](./bugs/BUG-002.md) added User Story 6 (including a worktree that
already exists), FR-027–FR-033 for it and FR-034–FR-035 for refusals that must not dismiss the form,
six Edge Cases, the "Included worktree" entity, SC-009–SC-011, and three Assumptions; annotated
FR-007, FR-012, and FR-021a.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Continue work on a branch that already exists (Priority: P1)

A developer started a piece of work outside the app — in a plain terminal, on another checkout, or in an earlier session — and a branch for it already exists in the repository. They now want to carry that work on inside the app. Today, creating a worktree whose derived branch name matches that existing branch is rejected outright with a duplicate-branch error, leaving the user no way in short of renaming their work or leaving the app. Instead, the user wants to be told the branch already exists and be offered the choice to **reuse** it — creating the worktree checked out on the existing branch, with all its existing commits intact.

**Why this priority**: This is the entire reason the feature was requested — picking up existing work is currently impossible from inside the app, and no workaround exists within the product. Reuse alone is a complete, shippable slice of value.

**Independent Test**: Can be fully tested by creating a branch outside the app with at least one distinctive commit on it, then creating a worktree with the same derived name, choosing "reuse" at the prompt, and confirming the new worktree is checked out on that branch with its commit history present and untouched.

**Acceptance Scenarios**:

1. **Given** a branch named `feat/reporting` already exists in the repository and no worktree is bound to it, **When** the user submits the create-worktree form with inputs that derive the branch name `feat/reporting`, **Then** creation pauses and the user is told the branch already exists and asked how to proceed, instead of failing with a duplicate-branch error.
2. **Given** that prompt is showing, **When** the user chooses to reuse the existing branch, **Then** a worktree is created at the target directory checked out on the existing branch, and every commit that was on the branch beforehand is still on it afterward.
3. **Given** the user reused an existing branch, **When** creation completes, **Then** the resulting worktree behaves like any other worktree in the app (appears in the sidebar, can host sessions, can be deleted) with no residual distinction in how it is used.
4. **Given** the prompt is showing, **When** the user cancels instead of choosing, **Then** nothing is created or modified — the branch, the repository, and the target directory are left exactly as they were, and the user is returned to the create form with their inputs preserved.
5. **Given** the user chose to reuse the branch, **When** the worktree creation itself fails partway through (for example the directory cannot be created), **Then** the pre-existing branch and its commits still exist afterward — recovery from the failure MUST NOT delete a branch the user did not create.

---

### User Story 2 - Pick an existing branch instead of guessing its name (Priority: P2)

Reaching an existing branch only by accidentally colliding with it requires the user to retype a type and name that derive exactly the right branch — guesswork if they don't recall the exact slug. A developer returning to earlier work wants to open the create-worktree form and **choose the branch from a list of branches that exist in the repository**, rather than reconstructing its name from memory.

**Why this priority**: This turns "continue work started outside the app" from a lucky collision into a first-class, discoverable path — the outcome the request is actually aiming at. It depends on the reuse behavior from Story 1 existing, but is independently valuable and independently testable once it does.

**Independent Test**: Can be fully tested by creating several branches outside the app, opening the create-worktree form, selecting one of them from the existing-branch list, and confirming a worktree is created on exactly that branch — without the user typing a name at all.

**Acceptance Scenarios**:

1. **Given** the create-worktree form is open, **When** the user chooses to work from an existing branch, **Then** the branches present in the repository are listed for selection.
2. **Given** the branch list is showing, **When** the user selects a branch that is not checked out anywhere, **Then** the form shows the worktree directory that will be created for it, and creation proceeds on that branch with its history intact — the same outcome as choosing "reuse" in Story 1.
3. **Given** the branch list is showing, **When** it contains a branch that is already checked out elsewhere, **Then** that branch is shown as unavailable together with the reason, rather than being silently omitted from the list or offered and then failing.
4. **Given** the user selected an existing branch, **When** they change their mind and return to creating a new branch, **Then** the form returns to its normal new-branch inputs with no leftover state from the selection.
5. **Given** a repository with no branches other than those already checked out, **When** the user opens the existing-branch list, **Then** they are told there are no branches available to reuse, rather than shown an empty control with no explanation.

---

### User Story 3 - Start over on a name that is already taken (Priority: P3)

A developer wants to start fresh work under a name that happens to already be used by a stale, abandoned branch — for example a branch from an experiment that was never cleaned up. They do not want that branch's history; they want the name. At the prompt raised in User Story 1, they want to choose **overwrite**: discard the existing branch and create a fresh one at the same starting point a brand-new worktree would use.

**Why this priority**: This is the second half of the requested choice and rounds out the prompt, but it destroys commits, so it is riskier and less frequently needed than reuse. Reuse can ship without it; this cannot ship without the prompt Story 1 introduces.

**Independent Test**: Can be fully tested by creating a branch with a distinctive commit outside the app, creating a worktree with the same derived name, choosing "overwrite", and confirming the resulting worktree starts from the repository's current head with the distinctive commit no longer reachable from that branch.

**Acceptance Scenarios**:

1. **Given** the existing-branch prompt is showing, **When** the user chooses to overwrite, **Then** they are warned in plain language that the existing branch's commits will be discarded and are asked to confirm before anything is changed.
2. **Given** the overwrite warning is showing, **When** the user confirms, **Then** the old branch is replaced by a new branch of the same name starting from the same point a brand-new worktree would start from, and the worktree is created on it.
3. **Given** the overwrite warning is showing, **When** the user declines the warning, **Then** they are returned to the choice prompt with reuse and cancel still available, and nothing has been modified.
4. **Given** the user chose to overwrite, **When** worktree creation fails after the branch was replaced, **Then** the failure is reported, the app is left in a consistent state with no half-registered worktree, and the user is told that the previous branch contents were already discarded.

---

### User Story 4 - Continue work pushed from somewhere else (Priority: P4)

A developer started work on another machine (or it arrived from a colleague or from a pull request) and the branch exists on a remote but not yet locally. They want the same continuation path: the app recognizes the remote branch, and reusing it starts a local branch at the remote branch's tip that tracks it — so their next push goes back to the right place.

**Why this priority**: It extends the same continuation story to a materially different source, and without it a remote-only name looks like "no conflict" and silently produces an empty branch that shadows the remote one. It is lower priority than the local cases because it depends on the same machinery and applies to fewer sessions.

**Independent Test**: Can be fully tested by making a branch available on a remote without a corresponding local branch, creating a worktree for that name, choosing to reuse, and confirming the worktree is on a local branch at the remote tip, tracking the remote branch.

**Acceptance Scenarios**:

1. **Given** a branch exists on a remote and no local branch of that name exists, **When** the user's inputs derive that name (or they select it from the existing-branch list), **Then** the app identifies it as a remote branch and offers to continue from it, rather than treating the name as unused.
2. **Given** the user chooses to continue from the remote branch, **When** creation completes, **Then** the worktree is on a local branch of that name positioned at the remote branch's tip and set to track it, and the remote branch itself is unchanged.
3. **Given** a remote-only branch is offered, **When** the user instead chooses to start fresh at the repository's current head under that name, **Then** a new local branch is created there, no commits are destroyed, and the user has been told the resulting branch will differ from the remote one of the same name.
4. **Given** both a local branch and a remote branch of the same name exist, **When** the conflict is presented, **Then** it is handled as the local-branch case from Stories 1 and 3 — the local branch is what reuse and overwrite act on.
5. **Given** the remote branch's state was last updated by an earlier fetch, **When** the app presents it, **Then** the app makes clear it reflects what is already known locally and does not contact the remote as part of this flow.

---

### User Story 5 - Understand when an existing branch cannot be used (Priority: P5)

Some existing branches cannot be reused or overwritten at all — most commonly because the branch is already checked out somewhere else, which git does not permit to be checked out twice. The holder is not always somewhere the user can see: it may be another of the app's worktrees, the repository's own current branch, an assistant-created worktree the app is currently hiding, or a worktree created by some other tool entirely, living outside the directory this app manages. Rather than a raw git failure — or, worse, a folder name that looks like a worktree in the list but is not in it — the user wants an explanation that identifies where the branch is in use well enough to actually go there.

**Why this priority**: This is a correctness and comprehension safeguard around the choices above rather than new capability. Without it the feature still works for the common case; with it the confusing failure mode disappears.

**Independent Test**: Can be fully tested by creating a worktree bound to branch `feat/x`, then attempting to create a second worktree that derives the same branch name, and confirming the app explains that the branch is already in use and where, offering no reuse or overwrite option.

**Acceptance Scenarios**:

1. **Given** an existing branch is already checked out in another worktree managed by the app, **When** the user attempts to create a worktree deriving that branch name, **Then** the app explains that the branch is already in use and identifies the worktree using it, and does not offer reuse or overwrite.
2. **Given** an existing branch is the repository's own currently checked-out branch, **When** the user attempts to create a worktree deriving that branch name, **Then** the app explains that the branch is in use by the project's main checkout, and does not offer reuse or overwrite.
3. **Given** an existing branch is checked out in a worktree the app does not manage — one outside the directory the app creates its worktrees in, such as another tool's worktree directory or an unrelated folder elsewhere on disk — **When** the user attempts to create a worktree on that branch, **Then** the app explains that the holder is outside the app and gives its full location, so the user does not search the worktree list for a folder name that will never appear there.
4. **Given** an existing branch is checked out in an assistant-created worktree while the reveal control is off, **When** the user attempts to create a worktree on that branch, **Then** the app explains that the holder is a hidden assistant worktree and how to reveal it, rather than naming a worktree that is absent from the list as shown.
5. **Given** any of the above blocked cases, **When** the message is shown, **Then** the repository is unchanged and the user can amend their inputs and try again without restarting the flow.

---

### User Story 6 - Include a worktree that already exists (Priority: P6)

*(Added by BUG-002.)*

A developer's branch is held by a worktree that already exists on disk — created by another tool, or
by hand in a sibling directory — and the app refuses the branch and explains where it is. The
explanation is now accurate and still a dead end: the work is right there, and the only ways to reach
it are to leave the app or to destroy something. What the developer wants is the obvious third
answer: **include that worktree** — tell the app to show it too, exactly where it is.

**Why this priority**: It is the answer to the question User Story 5 leaves the user holding, and
the one BUG-001 deferred as a product decision ("hard block by accident, not by decision"). Every
story above it can ship without this; this cannot ship without the block that names the holder.

**Independent Test**: Can be fully tested by creating a worktree outside the app's own worktree
directory (`git worktree add ../elsewhere feat/x`), opening the create form, selecting `feat/x`,
choosing to include the worktree it names, and confirming the worktree appears in the list, hosts a
session, and is byte-for-byte unchanged on disk.

**Acceptance Scenarios**:

1. **Given** a branch is blocked because it is checked out in a worktree the app does not manage,
   **When** the block is explained, **Then** the user is offered the option to include that worktree
   in the app, alongside the explanation of where it is.
2. **Given** the user chooses to include it, **When** inclusion completes, **Then** the worktree
   appears in the worktree list, its location is visible because it does not live where the app's own
   worktrees do, and it can host sessions like any other.
3. **Given** a worktree was included, **When** the app is restarted, **Then** it is still included —
   and still included only for the project it belongs to.
4. **Given** a worktree was included, **When** the user stops including it, **Then** it disappears
   from the list and nothing on disk or in the repository has changed.
5. **Given** the user chooses to include a worktree, **When** inclusion completes, **Then** nothing
   has been moved, copied, re-registered, or checked out — the app recorded a location and did no more.
6. **Given** a worktree has been included, **When** the user later attempts to create a worktree on
   the branch it holds, **Then** the branch is still blocked, but the holder is now described as one
   of the app's own worktrees rather than as one outside it.
7. **Given** the holder is the project's own checkout, or an assistant worktree the app already
   manages but is currently hiding, **When** the block is explained, **Then** inclusion is not
   offered — neither is a worktree the app is missing.

---

### Edge Cases

- **Branch exists and the target directory also exists**: the directory conflict is reported as it is today; the existing-branch choice is not offered, because creation cannot proceed at that location regardless of which branch option is chosen.
- **Branch is deleted or created by something else between the check and the user's answer**: the app re-verifies the branch state at the moment the user confirms; if the situation no longer matches what the prompt described, the operation is abandoned with an explanatory message rather than acting on stale information.
- **Reused branch has no commits beyond the repository's head**: reuse behaves identically — the worktree is created on the existing branch; nothing special is reported.
- **Overwrite is chosen for a branch whose commits exist nowhere else**: the warning states that the commits will no longer be reachable from any branch; the user's confirmation is still what authorizes it.
- **The existing branch cannot be replaced during overwrite** (for example it becomes checked out elsewhere in the interim): the failure is reported plainly, the branch is left as it was, and no worktree is created.
- **A worktree created by reuse is later deleted with "also delete the branch" selected**: the branch is deleted, as it would be for any other worktree — reuse does not make a branch permanently undeletable.
- **The same branch name exists on more than one remote**: the user is shown which remote each candidate comes from and picks explicitly; the app does not silently choose one.
- **A selected existing branch produces a worktree directory name that is already taken**: the directory conflict is reported before anything is created, and the user can choose a different branch or resolve the directory.
- **The branch is held by a worktree outside the app's own worktree directory** (another tool's worktree tree, or a folder anywhere else on disk): creation is blocked as for any other holder, but the explanation says the holder is outside the app and gives its full location. Naming it by folder name alone would describe something the worktree list can never show (BUG-001).
- **The branch is held by an assistant-created worktree while those are hidden**: creation is blocked, and the explanation says the holder is a hidden assistant worktree and how to reveal it. The holder is one of the app's own, so it is named as such — it is only the current view that omits it.
- **The repository's worktree list changes while the form is open**: the holder shown in a blocked explanation is whatever the pre-flight check observed; the check runs again at the moment of action, so a holder that has since released the branch does not block creation.
- **The repository has no remotes at all**: only local branches are considered; nothing in the flow requires a remote to exist.
- **A press on an unavailable branch row lands beyond the form's own edges** (the result list floats over and past the form): the press is consumed where it lands and nothing happens. It MUST NOT reach whatever sits behind the form, and in particular MUST NOT dismiss the form — a refusal that closes the form is indistinguishable from a cancel the user never made (BUG-002).
- **An included worktree's folder name matches one the app already lists**: both are shown and told apart by their locations. Inclusion never renames anything, because the folder is not the app's to rename.
- **An included worktree is removed, or unregistered, outside the app**: the entry is reported as missing rather than silently dropped, so removing it from the list stays the user's deliberate act.
- **A directory under the app's own worktree directory that git no longer knows about**: already listed as an invalid worktree, and not what inclusion covers — git holds no branch for it, so it never produces the block that offers inclusion. Repairing such a directory is out of scope.
- **The holder is the project's own checkout**: inclusion is not offered. The project checkout is already the project; there is nothing to add.
- **The holder is an assistant worktree the app manages but is currently hiding**: inclusion is not offered either, for the opposite reason — it is already included. The explanation says how to reveal it (FR-021b).

## Requirements *(mandatory)*

### Functional Requirements

#### Detecting and resolving a conflict

- **FR-001**: When the branch name derived for a new worktree already exists in the repository, the system MUST pause creation and present the user with an explicit choice rather than failing with a duplicate-branch error.
- **FR-002**: The choice presented MUST offer, at minimum: reuse the existing branch, overwrite the existing branch, and cancel.
- **FR-003**: The presented choice MUST identify the conflicting branch by name and state in plain language what reuse and overwrite each do, including that overwrite discards the existing branch's commits.
- **FR-004**: On reuse, the system MUST create the worktree checked out on the existing branch, leaving that branch's commit history unmodified.
- **FR-005**: On overwrite, the system MUST require an explicit confirmation of the destructive outcome before modifying anything, and MUST allow the user to back out of that confirmation to the original choice.
- **FR-006**: On confirmed overwrite, the system MUST replace the existing branch with a new branch of the same name starting from the same point used for a conflict-free new worktree, and create the worktree on it.
- **FR-007**: On cancel — at either the choice or the overwrite confirmation — the system MUST leave the repository, the branch, and the filesystem completely unmodified, and MUST return the user to the creation form with their entered values preserved. *(BUG-002: this describes a cancel the user chose. Only a deliberate cancel may close the form at all — see FR-034.)*
- **FR-008**: Failure recovery for a reuse-based creation MUST NOT delete the pre-existing branch; only branches the operation itself created may be removed during recovery.
- **FR-009**: The system MUST re-verify the existing branch's state immediately before acting on the user's choice, and MUST abandon the operation with an explanatory message if that state no longer matches what the user was shown.

#### Selecting an existing branch directly

- **FR-010**: The create-worktree form MUST let the user work from an existing branch chosen from a list, as an alternative to entering inputs for a new branch.
- **FR-011**: The list MUST include the repository's local branches and branches known on its remotes, indicating for each whether it is local, remote-only, and — for remote-only entries — which remote it comes from.
- **FR-012**: Branches that cannot be checked out because they are already in use MUST appear in the list marked unavailable with the reason, rather than being omitted without explanation. *(BUG-002: reaching for such a branch must cost the user nothing — see FR-034/FR-035. A reason that can only be read by losing the form is not a reason the user can act on.)*
- **FR-013**: When no branch in the repository is available to reuse, the system MUST say so explicitly instead of presenting an empty list.
- **FR-014**: When an existing branch is selected, the system MUST derive the worktree directory from that branch name using the same naming rules as new-branch creation, and MUST show the user the directory that will be created before they commit to it.
- **FR-015**: Selecting an existing branch and then returning to new-branch creation MUST clear the selection and restore the form's new-branch inputs, leaving no residual state.

#### Remote branches

- **FR-016**: When the derived or selected branch name exists only on a remote, the system MUST identify it as a remote branch and offer to continue from it rather than treating the name as unused.
- **FR-017**: Continuing from a remote branch MUST create a local branch of that name at the remote branch's tip, set to track that remote branch, and MUST leave the remote branch itself unchanged.
- **FR-018**: When a remote branch of the derived name exists, the system MUST also allow the user to start fresh at the repository's current head instead, having first stated that the resulting branch will diverge from the remote branch of the same name.
- **FR-019**: When both a local and a remote branch of the same name exist, the system MUST treat the conflict as the local-branch case; reuse and overwrite MUST act on the local branch.
- **FR-020**: The system MUST NOT contact any remote as part of this flow; remote branch information MUST come from what the repository already knows locally, and the system MUST make that clear to the user.

#### Blocked cases, parity, and reporting

- **FR-021**: When the existing branch is already checked out anywhere the repository knows about, the system MUST block creation with an explanation identifying where the branch is in use, and MUST NOT offer reuse or overwrite. There are three kinds of holder and the explanation MUST distinguish them: another worktree managed by the app, the repository's own current checkout, and a worktree the repository knows about that the app does not manage.
- **FR-021a**: When the holder is a worktree the app does not manage — one living outside the directory the app creates its worktrees in — the explanation MUST say that it is outside the app, and MUST identify it by its full location rather than by a bare folder name. A bare folder name is indistinguishable from an entry in the app's worktree list, which sends the user looking for something that is not there (BUG-001). *(BUG-002: this is also the one holder the user can do something about from here — the explanation MUST offer to include it, per FR-027.)*
- **FR-021b**: When the holder is a worktree the app manages but is not currently showing — an assistant-created worktree while the reveal control is off — the explanation MUST say that the holder is hidden and how to reveal it, rather than naming a worktree the user cannot see (BUG-001).
- **FR-022**: A pre-existing target directory MUST continue to block creation as it does today, and MUST be reported without offering the existing-branch choice.
- **FR-023**: A worktree created by reusing, overwriting, or continuing from a remote branch MUST be indistinguishable from any other worktree in subsequent use — listing, sessions, and deletion (including the option to delete the branch) behave identically.
- **FR-024**: Progress and outcome reporting for creation MUST cover the reuse, overwrite, and remote-continuation paths, naming the step being performed, consistent with the reporting shown for conflict-free creation.
- **FR-025**: Creating a worktree on a branch name that does not exist anywhere MUST behave exactly as it does today, with no additional prompts or steps.
- **FR-026**: User-facing documentation MUST describe the existing-branch choice, selecting a branch from the list, what reuse, overwrite, and remote continuation each do, and when none of them is available.

#### Including a worktree that already exists *(added by BUG-002)*

- **FR-027**: When a branch is blocked because it is checked out in a worktree the app does not manage (FR-021a), the system MUST offer to include that worktree in the app, from the same explanation that reports the block.
- **FR-028**: Including a worktree MUST NOT move, copy, rename, re-register, check out, or otherwise modify it, and MUST NOT modify the repository. The system records that it also shows this location, and does no more.
- **FR-029**: An included worktree MUST appear in the worktree list and behave as any other worktree in subsequent use — listing, sessions, selection, deletion — and MUST show its location, since it does not live where the app's own worktrees do and its folder name alone would not say where it is.
- **FR-030**: Inclusion MUST persist per project across restarts, and MUST be reversible: the user can stop including a worktree, which removes it from the list and leaves it exactly as it was on disk.
- **FR-031**: An included worktree that has since been removed from disk, or that the repository no longer registers, MUST be reported as such in the list rather than silently dropped or shown as if it were intact.
- **FR-032**: Once a worktree is included, the explanation for a branch it holds MUST describe it as one of the app's worktrees (FR-021) rather than as one outside the app (FR-021a). The two descriptions MUST follow from the same test that decides what the list shows, so "described as yours" and "shown in your list" cannot disagree — the rule BUG-001 established, extended to cover inclusion.
- **FR-033**: Inclusion MUST NOT be offered for a holder that is the project's own checkout, or a worktree the app already manages (including one it is currently hiding); and deleting an included worktree from the app MUST state that it lives outside the directory the app manages, and give its location, before anything is removed.

#### Refusals never dismiss the form *(added by BUG-002)*

- **FR-034**: An action the form refuses — in particular attempting to choose a branch that is unavailable — MUST leave the form open with every input intact. A refusal MUST NOT be indistinguishable from a cancellation the user did not make.
- **FR-035**: A press that lands on any surface the form is showing, including a result list that floats over and beyond the form's own edges, MUST be consumed there. It MUST NOT reach whatever is behind the form, and in particular MUST NOT reach the form's dismissal. Only a press genuinely outside every surface the form is showing may dismiss it.

### Key Entities

- **Existing branch conflict**: the situation where the branch name derived from the user's creation inputs matches a branch already present locally or on a remote. Carries the branch name, whether it is local or remote-only (and on which remote), whether it is currently checked out anywhere, and if so which location holds it.
- **Branch candidate**: an entry in the existing-branch list — its name, its origin (local or a named remote), and whether it is available for a new worktree or blocked with a reason.
- **Conflict resolution choice**: the user's decision for a given conflict — reuse, overwrite, continue from remote, start fresh, or cancel — together with, for overwrite, the separate explicit confirmation of the destructive outcome.
- **Included worktree** *(BUG-002)*: a worktree the repository already knows about, living outside the directory the app creates its own in, that the user has asked the app to show. It carries its location and the project it belongs to, and nothing else — no copy of the worktree's state, which stays where it has always been read from.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user who started work on a branch outside the app can bring it into a new worktree entirely from within the app, with no manual git commands, in under 30 seconds from opening the create form.
- **SC-002**: A user who does not remember the exact branch name can still reach it, selecting it from the list without typing the name, in no more than three interactions from opening the create form.
- **SC-003**: 100% of reuse operations preserve the existing branch's commits — no commit reachable from the branch before creation is unreachable from it afterward, including when the creation subsequently fails.
- **SC-004**: No branch is ever discarded without the user having seen and confirmed an explicit warning naming the branch and stating that its commits will be discarded.
- **SC-005**: 100% of worktrees created by continuing from a remote branch start at that remote branch's tip and track it, verified by comparing the new branch's position and upstream against the remote branch.
- **SC-006**: 100% of attempts to create a worktree on a branch that is already checked out elsewhere produce an explanation the user can act on — one that identifies the holder well enough to reach it: a listed worktree named, a hidden worktree named together with how to reveal it, or an unmanaged worktree given by its full location. Naming a holder that has no corresponding entry in the app's worktree list, and no way to find it, counts as a failure of this criterion even though a location was printed (BUG-001).
- **SC-007**: Cancelling at any point in the flow leaves the repository unchanged, verified by comparing the branch list, the worktree list, and the target directory before and after.
- **SC-008**: Creating a worktree whose branch name does not exist anywhere is unaffected — the same number of steps, the same prompts, and the same outcome as before this feature.
- **SC-009**: 100% of presses on an unavailable branch row leave the form open with every input unchanged — at every position the row can occupy on screen, including rows the result list renders beyond the form's own edges. A press that closes the form counts as a failure of this criterion however plausible the reason (BUG-002).
- **SC-010**: A user whose branch is held by a worktree the app does not manage can bring that worktree into the app and start a session in it without leaving the app and without typing a git command, in under 30 seconds from the block being explained.
- **SC-011**: Including a worktree changes nothing outside the app's own settings: the worktree's location, its git registration, its branch, and its working tree are identical before and after, verified by comparing all four.

## Assumptions

- The reuse/overwrite choice applies to branch-name collisions only. A pre-existing target directory remains a hard failure, since no branch choice can resolve it.
- "Overwrite" means the branch is recreated at the same starting point a brand-new worktree would use (the repository's current head), because the existing creation flow offers no way to choose a different base. Choosing an arbitrary base commit is out of scope.
- Reuse checks out the existing branch as-is: no fetch, pull, rebase, or merge is performed on the user's behalf. Bringing the branch up to date is the user's own action inside the resulting worktree.
- Remote branch information reflects what the repository already knows from a previous fetch. Fetching or refreshing remotes from within this flow is out of scope; a stale remote view is possible and is surfaced to the user rather than corrected automatically.
- Git does not permit the same branch to be checked out in two worktrees simultaneously; this is treated as an unresolvable block rather than something the app works around (for example by detaching or duplicating the branch).
- Deleting a remote branch, or any other remote-mutating action, is out of scope; overwrite acts on local branches only.
- Behavior is identical across Linux, macOS, and Windows; nothing in this flow is platform-specific.
- The feature builds on the existing create-worktree form, its name derivation, and its progress reporting; it adds a decision point and an alternative branch source to that flow rather than introducing a separate creation path.
- The existing-branch list is a selection control of the kind the form already uses; it is expected to reuse the shared UI primitives rather than introduce a bespoke one-off widget.
- **Inclusion is by reference** *(BUG-002)*: the app records a location it also shows. It never moves a worktree into the directory it manages, because the holder is frequently another tool's and relocating it would break that tool's own reference to it. Relocation, if it is ever wanted, is a separate and explicitly destructive action.
- Inclusion is the first persisted state this feature adds, and it is per project — the same scope as the project's other local settings. Nothing about it leaves the device.
- Re-registering a directory git has forgotten — repairing an orphaned worktree — is out of scope. Such directories are already listed as invalid, and they never produce the block that offers inclusion, because git holds no branch for them.
