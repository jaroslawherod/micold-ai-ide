# Contract: the status reading's pages and parse (core)

Changes to feature 040's `contracts/pull-request-source.md` §2 and §5 (R1, R2).

## Query, per branch `i`

| Alias | Arguments | Nodes | Was |
|---|---|---|---|
| `o<i>` | `states: OPEN`, newest 10 | `...pr` | unchanged |
| `r<i>` | newest 10 | `...pr` | unchanged |
| `m<i>` | `states: MERGED`, newest 10 | `...merged` (`number url state isDraft createdAt isCrossRepository`) | new |
| `oc<i>`, `mc<i>` | 100 | `isCrossRepository` | removed |

Fragment `pr`, variables (`$b<i>`, raw `-f` strings), `rateLimit`, chunking (50 branches)
unchanged; a second fragment `merged` is added.

Rate-limit cost per branch, by GitHub's documented rule (sum the requests each connection needs,
assuming it reaches its `first`/`last`; divide by 100, round; research R2): `o` 1 + 10 `commits` +
10 `contexts` = 21, `r` 21, `m` 1 = **43**, against today's `o` 21 + `r` 21 + `oc` 1 + `mc` 1 =
**44** (FR-008). 50 branches: 2,150 against 2,200, 22 points both.

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
- Q2 the `m<i>` connection asks no nested connection: its text up to the next alias has no
  `commits(` and no `...pr`, and the `merged` fragment has no `(` (FR-008 cost, R2).
- P1 four states on one branch → list in FR-003 order, counts 2 open (the draft counted) · 1 merged.
- P2 a node in both `o` and `r` (and `m`) is listed once.
- P3 forks are listed nowhere and counted nowhere; a full `o` with a fork reads `9+ open`.
- P4 10 open nodes → `10+ open`; 10 merged nodes → `10+ merged`.
- P5 a missing `m<i>` alias → `Passing`; an `m` node missing a `merged` field → `Passing`.
- P7 a number in both `r` and `m` is listed once with `r`'s node (its checks and review kept).
- P6 the first listed entry equals `statuses[branch]`'s number for every recorded answer (none
  has a `createdAt` tie or a fork-filled `r`; data-model §2 edge cases).
