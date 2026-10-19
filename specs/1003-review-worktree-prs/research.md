# Research: Review changes reaches every pull request of the worktree (#1003)

Builds on feature 775 (`specs/775-diff-view-session-prs/`). Only what changes is decided here.

## R1: Where the worktree's branch history is read

- **Decision**: in the client, with the same local `Git` (`app.caps.shared_git()`) and the same
  entry directory (`shell/changes.rs::local`) the Changes view already uses for its file list and
  diffs. Two read-only git runs in the worktree directory, on the blocking thread of the list read:
  1. `git log -g --date=unix --format=%gD%x09%H%x09%gs HEAD --` (this worktree's own HEAD reflog:
     git keeps one per worktree, under `.git/worktrees/<name>/logs/HEAD`);
  2. `git log -g --ignore-missing --date=unix --format=%gD%x09%H%x09%gs refs/heads/<n>… --` for
     every candidate name at once (one run; `--ignore-missing` skips a deleted branch instead of
     failing the run).
- **Rationale**: no protocol, daemon or stored state (FR-002 "without new stored app state"); git
  commands rather than reading `logs/` files, so the reftable ref backend reads the same. Verified
  on git 2.43: one `log -g` over several refs prints each ref's entries grouped, `%gD` carries the
  full ref name and the entry's Unix time (`refs/heads/x@{1792389208}`).
- **No local git** (the daemon runs elsewhere, `NOT_LOCAL`), or the HEAD run fails: the branch set
  is the current branch alone, with no notice (spec edge case "history cannot be read", as 775).
- **Rejected**: a new `ClientMsg` for the daemon to compute the set (a protocol bump for a read the
  Changes view already does locally for everything else); storing the branches the app creates
  (FR-002 forbids new stored state, and branches made by an agent's own `git switch -c` would be
  missed).

## R2: Which branches count as created in the worktree (FR-002)

git does not record which worktree created a branch. It is derived from two records that are:

- the worktree's HEAD reflog, whose `checkout: moving from <A> to <B>` entries name every branch
  HEAD was on here (an entry is the same for `switch` and `switch -c`), and whose
  `Branch: renamed refs/heads/<X> to refs/heads/<Y>` entries name renames;
- each branch's own reflog, whose oldest entry is its creation (`branch: Created from …`), with the
  commit and the second it was created.

- **Decision** (`worktree_branches::branch_set`, pure, data-model §1):
  1. **Candidates**: every `<A>`/`<B>` of the HEAD reflog's checkout entries that is a branch name
     (not a 40/64-hex commit, not `HEAD`), both names of every rename entry, and the current branch.
  2. **Created here** when the branch's reflog still exists and its oldest entry is a creation at
     commit `c`, time `t`, and this worktree's HEAD reflog has an entry at commit `c` within
     `t ± 2 s` after which HEAD is on that branch (or on a name it was renamed from). "Which branch
     HEAD is on after an entry" walks the HEAD reflog oldest first: a checkout entry moves it to
     `<B>`; entries before the first checkout (the worktree's own creation, `git worktree add -b`)
     are on that checkout's `<A>`, or on the current branch when there is no checkout.
  3. **Created elsewhere** (dropped) when the branch's reflog exists, its oldest entry is a
     creation, and either no HEAD entry matches as in 2, or the creation copies a remote branch of
     the same name: its `branch: Created from <src>` source is `refs/remotes/<remote>/<name>` or
     `<remote>/<name>` (one segment, then exactly the branch's own name `<name>`). That is what
     `git switch <name>` writes when it makes the local branch from `origin/<name>` (DWIM), and
     what `git switch -c <name> origin/<name>` writes; HEAD moves onto it in the same second at the
     same commit, so the time match alone would accept it. FR-002 counts the remote's branch as
     created elsewhere.
  4. **Record gone** (kept): the branch was deleted (its reflog went with it), or its reflog no
     longer reaches its creation (expired). The worktree's HEAD record shows it was on that branch;
     nothing shows it was made elsewhere. This is what keeps a merged-and-deleted branch's pull
     requests listed (spec edge case 1), the case that motivates the feature.
  5. A renamed branch counts under both names when either counts (its pull requests may have
     either head name).
  6. The current branch always counts (FR-002). The default branch never does: it is dropped after
     the GitHub read (R3), which names it.
- **Rationale**: matches the clarification (only-checked-out branches excluded) wherever git still
  holds the evidence, and falls to inclusion only where the evidence is gone, never to dropping a
  branch whose pull requests the issue asks for. `± 2 s`: the ref and HEAD are written by one git
  command, which can straddle a second boundary.
- **Known limit** (recorded, not fixed): a branch created elsewhere, checked out here, and then
  deleted is listed (rule 4); so is a remote branch checked out here (rule 3) once deleted. A new
  branch `x` made from a local branch `a/x` matches the remote form of rule 3 and is dropped; git's
  reflog text does not tell the two apart without the remote list, and the name pattern is rare.
  Two worktrees of one project creating branches at the same commit in
  the same two seconds cannot be told apart. Both are rare and only add entries; none is lost.
- **Rejected**: every branch HEAD was ever on (lists `main`/`develop` checkouts, contradicts the
  clarification); only branches whose reflog still exists (loses every merged-and-deleted branch);
  deciding on the `Created from` text alone, e.g. accepting only `Created from HEAD` or dropping
  every remote source (an agent's `git switch -c x origin/main` writes `Created from origin/main`
  and is a branch created here). The text is used only for the same-name remote copy (rule 3).

## R3: Reading many branches' pull requests in one request

- **Decision**: replace 775's one-branch `list_branch` with `list_branches(repo, branches, now)`:
  one `gh api graphql` request per chunk of `LIST_BRANCHES_PER_REQUEST` = 25 branches, each
  branch an alias `p<i>: pullRequests(headRefName: $b<i>, first: 100, orderBy: CREATED_AT DESC)`,
  plus `defaultBranchRef { name }` and `rateLimit`. Per pull request the 775 fields plus
  `headRefName` and the last commit's check counts, the same `statusCheckRollup` counts the sidebar
  reads (`FRAGMENT` of `status_query`), reduced by the existing `reduce_checks` (FR-003).
- Chunks run one after another; the first failing chunk fails the read (as `read_in_chunks`: whole
  or a failure, so no branch is silently dropped, FR-013, SC-007). Pull requests of the default
  branch are dropped; forks are dropped (FR-014); the merged list is sorted by `created_at`
  descending, then number descending (FR-004).
- **Why 25**: SC-007's 25 branches are one request; GitHub's cost for 25 × 100 pull requests ×
  1 commit × 1 rollup stays near 50 points, against 5 000 an hour, at one read per 5-minute
  trigger. 775's `first: 100` per branch is kept.
- **Rejected**: one request per branch (25 branches = 25 `gh` runs every 5 minutes); `search`
  API `head:` qualifiers (search has its own, lower rate limit and fuzzy matching); keeping
  `list_branch` beside the new method (one caller; a second path to maintain).

## R4: Check status for every state

- **Decision**: every listed pull request carries `checks: CheckStatus`, merged and closed
  included; the entry draws `PullRequestIndicator::new(mark).checks(..)` exactly as the sidebar
  does (`ui/sidebar.rs::indicator_marks`; `None` → no check mark). The sidebar shows no checks on a
  merged or closed pull request because `PrState` cannot hold them; the issue asks for check status
  on each entry, so the list shows the last commit's checks for every state.
- **Rejected**: reusing `PrState` for the list (would hide checks on merged/closed entries, which
  US1 scenario 3 lists without a state restriction).

## R5: Section states (FR-010, FR-011, FR-015)

- **Decision**: a pure `section(view, enabled, now) -> PrSection` in `features/changes.rs` maps
  the reducer's `PrList` to the spec's section-states table. `PrList` changes in one place: a
  `ReadingFailure::Unavailable` answer now keeps `Unavailable { last }` instead of 775's `Absent`,
  so no GitHub access before any list was read is the muted line, and after a list was read is the
  last list (FR-015). `Absent` remains only for setting off and the project root (FR-012).
- **Rejected**: drawing states straight from `PrList` in `ui/` (decision logic in render glue,
  Principle I).

## R6: Previous / next and the header (FR-006, FR-007)

- **Decision**: `Msg::PullRequestStepped(Step)` with `Step::{Previous, Next}`; the reducer finds the
  adjacent entry with the pure `adjacent(prs, number, step) -> Option<u64>` and then runs exactly
  the `SourceChosen(Some(n))` path (one code path for both ways of choosing). The UI disables a
  control when `adjacent` is `None`. The header above the diff (`pr_header(view)`) names
  `#number title`, the indicator with checks, and the base line; it is read from the current list
  each draw, so a refresh updates state and checks without touching the selection (US3 s3).
- `base_line` uses the pull request's own `head_branch`, not the entry's current branch (775 used
  `view.branch`, wrong once the list spans branches).
- **Last choice wins** (FR-016) holds already: `PullRequestDiffRead` is dropped unless its `seq`
  and number match the shown `PullRequestView` (775). A test pins it for three fast choices.

## R7: Refresh and selection (FR-008, FR-009)

- Unchanged from 775 R3/R4: the list is asked on the PR-status triggers while a view is open, and
  the selected number is kept per (project, entry) in `State.remembered`; `reconcile_source`
  returns to the working tree only when the number leaves the list. The branch set is recomputed
  on every list read, so a pull request from a branch created since the view opened appears on the
  next trigger (US3 s2).
