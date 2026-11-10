# Data model

## Verdict row (ledger *Review rounds* table)
`| Review | Round | Scope | Head | Snapshot | Verdict |`

| Field | Rule |
|---|---|
| Review | name, e.g. `Review B M2`, `Spec review` |
| Round | integer; latest per Review wins |
| Scope | PR number, milestone ID (`M2`), or `design` |
| Head | full 40-hex reviewed head `H` (`git rev-parse HEAD` at record time, fixes committed, before the ledger commit) |
| Snapshot | `<tree>:<head>` of `review-snapshot.sh`; dispatch-time only, never read by the guard |
| Verdict | starts with `CLEAN`, `MINOR-ONLY`, `CHANGES` or `pending`; free text may follow |

Approved = `CLEAN` or `MINOR-ONLY`. Legacy table: no `Head` column; the record helper adds `Scope`
and `Head` after `Round`, old rows keep them empty, the guard refuses them.

## Scope resolution
PR number → itself. Milestone ID → *Milestones* row's PR column. `design` → *Pull requests* row whose Purpose is Design.

## Patch identity
Ordered list of `patch-id --stable` over non-empty commits of `merge-base(X,base)..X` plus the set of touched paths with renames; ledger files (the four `ledger-of.sh` patterns: `specs/*/autopilot.md`, `specs/*/autopilot-*.md`, `specs/*/bugs/*.autopilot.md`, `specs/quick/*.autopilot.md`) excluded. A merge commit or empty range makes it undefined (no carry).

## Guard outcome
`ALLOWED` | `CARRIED` | `UNGUARDED` | `VERDICT-REFUSED`, see [contracts/cli.md](contracts/cli.md).
