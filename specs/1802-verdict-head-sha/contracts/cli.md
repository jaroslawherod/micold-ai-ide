# CLI contracts

## `scripts/autopilot/record-verdict.sh <ledger> <review> <round> <scope> <snapshot> <head> <verdict>`
Appends a row under *Review rounds*. Exit 0 on success. Exit 2 with a message on a head that is not 40 hex, a verdict outside CLEAN, MINOR-ONLY, CHANGES, pending, a missing ledger or section. Upgrades a legacy header in place (no `sed -i`).

## `scripts/autopilot/verdict-guard.sh <pr>`
Reads the PR head fresh via `gh`. Last line and exit code:

| Last line | Exit | Meaning |
|---|---|---|
| `ALLOWED <pr> <P>` | 0 | every required review approved for `P` (or ledger-only difference) |
| `CARRIED <pr> <H> <P>` | 0 | approved verdict carried over a clean replay; PR comment posted |
| `UNGUARDED <pr> no ledger` | 0 | head branch named by no flow ledger |
| `VERDICT-REFUSED <pr> reviewed <H\|none> current <P\|unknown> re-review: <reason>` | 7 | stale, no row, CHANGES/pending, unreadable head or ledger, legacy ledger, unreachable `H` |

Writes no file and never pushes.

## `scripts/autopilot/wait-merge.sh <pr> [--no-merge|--squash]`
Existing last lines unchanged, plus `VERDICT-REFUSED ...` (exit 7) before any merge. `--squash` guards then `gh pr merge --squash --match-head-commit <P>`. `--no-merge` skips the guard.

## `gate-hook.sh`
Denies a `gh pr merge` at command position (not `--help` or quoted text); `wait-merge.sh` stays allowed for this flow's PR.
