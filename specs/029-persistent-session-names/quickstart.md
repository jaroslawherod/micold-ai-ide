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
