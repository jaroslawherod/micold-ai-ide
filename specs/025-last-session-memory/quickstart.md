# Quickstart: Reopen on the session I was last using

Two parts. **§A** is what the machine checks. **§B** is the part that needs the application actually
stopped and started again — no test in this repository restarts the process, so the one claim the
whole feature rests on is only provable by quitting and reopening.

---

## §A — The automated suite

```bash
mise run test        # whole workspace, matching CI
mise run test-core   # store + workspace only; faster while iterating on persistence
```

Green is the gate. What each gate is watching, for this feature:

| Gate | Watching |
|---|---|
| `micold-core/tests/store_roundtrip.rs` | `last_session` survives save → load, and a project with no memory round-trips as `None` (§1.3) |
| `micold-core/tests/store_roundtrip.rs` | **backward compatibility**: a per-project file written *without* the field loads as `None` rather than failing — the claim that lets this ship without a `schema_version` bump (research R7) |
| `micold-core/tests/store_fault_isolation.rs` | an unreadable or malformed per-project file still yields a usable workspace with no memory, and never an error a launch could trip on (§3.7, FR-010) |
| `micold-core/tests/workspace.rs` | `foreground_by_project` keys by canonicalised path, the same as `sessions`, so the two cannot be looked up differently |
| `micold-client/tests/features_session.rs` | the resolution is unchanged by the move — the four `ForegroundChoice` cases still hold when the map is read from `Workspace` (§3.2) |
| `micold-client/tests/features_session.rs` | a memory naming a **closed** session is not restored, and one naming an **absent** session resolves to the existing fallbacks (§3.2, FR-005) |
| `micold-client/tests/app_state.rs` | restoring starts nothing: session lifecycles after applying a memory are exactly what they were before (§3.3, SC-005) |
| `micold-client/tests/app_state.rs` | applying a memory leaves other locations' open/closed state alone (§3.6, FR-006) |
| `micold-client/tests/switch_active.rs` | switching still works after the move, and still records into the new home (FR-008) |
| `micold-client/tests/switcher_forget_menu.rs` | forgetting a project discards its memory (§2.5, FR-009) |
| `micold-core/tests/schema_hash.rs` | **unchanged hash** — this feature adds no protocol message and edits none. If this moves, something reached for the wire that did not need to (research R3) |

**What §A cannot tell you**: whether any of this survives an actual restart. Every test above runs in
one process. That is §B, and it is not a formality here — it is the feature.

---

## §B — The manual pass

```bash
mise run run     # the application; it spawns/attaches the daemon itself
```

Prerequisites: two projects registered, each with at least one session that has a real conversation
(an empty session is pruned at boot and will not be remembered — see B5).

### B1 — Reopening lands where you left (US1, SC-001)

1. Open project A, select a session, and leave it in front of you.
2. Quit the application completely.
3. Start it again.

You are on that session, with **zero clicks**. Its location is open in the side panel and its row is
marked, exactly as when you switch to it.

Watch for a frame that shows the project overview first and then jumps — the memory is read from the
application's own load, so the very first frame should already be right. A jump means the value is
arriving from the daemon instead (research R3).

### B2 — It did not start anything (FR-004, SC-005)

Immediately after B1, before touching anything: the restored session shows its previous output and
its state, and its process is **not** running unless it already was. Nothing was resumed on your
behalf.

Confirm from outside as well — the number of `claude` processes should be what it was before the
restart.

### B3 — It did not take the keyboard (FR-013)

Immediately after B1, type something.

It must **not** go into the session's terminal. This is the one place the launch deliberately
differs from a project switch, which does take focus — so check it here specifically, and check the
switch still *does* focus (open another project and switch back; typing then reaches the terminal).

### B4 — Per project, and after switching (US2, SC-003)

1. Leave project A on one session and project B on a different one. Quit.
2. Restart, then switch to the other project.

Each lands on its own remembered session. Then switch back and forth a few times, quit, and restart:
each project still remembers the session you last had **in that project**, not the one you were on
when you quit.

### B5 — When the session is gone (US3, FR-005, FR-006)

Each of these, from a clean restart:

1. **Closed session**: close the session you were on, quit, restart. You land on the project
   overview or another session — never on the closed one, which is not listed at all.
2. **Deleted worktree**: delete the worktree holding the remembered session outside the app, quit,
   restart. Nothing is restored for that project, and the project's other rows are untouched.
3. **Empty session**: start a session, type nothing, quit, restart. It was pruned at boot, so it is
   not restored — and nothing else breaks.

### B6 — Nothing else moved

Open a project you have never used a session in: no session is current, the overview is shown, and
nothing about it differs from before this feature (SC-006).

---

## Recording the pass

§B is evidence, recorded the way features 006, 010, 020, 021, 022 and 024 recorded theirs: the date,
the platform, and any step that did not behave as written. A step that fails is a defect, not a note.

**On honesty about what was checked.** B1 is the feature; B2 and B3 are the two ways it could be
right and still wrong. None of the three can be automated, because all three need the process to
stop and start. A green §A is not this feature working. If §B was not run, say so here rather than
leaving the table blank and implying it was.

| Recorded | |
|---|---|
| Date | |
| Platform | |
| B1 — reopen lands on the session, first frame | |
| B2 — nothing started | |
| B3 — no keyboard focus at launch; switch still focuses | |
| B4 — per project, across several switches | |
| B5 — closed / deleted worktree / empty session | |
| B6 — a project with no memory is unchanged | |
