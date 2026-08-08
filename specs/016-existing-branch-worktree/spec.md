# Feature Specification: Reuse or Overwrite an Existing Branch When Creating a Worktree

**Feature Branch**: `fix/support-existing-branches`

**Created**: 2026-07-26

**Status**: Draft

**Input**: User description: "Creation of worktree creates a new branch but if the branch already exists should ask user what to do. Overwrite the branch or reuse it. It's required e.g. to continue the work that was started outside of micold ide"

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

## Requirements *(mandatory)*

### Functional Requirements

#### Detecting and resolving a conflict

- **FR-001**: When the branch name derived for a new worktree already exists in the repository, the system MUST pause creation and present the user with an explicit choice rather than failing with a duplicate-branch error.
- **FR-002**: The choice presented MUST offer, at minimum: reuse the existing branch, overwrite the existing branch, and cancel.
- **FR-003**: The presented choice MUST identify the conflicting branch by name and state in plain language what reuse and overwrite each do, including that overwrite discards the existing branch's commits.
- **FR-004**: On reuse, the system MUST create the worktree checked out on the existing branch, leaving that branch's commit history unmodified.
- **FR-005**: On overwrite, the system MUST require an explicit confirmation of the destructive outcome before modifying anything, and MUST allow the user to back out of that confirmation to the original choice.
- **FR-006**: On confirmed overwrite, the system MUST replace the existing branch with a new branch of the same name starting from the same point used for a conflict-free new worktree, and create the worktree on it.
- **FR-007**: On cancel — at either the choice or the overwrite confirmation — the system MUST leave the repository, the branch, and the filesystem completely unmodified, and MUST return the user to the creation form with their entered values preserved.
- **FR-008**: Failure recovery for a reuse-based creation MUST NOT delete the pre-existing branch; only branches the operation itself created may be removed during recovery.
- **FR-009**: The system MUST re-verify the existing branch's state immediately before acting on the user's choice, and MUST abandon the operation with an explanatory message if that state no longer matches what the user was shown.

#### Selecting an existing branch directly

- **FR-010**: The create-worktree form MUST let the user work from an existing branch chosen from a list, as an alternative to entering inputs for a new branch.
- **FR-011**: The list MUST include the repository's local branches and branches known on its remotes, indicating for each whether it is local, remote-only, and — for remote-only entries — which remote it comes from.
- **FR-012**: Branches that cannot be checked out because they are already in use MUST appear in the list marked unavailable with the reason, rather than being omitted without explanation.
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
- **FR-021a**: When the holder is a worktree the app does not manage — one living outside the directory the app creates its worktrees in — the explanation MUST say that it is outside the app, and MUST identify it by its full location rather than by a bare folder name. A bare folder name is indistinguishable from an entry in the app's worktree list, which sends the user looking for something that is not there (BUG-001).
- **FR-021b**: When the holder is a worktree the app manages but is not currently showing — an assistant-created worktree while the reveal control is off — the explanation MUST say that the holder is hidden and how to reveal it, rather than naming a worktree the user cannot see (BUG-001).
- **FR-022**: A pre-existing target directory MUST continue to block creation as it does today, and MUST be reported without offering the existing-branch choice.
- **FR-023**: A worktree created by reusing, overwriting, or continuing from a remote branch MUST be indistinguishable from any other worktree in subsequent use — listing, sessions, and deletion (including the option to delete the branch) behave identically.
- **FR-024**: Progress and outcome reporting for creation MUST cover the reuse, overwrite, and remote-continuation paths, naming the step being performed, consistent with the reporting shown for conflict-free creation.
- **FR-025**: Creating a worktree on a branch name that does not exist anywhere MUST behave exactly as it does today, with no additional prompts or steps.
- **FR-026**: User-facing documentation MUST describe the existing-branch choice, selecting a branch from the list, what reuse, overwrite, and remote continuation each do, and when none of them is available.

### Key Entities

- **Existing branch conflict**: the situation where the branch name derived from the user's creation inputs matches a branch already present locally or on a remote. Carries the branch name, whether it is local or remote-only (and on which remote), whether it is currently checked out anywhere, and if so which location holds it.
- **Branch candidate**: an entry in the existing-branch list — its name, its origin (local or a named remote), and whether it is available for a new worktree or blocked with a reason.
- **Conflict resolution choice**: the user's decision for a given conflict — reuse, overwrite, continue from remote, start fresh, or cancel — together with, for overwrite, the separate explicit confirmation of the destructive outcome.

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
