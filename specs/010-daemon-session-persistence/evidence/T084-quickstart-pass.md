# T084 — quickstart S1–S15, run and recorded

**Date**: 2026-08-25
**Where**: headless Xvfb `:83` (1600×1400) with Mesa **lavapipe** as the Vulkan ICD, not a real
display and not a real GPU. Sandbox `XDG_RUNTIME_DIR=/tmp/vp83`, `XDG_DATA_HOME` under the session
scratchpad, project `/home/jaro/.aaa-vp83d/r10` (a throwaway git repo with two worktrees).
**Binaries**: client and daemon `0.10.0`, built together and pinned to `~/vp83b/bin` in one
`build-lock.sh` invocation, verified with `strings` before launching and by
`client attached to daemon client_build=micold-ai-ide/0.10.0` in the daemon log.
**Agent**: the AiCli sessions run a PATH-shim standing in for `claude`, so hook/lifecycle traffic is
real and the model is not. Anything that depends on what the real `claude` binary emits is called out
where it matters.

The task said this needed "a human at the GUI". It did not: everything below except the two clauses
marked **out of reach** was driven headlessly with `xdotool` against a real, running client.

## Summary

| Scenario | Verdict |
|---|---|
| S1 Sessions outlive the UI | **Pass** |
| S2 Attach, drive, detach | **Pass** on driving; the 3 s cold-start budget is **not decidable** on lavapipe |
| S3 Activity signal | **Pass**, with a caveat about the shim |
| S4 Unattended supervision | **Pass** |
| S5 Interrupted-resumable after a daemon restart | **Split** — see `bugs/BUG-016.md` |
| S6 Exclusivity and takeover | **Pass** on every clause; banner wording open as `bugs/BUG-023.md` |
| S7 Version mismatch | **Pass** on every clause |
| S8 Mutation error semantics | **Pass** on the static rows; one row open as `bugs/BUG-020.md` |
| S9 Slow-client convergence and bounded memory | **Pass** |
| S10 Scrollback across a detached interval | **Fail** on responsiveness — `bugs/BUG-021.md`, `bugs/BUG-022.md`; every other clause passes |
| S11 Startup race and stale endpoint | **Pass** on the two race clauses; hostile-directory clause open as `bugs/BUG-019.md` |
| S12 Daemon lifetime | **Pass** |
| S13 Diagnostics | FR-047 **passes**; the log-reachability clause is open as `bugs/BUG-015.md` |
| S14 Linux logout survival | **Docs verified**; the logout itself is **out of reach** |
| S15 Cross-platform parity | **Out of reach** here — CI is the gate; see T083 and `bugs/BUG-008.md` |

Nothing below is a re-reading of the test suite. Where an automated test already covers the same
ground the scenario is still walked, because the point of a quickstart is the part the suite cannot
see — and four of the findings here (`BUG-015`, `BUG-018`, `BUG-019`, `BUG-020`, and now `BUG-021`,
`BUG-022` and `BUG-023`) sat behind a green suite.

---

## S1 — Sessions outlive the UI *(SC-001, SC-002)*

Run in two halves, because the scenario asks for two different things.

**The short half — a fully closed interval, exactly checked.** A Regular terminal was given
`echo MARK-BEGIN; sleep 8; seq 1 200000 | sed 's/$/ ....../'; echo MARK-END-DETACHED`, and the client
was killed two seconds in, while the `sleep` was still running. The flood therefore ran start to
finish with **no client attached**. A fresh client relaunched afterwards showed the session still
`Running`, its screen ending `199999 / 200000 / MARK-END-DETACHED / prompt`, and the scrollback
continuous into the closed interval. The same was then repeated with 4,000 lines and
`scrollback_lines: 10000` so the *entire* interval fitted inside the retention limit: every sampled
step back was exact — 50 wheel notches moved the top line by exactly 100, ten times running, from
`3936` to `3336`, with no gap and no repeat (see S10 for the table).

**The long half — ≥10 minutes.** With `for i in $(seq 1 900); do echo "TICK-$i $(date +%T)"; sleep 1;
done` running, the client was killed at 16:55:06 UTC. Immediately after: the daemon (PID 1192130) and
both of its children — the shim standing in for `claude` (1197579) and the terminal's `bash`
(1199499) — were alive and unaffected.

Ten minutes and fifty seconds later — 17:05:56 UTC, with **zero** clients connected for the whole
interval — all three were still alive and the loop was still ticking. A client was relaunched at
17:06:13 and logged `client attached to daemon` at 17:06:14.7 (~1.7 s). The session came back
`Running`, its tail current at `TICK-689 19:06:36`, i.e. the loop had run unattended for eleven
minutes and lost nothing.

The scrollback was then walked back through the closed interval, 50 notches at a time with a 6 s
settle per round (deliberately slow, to stay clear of BUG-021's blanking). The top line stepped by
exactly 100 every round, all the way back to the start of the loop:

| round | top line | its timestamp |
|---|---|---|
| 1 | `TICK-543` | 19:04:08 |
| 2 | `TICK-443` | 19:02:27 |
| 3 | `TICK-343` | 19:00:46 |
| 4 | `TICK-243` | 18:59:04 |
| 5 | `TICK-143` | 18:57:23 |
| 6 | `TICK-43`  | 18:55:42 |
| 7 | `TICK-3`   | 18:55:02 |

No gap, no repeat, and `TICK-3` at 18:55:02 local is four seconds before the client was killed —
so the history is continuous from before the detach, across it, and up to the live tail.

The "crash and rebuild" clause is covered by the same evidence and then some: the client was killed
outright (SIGTERM and SIGKILL) more than a dozen times across this pass, and rebuilt-and-relaunched
between scenarios, without ever costing a session.

**Verdict: pass.**

---

## S2 — Attach, drive, detach *(SC-003, SC-004, SC-005)*

Driving passes: input typed into a reattached session reaches the process (every command in this
document was typed through the GUI with `xdotool`, including into sessions the daemon had been
carrying unattended for minutes), selecting works and is anchored (S10), resizing is what the
`New num_cols is 165 and num_lines is 69` lines in the daemon log record. Switching between the three
sessions in the sidebar presents the right screen with no perceptible delay.

**The 3 s cold-start budget is not decidable here and is not claimed.** On lavapipe the client spends
tens of seconds of CPU on software rasterisation before the first frame; that number describes Mesa,
not this feature. What *is* measurable and does hold is the part the budget is about: from client
launch to `client attached to daemon` in the daemon log is consistently **under two seconds**,
including the cold-start case where the client has to spawn the daemon itself.

**Verdict: pass on what this environment can decide; the wall-clock budget needs a real GPU.**

---

## S3 — Activity signal *(SC-015, SC-016)*

Run 2026-08-25 and recorded in commit `b3d5fab`. A session driven by `UserPromptSubmit`/`PreToolUse`/
`PostToolUse` hooks read *working* for the whole 185 s run across 140 samples with zero flips; a
session with no hooks stayed *unknown*; the awaiting-input badge appeared 234 ms after a `Stop` hook
and 330 ms after a `Notification`, against a 5 s bound. The third claim fails:
`ActivityEvent::Ended` has no producer in production code (`bugs/BUG-018.md`).

**Caveat**: the hook posts were synthetic. They prove the daemon→UI path end to end; they do not
prove the real `claude` binary emits those hooks in that order.

**Verdict: pass on the two headline claims, one clause open as BUG-018.**
**Evidence**: `S3-activity-badges.png`.

---

## S4 — Unattended supervision *(SC-012)*

Run 2026-08-25 and recorded in commit `4af83ef`. With **no client attached**, the daemon noticed a
`kill -9`ed child and respawned it inside one supervision tick; a shim that could not survive a tick
produced exactly two restarts and then `Failed`, matching the L1 note's arithmetic. The failed state
survived a 20 s unobserved gap and was presented correctly on reattach.

**Verdict: pass on both halves.** (What a `Failed` session *tells* the user is thin — `bugs/BUG-017.md`.)
**Evidence**: `S4-failed-state.png`.

---

## S5 — Interrupted-resumable after a daemon restart *(FR-006a/b)*

Run 2026-08-25, recorded in commit `4af83ef` and `bugs/BUG-016.md`. Split verdict: a **non-displayed**
session behaves exactly as specified — `InterruptedResumable` tint, "not running" pane, no process
until one deliberate click. The **displayed** session is resumed with no user action at all, which is
precisely what FR-006b says must never happen. The daemon half is correct; the start request comes
from the client.

**Verdict: fail on FR-006b for a displayed session — BUG-016, open, needs a decision between two
features' requirements rather than obviously a code change.**

---

## S6 — Exclusivity and takeover *(SC-010)*

Run today on `:83` with two real client processes against one daemon (PID 1192130), one project
`r10`. Every clause the scenario names was exercised.

**B is refused, not silently displaced.** Window A held `r10`. Window B launched at 17:11:12 UTC and
asked for the same project; the daemon said no, in as many words:

```
17:11:13  INFO client attached to daemon client_build=micold-ai-ide/0.10.0
17:11:14  INFO attach refused: project busy client=12 project=/home/jaro/.aaa-vp83d/r10
```

B rendered a read-only banner with a **Take over** button (`S6-refused-banner.png`), and A was
untouched — no banner, session pane still live. *What the banner says* about the refusal is wrong in
two ways, filed as [BUG-023](../bugs/BUG-023.md); the decision it reports is right.

**Takeover transfers cleanly.** Clicking **Take over** in B produced `project attached client=12
project=… force=true` at 17:12:26. B attached; A's banner appeared and its pane stopped rendering P.

**A sends zero input and does not exit.** With A read-only, `ZZ-S6-NOINPUT-FROM-A` was typed into its
terminal pane. The string never reached the pty: B's view of the same session showed the loop finish
(`TICK-900 / TICK-END`) and a clean prompt with an empty command line, and A's own view after it
later reattached showed the same empty prompt. A's process (PID 1325869) stayed alive throughout —
it was still the same PID at the end of the scenario.

**A crashed holder frees the project without restarting the daemon.** `kill -9` on the holder
(window B, PID 1338894) at 17:14:27 produced `client disconnected client=12` and nothing else; the
daemon (running since 18:35:37 local) kept going. A's **Take over** then succeeded immediately —
`project attached client=11 … force=true` at 17:14:55 — and A drove the session again
(`echo S6-A-DRIVES-AGAIN` ran; `S6-readonly-then-reattached.png`). A does *not* learn on its own that
the project became free; the banner it is sitting under still names the dead holder until clicked.
That staleness is the third paragraph of BUG-023.

**A on P and B on Q do not interfere.** A second client was launched against the same daemon with its
own `XDG_DATA_HOME` pointing at `myrepo`. It attached with `force=false` — no refusal, no banner on
either side — while A kept driving `r10` (`echo S6-PQ-A-STILL-DRIVES` ran with B up).

**Verdict: pass on every clause of SC-010. The banner wording and holder identity are open as
BUG-023 (low).**

---

## S7 — Version mismatch *(SC-009)*

Run today with a deliberately mismatched pair: a client built with `PROTOCOL_VERSION = 7` against a
`0.8.0` daemon speaking v6. Every clause holds.

- The refusal is legible and names both sides: *"The session service is a different version — This
  app speaks contract v7; the running service (micold-daemon 0.8.0) speaks v6. Restart the service to
  match — running processes stop, but your sessions are preserved and resumable."*
- **One click** of "Restart service" — no typed command — stopped daemon 993411 and started 1073492
  (`micold-daemon starting build=micold-daemon 0.10.0`), which then accepted the client
  (`client attached to daemon client_build=micold-ai-ide/0.10.0`).
- The warning about running processes is honest and is shown *before* the fact, in both the banner and
  the snackbar. The live shim (993615) really did die.
- After the restart the daemon logged `presented interrupted-resumable sessions after restart count=2`
  and both rows rendered in the `InterruptedResumable` purple. **No agent was spawned until one
  deliberate click**, which produced `session started session=f911c1f1-… launch=Resume`.

**Caveat**: BUG-016 (the displayed-session auto-resume above) did not fire in this run because the
client came back with `active_session=None` — `switch: entered … choice=Some(NoneActive { sessions: 2 })`.
S7 passing is not evidence against BUG-016; it stays open.

**Verdict: pass on every clause.**
**Evidence**: `S7-version-mismatch-banner.png`, `S7-interrupted-resumable.png`.

---

## S8 — Mutation error semantics *(SC-007, SC-008)*

Run 2026-08-25, recorded in commit `b3d5fab`. Four of five rows pass: an already-checked-out branch is
prevented rather than attempted, a colliding path is refused before git is invoked, a read-only parent
surfaces git's own stderr verbatim including the command, and deleting a worktree with a live session
confirms first and stops the process rather than orphaning it. The fifth fails: with the daemon killed
mid-creation the dialog still read "Creating branch and worktree" 90 s later, over a sidebar that had
already reconnected and listed the worktree it had in fact created (`bugs/BUG-020.md`).

**Verdict: pass on the static rows; the daemon-dies-mid-flight row is open as BUG-020.**

---

## S9 — Slow-client convergence and bounded memory *(SC-006)*

Run today, twice, with 6M- and 4M-line floods in a Regular terminal.

- **Bounded memory**: client RSS 141,452 kB → 135,380 kB across a 6M-line flood (no growth at all);
  daemon 13,516 kB → a plateau of ~60,000 kB, which is the retained history, not a leak.
- **A stalled client does not replay**: `SIGSTOP` for 30 s, then `SIGCONT`. The view jumped from
  ~764,000 to 1,717,933 within two seconds — roughly 950,000 lines *skipped*, exactly as the
  coalescing design requires — and the final screen converged **exactly**:
  `3999997 / 3999998 / 3999999 / 4000000`, then the prompt.
- **The producer is not held back by a slow consumer**: the run with the 30 s stall finished in
  **112.6 s** (35,528 lines/s); the control run with no stall took **226.2 s** (17,680 lines/s).

**Verdict: pass.**

---

## S10 — Scrollback across a detached interval *(FR-017, FR-018, I2/I4)*

Four clauses of five pass; the responsiveness clause fails, and the failure has a second defect
behind it.

| Clause | Verdict |
|---|---|
| Content continuous across the closed interval | **Pass** — exact at every sampled step |
| Scrollbar sized from the advertised watermark | **Pass** — 24 px thumb for 69 visible rows of ~4,015 (1.9% vs 1.7% expected), position consistent |
| Scrolling past the retained limit clamps rather than erroring | **Pass** — with `scrollback_lines: 170`, repeated 400-notch over-scrolls pin at the oldest retained line and stay there |
| A selection is anchored to line ids, not viewport rows | **Pass** — a selection of lines 3968–3971 stayed on those lines while five new lines pushed the view up |
| Scrolling stays responsive; history fetched by range, not held whole | **Fail** — `bugs/BUG-021.md` |

The failure, briefly: `scroll_view` (`crates/micold-client/src/main.rs:836-855`) asks for
`LineId(from)..LineId(vt)` — from the first un-cached revealed line to the **live tail** — once per
wheel notch, so the request grows with scroll depth and the work is quadratic in it. At a brisk human
17 notches/s the pane is empty from about 500 lines back and fills in ~5 s after the wheel stops, at
exactly the right position. Faster scrolling keeps it empty and leaves the daemon saturated for ~15 s
of silence after a 3 s gesture (5.2 s then 7.3 s of daemon CPU in the two windows after input
stopped).

When that silence passes the 9 s liveness deadline the client reaps its own healthy connection and
reconnects — and the reconnect makes the window displace **itself**: "Another window took over this
project — micold-ai-ide/0.10.0 is now attached", read-only, scroll position lost, until the user
clicks "Take over". That is `bugs/BUG-022.md`, filed separately because the fix is elsewhere and any
stall over 9 s reaches it.

**Verdict: fail on FR-017's responsiveness; pass on everything else in the scenario.**
**Evidence**: `S10-detached-tail.png`, `S10-blank-then-filled.png`, `S10-selection-anchored.png`,
`S10-self-takeover-banner.png`.

---

## S11 — Startup race and stale endpoint *(FR-004)*

Run 2026-08-25, recorded in commit `b3d5fab`. Two clients started simultaneously yielded exactly one
daemon. A `SIGKILL`ed daemon's stale socket file was reclaimed cleanly by the next client. The
hostile-directory clause fails: `resolve()` chmods the `/tmp` fallback to 0700 *before* handing it to
`verify_owned_0700`, so a world-writable `/tmp/micold-<uid>` is silently tightened and bound instead
of refused (`bugs/BUG-019.md` — the unit test passes because it calls the verifier directly).

**Verdict: pass on the two race clauses; the hostile-directory clause is open as BUG-019.**

---

## S12 — Daemon lifetime *(FR-002, SC-006)*

Covered by the same window as S1's long half, and by every scenario that killed a client in this
pass. One session was running; the client was killed at 16:55:06 UTC; the daemon (PID 1192130) held
that session with **no client attached at all** for 10 min 50 s and was still serving it when a
client returned at 17:06:13. It did not exit on the last client's departure, and it did not need to
be restarted to be reattached to.

The complementary half — that the daemon *does* exit once its linger window expires with no sessions
— is the documented `--linger` behaviour (`docs/daemon.md`) and is covered by unit tests rather than
by this pass; the window here never had zero sessions.

**Verdict: pass.**

---

## S13 — Diagnostics *(SC-017, FR-043–047)*

**FR-047 — no terminal content or user input in any log — passes, checked adversarially.** A unique
string `ZZ-SECRET-FR047-payload` was typed into a session and echoed on screen; it appears in zero
lines of the daemon log, the client log, or anywhere under the sandbox `XDG_DATA_HOME`.

The reachability clause fails for a reason found in the same pass (commit `4af83ef`,
`bugs/BUG-015.md`): an inherited `JOURNAL_STREAM` silences the daemon's log file entirely — the check
reads an environment variable that survives every fork/exec instead of `fstat`-ing fd 2 — and the log
path the quickstart tells you to `ls` is not the path the code writes, so on a desktop-launched
install the `ls` finds nothing.

**Verdict: FR-047 passes; the "determinable from logs reachable through the UI" clause is open as
BUG-015.**

---

## S14 — Linux logout survival *(User Story 7)*

**Out of reach**: performing a real logout would end every session on this machine, including other
people's work. Not attempted, and not claimed.

What was verified is the half that is a documentation claim, and it holds: `docs/` states plainly that
survival requires `loginctl enable-linger`, that linger must be enabled **before** the daemon starts
because it is not retroactive, and that surviving logout is **unsupported on macOS and Windows**
rather than left for the user to discover.

**Verdict: docs clause verified; the logout itself unrun.**

---

## S15 — Cross-platform parity *(Principle VI)*

**Out of reach here** — this is a Linux machine. The gate is CI, which runs the suite on Linux, macOS
and Windows; T083 records that. What CI cannot yet say anything about is Windows *behaviour*:
`crates/micold-daemon/src/platform/windows.rs` is 8 lines and its `terminate_process_tree` is an empty
no-op, so a green Windows column means "the suite compiles and passes", not "process-tree teardown
works" (`bugs/BUG-008.md`).

**Verdict: unrun here, by nature; tracked elsewhere.**

---

## What this pass could not answer

- Anything about frame pacing or perceived smoothness. lavapipe is a software rasteriser; the client's
  CPU figures in S10 describe Mesa, not this feature. The *daemon*-side figures are unaffected by it.
- What the real `claude` binary emits. Every AiCli session here ran a PATH shim.
- A real logout (S14) and a real macOS/Windows run (S15).
