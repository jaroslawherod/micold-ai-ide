# Shadow run: staged review against the single-agent review

Follow-up issue #1808. Covers T016, T017, US3, SC-001, SC-002, SC-005, FR-012, FR-013.

## Method

- Inputs: the first three milestone code-diff PRs merged after #1804 (merged 2026-11-12): #1842, #1849, #1851. No unit recorded a shadow row in that time (see #1876), so the orchestrator ran both reviews itself as fresh subagents.
- Each PR was replayed at its reviewed head in a detached worktree. Base is the merge-base with main.
- Current review: review B per `tasks/review.md` (conformance rubric, `autopilot-reviewer`, `model: "sonnet"`).
- Staged review: `review-facts.py`, a plan (`autopilot-reviewer`, session model), 6 probes per PR (`autopilot-worker`, `model: "haiku"`), a verdict (`autopilot-reviewer`, session model).
- Both sides ran without cargo or tests. The Verify step was not run and is not a finding.
- Tokens are cache-read tokens, summed from each subagent's transcript usage (unique assistant messages). Two probes (1851 P5, 1842 P4) ended without delivering and were asked once to deliver. Their tokens are counted.

| PR | Feature / milestone | Head reviewed | Base | Diff |
|---|---|---|---|---|
| #1849 | 1724 M1 (round-1 snapshot, the one the real review A judged CHANGES) | 5c59d3f85 | 8753adb2d | 32 files +1747 -47 |
| #1842 | 1792 M1 (final head; earlier heads were rebased away) | 58769b7bd | 21a748def | 28 files +883 -477 |
| #1851 | 1792 M2 | b4ddd9c15 | 7628262fc | 3 files +93 -9 |

## Shadow record

Counts are BLOCKER/MAJOR/MINOR. Tokens are cache-read, with the number of calls in brackets.

| PR | Current findings | Staged findings | Parity | Tokens current | Tokens staged |
|---|---|---|---|---|---|
| #1849 | 0/1/2 | 0/2/3 | PASS | 126,133 (4) | 1,251,831 (43) |
| #1842 | 0/4/1 | 1/5/2 | PASS, one adjudicated miss | 125,642 (4) | 1,556,275 (47) |
| #1851 | 0/1/2 | 0/2/1 | PASS | 117,825 (4) | 1,336,746 (44) |
| Total | | | | 369,600 (12) | 4,144,852 (134) |

Staged split: plan and verdict (session model) 278,280; probes (Haiku) 3,866,572. Cache writes: current 88,675, staged 1,066,254. Output tokens: current 6,337, staged 104,630.

The facts file passed its 400-line target on two of three diffs (535 lines for #1849, 413 for #1842, 26 for #1851).

### #1849 (1724 M1)

Current: CHANGES, 0/1/2.
- F1 MAJOR cycle-log: no red phase for any M1 behaviour (Constitution I).
- F2 MINOR drift-M1.md: MISSING read-only references not recorded.
- F3 MINOR link.rs after_claim: a confirm() error returns Ok silently.

Staged: CHANGES, 0/2/3.
- F1 MAJOR cycle-log has no red phase (= current F1).
- F2 MAJOR drift-M1.md: 3 MISSING paths unrecorded, 6 stubs recorded only as a brace pattern (current F2, rated MAJOR).
- F3 MINOR Windows orchestrator pipe name unproven (Constitution VI).
- F4 MINOR non-Linux child_count returns 0, so the test passes trivially.
- F5 MINOR confirm() error is silent (= current F3).

Adjudication: PASS. Every current MAJOR was reported. The extra MAJOR F2 holds: the rubric makes an unrecorded MISSING path MAJOR, and `plan-drift.sh` exits 1 on it. The current review saw the same paths and under-rated them.

### #1842 (1792 M1)

Current: CHANGES, 0/4/1.
- F1 MAJOR drift-M1.md is false (composer.rs "unchanged" but it gets `.filled()`); UNPLANNED paths unrecorded.
- F2 MAJOR tdd/cycle-log.md missing.
- F3 MAJOR menu_select.rs:224 `filled()` is opt-in; other callers keep the old button; the scoping is unrecorded.
- F4 MAJOR T009 ticked, but the showcase does not show Select beside MenuSelect.
- F5 MINOR composer_fits (T005) test lacks width and message checks.

Staged: CHANGES, 1/5/2.
- F1 BLOCKER T009 ticked, showcase not side by side (= current F4).
- F2 MAJOR 4 UNPLANNED paths unrecorded (= current F1, part).
- F3 MAJOR composer.rs drift record is false (= current F1, part).
- F4 MAJOR cycle-log missing (= current F2).
- F5 MAJOR no focus or pressed test, though T002 (ticked) requires rest, hover, focus, pressed, open.
- F6 MAJOR composer_fits inverts the FR-016 up-chevron assertion, unrecorded.
- F7 MINOR trigger `.open(s.open)` against `list_open` with no rows.
- F8 MINOR weak anatomy tests.

Adjudication: PASS with one miss.
- Miss: current F3 (opt-in `filled()`). Not a real MAJOR. FR-003 and FR-009 cover the selects in the structured view, and every structured-view caller (composer.rs, mode_control.rs) opts in. The other MenuSelect callers (review source control, worktree filters) are outside the feature. What remains is T008's wording ("in place of the text Button"), a MINOR gap.
- Extra F5 holds: T002 is ticked and names focus and pressed, no test poses either, and visual-pass-M1.md says they are unverified.
- Extra F6 does not hold: research.md (Confirmed, M1, T001) records "the open chevron no longer flips (FR-009, as `Select`)", so the inversion is a recorded decision.

### #1851 (1792 M2)

Current: CHANGES, 0/1/2.
- F1 MAJOR picker_parity.rs compares geometry only: no selected-emphasis or state-layer comparison (T011, US2 1-2).
- F2 MINOR no tdd/cycle-log.md.
- F3 MINOR plan-drift MISSING menu_select.rs, no drift-M2.md.

Staged: CHANGES, 0/2/1.
- F1 MAJOR geometry only (= current F1).
- F2 MAJOR cycle-log missing (current F2, rated MAJOR).
- F3 MINOR no drift-M2.md (= current F3).

Adjudication: PASS. The extra MAJOR F2 holds as a gap (T011's own text asks for the cycle-log note). Severity is the only disagreement. The real M2 review A and B also raised F1 as MAJOR and declined it, recorded in the 1792 ledger.

## Decision

Do not switch. The single-agent review stays the code-diff review (FR-012, FR-013).

- SC-001 met. The staged review matched every real BLOCKER and MAJOR in the three PRs. The one current MAJOR it missed (1842 F3) was adjudicated not a real defect. It also found real extras (1842 F1 BLOCKER, 1842 F5, 1849 F2, 1851 F2).
- SC-002 fails by an order of magnitude. The staged review used 4,144,852 cache-read tokens against 369,600: 11.2x overall, 9.9x, 12.4x and 11.3x per PR. The probe workers are nearly all of it (3,866,572). Each probe is a fresh context that re-reads the diff, at about 25 to 50k tokens of base context per call and 4 to 9 calls each.
- The plan and verdict stages alone (278,280) cost about as much as three whole Sonnet reviews (369,600), before any probe.
- SC-005 (token cost) is not met, for the same reason.

Result (FR-013): shadow mode and the staged flow are removed from `.claude/agents/autopilot-reviewer.md`, `tasks/review.md` and `tasks/review-rounds.md`. `review.md` keeps one line that points here. `scripts/autopilot/review-facts.py` and its tests stay: it uses no model and is cheap, and a later change may hand its facts file to the single-agent reviewer.

With shadow mode gone, no unit writes shadow rows, so #1876 (who writes and commits shadow.md rows) is moot.
