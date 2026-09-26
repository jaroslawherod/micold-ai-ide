# Autopilot ledger — 032-untitled-session-labels

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: see the screen show the name of past session is still not displayed even when the bug was reported and fixed — traced as [029 BUG-001](../029-persistent-session-names/bugs/BUG-001.md) (ledger: [BUG-001.autopilot.md](../029-persistent-session-names/bugs/BUG-001.autopilot.md)); its size decision sent the flow to Phase 1.
- **Kind**: feature
- **Worktree branch**: fix/the-name-of-past-session-is-still-not-shown
- **Started**: 2026-09-19
- **Phase**: done
- **Next step**: handoff. Close PR **#401** is open and must read `MERGED` first (the orchestrator
  merges it); then send the WORK COMPLETE handoff with the *Follow-ups not done* list below.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #386 | Spec (carries 029 BUG-001 record) | MERGED 2026-09-19 | — |
| #388 | Design (clarify, plan, tasks, milestones) | MERGED 2026-09-19 | 6e21c19a |
| #395 | M1 — untitled `claude` sessions read their first turn | MERGED 2026-09-25 | c5135f57 |
| #396 | M2 — Copilot `summary:` titles and first-turn labels | MERGED 2026-09-25 | bc3bd95a |
| #399 | M3 — a running session reads its label whichever signal arrives first | MERGED 2026-09-26 | 4890b91e |
| #401 | Close — spec `Closed`, the close-phase audits and their remediation | open | — |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T027, T042, T043, T046 | Untitled `claude` sessions read their first turn after restart; a later title replaces it (US1 #1–4, US2); quickstart §B1/§B2 recorded | #395 | MERGED c5135f57 |
| M2 | T031–T038, T045 | Listed Copilot sessions read `name:`, else `summary:`, else their first-turn label (US1 #5–6) | #396 | MERGED bc3bd95a |
| M3 | T028–T030, T044, T047, T039–T041 | A running session shows its label within a minute of the first prompt, spinner first or not (US3); Polish, quickstart §B1 (Copilot) + §B3 recorded | #399 | MERGED 4890b91e |

M1 is 30 tasks (over milestones.md's ~15): kept whole because Setup + Foundational (16) have no deliverable of their own and every `claude` scenario (US1 #1–4) needs all of them, and US2 rides in M1 because without it a title arriving in the records could never replace a label (FR-006 regression on `main`). See tasks.md *Milestones*.

`speckit-analyze` (2026-09-19): 0 CRITICAL, 1 HIGH (quickstart B3 Copilot step tagged M2, fixed to M3), 1 MEDIUM (SC-006 unclaimed: added to M1 with the R7 bound + quickstart §B2 step 4), 1 LOW (phase ranges omitted T042–T045, fixed).

## Close phase (2026-09-26)

Run in the prescribed order. Spec `**Status**` set to
`Closed 2026-09-26 — shipped in PRs #386, #388, #395, #396, #399`.

| Skill | Result |
|---|---|
| `speckit-converge` | **Converged, zero findings.** Every contract clause C1–C8, FR-001–FR-016, FR-013's no-IPC-writer guard, Principles VI and VII and the leftover scan were verified against the code with `path:line` citations. `tasks.md` left byte-for-byte unchanged, as the skill requires when nothing remains. **No unbuilt behaviour**, so no further milestone. |
| `speckit-tdd-verify` | **FAIL**, 6 findings, `tdd/verification.md`. None of them adds or changes product behaviour, so all were in the close PR's remit. Finding 4 is **closed** here (see below); findings 1–3, 5 and 6 are not, and all five are carried as follow-ups. |
| `speckit-docguard-guard` | **Could not run.** DocGuard is an npm tool needing Node ≥18 and `npx`, and this machine has no Node runtime at all; its CDD artifacts (`TEST-SPEC.md`, `DRIFT-LOG.md`, `API-REFERENCE.md`) were never adopted in this Rust repo either. The docs dimensions it would cover are gated here by the repo's own checks, which run inside `mise run gate`: `scripts/tests/documentation-set.test.sh`, `check-user-guide-updated.test.sh`, `links.test.sh`, `media-references.test.sh`, plus `link_corpus.rs` and `documentation_is_not_read.rs`. Those passed. Not escalated: no user decision and no missing access — the tool does not apply to this project. |

What the close PR fixed, all test-evidence or documentation, no behaviour:

- **Finding 4 (three untracked tests)** — U73–U75 added to `tdd/test-list.md`.
- **Finding 3's documentation half** — A7/A8 keep `kind: example` with an evidence note saying they
  have no recorded red, instead of being relabelled `guard`, which would have hidden the gap.

What it did **not** fix, and why:

- **Findings 1–3 (no recorded red for 27 behaviours)** — `tasks.md` T048–T050 stay open. Two attempts
  at the missing mutation evidence produced none: the first was lost when two `speckit-tdd-verify`
  instances collided over deliberate mutants in this one worktree, the second when the test run never
  came out from behind another worktree's build lock before the file was restored. `tdd/cycle-log.md`
  *Cycle 6* records the attempt, the designed mutant set and the ids still unproven, so the gap is
  written down rather than left to be inferred. Cycle 6 did yield three code-reading results: **A7 is
  defended by three independent layers, so T049's prescribed single mutant would survive it**, U47 is
  not killable by one small mutation, and finding 6 is confirmed.
- **Finding 5 (FR-015 untested)** — a gate pinning the guide's FR-015 wording *was* written
  (`untitled_row_guide_claims.rs`, on the `macos_logout_claims_agree.rs` pattern) and the gate caught
  it: `crates/micold-core/tests/documentation_is_not_read.rs` refuses any test that reads a page
  declared `micold-docs`, because CI skips the whole build for docs-only changes, so such a test makes
  that skip unsound — the change that breaks the test is the change CI declines to run. The test was
  removed. The sanctioned `.gitattributes` carve-out (the repo carries six, five because a test reads the file) was **declined**:
  it would make every prose edit to the project's most-edited user-guide page run the full pipeline,
  for a LOW finding, and a gate over six sentences of ordinary prose fires on honest rewording rather
  than on a bug — the existing carve-outs pin single machine-checkable facts, not prose. FR-015 stays
  covered by `scripts/check-user-guide-updated.sh`, which catches an omission but not a drift.
  T052 is closed as **won't-do**, with the reasoning in `tdd/test-list.md`.
- **Finding 6 (the untested read-layer guard)** — recorded as a follow-up, not a task. Closing it
  needs a read-counting provider, and `DaemonState` builds its providers from the environment with no
  injection seam by design (`tasks.md` *Daemon tests*), so it would mean adding production structure
  for a test, against no requirement.

One defect was found and removed inside the close phase: the audit collision left a stray **duplicate**
of the `if !unlabelled { return None; }` guard in `crates/micold-daemon/src/state.rs`, which a
`git add -A` swept into an intermediate commit on this branch, since squashed away. It never reached
`origin/main`. It was
removed, and `git diff origin/main -- crates/` was then read in full and is **empty**: the close PR
changes no code and no test, only `specs/` — which is what a close PR should be.

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
| D11 | 4-M3 | Is T047 (the prompt hook always re-arming the lookup) new behaviour needing a spec change? | No: FR-010 already promises the label within a minute of the first prompt. The gap was in the contract's mechanism, so C6.3c and research R9 were amended and T047 added to M3; no requirement changed. | agent-resolved | spec.md FR-010, SC-007; contracts/first-turn-label.md C6.3c |

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
| M3 | A (code-review, high) | 1 medium, 1 low | F1 medium: cycle 5's re-arm is **one-shot per live session** — `SpinnerObserved` only moves `Unknown → Working`, so a CLI drawing a spinner while it starts up spends that transition before anything is typed and the prompt hook then re-arms nothing, leaving the first turn unread until the closing `Stop`. F2 low: the cycle log claimed `mise run gate` evidence before the gate was recorded. | Both accepted, nothing declined. F1 verified against `activity.rs:102-106` and `state.rs` `note_activity` (the re-arm was `|= changed` only), reproduced as a failing test (U72), fixed by T047 — added to M3 as a task, with contract C6.3c and research R9 amended — and the whole US3 file re-run green (26 tests). F2 fixed: the cycle-log line now points forward to the ledger, where the passing SHA is recorded. |
| M3 | B (conformance) | CHANGES, no BLOCKER/MAJOR | 3 MINOR: F1 the test list typed A11/U68/U70 as `example` while the cycle log records them as green on arrival; F2 `drain_signals`' doc comment did not name the new `name_stale` side effect; F3 T040/T041 (quickstart §B3 evidence, gate SHA) still open. | All three accepted and fixed (`10509fb4` for F1 and F2; F3 by running the §B1 Copilot probe and the §B3 visual pass and recording both in `evidence/quickstart-b.md`). Nothing declined. |
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

- Quickstart §B3 step 1 could not isolate a `claude` row showing the raw prompt *distinct from* a
  title: `claude` 2.1.283 titles a conversation in 0.4–3.5 s, so its untitled window is shorter than
  the pass can catch, even with the child frozen. The behaviour is proved on Copilot instead (step 3,
  same daemon path) and by the US3 tests; nothing is missing from the code. Recorded in
  `evidence/quickstart-b.md`.

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
- Close-phase `speckit-tdd-verify` findings 1–3 (`tasks.md` T048–T050): 27 of the feature's 83
  behaviours have no recorded red — U30, U32, U33, U34, U46–U49, U61, U62, A7 and A8 have neither a
  red nor a demonstrated kill. The behaviour is verified correct (converge found nothing); what is
  missing is evidence the tests *discriminate*. `tdd/cycle-log.md` *Cycle 6* carries the designed
  mutant set, and notes that A7's would need two simultaneous changes because three independent
  layers defend it.
- Close-phase `speckit-tdd-verify` finding 6: the read-layer guard at
  `crates/micold-daemon/src/state.rs:1355` is untested — deleting it survives U64, which asserts only
  the outcome. It costs one unnecessary bounded read per already-`Derived` session per recovery pass
  (bears on SC-006 without breaking it). Closing it needs daemon provider injection, i.e. production
  structure added for a test.
- `crates/micold-core/tests/first_turn_label_corpus.rs`'s two tests are `#[ignore]`d by design (they
  read the developer's real transcripts), so the corpus evidence behind SC-001, SC-002 and SC-005 is
  never regression-protected in CI; it is a manual quickstart §B1 step only.
- FR-015's wording in `docs/user-guide/worktrees-and-sessions.md` is not pinned by anything: a reword
  that drops what an untitled row reads, or that a later title replaces it, would pass the suite.
  `check-user-guide-updated.sh` catches an omitted guide update, not a drifted one. Pinning it needs a
  `.gitattributes` carve-out whose cost (every prose edit to that page runs the full pipeline) was
  judged to outweigh a LOW finding — revisit if that page ever carries a fact worth gating.
- Process, for the next run: never let two `speckit-tdd-verify` instances run against one worktree.
  The forked skill returned "standing by" while still live, a second was dispatched, and the two
  reverted each other's mutants and left a stray edit in the tree.
