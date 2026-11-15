# Research: 1945

## R1. Where the implicit parent lives
**Decision**: derive it at read time in `micold-core::worktree_parent` from the live names and the
views; record nothing.
**Rejected**: (a) writing a `default` link for every unparented worktree at discovery: contradicts
the clarification (no link recorded), migrates storage, and listings would start showing `parent`
(1857 FR-008). (b) a special case inside `Scope::new` only: the new tool would need a second copy of
the rule.

## R2. Tool names
**Decision**: `neighbourhood` and `send_parent_input`. Verb-less read tools here are `whoami`,
`list_*`; `send_*_input` mirrors `send_session_input`.
**Rejected**: `whoami` growing the neighbourhood: it is unrestricted today and its tests and the
"one cheap identity call" use stay stable; `send_session_input` with a `"parent"` ref: the `session`
argument is a typed `SessionRef` (UUID) validated by the parser.

## R3. Which parent session is the target
**Decision**: the first `Running` session of the parent worktree in `project.sessions`, i.e. catalog
append order (creation, attach, adoption; `claim_main_if_free` relies on the same order), ties by the
lowest session id. `Session` has no creation timestamp, and adding one would need a storage
migration for no gain.
**Rejected**: the `main` session flag (a session can be main yet stopped); most recently active
(non-deterministic across calls, violates scenario 3.2); lowest id (UUID v4, random, so unrelated to
age).
"Live" excludes `Idle`, `Failed`, `Restarting` and daemon-restart states; `Starting` is excluded too
because it cannot take input yet.

## R4. How the parent message is delivered
**Decision**: resolve to a session id and call the existing `send_session_input`.
**Rejected**: a second delivery path (duplicates policy, trust and atomicity; spec Assumptions
require reuse).

## R5. When to record the parent
**Decision**: inside `create_worktree_with_session`, after the worktree create and before the session
create. The window before the first prompt cannot be left open by callers.
**Rejected**: passing a callback (more surface for one call site); recording before the worktree
exists (a failed create would leave a link, breaking FR-009's second sentence and 1857).

## R6. Existing 1857 expectations
The visibility fixture treats `P` and `X` (unparented) as unrelated. Under FR-001 they are siblings
with `default` as parent. Those assertions are amended, not deleted: the spec says it amends 1857
FR-003 and FR-015.
