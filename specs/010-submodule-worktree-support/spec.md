# Feature Specification: Git Submodule Support for Worktree Creation

**Feature Branch**: `010-submodule-worktree-support`

**Created**: 2026-07-18

**Status**: Draft

**Input**: User description: "Support creating worktrees for repositories that use git submodules. Today, worktree creation runs `git worktree add -b <branch> <path> HEAD` and stops there, so any repo with a `.gitmodules` file ends up with empty, uninitialized submodule directories in the new worktree — the user has to manually run `git submodule update --init --recursive` themselves before the worktree is usable. Instead, when a worktree is created for a repository that has submodules, the app should automatically fetch and initialize them (recursively, so nested submodules are also populated) as part of worktree creation, with no extra user action required. Non-submodule repositories are unaffected and see no new behavior or prompts. While submodule fetch is running, the user should see it's in progress (this can be slow for large or numerous submodules) rather than the worktree form appearing hung. If submodule fetch fails partway (network failure, missing credentials for a private submodule remote, a submodule pointing at an unreachable commit), define what happens to the worktree that was just created. Also cover: repos with submodules that are already partially initialized in the parent repo, and cross-platform parity (macOS/Linux/Windows) for the submodule fetch step."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Create a usable worktree in a repository with submodules (Priority: P1)

A user creates a new worktree from a repository that declares one or more git submodules. Once creation finishes, every submodule (including submodules nested inside other submodules) is already fetched and checked out at the correct commit inside the new worktree — the user can start working immediately without running any submodule command themselves.

**Why this priority**: This is the core gap the feature closes. Without it, every worktree created from a submodule-using repository is silently incomplete, and the user only discovers missing code when a build or file lookup fails inside a submodule directory. It is the smallest slice that delivers the feature's value end to end.

**Independent Test**: Pick a repository that has at least one top-level submodule and one nested submodule, create a worktree from it, and verify — without running any git command by hand — that every submodule directory in the new worktree contains checked-out files matching the commit recorded by the parent repository.

**Acceptance Scenarios**:

1. **Given** a repository with a `.gitmodules` file and one or more submodules, **When** the user creates a worktree, **Then** every submodule is initialized and checked out at its recorded commit inside the new worktree, with no extra action from the user.
2. **Given** a repository whose submodules themselves contain submodules, **When** the user creates a worktree, **Then** the nested submodules are also initialized and checked out.
3. **Given** a repository with no `.gitmodules` file, **When** the user creates a worktree, **Then** worktree creation behaves exactly as it does today, with no submodule-related step, delay, or UI change.
4. **Given** a repository whose submodules are already checked out in the parent working copy, **When** the user creates a worktree, **Then** the new worktree's submodules are populated to match, regardless of the submodule state in the parent.

---

### User Story 2 - Know that submodule fetching is happening (Priority: P2)

While a worktree is being created for a repository with submodules, the user sees a clear indication that submodule content is being fetched, so a slow fetch (large submodules, many submodules, or a slow network) reads as "in progress" rather than as the application being frozen or unresponsive.

**Why this priority**: Submodule fetches can take substantially longer than the near-instant worktree creation users experience today. Without feedback, a slow-but-working fetch is indistinguishable from a hang, which erodes trust in the tool. This builds directly on Story 1 and only matters once automatic fetching exists.

**Independent Test**: Create a worktree from a repository with a submodule whose fetch takes several seconds, and confirm the creation UI visibly communicates that submodules are being fetched for the duration of the operation, then clearly indicates completion.

**Acceptance Scenarios**:

1. **Given** a worktree is being created for a repository with submodules, **When** submodule fetching begins, **Then** the user sees an indication that fetching is in progress.
2. **Given** submodule fetching is in progress, **When** it is still running after several seconds, **Then** the in-progress indication remains visible and the creation UI does not appear stuck or unlabeled.
3. **Given** submodule fetching completes, **When** the worktree is ready, **Then** the in-progress indication is replaced by a clear success state.

---

### User Story 3 - Understand and recover when submodule fetch fails (Priority: P2)

A submodule fetch can fail for reasons outside the user's control mid-creation: a network interruption, a private submodule remote the user isn't authenticated against, or a submodule pointing at a commit that no longer exists upstream. When this happens, the user is told clearly which submodule(s) failed and why, and the resulting state of the worktree (and its branch, on disk) is well-defined and consistent rather than a silent partial success.

**Why this priority**: Failure handling is what keeps the feature trustworthy once it's live — an automatic step that can fail unpredictably and leave unclear state is worse than no automation at all. It ranks alongside Story 2 because both are about making an inherently unreliable network operation legible to the user, but it depends on Story 1 existing first.

**Independent Test**: Create a worktree from a repository with a submodule pointing at an unreachable remote (or simulate a network failure during fetch), and verify the user receives a clear, specific error identifying the failed submodule and reason, and that the worktree/branch end up in the defined resulting state rather than an ambiguous half-fetched one.

**Acceptance Scenarios**:

1. **Given** a submodule's remote is unreachable, **When** the user creates a worktree, **Then** the user is shown which submodule failed and why (e.g., network error, authentication required, unreachable commit).
2. **Given** some submodules fetch successfully and others fail in the same worktree creation, **When** the operation concludes, **Then** the outcome for the worktree as a whole follows the same defined failure behavior as a total failure (not a silently mixed state).
3. **Given** submodule fetch has failed and the defined failure behavior has completed, **When** the user inspects the worktree list, **Then** its state matches exactly what the failure behavior specifies — no orphaned partial worktree, branch, or directory outside that definition.

---

### Edge Cases

- A repository has a `.gitmodules` file but zero submodule entries are currently active (e.g., all commented out or removed) — worktree creation proceeds with no submodule work, same as a repository without submodules.
- A submodule uses a relative URL (resolved against the parent repository's configured remote) — it must resolve correctly for a newly created worktree, not just the original checkout.
- A submodule is defined but its target commit no longer exists on the remote (upstream history was rewritten) — this is treated as a fetch failure for that submodule.
- The user closes or navigates away from the worktree creation UI while submodule fetching is still in progress.
- A very large submodule causes fetching to take minutes rather than seconds — the in-progress indication must still read as "working," not "stuck."
- The repository has many submodules (dozens) — the user can tell overall progress is being made, not just that "something" is happening.
- A submodule remote requires authentication the user's environment isn't configured for (no SSH key / credential helper entry) — this is treated as a fetch failure with a reason the user can act on (add credentials) rather than a generic error.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST detect whether the repository a worktree is being created from declares any git submodules (i.e., has an active `.gitmodules` entry).
- **FR-002**: When the repository declares submodules, the system MUST automatically initialize and fetch all of them, including submodules nested inside other submodules, as part of worktree creation, without any separate user action.
- **FR-003**: When the repository declares no submodules, the system MUST create the worktree exactly as today, with no additional step, delay, network activity, or UI change.
- **FR-004**: While submodule content is being fetched, the system MUST visibly indicate to the user that fetching is in progress, distinguishing this state from the worktree creation form being unresponsive.
- **FR-005**: When submodule fetch fails for one or more submodules (network failure, authentication failure, or an unreachable recorded commit), the system MUST roll back the entire worktree creation to a clean pre-creation state — worktree, branch, and directory all removed — consistent with the existing rollback flow used for other worktree-add failures. No worktree is left behind in an incomplete or partially-fetched state.
- **FR-006**: When submodule fetch fails, the system MUST report which specific submodule(s) failed and a reason category (network error, authentication/credentials required, unreachable commit) rather than a generic failure message.
- **FR-007**: System MUST authenticate to submodule remotes using the user's existing git/OS-level credential configuration (e.g., SSH agent, credential helper); it does not present its own credential entry UI.
- **FR-008**: System MUST correctly populate a new worktree's submodules regardless of whether those submodules are already initialized, partially initialized, or untouched in the parent repository's own working copy.
- **FR-009**: Automatic submodule fetch on worktree creation MUST behave consistently on macOS, Linux, and Windows.
- **FR-010**: For repositories without submodules, all existing worktree creation behavior — including pre-flight duplicate checks and the existing rollback flow for other failures — MUST remain unchanged.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of worktrees created from a repository with submodules have every submodule (including nested ones) fully checked out immediately after creation succeeds, with no manual follow-up command from the user.
- **SC-002**: Users can tell that submodule fetching is underway within 1 second of it starting, for every worktree creation involving submodules.
- **SC-003**: When submodule fetch fails, the user can identify which submodule failed and why directly from what's shown, without inspecting logs or running diagnostic commands, in 100% of failure cases.
- **SC-004**: Worktree creation time and behavior for repositories without submodules shows no observable change from the current experience.
- **SC-005**: Worktree creation outcomes (success and failure) for submodule repositories are identical in behavior and messaging across macOS, Linux, and Windows.

## Assumptions

- Submodule authentication reuses the user's existing git/OS-level credential configuration (SSH agent, stored credential helper); this feature does not add an in-app credential entry flow.
- Submodule fetching runs as a synchronous part of the create flow — the worktree is not presented to the user as ready until submodule fetching has concluded (succeeded, or reached the defined failure outcome).
- Re-syncing submodules for a worktree that already exists (e.g., after `.gitmodules` changes upstream and the user pulls) remains a manual action outside this feature's scope; this feature only covers the moment of worktree creation.
- A git version capable of submodule operations is available in the environment the application runs in; no bundled or alternate submodule implementation is introduced.
- Relative submodule URLs resolve the same way for a linked worktree as for the main working tree, since git resolves them against the parent repository's configured remote rather than the local checkout path.
