# Research: verdicts bound to the PR head SHA

## R1: How to tell a clean replay
- **Decision**: ordered `git patch-id --stable` of each non-empty commit in `merge-base(X,base)..X`, ledger files excluded via `:(exclude)` pathspec, plus the touched-file set (`git diff --name-status -M`), for both `H` and `P`.
- **Rationale**: patch-id ignores line numbers, whitespace and commit messages, so a rebase and an amended message carry; a conflict resolution changes the patch. Merge-base per side makes a moved base irrelevant.
- **Alternatives**: `git range-diff` (human-oriented output, fragile to parse); tree equality (breaks on any base move); comparing `git diff` text (breaks on context shifts).

## R2: Where the guard runs
- **Decision**: inside `wait-merge.sh`, plus the standalone `verdict-guard.sh`; the hook denies bare `gh pr merge`.
- **Rationale**: one merge path, so no unguarded merge. The hook cannot see inside scripts.
- **Alternatives**: a git hook or GitHub branch protection (out of scope: autopilot-local).

## R3: Race between guard and merge
- **Decision**: `gh pr merge --match-head-commit <P>` (rebase and squash) pins the merge to the guarded head.
- **Rationale**: replaces the hand-run API call with `sha=` in `ci.md`; a push in the gap is refused by GitHub.
- **Alternatives**: re-read the head after the guard (still racy).

## R4: Recording a carry
- **Decision**: a PR comment `verdict carried H -> P`; ledger row never rewritten.
- **Rationale**: a push or ledger commit would move the head and restart CI. Chains compare to the original `H`.
- **Alternatives**: appending a ledger row (needs a commit, moves head).

## R5: Reading the ledger of the PR's branch
- **Decision**: the checkout of that branch from `git worktree list` if present, else `git show origin/<branch>:<path>` after fetch.
- **Rationale**: the orchestrator may run on another branch; a local ledger may be newer than the pushed one.
- **Alternatives**: only the pushed copy (misses unpushed rows, but those cannot be merged anyway; chosen as fallback only).

## R6: Unreachable `H`
- **Decision**: refuse with `re-review`. A force-push drops `H` from the remote, but a local clone that recorded it still holds it; if neither has it, the carry cannot be proven.
- **Alternatives**: fetch `H` by SHA from the remote (GitHub does not serve unreferenced commits reliably); treat unreachable as stale without a reason (loses the diagnosis).
