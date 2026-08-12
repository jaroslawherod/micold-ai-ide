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
| daemon tests (`micold-daemon/`) | a report of **no session** leaves the memory untouched (§2.6, FR-005a) — the clause that stops closing a session from silently costing the user their place |
| daemon tests (`micold-daemon/`) | a report naming the session already remembered writes nothing; one naming a different session writes (§2.3, FR-001a) |
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

### B3 — The restored terminal is ready to type (FR-013)

Immediately after B1, type something.

It goes into the restored session's terminal. Focus follows the session being displayed (feature
023), so reopening on a session means you can type into it — the same as arriving there by any other
route. Then open another project and switch back: typing still reaches the terminal, which is the
other half of the same rule.

*(This step formerly asked for the opposite. See the note at the end.)*

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

### B6 — Closing does not cost you the memory (FR-005a)

1. In project A, select session X. Then select session Y. Then **close** Y.
2. Quit and restart.

You land on Y's absence — the overview — because Y was closed and cannot be restored. Now instead:

1. In project A, select X, then select Y, then close **X** (not the one you are on).
2. Quit and restart.

You land on Y. Closing another session did not disturb the memory, and neither did the pointer
moving around while you worked.

### B7 — Nothing else moved

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
| Date | 2026-08-11 (B1), 2026-08-12 (B3, B7) |
| Platform | B1 on the user's own install; B3 and B7 on Xvfb + lavapipe in an isolated sandbox (see below) |
| B1 — reopen lands on the session, first frame | **PASS** — confirmed by the user on their own install ("it works") |
| B2 — nothing started | **NOT RUN** — attempted; see why it does not count below |
| B3 — the restored terminal is ready to type; a switch still focuses | **PASS** — [evidence](./evidence/B3-focus-states.png) |
| B4 — per project, across several switches | **NOT RUN** |
| B5 — closed / deleted worktree / empty session | **NOT RUN** |
| B6 — closing a session does not erase the memory | **NOT RUN** |
| B7 — a project with no memory is unchanged | **PASS** |

### How B3 and B7 were run

Not by hand at a desk, and not on the user's install. The client was launched on a private X server
(`Xvfb :78`, lavapipe software Vulkan) against a **fully isolated world**: its own `XDG_DATA_HOME`
(so a separate `projects.json`), its own `XDG_RUNTIME_DIR` (so a separate daemon socket — the client
spawned its own daemon and could not reach the user's), and its own `HOME` (so a separate
`~/.claude`). Two throwaway git repos stood in for projects. The user's running application and
daemon were never touched, and everything started was stopped afterwards by PID.

**B3 is the one that needed thought.** A restored session is not running, so nothing echoes what you
type — "ready to type" has no observable in the terminal itself. The observable is the terminal
bar's **release-focus control**, which is always present but enabled only while the terminal holds
the keyboard (feature 023, FR-008a). The evidence image stacks three readings of that control:

1. **green** — immediately after a restart that restored the session: **enabled**. The restored
   terminal holds the keyboard.
2. **orange** — after clicking it to release focus: **disabled**. This is what makes reading 1 mean
   something; without it, "bright icon" is an assumption rather than a comparison.
3. **blue** — after switching away to the other project and back: **enabled** again. A switch still
   focuses (the second half of B3), and because this run *started* from the released state of
   reading 2, the re-focus is unambiguously the switch's doing rather than something inherited.

One false alarm worth recording: reading 3 first came out disabled, which looked like a failure to
re-focus. The project switcher menu was still open, and an open overlay takes the keyboard by
design. Closing it gave the reading above. A coordinate means something different with an overlay
open — the same trap the `visual-pass` skill warns about.

### What was run, and what was not

**B1 passed on the user's own install**, reported after the feature merged: they restarted and
landed on the session they had been using. That is the claim the whole feature rests on, and it is
the one no test in this repository can make — every test here runs in a single process.

**B2 was attempted and does not count.** The client was restarted, but the **daemon outlived it** —
it is a separate process that owns sessions by design — so the session never stopped and its
`claude` process was still running from before. Nothing was started *by the restore*, but that is
not what the frame proves, because there was nothing to start. A real B2 needs the daemon stopped
too, so the session is genuinely idle at launch. Recorded as not run rather than passed.

**B4, B5 and B6 remain unrun.** Each needs the application quit and started again under
particular conditions (two projects on different sessions, a closed session, a deleted worktree),
and each needs the sandbox seeded into a particular shape first. They are not blocked on anything —
the sandbox recipe above makes them cheap to run — they simply have not been exercised.

Two findings that make a future run cheaper. The daemon socket path is bounded by `sun_path`
(108 chars), so the isolated `XDG_RUNTIME_DIR` must be short — the application reports this clearly
rather than failing obscurely. And a session's record is persisted *before* any `claude` spawn, so
sessions can be created in a sandbox even where the AI CLI is unavailable.

B1 passing does not carry the rest: **a green §A plus one passing step is not the whole of §B.**
What §A establishes is narrower than it looks but is not nothing — the memory
round-trips through the real store including the backward-compatible read, the daemon writes it and
declines to write when nothing changed, a no-session report leaves it alone, applying it starts no
process, forgetting a project discards it, and a corrupt project file yields no memory rather than
an error. What no test touches is the actual sequence: quit, start, land.

**B3 changed during implementation** and its old form would now fail. It asked that the restored
session's terminal *not* take the keyboard; feature 023 has since made focus derived from a session
being displayed, so the restored terminal is ready to type — which is what the step now checks, in
both directions. Recorded here because a reviewer holding the original quickstart would otherwise
read a passing implementation as a defect.
