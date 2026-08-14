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

**Stop the daemon too, not just the client.** The daemon outlives the client by design and owns the
sessions, so restarting only the client leaves the remembered session's process running from before
— "nothing was started" is then true but vacuous, because there was nothing to start. The step is
only meaningful from a genuinely idle start.

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
   restart. That session **is** restored, shown the way any session in a missing worktree is shown
   — an error-tinted location row carrying a `missing` tag. The project's other rows are untouched.

   *(This step formerly asked for the opposite. See the note at the end.)*
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
leaving the table blank and implying it was — and if a step was attempted but does not prove what it
claims to, that is not run either. B2 sat in that state for two days before being redone properly.

| Recorded | |
|---|---|
| Date | 2026-08-11 (B1), 2026-08-12 (B3, B7), 2026-08-14 (B2, B4, B5, B6) |
| Platform | B1 on the user's own install; everything else on Xvfb + lavapipe in an isolated sandbox (see below) |
| B1 — reopen lands on the session, first frame | **PASS** — confirmed by the user on their own install ("it works"), and reproduced in the sandbox on every restart below |
| B2 — nothing started | **PASS** — [evidence](./evidence/B2-restore-starts-nothing.png) |
| B3 — the restored terminal is ready to type; a switch still focuses | **PASS** — [evidence](./evidence/B3-focus-states.png) |
| B4 — per project, across several switches | **PASS** — [evidence](./evidence/B4-per-project-memory.png) |
| B5 — closed / deleted worktree / empty session | **PASS**, all three — [evidence](./evidence/B5-B6-kept-and-declined.png). B5.2 passes against the *corrected* expectation; the step as previously written would have failed |
| B6 — closing a session does not erase the memory | **PASS**, both halves — [evidence](./evidence/B5-B6-kept-and-declined.png) |
| B7 — a project with no memory is unchanged | **PASS** |

Every step in §B has now been run. Two defects were found along the way, neither of them a failure
of a §B step — see **Found while running this** below.

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

### How B2, B4, B5 and B6 were run

Same sandbox as above, on `:79`, with `CLAUDE_CONFIG_DIR` pointed into it as well — the provider's
transcript store decides which sessions survive `prune_empty_sessions`, so it has to be isolated
too, and setting it explicitly is clearer than relying on `HOME`. Two throwaway git repos (`alpha`,
`beta`) stood in for projects.

**Sessions were seeded, the behaviour was not.** Each scenario needs a project in a particular shape
(two live sessions, one closed, one in a worktree), and creating those by hand costs a lot of
clicking. They were written straight into `projects.json` using the **pre-split** `StoredProject.
sessions` shape, which `JsonFileStore::load` still honours as the BUG-001 migration fallback for any
project with no per-project state file. That is a real load path with its own tests, so the fixture
goes in through the application's own reader rather than a format invented for the sandbox.

What was deliberately **not** seeded is `last_session` itself. It exists only in the per-project
state file, and it is the value under test — every memory asserted below was written by the
application, in response to a session being selected in the UI.

Assertions are against **the file on disk** (`projects/<hash>.json`) as well as the screen, because
the memory is a durable claim and a screenshot cannot show whether it survived the process.

**B2 is the one that changed.** The earlier attempt did not count, because only the client was
restarted and the daemon outlived it, so the session's `claude` process was still running from
before — nothing was started by the restore, but there was nothing to start. This run stopped the
client **and** the daemon (0 `micold` and 0 `claude` processes in the sandbox, verified), then
launched. After the restore: still **0**, and still 0 twelve seconds later.

That alone would only show the restore is inert. The evidence image pairs it with the contrast that
gives it meaning: clicking a session row in the sidebar **does** resume it — one `claude --resume`
appears within seconds, and the terminal fills with real output. Same session state, two routes,
opposite outcomes. The restore path is inert *by comparison*, not by assertion.

**B4** used both projects. Alpha was left on `A1` and beta on `B1`; the client and daemon were both
stopped, so the memory could only come from disk. The restart landed on **alpha A1** — not `beta B1`,
which was the session actually in front of the user at the moment of quitting. That is the
discriminating half of the step: a per-project memory and a "last session anywhere" memory would
agree on every simpler scenario and disagree only here.

**B5** ran all three cases, each from a clean restart:

- *closed session* — the row is not listed at all, and the project overview is shown. The
  application did not quietly pick the surviving session instead (FR-007).
- *empty session* — started, never typed into, pruned at boot. The memory named it, the restore
  declined it, the overview was shown, and the other session was untouched (FR-006).
- *deleted worktree* — the session **is** restored, under a location row tinted as an error and
  tagged `missing`. See the correction note at the end: this step used to ask for the opposite.

**B6** ran both halves. Closing the session you are *on* leaves the memory pointing at it — verified
on disk, `last_session` unchanged after the close — and the next launch shows the overview, because
the restore declines a closed session. Closing the *other* session still lands you on yours. Those
two together are the whole of FR-005a: the memory is **kept** on disk and **declined** at resolve
time, which is why a stale memory costs nothing and an erased one would cost the user their place.

### Found while running this

Neither is a §B step failing. Both are recorded rather than fixed here, because both are outside
what this feature changed.

**1. "Starting…" is the wrong thing to say about a restored session.** The terminal body renders a
`Starting…` placeholder whenever it has no grid yet (`ui/terminal.rs`, the `grid: None` branch). A
restored session has no grid and is not running, so the placeholder sits there indefinitely while
the status bar one row below says `interrupted` and offers `restart`, and no process exists. The
two contradict each other, and the placeholder is the one that is wrong.

This is not a new branch — but before this feature no session was current at launch, and the other
way to reach a current-but-not-running session (clicking its row) *does* start it, which makes the
placeholder briefly honest. So this feature is what makes a misleading first screen the normal case
for anyone who quits on an idle session. Visible in the left half of the B2 evidence image.

Filed as [BUG-001](./bugs/BUG-001.md).

**2. `last_active` no longer follows the user's project switches.** Switching projects sends no
message that tells the daemon which project is active — there is no such message in the protocol —
so the daemon's `workspace.active` stays at whatever it loaded, and every save rewrites `last_active`
from that stale value. The effect is that *which project opens at launch* stops tracking what the
user did. Observed directly: after switching to beta and working there, `last_active` was still
alpha across many daemon writes, so it is not an artefact of how the sandbox was shut down.

Out of scope for this feature — contract §5 explicitly excludes which project opens — and this
feature's own memory is per project and was correct throughout. Recorded because it shapes what B1
and B4 look like, and because it is likely a consequence of the daemon split (feature 021 T052)
rather than anything intended.

### An edge the spec does not cover

The two ways to delete a worktree do not present the same way, and only one of them is written down:

- `rm -rf` the directory leaves git's record, so the worktree is still discovered and shown as
  `missing`. The restored session appears under it. This is the case B5.2 now describes.
- `git worktree remove` deletes git's record too, so the worktree is not discovered at all. The
  session is still restored and still named in the status bar — but it has **no row anywhere in the
  sidebar**, so FR-012's "its location is revealed in the side panel" has no location to reveal.

Not a defect against any stated requirement, and arguably the least-bad outcome. Noted because a
current session with no visible row is a state nothing in the spec anticipated.

### Notes for a future run

The daemon socket path is bounded by `sun_path` (108 chars), so an isolated `XDG_RUNTIME_DIR` must
be short — the application reports this clearly rather than failing obscurely. A session's record is
persisted *before* any `claude` spawn, so sessions can be created in a sandbox even where the AI CLI
is unavailable. A seeded `projects.json` that fails to deserialize is moved aside to
`projects.json.bak` and the app recovers to empty — so a sandbox that opens on "No project open" is
usually a malformed fixture, not a broken build (`mode` must be `AiCli` or `Regular`; `Default` is
not a variant). And a worktree session's cwd is `<repo>/.claude/worktrees/<dir>`, which is where its
transcript has to be for `prune_empty_sessions` to spare it.

What §A establishes is narrower than it looks but is not nothing — the memory round-trips through
the real store including the backward-compatible read, the daemon writes it and declines to write
when nothing changed, a no-session report leaves it alone, applying it starts no process, forgetting
a project discards it, and a corrupt project file yields no memory rather than an error. What no
test here touches is the actual sequence — quit, start, land — because every test in this repository
runs in a single process. That sequence is now covered by §B rather than by argument.

### Two steps changed during implementation

Both asked for the opposite of what the feature now does, and both would now fail as originally
written. Recorded here because a reviewer holding the original quickstart would otherwise read a
passing implementation as a defect — twice.

**B3** asked that the restored session's terminal *not* take the keyboard. Feature 023 has since
made focus derived from a session being displayed, so the restored terminal is ready to type —
which is what the step now checks, in both directions.

**B5.2** asked that a session whose worktree was deleted *not* be restored. The spec was reversed
during implementation (clarification Q4) for two reasons: the application already lists such a
session and lets the user select it, so declining to *return* them to it would repeat the
inconsistency feature 008's BUG-001 was about; and declining would need the project's worktree list
at resolve time, which a project switch discovers asynchronously and does not have — so the same
rule would break switching in order to decline a case the user can see for themselves.

The step's text was stale until this run: the spec changed, the quickstart did not. It was caught
only because B5.2 was actually executed, which is the argument for running §B rather than reasoning
about it.
