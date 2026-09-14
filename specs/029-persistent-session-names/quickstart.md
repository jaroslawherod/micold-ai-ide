# Quickstart: A session keeps its name when nothing is running it

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Contract**: [session-name-persistence.md](./contracts/session-name-persistence.md)

Two parts. **§A** is what the machine checks and is the merge gate. **§B** is the part no test in
this repository can reach: nothing here restarts the daemon process, and "after a daemon restart" is
the entire complaint. §A proves a reloaded catalog carries the name; only §B proves the running
system does.

---

## §A — The automated suite

```bash
mise run test        # whole workspace, matching CI — the gate
mise run test-core   # store + session + provider only; faster while iterating
```

Green is the gate. What each gate is watching, for this feature:

| Gate | Watching |
|---|---|
| `micold-core/tests/session_name_round_trip.rs` | a `Named` label survives save → load through the per-project state file, and a `Pending` one round-trips as `Pending` — the claim the whole feature rests on (data-model §`StoredSession::title`) |
| `micold-core/tests/session_name_round_trip.rs` | **no schema bump**: a session record written *without* `title` loads as `Pending` rather than failing, so an older file and a newer reader agree (contract §1) |
| `micold-daemon/tests/session_name_persistence.rs` | an observed name is written to the catalog, and a **second `Catalog::load` over the same data directory** still has it — the in-process stand-in for a daemon restart (US1, FR-001, SC-001) |
| `micold-daemon/tests/session_name_persistence.rs` | re-observing the name already recorded writes nothing (`Ok(false)`) — the guard that keeps a per-tick observation from rewriting a whole project's records (C2, research R3, SC-007) |
| `micold-daemon/tests/session_name_persistence.rs` | a re-title replaces the recorded name; the previous one is not recoverable from the file (FR-005, US3, SC-005) |
| `micold-daemon/tests/session_name_persistence.rs` | two sessions in one project keep distinct names; recording one leaves the other's label and every other field untouched (FR-012, C6, SC-006) |
| `micold-daemon/tests/session_name_persistence.rs` | recording against a non-persisting catalog (`Catalog::ephemeral`) still updates the label in memory and reports the failure rather than reverting it (FR-009, C5, research R8) |
| `micold-daemon/tests/session_name_persistence.rs` | `drain_signals` reports each title change **once** and reports nothing while the title is unchanged (C9, C10) |
| `micold-daemon/tests/session_name_recovery.rs` | a known session labelled `Pending` whose provider records hold a name gets it, without the session being started (US2, FR-006, SC-004) |
| `micold-daemon/tests/session_name_recovery.rs` | the recovered name is persisted, so a second pass recovers `0` and a catalog reload still has it (FR-007, C18, C19) |
| `micold-daemon/tests/session_name_recovery.rs` | a `Named` session is skipped — including one whose transcript has since been **deleted**, which keeps its name (FR-008, C15, US3) |
| `micold-daemon/tests/session_name_recovery.rs` | a session with no name anywhere stays `Pending` and writes nothing; an unreadable or empty record is a `None`, never an error and never a wrong name (FR-004, C17) |
| `micold-daemon/tests/session_name_recovery.rs` | each session is asked **its own** provider — a Copilot session's name is not looked for in `claude`'s store (C16), and a provider with no `config_dir()` does not suppress the other (C20) |
| `micold-daemon/tests/activity_pipeline.rs` | unchanged and still green: the OSC-0 title still becomes the live session title, glyph-stripped, and the spinner edge is still separate evidence (research R1 — this feature must not disturb it) |
| `micold-daemon/tests/catalog_adoption.rs` | unchanged and still green: a discovered session's title still survives adoption (the path that worked before this feature) |
| `micold-client/**` | **no new test, and no changed test.** The client is untouched by design (research R7); a diff here is a review failure (contract §4) |

**US3 needed no implementation of its own.** The re-title and never-named gates
(`session_name_persistence.rs`: `a_second_name_replaces_the_first_and_the_old_one_is_gone`,
`a_never_named_session_stays_pending_beside_a_named_one`) were written after the US1 work and passed
on their first run — latest-wins and "no session wears another's name" fall out of the write path's
own shape (`Session::set_title` replaces; the lookup is by `SessionId`), not out of a rule added for
them. Recorded here because a gate that passed without a change behind it is evidence about the
design, and would otherwise look like a gate nobody ran.

Also expected green, and worth naming because this feature could plausibly have broken them:

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check     # CI stops here first, before any other job
```

---

## §B — The manual pass

Needs the real daemon stopped and started, so it is run against a built pair, not `cargo test`.

**Prerequisites**

```bash
mise run build                      # or: mise run run, which spawns the daemon itself
```

Use a project with at least three sessions, at least two of which have been named by their AI CLI
(let each one run long enough that its conversation has a title). Confirm the client and daemon
binaries are the matching pair before starting — a mixed pair refuses to connect and prints both
version numbers.

### B1 — The names survive a daemon restart (US1, FR-001, FR-002, SC-001, SC-003)

1. Open the project. Note each session row's name.
2. Stop the daemon (leave the client running, or close it — both are in scope; do both once each).
3. Start the daemon and open the project again. **Do not open any session.**

**Expect**: every row that had a name still has it, present as soon as the list appears — no row
passes through "New session" on its way to being named (SC-003). Rows are distinguishable by name
alone (SC-002).

### B2 — Opening a restored session does not disturb its name (US1 scenario 3)

From B1's state, open a named session and let it come back to life.

**Expect**: the name does not flicker, blank, or change to "New session" at any point in the
handover from the persisted label to the live title.

### B3 — My existing sessions got their names back (US2, FR-006, FR-010, SC-004)

This is the reporter's actual state, and it needs a session the *application* has no name for but
the *CLI* does. Either use a real one from before the change, or make one: stop the daemon, remove
the `title` key from one session's record in that project's state file, and start the daemon again.

**Expect**: on opening the project — without opening the session — the row shows the name from the
CLI's own records. Stop and start the daemon once more: the name is still there, and this time it
came from the application's own record (check that the `title` key is back in the file).

### B4 — The name follows a re-title (US3, FR-005, SC-005)

With a session running, drive its conversation until the CLI re-titles it.

**Expect**: the row follows immediately. Restart the daemon: the row shows the **new** name, and the
previous one is nowhere.

### B5 — An unnamed session still says so (FR-004, US3 scenario 2, SC-006)

Create a session and do not talk to it.

**Expect**: "New session" before and after a daemon restart. No session anywhere in the list is
wearing another session's name.

### B6 — A storage failure is not a session failure (FR-009)

Make the data directory unwritable, then let a running session's title change.

**Expect**: the row shows the new name for as long as the daemon runs; no error banner, no `Failed`
lifecycle, no interruption to the session. A warning in the daemon log is the only trace. Restore
write permission and confirm the next name change persists again.

### B7 — Nothing else moved

Sanity sweep after B1–B6: the busy/idle indicator, the per-row CLI label, discovered sessions, the
"reopen where you left off" restore, and worktree rows all behave as before.

---

## Recording the pass

§B is the constitutional evidence for a change whose one claim no automated test can make, so record
it the way this repository records the others: for each step, what was done, what was seen, and —
for the steps that are about what is on screen — a screenshot. The repo's `visual-pass` skill runs
this against the real GUI on a private display without a human; take a private display and pin
directory so a concurrent session's window or build cannot land in the run.

Findings go under a dated heading below, including anything the pass turned up that this spec does
not cover.

### The pass (2026-09-12)

| Recorded | |
|---|---|
| Date | 2026-09-12 |
| Platform | Xvfb `:79` + lavapipe (software Vulkan), not a real display. Private `XDG_RUNTIME_DIR=/tmp/vp79`, private `XDG_DATA_HOME`, private `CLAUDE_CONFIG_DIR`, a throwaway git repo as the project |
| Build | `micold-ai-ide` and `micold-daemon` from one `cargo build`, copied to `~/vp029/bin/` before the run and launched from there — never out of `target-shared/`. The daemon carries this feature (`strings` finds its recovery log lines), the client does not (it is unchanged by design). The pair connects: `client attached to daemon client_build=micold-ai-ide/0.12.1`, no build-mismatch refusal |
| B1 — the names survive a daemon restart | **PASS** — [evidence](./evidence/B1-after-a-full-restart.png) |
| B2 — opening a restored session does not disturb its name | **PASS** — [evidence](./evidence/B2-handover-six-frames.png) |
| B3 — my existing sessions got their names back | **PASS** — [evidence](./evidence/B3-recovered-from-the-cli.png) |
| B4 — the name follows a re-title | **PASS** — [evidence](./evidence/B4-retitled.png) |
| B5 — an unnamed session still says so | **PASS** — in [B1](./evidence/B1-after-a-full-restart.png) and [B7](./evidence/B7-final-state.png) |
| B6 — a storage failure is not a session failure | **PASS** — [evidence](./evidence/B6-unwritable-data-dir.png) |
| B7 — nothing else moved | **PASS, with two things not reached** (busy/idle indicator, worktree rows — see below) — [evidence](./evidence/B7-final-state.png) |

**The AI CLI was a stand-in, and that is the one caveat worth reading first.** Driving a real
`claude` to title, re-title and *not* title three conversations on demand is not something this pass
can arrange, so `claude` on the run's `PATH` was a ~20-line script that does exactly the two things
this feature reads: it emits the conversation's name as an **OSC-0 terminal title**, and it appends
`{"type":"ai-title","aiTitle":"…"}` to a transcript at
`$CLAUDE_CONFIG_DIR/projects/<encoded-cwd>/<session-id>.jsonl`. Those two interfaces are the entire
surface the daemon touches (`provider.rs`), and the application was not told it was talking to a
stand-in. What is therefore *not* evidenced here is that the real `claude` still emits what it
emitted when `read_title`/`parse_title` were written — the existing provider gates cover the parsing,
not the emitting.

**How each step was run.**

- The project was seeded with **three sessions, all `Pending` on disk** (a pre-split `projects.json`
  is a valid seed), each with a transcript so `prune_empty_sessions` would keep it. Exactly one of
  the three (`2222…`) had an `ai-title` in its transcript the application had never seen — the
  reporter's actual state, made deliberately.
- **B3** is the first thing that happened: opening the project, *without opening any session*, logged
  `recovered session names from the AI CLI's own records … count=1` and the row went from "New
  session" to "Rename the daemon socket". The `title` key was then present in the project's state
  file — so the second half of B3 is evidenced by the **second** run, whose log has no recovery line
  at all (`count` never reported again) while the name is still on the row: it came from the
  application's own record that time.
- **B1 was run both ways.** Daemon only (client left running): killed by PID, the client showed
  "Not connected … Reconnecting", respawned the daemon, and the rows kept their names. Then both
  stopped and the client relaunched from cold. For that second run the stand-in was made **silent** —
  its name file emptied, so no OSC-0 title is emitted for any session — and `last_session` was
  removed from the state file so nothing auto-opened. Every name then on screen can only have come
  from the record. Two named rows showed their names, the third showed "New session".
- **B4** drove a re-title on a live session (`Fix the flaky login test` → `Cache the worktree list`):
  the row followed within one debounce tick and the state file held only the new name — the previous
  one is nowhere in it. After the restart in B1 the row still read the newest name.
- **B6** ran `chmod 500` on the data directory and then re-titled the live session. The row showed
  the new name, the session stayed `running`, no banner, no `Failed` — and the only trace was
  `WARN … could not persist the session's name; it is still shown … err=Permission denied (os error
  13)`, with the file still holding the previous name. Write permission restored, the next title
  change persisted again (`Persist the session name`, new mtime).
- **B7** also checked the path this feature could most easily have broken: a transcript for an
  **unknown** session id was dropped into the CLI's store, and the next attach adopted it *with its
  title* (`adopted sessions started outside this application … count=1`, row reads "Discovered from
  the CLI"). A session created through the UI and never named stayed "New session" across a restart.
  Five rows, five distinct labels, no row wearing another's name.

**Not covered, and not marked passed.**

- **The busy/idle indicator.** `claude`'s activity reaches the daemon through its hooks, which the
  stand-in does not install, so no row was ever driven busy. Unchanged by this feature's diff, but
  not seen.
- **Worktree rows.** The throwaway project has no worktrees; only the Default (project-root) row and
  the "No worktrees yet" empty state were exercised. `SessionLocation::Worktree` is covered by the
  automated gates but not by this pass.
- **SC-003's "no row passes through 'New session' on its way to being named"** is argued here rather
  than caught frame-by-frame: with the CLI emitting nothing at all and no session started, a name on
  screen has no other source than the record. A screenshot pipeline cannot reliably sample the first
  paint after attach, which is the frame that claim is about.

### The real-CLI re-run (2026-09-13)

Task T027, after the convergence fixes T025 and T026. The 2026-09-12 pass above drove a stand-in; this
one drove the real `claude` 2.1.270.

**Platform.** Xvfb `:81` with lavapipe (`WGPU_BACKEND=vulkan`), not a real display. The pair was built
from `fccb5a31` and pinned out of `target-shared`, and it connected (`client attached to daemon`). It
ran with a private `XDG_RUNTIME_DIR`/`XDG_DATA_HOME` and a throwaway git project. `claude` used the
default `~/.claude` config, with every `CLAUDE*`/`ANTHROPIC*` variable removed from its environment so
it behaved as a top-level session. With them inherited it reported "Transcript saving is off" and
wrote no transcripts.

**What the real CLI emits.** Probed under `script` with `TERM=xterm-256color`: `claude` titles its
terminal `✳ Claude Code` as soon as it starts, before anything is asked, and `copilot` titles it
`GitHub Copilot`. Before T025 the daemon recorded that placeholder as the session's name. Bash's
default prompt titles the terminal `user@host: dir`, which a shell tab (T026) would have recorded.

**Results.**

- **A startup title is not a name (FR-004, SC-006).** A new session left at `claude`'s welcome screen
  read "New session", and its `title` was `null` on disk while the CLI was running.
- **A real name is recorded (FR-001).** Given one prompt ("In one sentence, what is a git worktree?"),
  `claude` named session `78461383` "Git worktree explanation". The row, the bottom bar and the state
  file all showed that name. The busy/idle indicator was lit on the row while the CLI worked, which
  the stand-in pass could not show.
- **A shell tab does not rename the session (FR-011).** A shell tab opened on that session showed its
  `user@host:/tmp` prompt after `cd /tmp && pwd`. The row and the bar still read "Git worktree
  explanation", and the file was unchanged. See `evidence/T027-shell-tab-keeps-the-name.png`.
- **B1 with the real CLI.** A second session `858d5bf5` was created and left untouched, and it read
  "New session". Then the client and daemon were both stopped by PID, `last_session` was removed from
  the state file, and the app was relaunched from cold. With no session started and no `claude`
  process running, the Default row read "Git worktree explanation", so the name came from the record.
  Opening the session resumed `claude`, which put its `✳ Claude Code` title back up, and the row kept
  its name. `evidence/T027-never-named-then-restart.png` stacks three crops: before the restart (red),
  after it with nothing running (blue), and after resuming (green).

**Not covered, and not marked passed.**

- **A never-named session across a restart.** On relaunch `858d5bf5` and the first unprompted session
  were archived, not shown as "New session", because `prune_empty_sessions` archives any session
  without a transcript that nothing is running. That behaviour predates this feature. A session that
  is never named therefore cannot survive a restart to be looked at. Its "New session" label was
  verified only while it was live, and by the automated `a_regular_terminal_sessions_shell_title_is_not_a_name`
  and `an_ai_clis_own_startup_title_is_not_a_name` tests.
- **Copilot through the UI.** Only its startup title was probed. The Copilot branch of T025 is covered
  by the automated test, not by this pass.
- **Worktree rows**, for the same reason as the pass above: the project has none.
