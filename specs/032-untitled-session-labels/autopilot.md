# Autopilot ledger — 032-untitled-session-labels

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: see the screen show the name of past session is still not displayed even when the bug was reported and fixed — traced as [029 BUG-001](../029-persistent-session-names/bugs/BUG-001.md) (ledger: [BUG-001.autopilot.md](../029-persistent-session-names/bugs/BUG-001.autopilot.md)); its size decision sent the flow to Phase 1.
- **Kind**: feature
- **Worktree branch**: fix/the-name-of-past-session-is-still-not-shown
- **Started**: 2026-09-19
- **Phase**: 4-milestones
- **Next step**: M2 PR #396 open, waiting for CI; then M3 (T028–T030, T044, T039–T041)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #386 | Spec (carries 029 BUG-001 record) | MERGED 2026-09-19 | — |
| #388 | Design (clarify, plan, tasks, milestones) | MERGED 2026-09-19 | 6e21c19a |
| #395 | M1 — untitled `claude` sessions read their first turn | MERGED 2026-09-25 | c5135f57 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T027, T042, T043, T046 | Untitled `claude` sessions read their first turn after restart; a later title replaces it (US1 #1–4, US2); quickstart §B1/§B2 recorded | #395 | MERGED c5135f57 |
| M2 | T031–T038, T045 | Listed Copilot sessions read `name:`, else `summary:`, else their first-turn label (US1 #5–6) | #396 | open |
| M3 | T028–T030, T044, T039–T041 | A running session shows its label within a minute of the first prompt, spinner first or not (US3); Polish, quickstart §B3 recorded | — | pending |

M1 is 30 tasks (over milestones.md's ~15): kept whole because Setup + Foundational (16) have no deliverable of their own and every `claude` scenario (US1 #1–4) needs all of them, and US2 rides in M1 because without it a title arriving in the records could never replace a label (FR-006 regression on `main`). See tasks.md *Milestones*.

`speckit-analyze` (2026-09-19): 0 CRITICAL, 1 HIGH (quickstart B3 Copilot step tagged M2, fixed to M3), 1 MEDIUM (SC-006 unclaimed: added to M1 with the R7 bound + quickstart §B2 step 4), 1 LOW (phase ranges omitted T042–T045, fixed).

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | 1-spec | Where does the fix go? | A new spec, 032, on this worktree's branch. The 029 BUG-001 size decision found the fix is new behaviour (contradicts 029 FR-004). | agent-resolved | 029 BUG-001.md §Size decision; BUG-001.autopilot.md D2 |
| D2 | 2-clarify | FR-002: which typed text is the label? | (b) the first turn: a slash command's arguments, else the command name; otherwise the prompt text. | decided by user | spec.md *Label-source evidence* |
| D3 | 2-clarify | FR-006: does a later title replace the label? | Yes, the title replaces the label on the row and in what is remembered. | decided by user | BUG-001 recommendation |
| D4 | 2-clarify | FR-012: is GitHub Copilot in scope? | Yes: `claude` and Copilot both get the fallback label (user chose over the recommended claude-only). | decided by user | spec.md FR-012 |
| D5 | 2-clarify r1 | Is a CLI-handled `claude` slash command (`/model`) a turn? | No; only prompt-sending commands (skills, custom commands) are turns. | agent-resolved | spec.md US1 #4, FR-001; Copilot records none (2/87 claude sessions open with bare `/model`) |
| D6 | 2-clarify r1 | Are identical first-turn labels disambiguated? | No, as identical titles are not. | agent-resolved | 029-persistent-session-names spec; 029-pi-cli-provider FR-011 |
| D7 | 2-clarify r1 | Is a label styled differently from a title? | No; same places, same presentation. | agent-resolved | spec.md Assumptions |
| D8 | 2-clarify r1 | Must FR-014's bound cover the Copilot sessions? | Yes, the 44 sessions' first turns (SC-009 since D9) (first turn at record 2–10). | agent-resolved | spec.md *Copilot evidence* |
| D9 | 3-design | Is Copilot's `summary:` read as the session title when `name:` is absent (Copilot 1.0.10–1.0.36 sessions)? | Yes: "read the old copilot summary as the name too". `name:` wins; `summary:` ranks above a derived label. Applied as FR-016, US1 #6, SC-008 (rewritten), SC-009. | decided by user | spec.md *Copilot evidence*; 44 sessions on the dev machine |
| D10 | 3-design | The 44 old Copilot sessions are in no Copilot index and not in micold's catalog, so never rows: discover them by scanning `session-state/*/workspace.yaml`? | (A) No: FR-016 applies to listed sessions; US1 #6, SC-008, SC-009 rescoped to listed sessions, verified with fixtures; no new discovery. | decided by user | plan review r1 F1; 0 of 44 in the 4 index ids |

D2–D4 applied to spec.md (FR-002, FR-006, FR-012 + *Copilot evidence*, US1 #5, SC-008) on 2026-09-19.

| Clarify round | Questions | Agent-resolved | Escalated |
|---|---|---|---|
| 1 | 4 | 4 (D5–D8) | 0 |
| 2 | 0 (no critical ambiguities) | — | — |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A (code-review, high) | F1 BLOCKER: `first_turn.rs:81` carries an uncommitted `// MUTANT` line that makes `claude_first_turn` always return `None`. | Not in the diff. The reviewer read the working tree while this unit's own mutation probe was running in it (cycle log, cycle 3 *mutant*); the probe restored the file with `git checkout --` minutes later, and `git show HEAD:…/first_turn.rs` never held the line. The committed diff was the reviewer's own verdict: clean. |
| M2 | A (code-review, high) | F2 low: `specs/026-multi-provider-sessions/contracts/copilot-cli.md:124` still says only `name:` is read from `workspace.yaml`, which this milestone makes false. | The fix is right, the file is not ours: 026 is another feature's spec directory and this flow never edits one (SKILL.md *Ownership*). Resolved from our own side instead — `CopilotProvider`'s rustdoc now names the superseded clause and points at 032's C7 — and the 026 amendment is recorded under *Follow-ups not done*. |
| M2 | B (conformance) | F3 MINOR: `first_turn_label_corpus.rs` re-implements the provider's scalar rule instead of calling it, so the two can drift. | Declined, as the reviewer itself allowed ("none required"). The probe walks a *directory listing*, and `read_title` finds a session by id through a path the listing cannot give back; exposing the private scalar reader to widen a report that is explicitly evidence and not a verdict (D10) buys less than it costs. The copy is labelled as one in its doc comment. |
| M1 | visual pass (§B2 step 2) | The hover tooltip on a session row does not show the label. | Session rows have no tooltip on `main` either (`session_tree_item()` in `crates/micold-client/src/ui/sidebar.rs` never calls `.row_tooltip(..)`, and that file is unchanged by this branch), so a label is treated exactly as a title is (D7, FR-015). Adding one is new behaviour, outside M1's tasks; recorded under *Follow-ups not done*. |

### Milestone code review log

| Milestone | Review | Verdict | Findings | Resolution |
|---|---|---|---|---|
| M2 | A (code-review, high) | 2 low | F1 the block-scalar guard ran after quote stripping, so a quoted title beginning with `>` or `|` was discarded; F2 the 026 Copilot contract contradicts FR-016. | F1 fixed (`05d48d15`, raw-value test) — the reviewer's own corpus check found 2 real sessions with `name: |-` that used to title a row `|-`, so the guard is a fix and not only a guard. F2 declined, see above. |
| M2 | B (conformance) | CLEAN | 3 MINOR: F1 (same block-scalar finding as A's), F2 A5/A6 drove discovery only, not recovery, F3 the corpus probe copies the scalar rule. | F1 and F2 fixed; F3 declined, see above. |

### Spec review log

| Round | Model | Verdict | Findings | Resolution |
|---|---|---|---|---|
| 1 | session (opus) | CHANGES | F1 MAJOR: FR-006 left open but US2/FR-008/entity presumed "yes". F2 MAJOR: bounded read decided only in an edge case, clashing with 100% SCs. F3 MAJOR: FR-002 marker lacked evidence; (c) undefined for claude. F4–F9 MINOR: 80-char cap unjustified, concurrency hazards, sensitive text, SC-005 unmeasurable, 029 assumptions superseded silently, checklist ticks. | All accepted. US2/FR-008/entity made conditional on FR-006; FR-014 bound + FR-015 user guide added; *Label-source evidence* table read from the four transcripts ((a) gives 3 identical rows); FR-003 reason + grapheme + ellipsis + tooltip; edge cases added; SC-005 measurable; superseded 029 assumptions listed; checklist unticked with note. |
| 2 | sonnet | CLEAN | one MINOR: "clear acceptance criteria" tick while FR-002/006/012 pending. | Note added to the checklist. |

### Tasks review log

| Round | Model | Verdict | Findings | Resolution |
|---|---|---|---|---|
| 1 | session (opus) | CHANGES | F1 MAJOR: daemon has no provider injection; T020/T026 and R11 said "fake provider". F2 MAJOR: read-only data dir is not a cross-platform persist failure. F3 MAJOR: P1 Copilot milestone after P3 US3. F4–F11 MINOR: M1 note wording, `MinimalProvider` + T013 Copilot-None assertion, T028 file/harness, guard tests unmarked, (new) markers, no M1 quickstart task, plan.md M2 refs, spec.md names `CopilotProvider::read_title`. | All accepted. Real providers under temp env + `ENV` mutex (tasks, R11, test-list, plan); `tmp/file/data` persist failure; milestones reordered (M2 Copilot, M3 US3 + Polish); T046 added for §B1/§B2 in M1; guard kind in test-list; spec wording made implementation-free. |
| 2 | sonnet | CLEAN | none; F1–F11 verified; all 16 checklist items CONFIRMED. | — |

### Plan review log

| Round | Model | Verdict | Findings | Resolution |
|---|---|---|---|---|
| 1 | session (opus) | CHANGES | F1 BLOCKER: the 44 old Copilot sessions are in no Copilot index and not in micold's catalog, so they are never rows; reading `summary:` changes no row, SC-008/US1 #6 unmeetable. F2 MAJOR: claude CLI-command follower can be a `system/local_command` record (`/reload-plugins`). F3 MAJOR: labels not counted toward the live broadcast. F4 MAJOR: spinner-first order never re-arms `name_stale`. F5–F8 MINOR: Pending re-read cost, per-session writes, schema hash does not cover session.rs, FR-013 row / R6+R10 alternatives / workspace dep. | F2–F8 fixed in contract C3.5a–b′, C6.3a–b, research R3/R5/R6/R7/R8/R9/R10/R11, plan. F1 verified (0 of 44 in the 4 index ids), escalated (category 6), answered D10: spec rescoped to listed sessions. |
| 2 | sonnet | CLEAN | none; F1–F8 fixes verified against code. | — |

## Open escalation

None (the 2026-09-19 old-Copilot-session escalation was answered: D10).

## Follow-ups not done

- Session rows have no hover tooltip at all (worktree rows do), so a label or a title cut at 80
  characters cannot be read in full. Pre-existing on `main`, found by M1's quickstart §B2 pass;
  needs its own spec.

- Old Copilot sessions missing from Copilot's per-cwd index are not discovered (D10); a scan of `~/.copilot/session-state/*/workspace.yaml` by `cwd:` would surface 30 of the 44 under `max-speed`.
- `specs/026-multi-provider-sessions/contracts/copilot-cli.md` still says only `name:` is ever read
  from a Copilot `workspace.yaml`. M2's FR-016 makes that false (`summary:` behind it, and a
  `|`/`>` value rejected). Found by M2's review A; not fixed here because 026 is another feature's
  spec directory. `CopilotProvider`'s rustdoc names the superseded clause in the meantime.
- Carried from 029 BUG-001 ledger: `custom-title` / `agent-name` records ignored by `ClaudeProvider::parse_title` (candidate 029 BUG, out of scope here).
- Carried from 029 BUG-001 ledger: unverified `micold_time_track` untitled rows whose transcripts hold `ai-title`s.
