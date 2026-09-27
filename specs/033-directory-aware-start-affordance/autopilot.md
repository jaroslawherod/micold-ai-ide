# Autopilot ledger — 033-directory-aware-start-affordance

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: The start affordance's chevron, and its primary press, read one global AI-CLI availability set rather than the answer for the row's own directory — so after a reconnect or after Settings is opened, that set holds the home directory's answer, and a CLI that only a project-local environment-include script puts on PATH can stay unreachable in the very project whose script provides it. The directory-aware answer already exists: feature 029's FR-003b made the per-session override and the missing-CLI list ask about their own directory, and crates/micold-client/src/main.rs (the start-affordance arm, around line 667) is what still reads the global set. This was raised by review A of feature 029's BUG-001 and declined there for a stated reason — FR-003b names only two directory-aware placements, and making the affordance itself directory-aware would mean a PATH lookup per sidebar row per render, which SC-006 and research R11 forbid. Read that declined finding in specs/029-pi-cli-provider/bugs/BUG-001.autopilot.md before deciding anything. So this is a product and design question, not a defect with an obvious fix: the work is to decide whether the affordance should be directory-aware at all, and if so how to get there without a per-row lookup on every render — for example by caching each row's answer and refreshing it on the events that can change it. If that turns out to add behaviour feature 029 never intended, take the Phase 1 route and specify it rather than patching 029.
- **Kind**: feature
- **Worktree branch**: fix/a-project-local-cli-stays-unreachable-in-that-project
- **Started**: 2026-09-27
- **Phase**: 1-spec
- **Next step**: wait for PR 1 (#413, spec) to go green and merge it, then Phase 2 (clarify) on the three
  open markers FR-001, FR-006, FR-007.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #413 | Spec | open | |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D2 | spec | Spec review outcome | Round 1: CHANGES (2 MAJOR: an edge case contradicting FR-010; FR-004 pre-answering FR-006's open question — 4 MINOR), all fixed. Round 2 (sonnet): CLEAN. | agent-resolved | fresh-subagent reviews, spec rubric |
| D1 | bug | Does the report reproduce, and is it a 029 patch (Phase 0) or a new feature (Phase 1)? | Reproduces at code level on `main` 23a0e0ee. **Phase 1**: making the affordance directory-aware adds a third directory-aware placement that 029 FR-003b deliberately did not name, so it is behaviour 029 never intended (autopilot Phase 0 step 5). 029 is Closed and BUG-001's ledger is `done`. | agent-resolved | Trace: one window-wide `session.available_providers` (`crates/micold-client/src/features/session.rs:184`) is overwritten on every `DaemonMsg::AiCliAvailability` (`shell/daemon_sync.rs:785`); asked with `cwd: None` (home, `server.rs:664`) on connect (`daemon_sync.rs:964`) and Settings open (`shell/persist.rs:196`), and with the row's cwd only on `StartMenuOpened` (`main.rs:666–678`). The chevron (`start_affordance_offers_a_choice`, `session.rs:1847`, used at `ui/sidebar.rs:622,713`) and the primary press (`start_intent`, `session.rs:1857`, via `sidebar.rs` `start_press`) take no location. With home=`[claude]` and a project script adding `pi`, the set has length 1, no chevron is drawn, and the only directory-aware ask sits behind that chevron. A state-level test in the style of `crates/micold-client/tests/features_session.rs:484` shows it: neither method can yield a per-row answer. Declined finding: `specs/029-pi-cli-provider/bugs/BUG-001.autopilot.md` *Declined review findings*, M1/A `main.rs:667`. R11 is `specs/026-multi-provider-sessions/research.md` §R11. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Open escalation

None.

## Follow-ups not done

None yet.
