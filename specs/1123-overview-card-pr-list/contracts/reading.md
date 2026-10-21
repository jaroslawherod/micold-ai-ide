# Contract: the status reading's pages and parse (core)

Changes to feature 040's `contracts/pull-request-source.md` §2 and §5 (R1, R2).

## Query, per branch `i`

| Alias | Arguments | Nodes | Was |
|---|---|---|---|
| `o<i>` | `states: OPEN`, newest 10 | `...pr` | unchanged |
| `r<i>` | newest 10 | `...pr` | unchanged |
| `m<i>` | `states: MERGED`, newest 10 | `...pr` | new |
| `oc<i>`, `mc<i>` | 100 | `isCrossRepository` | removed |

Fragment, variables (`$b<i>`, raw `-f` strings), `rateLimit`, chunking (50 branches) unchanged.
Points per branch: 260 → 90.

## Parse

`parse_status(body, branches) -> Result<StatusReading, ReadingFailure>`:

1. Unchanged failure rules: JSON error, an `errors` entry other than a refused `statusCheckRollup`,
   a `null` repository, a missing alias (`o`, `r` or `m`), an unknown `state` → `Passing`.
2. `statuses[branch]` = `select_pull_request(o, r)` as today (FR-013).
3. `lists[branch]` = `BranchPrList::new(own(o ∪ r ∪ m), o.len() == 10, m.len() == 10)` for every
   asked branch.

Tests (`crates/micold-core/tests/pull_request_parse.rs`, `pull_request_counts.rs`,
`pull_request_query.rs`, over recorded answers updated to the new aliases):

- Q1 the query has `m<i>` and no `oc<i>`/`mc<i>`; no branch name in the document.
- P1 four states on one branch → list in FR-003 order, counts 1 open (draft counted) · 1 merged.
- P2 a node in both `o` and `r` (and `m`) is listed once.
- P3 forks are listed nowhere and counted nowhere; a full `o` with a fork reads `9+ open`.
- P4 10 open nodes → `10+ open`; 10 merged nodes → `10+ merged`.
- P5 a missing `m<i>` alias → `Passing`.
- P6 the first listed entry equals `statuses[branch]`'s number for every recorded answer.
