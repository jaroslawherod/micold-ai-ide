# Data Model: Pull Request and Check Status for Each Worktree

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

Nothing here is stored except §5's one settings field. Everything else lives in the memory of the
window that shows the project and is gone when the project is switched or the application exits
(FR-032).

---

## 1. `PullRequestStatus` (`micold_core::pull_request`)

What one reading found for one branch.

| Field | Type | Source (R2 fragment) | Notes |
|---|---|---|---|
| `number` | `u64` | `number` | |
| `title` | `String` | `title` | never logged, never serialised (FR-032) |
| `url` | `String` | `url` | never logged, never serialised; opened only when it starts with `https://github.com/` (FR-014) |
| `state` | `PrState` | `state`, `isDraft` | below |
| `review` | `ReviewState` | `reviewDecision` | below |
| `head` | `String` | `headRefOid` | the commit the pull request ends at; input of §6 |

```text
PrState      = Open { checks: CheckStatus } | Draft { checks: CheckStatus } | Merged | Closed
CheckStatus  = None | Pending | Passing | Failing
ReviewState  = None | Approved | ChangesRequested | ReviewRequired
```

- **`CheckStatus` exists only inside `Open` and `Draft`** (FR-003): a merged or closed pull request
  with a check status is unrepresentable. `CheckStatus::None` is "no checks at all" (story 1
  scenario 8).
- `reviewDecision` `APPROVED` / `CHANGES_REQUESTED` / `REVIEW_REQUIRED` map to the three named
  variants; `null` and any other value map to `ReviewState::None` (no review line, story 2
  scenario 3).
- **Derives**: `Clone`, `PartialEq`, `Eq`. **No `Serialize`, no `Deserialize`.** `Debug` is written
  by hand and prints `number`, `state`, `review` and nothing else. A source gate holds both (R15).
- The spec's entity also lists "whether the branch has commits beyond it" and "when it was read".
  Those are not fields of this type: the first is per reading and comes from the daemon (§6, held
  in §3's `removable`), the second is one time for the whole project (§3's `read_at`).

A successful reading yields `BTreeMap<String, PullRequestStatus>`: branch name → status, with an
entry only for a branch that has a pull request (R3 step 4 yields no entry).

---

## 2. `ReadingFailure` (`micold_core::pull_request`)

Why a reading produced no statuses (R10). `Debug`, `Clone`, `PartialEq`, `Eq`.

| Variant | Meaning | Effect on §3 |
|---|---|---|
| `Unavailable` | pull requests cannot be read at all: no GitHub remote, `gh` missing, not signed in, the sign-in cannot see the repository | `statuses`, `removable` and `read_at` cleared (FR-025) |
| `Passing` | waiting may help: offline, no answer in 10 s, a daemon step failed, an answer that cannot be understood | nothing changes (FR-019) |
| `RateLimited { until: u64 }` | GitHub's request limit; `until` is Unix seconds (R9) | nothing changes; `pause_until = Some(until)` (FR-024) |

No variant carries text: nothing about a failure is shown (FR-025), and the debug log line names
the variant and the branch count only.

---

## 3. Client state: `features::pr_status::State` (`micold-client`)

One per window. It describes the project the window shows, and no other.

| Field | Type | Meaning |
|---|---|---|
| `enabled` | `bool` | the live value of §5's setting, from `Welcome` and `SettingsChanged` |
| `phase` | `Phase` | below |
| `next_seq` | `u64` | sequence number given to the next reading |
| `statuses` | `BTreeMap<String, PullRequestStatus>` | branch → status of the last successful reading |
| `removable` | `BTreeSet<String>` | branches whose merged pull request holds all of the branch's work (§6 answered `Contained`) |
| `read_at` | `Option<u64>` | Unix seconds at which the last successful reading **started**; `None` before the first |
| `pause_until` | `Option<u64>` | no reading starts while `now < pause_until` (R9) |

```text
Phase = Idle | Reading { seq: u64, again: bool }
```

**Invariants** (each a reducer test):

1. At most one reading is under way: a start in `Reading` never produces a second task (FR-022).
2. An answer whose `seq` is not the `Reading`'s `seq` is dropped whole (project switched, switch
   turned off and on, spec Edge Cases "project closed while a reading is under way").
3. `enabled == false` ⇒ `statuses`, `removable` are empty, `read_at` and `pause_until` are `None`,
   `phase` is `Idle` (FR-029: turning off removes every indicator at once).
4. Switching the shown project resets everything but `enabled` and `next_seq`, including
   `pause_until`: the pause belongs to an answer for the project that was shown. The first reading
   of the new project is sent; if the limit still holds, GitHub rejects it without counting it and
   the window pauses again (the same rule FR-024 gives for another window).
5. `removable ⊆ keys(statuses)`, and every branch in `removable` has `state == Merged` (FR-017).
6. The reducer writes no other part of the application's state (FR-020).

`is_stale(read_at, now) = now.saturating_sub(read_at) >= 600` lives in `micold_core::pull_request`
(R13); `State` holds no stale flag.

---

## 4. Row projection (`features::sidebar`)

The sidebar's row model gains one optional field, filled at projection time:

```text
RowPullRequest { status: &PullRequestStatus, stale: bool, removable: bool }
```

| Row | Looked up by | Result |
|---|---|---|
| worktree with `branch: Some(b)` | `statuses.get(b)` | `Some(RowPullRequest)` when found, else `None` |
| worktree with `branch: None` (detached) | — | `None` (FR-007) |
| the "Default" entry | — | `None`, whatever branch the project root has checked out (FR-007) |
| session rows | — | untouched |

`stale` is `is_stale(read_at, now)` with `now` passed in by the view glue. `removable` is
`removable.contains(b)`. A row with `None` is projected, drawn and described by its tooltip exactly
as today (FR-001, FR-011): the join by branch name is the only coupling, so a worktree removed,
renamed or re-branched between readings just stops matching.

---

## 5. Setting: `Settings.pr_status_enabled` (`micold_core::settings`)

| | |
|---|---|
| Type | `bool` |
| Default | `false`, by `#[serde(default)]`; a `settings.json` written before this feature reads as off (FR-030) |
| Owner | the daemon's half of `settings.json` (as `tool_server_enabled`) |
| Wire | `DaemonSettings.pr_status_enabled: bool`; `ClientMsg::SettingsSet { pr_status_enabled: Option<bool> }` (`None` = leave as is) |
| Version | `SETTINGS_VERSION` unchanged (additive, defaulted) |

It is the only thing this feature persists (FR-030 "kept across restarts").

---

## 6. `MergedBranchQuery` / `BranchContainment` (`micold_core::protocol`)

The wire types of R11's RPC. Both `Serialize + Deserialize`; they hold a branch name and a commit
id, never a title or an address.

```text
MergedBranchQuery { branch: String, head: String }      // head = PullRequestStatus.head
BranchContainment = Contained | Beyond | Unknown
```

| Local facts | Answer | Removal suggested |
|---|---|---|
| branch tip = `head` | `Contained` | yes |
| branch tip is an ancestor of `head` (the branch is behind its pull request) | `Contained` | yes |
| branch tip is not an ancestor of `head` (commits made after) | `Beyond` | no |
| the branch does not exist locally, `head` is not a local object, or git failed | `Unknown` | no |

The decision is the pure `containment(tip: Option<&str>, head: &str, ancestor: Option<bool>)` in
`micold_core::git`. Answers come back in the order of the queries; an answer list of another length
is treated as no answer (no suggestion, statuses still applied).

---

## State transitions of one reading

```text
start(now) ── enabled ∧ project shown ∧ connected ∧ now ≥ pause_until ∧ phase = Idle
   └▶ phase = Reading { seq, again: false }; task(seq, project, branches)

finished(seq, Ok { statuses, removable, started_at })
   └▶ statuses, removable replaced; read_at = started_at; pause_until = None; phase = Idle
finished(seq, Err(Unavailable))      └▶ statuses, removable cleared; read_at = None; phase = Idle
finished(seq, Err(Passing))          └▶ nothing but phase = Idle
finished(seq, Err(RateLimited{until}))└▶ pause_until = Some(until); phase = Idle; `again` dropped

after any finished: again ∧ not paused ⇒ start(now)
```
