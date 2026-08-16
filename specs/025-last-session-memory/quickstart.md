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
| `micold-client/tests/app_state.rs` | restoring starts only what it displays: the reducer moves no lifecycle itself (the resume is a `SessionStart` to the daemon), and one project's restore leaves every other project's sessions and memory untouched (§3.3b, SC-005a, BUG-002) |
| `micold-client/src/main.rs` (tests) | connecting **starts** the restored session rather than only viewing it, starts exactly one, and starts nothing when nothing is remembered (§3.3a, FR-004a, SC-005a, BUG-002) |
| `micold-client/src/shell/daemon_sync.rs` (tests) | a project switch does the same, and orders the start between the attach and the view (§3.3a, BUG-002) |
| `micold-client/tests/app_state.rs` | applying a memory leaves other locations' open/closed state alone (§3.6, FR-006) |
| `micold-client/tests/switch_active.rs` | switching still works after the move, and still records into the new home (FR-008) |
| `micold-client/tests/switcher_forget_menu.rs` | forgetting a project discards its memory (§2.5, FR-009) |
| daemon tests (`micold-daemon/`) | a report of **no session** leaves the memory untouched (§2.6, FR-005a) — the clause that stops closing a session from silently costing the user their place |
| daemon tests (`micold-daemon/`) | a report naming the session already remembered writes nothing; one naming a different session writes (§2.3, FR-001a) |
| `micold-core/tests/schema_hash.rs` | **unchanged hash** — this feature adds no protocol message and edits none. If this moves, something reached for the wire that did not need to (research R3) |
| `micold-client/src/ui/terminal.rs` (unit tests) | a session that is **not running** is not described as starting, and one that is still says so — the empty state and the `restart` control derive from one predicate, so they cannot disagree (FR-014, §4.3, BUG-001) |
| `micold-client/tests/terminal_empty_state.rs` | the pane still *asks*. The unit tests above drive the decision function directly, so reverting the pane to a hardcoded string leaves them green; this reads the source and fails (BUG-001) |

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

### B2 — It came back up, and only it did (FR-004a, SC-005a)

> **Reversed by [BUG-002](./bugs/BUG-002.md).** This step used to read "It did not start anything"
> and asked you to confirm the process was **not** running — FR-004, now superseded. Restoring
> resumes the session, as selecting it by hand always has.

Immediately after B1, before touching anything: the restored session's terminal is **live**. It came
back up on its own, and you can work in it without pressing anything.

**Stop the daemon too, not just the client.** The daemon outlives the client by design and owns the
sessions, so restarting only the client leaves the remembered session's process running from before
— the resume is then vacuous, because there was nothing to start. The step is only meaningful from a
genuinely idle start.

Then check the bound, which is the half that still matters. From outside:

```bash
pgrep -c claude     # before quitting, and again after reopening
```

Exactly **one** more than before — the session you were on. With two projects each remembering a
different session, only the project that actually opened has resumed anything; the other project's
session is still stopped until you switch to it.

**The FR-014 case is now the exception, and still worth reaching** ([BUG-001](./bugs/BUG-001.md)).
Make the resume **fail**, then reopen: the terminal must say the session is not running and point at
the `restart` control, never claim to be starting. The ordinary path no longer reaches that screen.

The route that works is to take `claude` off `PATH` before reopening — the daemon is asked to start
the session, cannot spawn the process, and leaves it exactly as FR-014 describes. A second window
holding the project does **not** work: the daemon keeps streaming, so the pane is never empty. A
deleted worktree did not work either when this was recorded, for the same reason — but that was
[`010` BUG-012](../010-daemon-session-persistence/bugs/BUG-012.md), fixed the same day, and since
`010`'s FR-006c refuses a start whose directory is gone that route now fails the spawn too. The
`PATH` route is still the one to use: it is the one whose pane was watched. See the recorded runs
below.

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
| Date | 2026-08-11 (B1), 2026-08-12 (B3, B7), 2026-08-14 (B2, B4, B5, B6; then **all of §B re-run** after BUG-001 was fixed; then **B2 re-run again** after BUG-002 reversed it); 2026-08-16 (B2's FR-014 half, reached at last) |
| Platform | B1 on the user's own install; everything else on Xvfb + lavapipe in an isolated sandbox (see below) |
| B1 — reopen lands on the session, first frame | **PASS** — confirmed by the user on their own install ("it works"), and reproduced in the sandbox on every restart below |
| B2 — it came back up, and only it did | **PASS** on the resume half (2026-08-14, after BUG-002) — [evidence](./evidence/B2-postbug002-restore-resumes.png). From a genuinely idle sandbox (0 micold, 0 `claude`, verified), launching produced **exactly one** `claude`, and its command line is `claude --resume ede92724-a79a-444c-84f9-a6c67c91b08e` — the id `last_session` held on disk. Zero clicks. The terminal is live and rendering real output, not a placeholder. **Reproduced by a second, independent run** the same day — [evidence](./evidence/B2-resumes-exactly-one.png) — which also held the *"and only it did"* clause across two projects: beta's remembered session stayed stopped. **The FR-014 half is now PASS too** (2026-08-16) — [evidence](./evidence/B2-fr014-refused-start.png). Neither of the two runs above reached it; between them three routes were tried and none produces a refused resume. A fourth does: with `claude` off `PATH`, the daemon logs `session start failed … No viable candidates found in PATH`, and the pane reads "This session is not running. Choose restart below to resume it." with `restart` beside it. The earlier PASS — [superseded evidence](./evidence/B2-restore-starts-nothing.png) — recorded 0 `claude` after a restore, which was correct for the build it was taken on and is what BUG-002 reversed |
| B3 — the restored terminal is ready to type; a switch still focuses | **PASS** — [evidence](./evidence/B3-focus-states.png) |
| B4 — per project, across several switches | **PASS** — [evidence](./evidence/B4-per-project-memory.png) |
| B5 — closed / deleted worktree / empty session | **PASS**, all three — [evidence](./evidence/B5-B6-kept-and-declined.png). B5.2 passes against the *corrected* expectation; the step as previously written would have failed |
| B6 — closing a session does not erase the memory | **PASS**, both halves — [evidence](./evidence/B5-B6-kept-and-declined.png) |
| B7 — a project with no memory is unchanged | **PASS** |

### B2, re-run after BUG-002 (2026-08-14)

Same method as the runs below — Xvfb `:79` + lavapipe, an isolated sandbox with its own
`XDG_DATA_HOME`, `XDG_RUNTIME_DIR`, `CLAUDE_CONFIG_DIR` and `MICOLD_DAEMON_BIN`, and a throwaway git
repo as the project. What is different, and worth copying next time:

- **Both binaries were pinned to a private directory before the run.** `target-shared/` is shared
  across worktrees, and midway through this pass another worktree rebuilt `micold-daemon` into it —
  the client then showed a build-mismatch banner because it had spawned a daemon from a different
  branch. Copy `micold-ai-ide` and `micold-daemon` out of the target dir and point
  `MICOLD_DAEMON_BIN` at the copy, or the thing under test can change under you mid-run.
- **`XDG_RUNTIME_DIR` must be short.** The scratchpad path exceeds `sun_path`'s 108 bytes, and the
  client fails to connect with "local socket name length exceeds capacity of sun_path". `/tmp/<short>`
  works.
- **The session was created through the UI; only the *project* was seeded**, and the transcript was
  written by hand at `<CLAUDE_CONFIG_DIR>/projects/<encoded-cwd>/<session-id>.jsonl` so
  `prune_empty_sessions` would keep it. `last_session` was never seeded — the application wrote it,
  and it is what the restore was then measured against.

**Result: PASS.** From 0 micold and 0 `claude`, launching produced exactly one `claude`, whose
command line is `claude --resume <the id in last_session>`, with no clicks. The terminal renders real
output rather than a placeholder. A second window opened onto the same project was refused (read-only
banner) and started **no** additional process, which is the SC-005a bound holding at the one seam
where it could plausibly have been broken.

**Two things this run could not establish, and did not:**

- **A resumed *conversation*.** The sandbox's `CLAUDE_CONFIG_DIR` has no credentials, so `claude`
  comes up at its first-run screen rather than replaying the seeded transcript. What is proven is
  that the process is spawned, with `--resume` and the right id, and that its output reaches the
  pane. That the conversation itself is restored is the provider's behaviour, not this feature's.
- **FR-014 by the intended route.** Holding the project in a second window did not empty that
  window's pane — the daemon was still hosting the session and still streamed it. The FR-014 wording
  *was* seen on screen earlier in this pass, in the mismatched-daemon run, where the resume spawned a
  process that then exited: the body read "This session is not running. Choose restart below to
  resume it." with the bar at `idle` beside it
  ([evidence](./evidence/B2-postbug002-refused-resume-fr014.png)). That is a real instance of the
  state FR-014 exists for, reached by accident rather than by design, and it is weaker evidence than
  a deliberate refusal would be. **Since resolved** — see *FR-014, reached deliberately* below.

**One observation worth following up, not caused by BUG-002.** With the session live and streaming,
the bottom bar continued to read `interrupted` and offer `restart` rather than `running`. It does the
same after clicking the session row by hand, so it is not specific to the restore path and is not a
regression from this change — but it is the bar and the body disagreeing about one session, which is
the shape BUG-001 was about. Now reported as
[`010` BUG-011](../010-daemon-session-persistence/bugs/BUG-011.md): the daemon never records that a
started process is running, so no session reaches `Running` except by crashing and being respawned.

### FR-014, reached deliberately (2026-08-16)

**Result: PASS** — [evidence](./evidence/B2-fr014-refused-start.png). Three routes had been tried
across the two runs above and none produced a refused start. The fourth does: **take `claude` off
`PATH` and reopen.** The daemon is asked to start the remembered session, cannot spawn the process,
and logs it —

```
WARN micold_daemon::server: session start failed session=7f230d6e-… err=Unable to spawn claude
because: No viable candidates found in PATH "/usr/bin:/bin"
```

— leaving the session exactly where FR-014 describes: current, displayed, no process, no grid. The
pane reads *"This session is not running. Choose restart below to resume it."* and the bar beside it
reads `interrupted` with `restart`. No claim to be starting, at either.

**Why this route and not the others.** The three that failed all leave the daemon *hosting* the
session, so frames keep arriving and the pane is never empty — a second window is refused the
attachment but not the stream, and a deleted worktree is started anyway (in `$HOME` — that was
[`010` BUG-012](../010-daemon-session-persistence/bugs/BUG-012.md), fixed the same day, so that route
now fails the spawn and reaches this screen too). The empty pane needs a start that produced no
process at all, which means the spawn itself has to fail. That is not a contrivance: an uninstalled,
unlinked or not-yet-on-`PATH` `claude` is exactly this, and is a thing that happens on a developer
machine after a reboot.

**Run conditions.** Xvfb `:81` + lavapipe, isolated `XDG_*` / `CLAUDE_CONFIG_DIR` /
`MICOLD_DAEMON_BIN`, throwaway git repo, no provider credentials — `claude` was a stub script that
prints its arguments and then `exec cat`, so "the process is alive and its output reaches the pane"
is observable without authenticating anything. `last_session` was written by the application, not
seeded. The same pass confirmed both faces of
[`010` BUG-011](../010-daemon-session-persistence/bugs/BUG-011.md), whose evidence image is the bar
from this state and from the *running* state cropped at identical geometry: they are
indistinguishable.

**One method note, learned the hard way again.** The first launch of this pass refused its own
daemon — `contract or build mismatch` with *matching* version numbers printed on both sides, which
is the schema-hash case. The worktree `fix-can-t-select-existing-branch-and-existing-worktree` sits
on a different protocol commit and builds into the same `target-shared`, so pinning the two binaries
is not enough on its own: pin them, then **verify the pair connects** before believing anything on
screen. The daemon log says `client attached to daemon` when it is sound.

> **[BUG-002](./bugs/BUG-002.md) unsettles this table again, and only partly.** The fix changes what
> a restored session *does* — it now resumes — so B2's recorded pass is of a rule that no longer
> exists, and B1, B4, B5 and B6 all end on a session that is now running rather than idle. Their
> claims still hold (they are about *which* session, and about the memory), but the screenshots show
> an idle terminal where a live one would now appear. B2 has since been re-run against the current
> behaviour and re-recorded above; the rest are left as passes because what they assert did not
> change. Anyone re-running §B should still expect the difference in their screenshots.

Every step in §B has now been run, and then **run again end to end against the build with BUG-001
fixed** — the fix changed what the terminal area says in exactly the state most of these steps land
in, so the earlier evidence above is all pre-fix and no longer shows what the application does. Every
step passed on the second pass too: [evidence](./evidence/B-postfix-pass.png),
[B3](./evidence/B3-focus-postfix.png). See **The second pass** below, including the methodology
hazard that nearly made it report a false failure.

Two defects were found along the way, neither of them a failure of a §B step — see **Found while
running this** below.

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
sessions` shape, which `JsonFileStore::load` still honours as the feature 008 BUG-001 migration fallback for any
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

> **That contrast was the bug, and this pass called it a pass.** [BUG-002](./bugs/BUG-002.md) later
> found FR-003 and FR-004 could not both hold — FR-003 requires the restore to match hand-selection,
> and hand-selection resumes — and resolved it in FR-003's favour. So "restore starts nothing, click
> starts one" is not two correct behaviours; it is one seam missing the start the other has. The
> evidence image above is an accurate record of a defect, presented at the time as proof of
> correctness. Left in place, with this note, because deleting it would hide how the reading went
> wrong: every observation in it is right, and the conclusion drawn from it was not.

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

### The second pass — every step, against the fixed build

Run 2026-08-14 after [BUG-001](./bugs/BUG-001.md) merged, in the same isolated sandbox on `:80`.
Every step of §B, in one sitting, in order. All pass.

| Step | Result on the fixed build |
|---|---|
| B7 | project with no memory → overview, no session current, 0 processes |
| B1 | restart → alpha A2, marked, location expanded, named in the bar |
| B2 | ~~0 `claude` before and after; the terminal now reads *"This session is not running…"* and the bar beside it reads `interrupted` — they agree~~ **Superseded by BUG-002**: this recorded the pre-resume behaviour as a pass. Re-run separately, twice — see *B2, re-run after BUG-002* above and *B2 under FR-004a* below |
| B3 | release-focus control **bright** after the restore, **dim** after clicking it, **bright** again after switching away and back |
| B4 | switch → beta B1; then quit *while beta was in front* and restart → **alpha A2**, alpha's own memory rather than the session last seen |
| B5 | closed → declined, overview, X listed but not chosen; empty → pruned, declined; worktree deleted → W restored under an error-tinted `missing` row |
| B6 | closed the session I was on → overview, memory on disk still naming it; closed the other → still land on mine |

**Why the whole pass rather than B2 alone.** The fix changed the sentence shown for a restored,
not-running session — which is the state B1, B2, B4, B5 and B6 all end on. Re-running only the step
that found the bug would have left every other step's evidence showing a screen the application no
longer draws.

#### The hazard that nearly produced a false failure

Mid-pass, B4 showed `Starting…` again — the fixed build apparently regressing, 38 seconds in, with
0 processes and the bar saying `interrupted`. That is impossible by construction: both derive from
`attached_process_restartable`, and a test asserts they cannot disagree.

They had not. `/proc/<pid>/exe` read `…/micold-ai-ide (deleted)`: **another worktree's build had
replaced the binary on disk while the client was running**, and a later launch in the same pass
picked up *their* build — at `380b570`, which predates the fix. Five worktrees share
`target-shared/` (CLAUDE.md explains why), and several were building throughout.

Two things made this worth an hour rather than a minute. The first is that the symptom is a perfect
imitation of the bug the pass exists to check. The second is that the obvious diagnostic lied:
`cp target-shared/debug/micold-ai-ide` from inside a worktree fails with *No such file or
directory*, because `build-lock.sh` exports `CARGO_TARGET_DIR` to the directory beside the **main**
checkout — so a copy loop that suppressed stderr reported "the fix is not in the binary" five times
in a row while copying nothing at all. `./scripts/build-lock.sh --print-target-dir` resolves it, and
CLAUDE.md says so.

**The fix for the method: pin the binary.** Build, copy client and daemon into the sandbox, *verify
the copy contains the change under test*, and launch from there. Then no concurrent build can swap
the subject of the experiment halfway through. Everything above was run from a pinned, verified
copy, and `readlink /proc/<pid>/exe` was checked once per pass to prove it.

The general lesson is worth more than the specific one: **a visual pass must be able to name the
build it ran.** Without that, every observation is conditional on an assumption the environment is
actively falsifying — and the failure mode is not a crash, it is a plausible wrong answer.

*(Noted in passing: a sibling worktree is named
`fix-after-restart-the-session-is-stuck-at-starting`. Whether that is this bug, or the different one
of a session whose lifecycle really is wrong after a restart, is not something this pass can say.)*

### B2 under FR-004a — a second, independent run

The run recorded above was done separately and found first. This one was run in parallel, without
sight of it, against `2cf6cea` (BUG-001 and BUG-002 both in), in its own sandbox on `:81`, binary
pinned and `readlink /proc/<pid>/exe` confirmed. It is kept rather than folded in because two
observers reached the same verdict by different setups, and because it reaches two bounds the other
run did not: **two projects** rather than one, which is what the step's *"and only it did"* clause
actually asks about, and a **second FR-014 route**. Where the two overlap they agree.

Two projects, alpha remembering `A2`, beta remembering `B1`, both running before the quit.

**The resume half passes.** Client *and* daemon stopped — `0` `claude` verified, a genuinely idle
start — then reopened:

| | |
|---|---|
| `claude` after reopening | **1** |
| which one | `--resume 9707d1f0…` = **alpha A2**, the remembered session of the project that opened |
| beta's remembered `B1` | **still stopped** — not woken by a project the user did not open (SC-005a) |
| terminal | **live**, real output, no clicks (FR-004a) |

[Evidence](./evidence/B2-resumes-exactly-one.png).

**The FR-014 half is not reached, and both suggested routes fail to reach it.** Recorded rather
than quietly dropped, because the step asks for it:

- *A second window holding the project* — the newcomer **takes over** rather than being refused. The
  first window gets "Another window took over this project… read-only until you take it back", and
  the second attaches and resumes normally. Takeover is automatic, so this route produces a
  displaced *first* window, never a refused resume in the second.
- *A session whose worktree was deleted* — **still resumed**. The row is tagged `missing`, and the
  daemon starts the process anyway, with cwd `$HOME` rather than the vanished worktree.

Taken with the run above, which tried the second-window route in its own sandbox and saw the pane
keep streaming, that is three attempts and no refusal. So on this build the FR-014 wording appears
to have no reachable path at all through the UI *by design* — the one time it was seen (that run's
mismatched-daemon accident) the process had started and then died, which is not a refusal. That is
consistent with BUG-002's own framing ("the ordinary path no longer reaches that screen"), but the
extraordinary path was not found either. The requirement and its tests stand; what is unverified is
that any user can still arrive there. Someone who knows a way the daemon refuses a start should say
so here.

**Answered, 2026-08-16.** All three failed routes share one property: the daemon keeps hosting the
session, so frames keep arriving and the pane is never empty. The empty pane needs a start that
produced *no process*, which means the spawn has to fail — remove `claude` from `PATH` and reopen.
Recorded above under **FR-014, reached deliberately**.

> **And since [`010` BUG-012](../010-daemon-session-persistence/bugs/BUG-012.md) was fixed the same
> day, the deleted-worktree route reaches it too.** That route was not a dead end but a *defect*: the
> daemon started the session anyway, in `$HOME`, which is why it streamed. `010`'s FR-006c now refuses
> a start whose directory is gone, so the spawn produces no process — the same mechanism the `PATH`
> route uses, arrived at from the other side. Noted as a consequence of that fix rather than as a
> second recorded run: the refusal has executable coverage (`010` SC-024), but the `PATH` route above
> is the one whose pane was actually watched, and it stays the route this step asks for.

#### Two findings from this run

**1. A resumed session's status stays `interrupted`, and keeps offering `restart`.** The terminal is
live and streaming, the process has been up for over a minute — and the bar still reads
`interrupted` beside a `restart` control, which would restart a running session. It does not settle
(checked at 12s, 24s and 1m45s) and it survives switching away and back. This is the mirror image of
BUG-001: there the bar was right and the body lied; here the body is right and the bar lies. Visible
in the lower half of the evidence image. **The other run saw the same thing independently**, and
established what this one did not: it happens after clicking the session row by hand too, so it is
not specific to the restore path and is not a regression from BUG-002.

> Filed as [`010` BUG-011](../010-daemon-session-persistence/bugs/BUG-011.md). The lifecycle is never
> advanced out of `InterruptedResumable` — `start_session` spawns the process and registers it live
> without touching the durable record, and the live overlay does not project `lifecycle`, so the
> daemon's own catalog is wrong rather than the client being out of date. FR-006a specifies how a
> session *enters* that state and never how it leaves.
>
> A later pass added the second face: `Running` is reachable **only** by crashing a session and
> letting supervision respawn it, so a *newly created* session reads `starting…` for its whole life
> by the same omission. Both faces were confirmed on screen there.
>
> **Fixed** (2026-08-16, `010` Phase 26): `start_session` now marks the durable record `Running` and
> broadcasts it. Anyone re-running §B should expect `running` in the bar where these screenshots show
> `interrupted`, and no `restart` control beside a live session.

**2. A session whose worktree is gone is resumed in `$HOME`.** `readlink /proc/<pid>/cwd` gives
`/tmp/mb81/home`, not the worktree path that no longer exists. Under FR-004 this could not arise —
nothing was started. Under FR-004a, reopening can silently start an AI CLI session rooted at the
user's home directory rather than in the project, which is a different thing from declining to start
it. Whether that fallback is deliberate is not something this pass can tell.

> Filed as [`010` BUG-012](../010-daemon-session-persistence/bugs/BUG-012.md), and the fallback is
> **not** deliberate: nothing in this project chooses it. `start_session` never checks the directory
> exists, and `portable-pty` filters a non-existent `cwd` out and substitutes `$HOME` without an
> error. The daemon computes `WorktreeStatus::Missing` for the badge and never reads it on the spawn
> path. Raised to High there rather than noted here, because the session an agent is given
> instructions in can now be rooted at the user's home directory with nothing on screen saying so.
> It also bears on this feature: 025's clarification that a deleted-worktree session should still be
> restored was decided when restoring started nothing, and BUG-002 changed what restoring means
> without that answer being revisited. **Fixed 2026-08-16** (`010` FR-006c/SC-024): such a start is
> now refused rather than redirected. The clarification still holds as written — the session is still
> *restored*, listed and selectable as any missing-worktree session is; what no longer happens is a
> process being started for it somewhere else.

Neither is a §B step failing, and neither is in this feature's scope to fix — both belong to what
BUG-002 changed rather than to the memory itself.

### Found while running this

Neither is a §B step failing. Both are recorded rather than fixed here, because both are outside
what this feature changed.

**1. "Starting…" is the wrong thing to say about a restored session.** *(Fixed — see
[BUG-001](./bugs/BUG-001.md). Kept here as found, because the record of a pass is what it saw.)* The terminal body renders a
`Starting…` placeholder whenever it has no grid yet (`ui/terminal.rs`, the `grid: None` branch). A
restored session has no grid and is not running, so the placeholder sits there indefinitely while
the status bar one row below says `interrupted` and offers `restart`, and no process exists. The
two contradict each other, and the placeholder is the one that is wrong.

This is not a new branch — but before this feature no session was current at launch, and the other
way to reach a current-but-not-running session (clicking its row) *does* start it, which makes the
placeholder briefly honest. So this feature is what makes a misleading first screen the normal case
for anyone who quits on an idle session. Visible in the left half of the B2 evidence image.

Filed as [BUG-001](./bugs/BUG-001.md), and **fixed** — the empty state now answers from the same
predicate that decides whether the `restart` control exists, so the two cannot disagree. Re-run
under B2's conditions and confirmed on screen: [before and
after](./evidence/BUG-001-before-after.png). The B2 evidence image above is the *pre-fix* state, and
is kept because it is what the step actually found.

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

**Pin the binary first.** Every worktree's `mise run` build compiles into one shared
`target-shared/`, so a build in another checkout replaces the binary under a running client — and a
later launch in the same pass silently runs someone else's branch. Build, copy the client *and* the
daemon into the sandbox, assert the copy contains the change under test, and launch from there:

```bash
TD=$(./scripts/build-lock.sh --print-target-dir)     # NOT ./target-shared — see CLAUDE.md
./scripts/build-lock.sh cargo build -p micold-client -p micold-daemon
cp "$TD/debug/micold-ai-ide" "$TD/debug/micold-daemon" "$SB/bin/"
strings "$SB/bin/micold-ai-ide" | grep -q '<a string only the new build has>' || exit 1
readlink /proc/$CLIENT_PID/exe                        # confirm once per pass
```

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
