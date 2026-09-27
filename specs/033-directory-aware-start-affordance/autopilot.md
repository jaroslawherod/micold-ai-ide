# Autopilot ledger — 033-directory-aware-start-affordance

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: The start affordance's chevron, and its primary press, read one global AI-CLI availability set rather than the answer for the row's own directory — so after a reconnect or after Settings is opened, that set holds the home directory's answer, and a CLI that only a project-local environment-include script puts on PATH can stay unreachable in the very project whose script provides it. The directory-aware answer already exists: feature 029's FR-003b made the per-session override and the missing-CLI list ask about their own directory, and crates/micold-client/src/main.rs (the start-affordance arm, around line 667) is what still reads the global set. This was raised by review A of feature 029's BUG-001 and declined there for a stated reason — FR-003b names only two directory-aware placements, and making the affordance itself directory-aware would mean a PATH lookup per sidebar row per render, which SC-006 and research R11 forbid. Read that declined finding in specs/029-pi-cli-provider/bugs/BUG-001.autopilot.md before deciding anything. So this is a product and design question, not a defect with an obvious fix: the work is to decide whether the affordance should be directory-aware at all, and if so how to get there without a per-row lookup on every render — for example by caching each row's answer and refreshing it on the events that can change it. If that turns out to add behaviour feature 029 never intended, take the Phase 1 route and specify it rather than patching 029.
- **Kind**: feature
- **Worktree branch**: fix/a-project-local-cli-stays-unreachable-in-that-project
- **Started**: 2026-09-27
- **Phase**: 5-close
- **Next step**: Phase 5 close: speckit-converge → speckit-tdd-verify → speckit-docguard-guard, then the close PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #413 | Spec | merged | 2e28015f |
| #417 | Design (clarify, plan, tasks, milestones) | merged | 84d61968 |
| #426 | M1 | merged | 2bd447b2 |
| #443 | M2 | merged | 7f9b23be |
| #448 | M3 | merged | 813322a6 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T019, T030–T036, T040 | Each row offers the CLIs its own directory provides; Settings, reconnect and another row's list never replace it | #426 | merged |
| M2 | T020–T027, T037–T039 | Env-include changes refresh every row; rows that go drop answers, rows that come back are asked; tripwire pins the askers | #443 | merged |
| M3 | T028–T029 | quickstart §B recorded passing end to end; no comment describes the window-wide set | #448 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D10 | design | Milestone cut: US2 folded into M1? M1 over 15 tasks? | US2 has no code of its own (keyed filing + keyed list ask), so its tests ride in M1. M1 (27 tasks incl. 7 outer-loop gates) is not split: the only observable half (keyed filing + keyed list ask + per-row readers, without eager asks) would ship most of P2 before P1's core — the eager ask behind US1-1/2/4/5 — against milestones.md rule 1. Pruning (FR-003 "rows that exist", FR-012) is M2's observable "rows that go drop answers". | agent-resolved | references/milestones.md rules 1–3; tasks review round 1 F11 |
| D11 | design | Tasks + milestone review | Round 1: CHANGES (2 MAJOR: the Phase 2 API change left consumers unmigrated so nothing compiled until US1 — added T040; T007 missed `a_field_note_shares_its_fields_column.rs` and two `main_tests.rs` cases, one whose premise FR-009 reverses — 9 MINOR), all fixed. Checklist: all 16 items reviewer-confirmed. | agent-resolved | fresh-subagent review, tasks + milestone rubric |
| D9 | design | speckit-analyze | 0 CRITICAL/HIGH; 2 MEDIUM (stale checklist item, SC-001/002 missing from plan map), 2 LOW (file count, FR order): all fixed. | agent-resolved | speckit-analyze report |
| D8 | design | Plan review outcome | CHANGES (1 BLOCKER: the env-include diff was against fields the window's own save overwrites; 1 MAJOR: the wanted set included hidden agent worktrees; 4 MINOR: A5 sites, key ambiguity for included worktrees, state_scan vocabulary, tripwire test scope). All fixed; the reveal toggle added as an asker (revealing makes rows appear, FR-004 first ask). Round 2 (sonnet): CLEAN. | agent-resolved | fresh-subagent review, plan rubric |
| D7 | design | Reuse the existing per-directory request or add a protocol message? | Reuse `AiCliAvailabilityRequest { req, cwd }` unchanged; the client maps `req → directory` (reply does not echo cwd). No PROTOCOL_VERSION/schema change. | agent-resolved | research.md R1 |
| D6 | clarify | Round 2 scan: any critical ambiguity left? | None. Closing a project drops its held answers (FR-012, agent-resolved from FR-003). | agent-resolved | spec.md FR-003 |
| D5 | clarify | FR-006: when is a row's first answer asked for? | Eagerly, as soon as its directory appears (project open incl. restore, worktree added/discovered), once per distinct directory. | user | AskUserQuestion 2026-09-27 |
| D4 | clarify | FR-001: make the affordance directory-aware at all? | Yes, per-directory answers. | user | AskUserQuestion 2026-09-27 |
| D3 | clarify | FR-007: worktree answered for its own directory or its project root? | Its own directory; Default rows for the root. | agent-resolved | 029 FR-003b; constitution III; `crates/micold-daemon/src/state.rs` `env_include_vars_for` keys the cache by spawn cwd. |
| D2 | spec | Spec review outcome | Round 1: CHANGES (2 MAJOR: an edge case contradicting FR-010; FR-004 pre-answering FR-006's open question — 4 MINOR), all fixed. Round 2 (sonnet): CLEAN. | agent-resolved | fresh-subagent reviews, spec rubric |
| D1 | bug | Does the report reproduce, and is it a 029 patch (Phase 0) or a new feature (Phase 1)? | Reproduces at code level on `main` 23a0e0ee. **Phase 1**: making the affordance directory-aware adds a third directory-aware placement that 029 FR-003b deliberately did not name, so it is behaviour 029 never intended (autopilot Phase 0 step 5). 029 is Closed and BUG-001's ledger is `done`. | agent-resolved | Trace: one window-wide `session.available_providers` (`crates/micold-client/src/features/session.rs:184`) is overwritten on every `DaemonMsg::AiCliAvailability` (`shell/daemon_sync.rs:785`); asked with `cwd: None` (home, `server.rs:664`) on connect (`daemon_sync.rs:964`) and Settings open (`shell/persist.rs:196`), and with the row's cwd only on `StartMenuOpened` (`main.rs:666–678`). The chevron (`start_affordance_offers_a_choice`, `session.rs:1847`, used at `ui/sidebar.rs:622,713`) and the primary press (`start_intent`, `session.rs:1857`, via `sidebar.rs` `start_press`) take no location. With home=`[claude]` and a project script adding `pi`, the set has length 1, no chevron is drawn, and the only directory-aware ask sits behind that chevron. A state-level test in the style of `crates/micold-client/tests/features_session.rs:484` shows it: neither method can yield a per-row answer. Declined finding: `specs/029-pi-cli-provider/bugs/BUG-001.autopilot.md` *Declined review findings*, M1/A `main.rs:667`. R11 is `specs/026-multi-provider-sessions/research.md` §R11. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F1/F2/F3/F8: pruning of gone rows, env-include change refresh, reveal/forget askers missing | M2 scope by design (T024–T026, D10); M1 asks on C1 A1–A5 only |
| M1 | A | F4: a row with no answer yet reads home's answer | FR-005 specifies the home fallback while a row's own answer is pending; 026 FR-010's launch check is the backstop |
| M1 | A | F5: a lost reply leaves a directory "in flight" forever | The daemon always replies to `AiCliAvailabilityRequest`; a reconnect clears the store (FR-011). Listed as a follow-up |
| M1 | A | F9 (MINOR) | Low impact, no behaviour at stake; not changed in M1 |
| M2 | A | F2: a change to a disabled env-include's path or timeout still refreshes | Contract C1 A7 triggers on the echoed settings differing from `asked_under`; normalising "effectively off" would redefine that. It is one user-initiated save, not a per-render ask, so SC-003 is not at stake |
| M2 | A | F3: a refresh starts N+1 script runs at once on the daemon | Daemon-side concurrency is spec Out of Scope (research R11, the daemon's missing single-flight); the client sends one request per row, as C1 A7 specifies |
| M2 | A | F4: sync from a generic "wanted set changed" check instead of per-event sites | Contract C1 is a closed list of named askers, pinned by the tripwire; a generic hook after every update is the design C1 chose against |
| M3 | B | F5 (MINOR): comments say the list "opens" though `StartMenuOpened` also toggles it closed | Harmless; the message name and contract C1 A3 both call it opening |
| M1 | B | F3 (and A F7): `location_dir(..).unwrap_or_default()` empty-path sentinel in sidebar | Unreachable: rows exist only with a project open, so `location_dir` is always Some there. Kept minimal for M1 |

### M2 review outcomes

- Review B round 1: CHANGES — F1 MAJOR (tripwire stopped at a file's first `#[cfg(test)]`, hiding production code in `ui/mod.rs`, `ui/material/mod.rs` and three shell files), F2 MINOR (bare request marker), F3 MINOR (A9/A10 red only via mutants; logged). F1, F2 fixed; F3 accepted as logged.
- Review A: 7 findings. Fixed: F1 (= B F1), F5 (one `env_include_settings` helper), F6 (ForgetConfirmed comment), F7 (`*`-prefixed lines no longer skipped). Declined: F2, F3, F4 (table above).
- M1 review A's deferred F1/F2/F3/F8 are covered: prune in `sync_cli_availability` (T026), the `SettingsChanged` refresh (T024), the `ForgetConfirmed` and `ShowAgentWorktreesToggled` arms (T025), each with a test (U28–U33).

### M3 review outcomes

- Visual pass (quickstart §B, forked Sonnet on a private Xvfb display, pinned pair from this worktree): all 8 steps PASS; `evidence/quickstart-b.md`.
- Review A: 7 findings, all comment/record accuracy, all fixed: F1 (the StartMenuOpened doc claimed the reducer checks after the refresh; the ask is asynchronous), F2/F3 (stale R11/`PATH`-probe wording in `tests/session_start_press.rs` and `tests/unavailable_default_says_so.rs`), F4 (T029's own grep hits), F5 (startup comment: wanted set, not every row; a round trip, not one frame), F6 (main_tests wording), F7 (line width).
- Review B round 1: CHANGES — F1 MAJOR (wrong C1 event number, A2 → A1), F2 MAJOR (worktree-row chevrons are hover-revealed, so the step 1/6/7 images show Default rows only), F3–F8 MINOR. Fixed F1, F3, F4, F6, F7, F8; F2 by stating in the record what the worktree-row claims rest on (log, `/proc`, M1 tests) rather than re-capturing; F5 declined (table above). Round 2 (sonnet): CLEAN.

## Open escalation

None.

## Follow-ups not done

- Review A M3 F1: the not-installed banner is judged against the row's previous answer because the fresh ask is asynchronous; a CLI that became available since the last event gets a banner the arriving reply then contradicts. Pre-existing ordering, not changed in M3.
- Review A M1 F5: time out an in-flight availability request that never gets a reply (no daemon path drops one today).
