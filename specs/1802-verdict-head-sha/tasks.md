# Tasks: Review verdicts bound to the PR head SHA

**Input**: `specs/1802-verdict-head-sha/` (spec.md, plan.md, research.md, data-model.md, contracts/cli.md, quickstart.md)
**Tests**: required (Constitution I, FR-010): each test task precedes the implementation it covers and is written red first.
**Format**: `- [ ] T### [P?] [US?] description with file path`

## Phase 1: Setup

- [ ] T001 Create `scripts/tests/verdict-guard.test.sh`: harness only (throw-away git repos with a bare `origin`, a `gh` stub on the pattern of `scripts/tests/autopilot.test.sh`, `pass`/`fail` helpers, ledger fixture writer with the new `Scope`/`Head` columns); no cases yet. `mise run gate` runs it via `scripts/tests/*.test.sh`.
- [ ] T002 Create `scripts/autopilot/verdict-lib.sh` (sourced, bash 3.2, no GNU-only flags): `LEDGER_PATHSPECS` (the four `:(glob)` patterns of plan.md, matching `ledger-of.sh`) and a header-name column lookup plus row parser for the *Review rounds* table (columns found by name, not position).

## Phase 2: Foundational

- [ ] T003 Add to `scripts/autopilot/verdict-lib.sh` the vocabulary check (`CLEAN`, `MINOR-ONLY`, `CHANGES`, `pending`; approved = `CLEAN` or `MINOR-ONLY`) and the 40-hex head check, shared by the recorder and the guard.

## Phase 3: User Story 3 - Record the head with every verdict (P1, built first)

**Goal**: every finished review round writes a row holding scope and the full reviewed head.
**Independent test**: `bash scripts/tests/verdict-guard.test.sh` record cases.

- [ ] T004 [US3] Add record cases to `scripts/tests/verdict-guard.test.sh`, red first: row appended with Scope and 40-char Head (US3-1); malformed SHA and unknown verdict word refused with exit 2 (US3-2); missing ledger or *Review rounds* section exit 2; legacy 4-column header upgraded in place with old rows keeping empty Scope/Head.
- [ ] T005 [US3] Implement `scripts/autopilot/record-verdict.sh <ledger> <review> <round> <scope> <snapshot> <head> <verdict>` per contracts/cli.md: validate, upgrade a legacy header in place (temp file + `mv`, no `sed -i`), append the row (FR-001, FR-002).
- [ ] T006 [P] [US3] Update `.claude/skills/speckit-autopilot/templates/autopilot-ledger.md` *Review rounds* header to `| Review | Round | Scope | Head | Snapshot | Verdict |` (FR-009).
- [ ] T007 [P] [US3] Edit `.claude/skills/speckit-autopilot/tasks/review.md` and `tasks/review-rounds.md`: record each round's verdict with `record-verdict.sh` after the review's fixes are committed and before the ledger commit, and point to `tasks/merge.md` for the rule (FR-009).

## Phase 4: User Story 1 - Refuse a merge on a stale verdict (P1) 🎯

**Goal**: no merge goes through on a verdict for a different head.
**Independent test**: approve at head A, move the PR to B with a different patch, run the guard: exit 7, names A and B and "re-review".

- [ ] T008 [US1] Add guard cases to `scripts/tests/verdict-guard.test.sh`, red first: stale refused naming both SHAs (US1-1, SC-001); current allowed (US1-2); no row refused (US1-3); `CHANGES` and `pending` refused (US1-4); ledger-only commit allowed; legacy ledger (no `Head`) refused; no-ledger PR prints `UNGUARDED <pr> no ledger`; `gh` failure refused; ledger unreadable (fetch fails) refused with exit 7; `design` scope mapped to the design PR; milestone-ID scope mapped through *Milestones*; bugs-ledger (`specs/*/bugs/*.autopilot.md`) excluded from the comparison.
- [ ] T009 [US1] Extend `scripts/autopilot/verdict-lib.sh`: resolve the ledger of the PR's head branch (checkout from `git worktree list`, else `git fetch` + `git grep -lxF` + `git show` of `origin/<branch>`; failed fetch/grep/show is "unreadable", no match is "no ledger"); required-row selection by scope (PR number, milestone ID via *Milestones* PR column, `design` via *Pull requests* Design row); latest round per review name; ledger-excluded `git diff` between two heads.
- [ ] T010 [US1] Implement `scripts/autopilot/verdict-guard.sh <pr>` for stale, current, ledger-only, no row, `CHANGES`/`pending`, legacy, unguarded and `gh`/ledger failure, with the exit protocol of contracts/cli.md (`ALLOWED`, `UNGUARDED`, `VERDICT-REFUSED ... re-review: <reason>` exit 7); writes no file and never pushes (FR-003, FR-006 same-head part, FR-007, FR-008).
- [ ] T011 [US1] Add wait-merge cases to `scripts/tests/verdict-guard.test.sh`, red first: guard refusal stops `wait-merge.sh` with exit 7 before any merge call; `--squash` guarded then merges with `--squash --match-head-commit <P>`; rebase mode passes `--match-head-commit <P>`; `--no-merge` skips the guard.
- [ ] T012 [US1] Edit `scripts/autopilot/wait-merge.sh`: after green and before the merge call, run `verdict-guard.sh`; on refusal print its last line and exit 7; add `--squash`; merge with `--match-head-commit <P>` (FR-004).
- [ ] T013 [US1] Update the two `gate-hook` cases in `scripts/tests/autopilot.test.sh` (line 514, a merge of PR 11 with `--rebase` after `test -d x &&`, and line 518, the bare merge of PR 12) to expect exit 2 with the use-wait-merge message, and add a `wait-merge.sh 12` allow case plus a `--help`/quoted-text allow case; re-point the line 514 `-d` parsing check at a case that still reaches that code (for example `wait-merge.sh 11 test -d x`); red first.
- [ ] T014 [US1] Edit `scripts/autopilot/gate-hook.sh`: deny a direct PR-merge command at command position (`(^|[;&|(][[:space:]]*)gh[[:space:]]+pr[[:space:]]+merge`), placed after the existing `--delete-branch` and `--admin` checks so their messages stay (plan Design 5).
- [ ] T015 [P] [US1] Write the rule once in `.claude/skills/speckit-autopilot/tasks/merge.md` (binding model, required reviews, refusal and its `VERDICT-REFUSED` row, `--squash`, hook denial); edit `tasks/ci.md` *Merge problems* to use `wait-merge.sh <n> --squash` instead of the hand-run API call; update the flow diagram merge node in `.claude/skills/speckit-autopilot/README.md` to name `wait-merge.sh` and the guard (FR-009).

## Phase 5: User Story 2 - Carry a verdict across a clean replay (P2)

**Goal**: a rebase or amended message with an identical patch merges without a new review round.
**Independent test**: approve at A, rebase to B with the same patch, run the guard: `CARRIED`, PR comment `verdict carried A -> B`.

- [ ] T016 [US2] Add carry cases to `scripts/tests/verdict-guard.test.sh`, red first: clean replay onto a moved base carried with one PR comment (US2-1, SC-002); amended message only carried; changed hunk or conflict resolution refused (US2-2, SC-003); `CHANGES` at A with a clean replay refused (US2-3); chained carry compares to the original `H` (US2-4); merge commit in range and empty range refused; unreachable `H` refused with `re-review`; the comment is not posted twice for the same (H,P).
- [ ] T017 [US2] Add patch identity to `scripts/autopilot/verdict-lib.sh`: ordered `git patch-id --stable` list of non-empty commits in `merge-base(X,base)..X` plus the touched-path set (`git diff --name-status -M`), ledger files excluded via `:(exclude)`; a merge commit or empty range makes it undefined.
- [ ] T018 [US2] Add the carry path to `scripts/autopilot/verdict-guard.sh`: equal identities of `H` and `P` carry an approved verdict, post a `verdict carried H -> P` PR comment once per (H,P) (skip when an existing comment has it), print `CARRIED <pr> <H> <P>`; anything else refuses (FR-005, FR-006, R1, R4, R6).
- [ ] T019 [P] [US2] Add the carry rule and the `CARRIED` outcome to `.claude/skills/speckit-autopilot/tasks/merge.md` (FR-009).

## Phase 6: Polish

- [ ] T020 Run `quickstart.md` steps 1 to 4 and record the results; run `mise run gate` (SC-004). No code change expected.

## Dependencies

- T001, T002 -> T003 -> US3 (T004 -> T005; T006, T007 parallel) -> US1 (T008 -> T009 -> T010; T011 -> T012; T013 -> T014; T015 after T010, T012, T014) -> US2 (T016 -> T017 -> T018; T019 after T018).
- US1 and US2 need US3's record helper (the test fixtures use it). US2 extends the guard of US1.
- Parallel: T006 with T007; T015 with later test tasks; T019 with T018's review.

## Implementation Strategy

MVP first: M1 delivers the recorder and the guard that refuses stale, missing and `CHANGES` verdicts, wired into `wait-merge.sh` and the hook, so no unguarded merge path remains. M2 adds carry-forward so routine rebases stop costing a review round. Until M2 merges, a rebase onto a moved base is refused and re-reviewed (safe, never silent). The close unit does T020.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). T020 (Polish, no code change) runs in the close unit. T002 and T003 are covered by the red record cases of T004.

### M1 — Record verdicts and refuse stale merges 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: `record-verdict.sh` writes head-bound ledger rows, `wait-merge.sh` (rebase and `--squash`) refuses a merge whose approved verdict is for another head with `VERDICT-REFUSED ... re-review` (exit 7), and the gate hook denies a direct PR-merge command.
- **Satisfies**: US3 acceptance scenarios 1–2; US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-004, FR-006 (ledger-only part), FR-007, FR-008, FR-009 (merge.md refusal part, review.md, review-rounds.md, template, ci.md), FR-010 (non-carry cases); SC-001, SC-004
- **Verify**: `bash scripts/tests/verdict-guard.test.sh && bash scripts/tests/autopilot.test.sh`; quickstart steps 1 and 3
- **Depends on**: —
- **Tier**: full

### M2 — Carry a verdict over a clean replay

- **Tasks**: T016–T019
- **Deliverable**: a rebase or amended message with an identical patch merges with `CARRIED <pr> <H> <P>` and a `verdict carried H -> P` PR comment, and any changed patch is still refused.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-005, FR-006 (carry part), FR-009 (carry text), FR-010 (carry cases); SC-002, SC-003
- **Verify**: `bash scripts/tests/verdict-guard.test.sh`; quickstart step 2
- **Depends on**: M1
- **Tier**: full
