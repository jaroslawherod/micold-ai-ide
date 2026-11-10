# Implementation Plan: Review verdicts bound to the PR head SHA

**Branch**: `feat/1802_autopilot-bind-review-verdicts-to-the-pr-head-sha` | **Date**: 2026-11-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1802-verdict-head-sha/spec.md`

## Summary

Add two bash helpers under `scripts/autopilot/`: `record-verdict.sh` (writes a *Review rounds* row
with `Scope` and `Head`) and `verdict-guard.sh <pr>` (decides merge-or-refuse by comparing the
recorded head with the PR's current head, carrying an approved verdict over a clean replay by
ledger-excluded `git patch-id --stable` equality). `wait-merge.sh` calls the guard before every
merge and gains `--squash`; `gate-hook.sh` additionally denies a bare `gh pr merge` so the script is
the only merge path. The rule lives once in `tasks/merge.md`; `review.md`, `review-rounds.md`,
`ci.md` and the ledger template point to it. No Rust change.

## Technical Context

**Language/Version**: bash (3.2-compatible, as the other helpers), `git`, `gh`, `jq`

**Primary Dependencies**: `git patch-id --stable`, `git merge-base`, `gh pr view|comment|merge`

**Storage**: the autopilot ledger markdown (a table); no new file

**Testing**: `scripts/tests/verdict-guard.test.sh` (real throw-away repos, `gh` stub as in `autopilot.test.sh`), run by `mise run gate`

**Target Platform**: Linux, macOS, Windows (Git Bash): no GNU-only flags

**Project Type**: workflow scripts and skill docs

**Performance Goals**: guard under a few seconds (one `gh pr view`, two short `git log` ranges)

**Constraints**: guard never writes a file or pushes; only a PR comment on a carry; `gh` failure is a refusal

**Scale/Scope**: one PR and one ledger per call

## Constitution Check

- [x] **I. Test-First**: tasks write `verdict-guard.test.sh` cases red before each helper behaviour (FR-010 list).
- [x] **II. Multi-Session**: guard reads one PR and one ledger, writes only a PR comment; nothing shared.
- [x] **III. Worktree Integration**: resolves the ledger by branch (`ledger-of.sh`), no manual git steps for the user.
- [x] **IV. Local-First**: ledger is local; only the existing `gh` calls the merge already makes.
- [x] **V. Rust + iced**: N/A, no app code (scripts like the other autopilot helpers).
- [x] **VI. Cross-Platform**: bash plus git/gh, no `sed -i`/`readlink -f`/GNU `date`; test CI-covered on all runners that run script tests.
- [x] **VII. Documentation**: skill docs (`merge.md`, `review.md`, `review-rounds.md`, `ci.md`, ledger template) updated in the same change; no user-guide impact.
- [x] **VIII. UI Components**: N/A.

Post-design re-check: unchanged, all PASS.

## Project Structure

### Documentation (this feature)

```text
specs/1802-verdict-head-sha/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/cli.md
└── tasks.md          (tasks unit)
```

### Source Code (repository root)

```text
scripts/autopilot/
├── record-verdict.sh      new: append a verdict row, upgrade legacy header
├── verdict-guard.sh       new: merge-or-refuse, carry-forward
├── wait-merge.sh          edit: guard before merge, --squash, --match-head-commit
├── gate-hook.sh           edit: deny `gh pr merge` at command position
├── verdict-lib.sh         new: sourced by the two helpers (see Structure Decision)
└── ledger-of.sh           reused (sourced)
scripts/tests/verdict-guard.test.sh   new
scripts/tests/autopilot.test.sh       edit: hook cases at lines 514 (`test -d x && ... merge 11 --rebase`) and 518 (bare merge of PR 12), allowed today, now expect exit 2 with the use-wait-merge message; add a `wait-merge.sh 12` allow case
.claude/skills/speckit-autopilot/
├── tasks/merge.md         edit: the rule, VERDICT-REFUSED row
├── tasks/review.md, review-rounds.md   edit: point to it, record after fixes committed
├── README.md              edit: flow diagram merge node names wait-merge.sh and the guard
├── tasks/ci.md            edit: Merge problems uses `wait-merge.sh <n> --squash`
└── templates/autopilot-ledger.md       edit: new columns
```

**Structure Decision**: scripts beside `wait-merge.sh`; the guard and record helper share one small
sourced lib `scripts/autopilot/verdict-lib.sh` (column lookup, row parse, ledger-excluded range
patch-ids) so the logic exists once.

## Design

1. **Ledger table**: `| Review | Round | Scope | Head | Snapshot | Verdict |`. Columns found by header
   name, not position. `record-verdict.sh <ledger> <review> <round> <scope> <snapshot> <head> <verdict>`
   validates (40-hex head, verdict word in CLEAN, MINOR-ONLY, CHANGES, pending), upgrades a legacy
   4-column header in place (old rows get empty Scope/Head), appends via temp file + `mv`.
2. **Guard** `verdict-guard.sh <pr>`:
   a. `gh pr view` → head `P`, `headRefName`, `baseRefName`; failure → refuse.
   b. Ledger: `ledger_of` on a worktree checking out that branch (`git worktree list`), else
      `git fetch origin <branch>` then `git grep -lxF -- "- **Worktree branch**: <b>" origin/<b> --
      <LEDGER_PATHSPECS>` and `git show` of the hit. A failed fetch, grep (any error other than
      "no match") or show → `VERDICT-REFUSED ... re-review: ledger unreadable`, exit 7. Only a
      successful search with no match → `UNGUARDED <pr> no ledger`, exit 0.
   c. Required rows: scope == PR number, or milestone ID whose *Milestones* PR column is the PR, or
      `design` when the PR is the *Pull requests* Design row. Latest round per review name decides.
      No `Head` column or no rows → refuse (legacy / no verdict).
   d. Per review: not approved → refuse; `H == P` allow; else compare ledger-excluded diffs
      (`H` vs `P` via `git diff` with `:(exclude)` for each of `LEDGER_PATHSPECS`; empty → same head);
      else FR-005 carry check; unreachable `H` → refuse.
   e. Carry → `gh pr comment <pr> --body "verdict carried H -> P"` once per (H,P) (skip when an
      existing comment has it), print it.
3. **Exit protocol** (last line): `ALLOWED <pr> <P>` 0, `CARRIED <pr> <H> <P>` 0, `UNGUARDED <pr> no
   ledger` 0, `VERDICT-REFUSED <pr> reviewed <H|none> current <P> re-review: <reason>` exit 7.
4. **wait-merge.sh**: after green and before `gh pr merge`, run the guard; on refusal print its last
   line and exit 7. Merge becomes `gh pr merge <n> --rebase|--squash --match-head-commit <P>` so a head
   that moves between the guard and the merge is refused by GitHub. `--no-merge` skips the guard.
5. **gate-hook.sh**: spec FR-004 says the hook denies every direct merge, but today it denies only
   `--admin`, `--delete-branch` and API merges. Add: a `gh pr merge` at command position (`(^|[;&|(][[:space:]]*)gh[[:space:]]+pr[[:space:]]+merge`, so `grep`/`--help` text is not hit) is denied. The new deny sits after the existing `--delete-branch` and `--admin` checks, so the cases at 512, 513 and 515 keep their messages; 514 keeps proving `-d` is read only from the merge's own arguments by asserting the new message, not the `--delete-branch` one with "use
   wait-merge.sh". (Scripts' internal calls are not seen by the hook.)
6. **Docs** per FR-009. Tests per FR-010, plus a bugs-ledger exclusion case and a ledger-unreadable refusal case (fetch fails).

`LEDGER_PATHSPECS` (one list in `verdict-lib.sh`, matching `ledger-of.sh`), each with git's `:(glob)` magic so `*` does not cross `/` as in the shell globs: `specs/*/autopilot.md`,
`specs/*/autopilot-*.md`, `specs/*/bugs/*.autopilot.md`, `specs/quick/*.autopilot.md`. Every
exclusion and ledger search uses it.

## Complexity Tracking

None.
