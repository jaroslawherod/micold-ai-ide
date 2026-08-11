# Contract: the last-session memory

Feature 025. What is remembered, who may write it, and what a launch does with it. Every clause is
stated so it can be asserted against the store, the daemon's catalog, or the render-free
resolution — not against pixels.

## §1 What is remembered

**§1.1** For each project, at most one session id: the one most recently made current for that
project. *(FR-001, FR-008)*

**§1.2** The memory is per project and independent. Making a session current in one project MUST
NOT change any other project's memory. *(FR-008)*

**§1.3** It is stored in that project's own state file, beside its sessions, as an optional field.
Its absence means "no memory" and is indistinguishable from a project that has never had one.
*(FR-010)*

**§1.4** It is a **hint, not a promise**: it may name a session that is gone, closed, or in a
worktree that no longer exists. Nothing validates it at load. *(Invariant I1)*

## §2 Who writes it

**§2.1** The **daemon** writes it, on `ClientMsg::SetViewedSession`, which the client already sends
whenever the session in front of the user changes. No new message; no message changes shape.

**§2.2** The **client never persists it.** It reads the memory at load and keeps it current in
memory for the run. `store.rs` has no locking, so a client-side write would clobber whatever the
daemon had written since the client loaded — the hazard `main.rs` already records for
`projects.json`. *(Invariant I2)*

**§2.3** It is written **whenever it changes value, and only then** — not at exit, and not on a
report that names the session already remembered. Attach re-sends the current id and a session start
names a session that may already be in front of the user; neither is a change, and neither writes.
A force-kill therefore costs at most the single most recent change rather than the whole memory.
*(FR-001a, SC-007)*

**§2.4** With two windows open, both report through the same daemon, which serialises the writes.
The last write wins. Neither window is blocked by the other, and the file is never left partially
written. *(Edge Cases)*

**§2.5** Forgetting a project discards its memory, because the memory lives in the per-project state
file that forgetting already deletes. *(FR-009)*

**§2.6** A report of **no session** MUST NOT clear the memory. The pointer goes to nothing for
reasons the user did not take — closing a session, an internal cleanup after a reconcile — and
erasing the memory on those would silently cost them the place they would have returned to. The
memory is replaced only by another session becoming current in that project. A memory naming a
session that can no longer be restored is harmless, because §3.2 declines it. *(FR-005a)*

## §3 What a launch does with it

**§3.1** After loading, if a project is active, the application resolves that project's memory and
makes the result the current session. *(FR-002)*

**§3.2** The resolution is `explain_foreground` — the same one a project switch uses. A remembered
session is restored **whether or not its process is running**, provided it is present and not
`archived`; otherwise the existing fallbacks apply (first running session, then none).
*(FR-003, FR-005, feature 008 FR-003a)*

**§3.3** Restoring **starts nothing**. No process is spawned, resumed, or signalled. The number of
running sessions immediately after launch is what it would have been without this feature.
*(FR-004, SC-005, Invariant I4)*

**§3.4** The restored session's terminal **is** ready to type in, exactly as one reached by any
other navigation is. Focus is derived from a session being displayed and the user not having given
the keyboard away (feature 023); withholding it here would need a writer of that flag which no
navigation has, and would make the launch the one special case in a model built to remove them.
*(FR-013, research R5)*

**§3.5** When nothing can be restored, no session is current and the project overview is shown. The
application MUST NOT choose a session on the user's behalf. *(FR-007)*

**§3.6** A memory that cannot be honoured leaves the rest of the project untouched — its other
sessions, its locations, and their open/closed state. *(FR-006)*

**§3.7** An unreadable, missing or outdated stored memory is treated as no memory. A launch MUST NOT
fail, warn, or block on it. *(FR-010)*

## §4 How the restored session is presented

**§4.1** It is presented exactly as a session made current by any other means. In particular its
location is revealed in the side panel, because that follows from the session becoming current
(feature 024) and not from anything this feature does. *(FR-012)*

**§4.2** A user MUST NOT be able to tell from the panel how the session became current. There is no
"restored" styling, badge or notice.

## §5 What this contract does not cover

- **Which project opens at launch.** Existing behaviour, decided by the stored last-active project.
  This feature only decides which session is in front of the user once a project has opened.
  *(spec Assumptions)*
- **How a session becomes current during a run.** Unchanged — the switch path and the sidebar own
  that, and this feature persists their outcome rather than influencing it.
- **Session run state across restarts.** Not persisted, and this feature does not change that. It is
  the reason the common case at launch is restoring a session that is not running. *(spec
  Assumptions)*
