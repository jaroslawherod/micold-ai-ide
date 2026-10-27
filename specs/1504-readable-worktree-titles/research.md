# Research: Readable worktree titles without a GitHub issue title

Feature 1504. Each decision: what was chosen, why, and what was rejected.

## R1 One pure chooser in core, read by every surface

**Decision**: New module `micold_core::location_title` holds the order of FR-001 as one pure
function, `choose(&TitleInputs) -> LocationTitle` (text + `TitleSource`). It replaces
`micold_core::overview_text::card_title`, whose two callers (`ui/overview.rs::card`,
`features/sidebar.rs::row_title`) and the header (`context_header::HeaderText`, which reads
`CardModel.title`) all go through one client helper, `features::overview::location_title(app,
worktree, display_name)`, which gathers the inputs from state.

**Rationale**: FR-007 (card and row agree) is today guaranteed by both calling `card_title`; keeping
one chooser keeps that guarantee. Pure core logic is unit-testable without iced (Principle I), and
every decision (order, trimming, first line, placeholders) sits there, so the client glue has no
branching of its own.

**Alternatives rejected**:
- Extend `card_title` with five more `Option<&str>` parameters: positional soup, and it cannot
  report the source FR-008 needs.
- Store the chosen title in state on each refresh: a second copy of facts already in state, which
  can go stale (FR-010 wants the title to follow its inputs on the next render).

## R2 Title ticket from the branch name

**Decision**: `location_title::title_ticket(dir_name, branch) -> Option<TitleTicket>`:
1. `naming::ticket(dir_name)` when present (the linked ticket, unchanged);
2. else, with a branch, `naming::ticket(&naming::dir_name_from_branch(branch))` (the app's own
   convention `<type>/<ticket>_<name>`, which carries `_` as the ticket boundary, `TICKET_SEP`);
3. else the number `overview_text::short_branch` already recognises in `<type>/<digits>` followed
   by the end, `_` or `-` (so `feat/1504-readable-titles` gives 1504).

A ticket of all digits is `TitleTicket::Github(n)`; one matching `<LETTERS>-<digits>` is
`TitleTicket::Jira(KEY)` (uppercased); anything else (e.g. `fix/some_bug` giving `some`) is no
title ticket. A folder ticket wins over a branch one (FR-002, US1.3).

**Rationale**: The spec's Assumption names the branch form `<type>/<ticket>-<name>`, but the
naming parser only splits on `_` (`naming.rs::split_name`), so the spec's own example
`feat/1504-readable-titles` would yield no ticket from step 2 alone. Step 3 reuses the rule the
card's short-branch line already applies to such branches (feature 1120 C5), so no new convention
is invented. A Jira key with `-` boundaries (`feat/ABC-42-login`) stays ambiguous and is read only
through step 2 (`feat/ABC-42_login`).

**Alternatives rejected**:
- Only step 2: misses the spec's P1 example (US1.1).
- A new regex over the whole branch: invents a second convention beside `naming` and
  `short_branch`; a duplication the gate's budget counts.
- Widening the linked ticket (`worktree_tags`) to the branch: changes the issue tag, the Jira chip
  and the Jira status keys, which FR-002 keeps unchanged (Decision D1).

## R3 Title lookup states and the retry (FR-003, FR-009)

**Decision**: `features::overview::TitleLookup` becomes
`Pending | Known(String) | NoIssue | Failed { at: u64 }` (`at`: Unix seconds, as `pr_status`
keeps its times). `titles_to_fetch(state, tickets, github_available, now)` returns each GitHub
title ticket with no entry, or with `Failed { at }` where `now - at >= TITLE_RETRY_AFTER`
(300 s). `Pending`, `Known` and `NoIssue` are never asked again in a run. The shell maps answers:

| Answer | State |
|---|---|
| `read_title` `Ok(Some(t))` | `Known(t)` |
| `read_title` `Ok(None)` (no issue, a pull request) | `NoIssue` |
| `read_title` `Err(Load(_))`, `gh` not found, blocking task lost, remotes read failed (`on_title_remotes(None)`), daemon lost | `Failed { at: now }` |
| Remotes name no GitHub repository | `NoIssue` (nothing to retry in this project) |

The occasions are the ones that already call `request_titles`: Overview opened, worktree list
changed (`on_worktrees_changed`), titles became readable (service route opened, GitHub switch on),
header followed a worktree. No timer is added (FR-009 "next occasion"; SC-004 no new polling).

**Rationale**: The existing `Unavailable` conflates "not an issue" (permanent) with "could not
read" (transient), which is why a blip lasts the run. A timestamp in the failed state makes the
5-minute rule a pure function of `(state, now)`, testable in the reducer.

**Alternatives rejected**:
- A retry timer/subscription: a new polling loop (SC-004, 813 FR-019).
- Retrying `Failed` on every occasion: hammers GitHub when it is down (spec Assumption).
- Keeping `Unavailable` and clearing entries on reconnect: misses timeouts that are not a
  disconnect, and re-asks permanent "no issue" answers.

## R4 Jira summary in the status request (FR-004)

**Decision**: The status request (`shell/jira_http.rs::read_chunk`) asks
`"fields": ["status", "summary"]`; `jira_status::parse_statuses` returns
`KeyedIssue { key, status, summary }` (replacing the `KeyedStatus` tuple). `StatusBoard` gains
`summaries: BTreeMap<String, String>` and `summary_of(key)`. `features::jira_status::State::plan`
asks for the linked keys (as today) plus the Jira title tickets found only in branch names
(`branch_title_keys`), validated through the same `statuses_jql`; `StatusBoard::begin` records which
of the asked keys are status keys, and `apply` stores a status only for those (no chip for a
branch-only key) and a summary for every asked key. Summaries follow the statuses' lifetime: kept
(not cleared) on a failed refresh until `FAILURES_BEFORE_RETIRING`, cleared on `clear()`. FR-004's "a failed or absent
reading gives no title" is read as "a failure never yields a title of its own": a summary kept from
an earlier successful read is that read's title, not the failure's (ledger D5).

**Rationale**: One request on the same two occasions under the same gate (`jira_request`), as
FR-004 requires; `tests/jira_is_requested_only_on_named_events.rs` keeps counting the same callers.
A summary is a title: kept like a status so a refresh blip does not flip cards back to raw names.

**Alternatives rejected**:
- A second "summary" request: FR-004 forbids a reading of its own for linked keys.
- A separate summary board: two epochs to keep in step for one request.
- Storing branch-only statuses too: the chip reads `status_of` by folder tag today, so it would
  not show, but FR-002 says the status set is unchanged; storing only summaries keeps that literal.

## R5 Spec and commit sources: one off-thread read per occasion (FR-006, FR-010, SC-005)

**Decision**: Facts are read on two occasions that already exist; there is no periodic worktree
refresh to ride, and none is added:
1. **The worktree list or the active project changed**, seen by `main.rs`'s after-update block,
   which already compares `issues::worktrees_fingerprint` around every message (feature 1125). This
   covers `app::State::set_worktrees` (open, reopen, start, catalog sync) and the paths that do not
   go through it: `features::worktree::created`/`included` (`insert_sorted`) and `excluded`.
2. **The Overview opened** (`issues::on_overview_opened`, the occasion that already reads titles):
   every worktree is read again, so a spec written or a commit made after the list was read is
   picked up there, or on the next list change.

New `shell::location_titles::read_facts(app, dirs)` applies `Msg::TitleFactsRequested { project,
dirs }` (the reducer bumps `facts_round` and records it per dir in `facts_asked`), then for each
valid worktree starts one `spawn_blocking` read (bounded by the review reads' git deadline) of:
- **spec**: `location_title::read_spec_title(root, ticket)` — the feature folder from
  `task_progress::feature_dir` (`.specify/feature.json`, else the one `specs/<n>-*`), with `n` the
  number of the worktree's GitHub title ticket (R2), so a ticket only in the branch finds its spec
  folder too; then the heading of its `spec.md` via `location_title::spec_title(text)`;
- **commit**: new `Git::own_commit_subject(dir) -> Option<String>`: `review_base(dir)`'s merge-base
  with the default branch, then `git log -1 --format=%s <base>..HEAD`; `None` when the base is
  unavailable or the range is empty (branch with no commits of its own), or on any error/timeout.

Skipped (no title, nothing read) when the worktree has no branch (detached; commit only), or the
window has no local git (`caps.shared_git()` is `None`, remote daemon placement, feature 027): the
paths are the daemon's, not this machine's. The answer `Msg::TitleFactsRead { project, dir, round,
facts }` is dropped when the project is no longer active, the dir is no longer listed, or `round` is
not the one in `facts_asked` for that dir (stale, spec edge case).

**Rationale**: The sidebar needs these titles with the Overview closed, so the progress watch
(which runs only while the Overview is open) cannot carry them. Both occasions already exist
(SC-004), and each reads a worktree at most once. FR-006's "at most once per worktree list
refresh" is read as once per occasion that refreshes the titles, the Overview opening being one
(spec US3 names it beside the list refresh; ledger D6). Between occasions the facts held are the
last read's. Reusing `feature_dir`, `read_text` limits, `review_base` and the git read deadline
keeps the rules (default branch detection, timeouts) in one place.

**Alternatives rejected**:
- A `listing` counter bumped only in `set_worktrees`: misses worktrees the app creates or
  includes (`insert_sorted`), and a spec or commit added later, until the project is reopened.
- Add the spec title to `FileProgress`: only read while the Overview is open.
- A notify watch per worktree for titles: a new watcher, more than FR-006 asks.
- `git log -1 HEAD` without a base: on a fresh branch shows the default branch's subject, which the
  spec's edge case rules out.
- Reading through the daemon (new protocol op): a wire change for data a local read gives; remote
  placements lose two low-ranked sources, which the spec's fallbacks cover.

## R6 Where "the title's tooltip" lives on each surface (FR-008, US2.6)

**Decision**:
- **Sidebar row**: its row tooltip (`features::sidebar::with_title_line`) gets a source line right
  after the title line: `From pull request #1520`. The `Name:`/`Branch:`/`Folder:` lines stay, so
  the folder name is there already. A title that is the folder name (source `Folder`) keeps today's
  tooltip; source `Branch` adds `Branch name` only when the title line is shown (it is not today,
  since the `Branch:` line already says it).
- **Session-area header**: `HeaderText.title_tooltip` becomes title, then the source words, then
  the display name when it differs.
- **Overview card**: a collapsed card's title has no tooltip (feature 1121 FR-004: the expanded card
  shows full text instead, and hovering a card expands it). `ExpandedText` gains `source:
  Option<String>`, drawn under the title beside the display name line, so hovering the title shows
  the folder name and the source.

**Rationale**: Restoring a tooltip on the collapsed card would contradict 1121 FR-004 and the user
guide (*The title, branch and activity have no tooltip of their own*). The pointer rest that
expands a card is the card's "hover", so the expanded text is where that surface answers US2.6.

**Alternatives rejected**: a tooltip on the collapsed title (reverts 1121); a source chip on the
collapsed card (new visual element, layout change, beyond FR-008).

## R7 Text normalisation (FR-012)

**Decision**: `location_title::first_line(text) -> Option<&str>`: strip a leading BOM, take up to
the first `\n`, strip a trailing `\r`, trim; empty gives `None`. Applied to every source by
`choose`. `spec_title`: the first line starting with `# `, normalised, its `Feature Specification:`
prefix removed (then trimmed), and `None` when what is left is empty or `[FEATURE NAME]` (the
`.specify/templates/spec-template.md` placeholder) or starts with `[` and ends with `]`.

**Rationale**: One normaliser; each edge case in the spec is one unit test.

**Alternatives rejected**:
- Normalise in each source's reader: six copies of the BOM/CRLF/first-line rule, and `choose`
  could not guarantee FR-012 for an input it did not clean.
- Pass the whole trimmed text and let the widget ellipsise: a multi-line PR or session title would
  draw several lines, which FR-012 rules out.

## R8 Session source (FR-005, FR-010)

**Decision**: The main session (`occupancy::main_session`, as the card and the row already use)
of the location among un-archived sessions; `SessionLabel::Named(t) | Derived(t)` give `t`,
`Pending` gives none. Read from the session list at render; no new reading.

**Alternatives rejected**: any un-archived session's title (spec: only the main session).

## R9 Pull request source (FR-005)

**Decision**: `pr_status.statuses.get(branch)` when `pr_status.enabled && read_at.is_some()` (the
same guard as `overview::pr_signal`) and its `state` is `PrState::Open | PrState::Draft`; the title
and number. Merged/closed give none (spec edge case).

**Alternatives rejected**:
- A merged or closed PR's title as a fallback: the spec's edge case rules it out (the branch's
  work is done; its title no longer names the worktree's current work).
- Reading the PR afresh for its title: a new reading, which FR-005 forbids; the PR reading already
  holds it.

## R10 Stale and per-window behaviour

**Decision**: All title state is per window (`app::State`), so two windows each read at most once
per occasion; a `Pending` entry is never re-asked (existing). Facts carry `round`; titles carry
`project` (existing `Msg::read_project` drop). Project switch clears `titles`, `facts`,
`facts_asked` (new) and the board (existing).

**Alternatives rejected**:
- One title cache shared by all windows: each window keeps its own `app::State` today; sharing
  would need cross-window sync for a saving of one read per window.
- Re-asking a `Pending` entry after a timeout: a hung read would be sent again while in flight; a
  read that ends in an error already lands in `Failed` and retries after 5 minutes.

## R11 Privacy (FR-011)

**Decision**: `TitleFacts`, `LocationTitle` and `TitleSource` get hand-written `Debug` that omit
text (as `TitleLookup` and `PullRequestStatus` do). The facts read logs only `dir` and the outcome
class; commit subjects and spec headings never reach `log_line`/`eprintln!`. A test asserts the
`Debug` of each holds no title text.

**Alternatives rejected**:
- Redact at the log sink (`log_line`): it cannot tell a title from other text.
- Derived `Debug` and care at each log call: one `{:?}` of a state leaks a title, and no test
  would catch it.
