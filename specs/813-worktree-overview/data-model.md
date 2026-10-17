# Data Model: Worktree Overview

**Feature**: 813 | **Plan**: [plan.md](./plan.md) | Research: [research.md](./research.md)

Everything here is derived and held in memory; nothing is persisted (Principle IV). Core types
live in `crates/micold-core/src/overview.rs`, `task_progress.rs` and `hook_activity.rs`; client
state in `crates/micold-client/src/features/overview.rs`.

## LocationType (core, R7)

`Feature | Bugfix | Chore | Other | Included | OrphanInvalid | ProjectRoot`, each with a word
(`label()`: *Feature*, *Bugfix*, *Chore*, *Other*, *Included*, *Orphan*, *Project root*) and an
icon. Precedence: ProjectRoot (Default) → Included → OrphanInvalid (`Missing` / `Invalid`) →
branch type (`feat` → Feature, `fix` → Bugfix, other conventional type → Chore, none → Other).

## CardSignals (core, R4)

| Field | Type | Source |
|---|---|---|
| `location` | `LocationState` (905) | `location_state` over un-archived sessions |
| `agent_request` | `bool` | a queued `agent_confirm::Prompt` whose `session` is in the location (R5) |
| `pr` | `PrSignal` | pull request reading (R6) |

`PrSignal`: `Unknown` (reading off, unavailable, not read, or no branch / Default) | `Known {
current: Option<CurrentPr>, removable: bool }`; `CurrentPr { checks_failing, changes_requested,
conflict, merged, open }` from the branch's current `PullRequestStatus` (`checks_failing`:
`CheckStatus::Failing` on Open/Draft; `changes_requested`: `ReviewState::ChangesRequested`;
`conflict`: R6).

## CardState and reason (core, R4)

`CardState`: `NeedsAction < NeedsAttention < Working < Done < Idle` (derived `Ord`, worst first).
`card_state(&CardSignals) -> (CardState, CardReason)`, first matching row wins:

| # | Condition | State | Reason (tooltip word) |
|---|---|---|---|
| 1 | `location == Blocked(NeedsPermission)` | NeedsAction | *needs permission* |
| 2 | `agent_request` | NeedsAction | *agent request waiting* |
| 3 | `location == Blocked(Failed)` | NeedsAttention | *session failed* |
| 4 | `location == Blocked(Resumable)` | NeedsAttention | *resumable* |
| 5 | `pr` Known, current open with `checks_failing` | NeedsAttention | *checks failing* |
| 6 | `pr` Known, current open with `changes_requested` | NeedsAttention | *changes requested* |
| 7 | `pr` Known, current open with `conflict` | NeedsAttention | *merge conflict* |
| 8 | `location == Running` | Working | *working* |
| 9 | `pr` Known, current `merged`, and `removable` | Done | *merged* |
| 10 | otherwise (Idle, Unknown, no session) | Idle | *idle* (Unknown: *activity unknown*) |

Invariants: `pr == Unknown` never yields rows 5–7 or 9 (edge case); `Unknown` location never
yields Working or Done by itself; rows 5–7 apply only to an open (or draft) current pull request.
Several signals: the first row wins, which is the worst state (scenario 1.5).

Visual mapping (R11): NeedsAction → `error` family + `CardNeedsAction`, word *Needs action*;
NeedsAttention → `warning` + `CardNeedsAttention`, *Needs attention*; Working → `primary` +
`StateRunning`, *Working*; Done → `success` + `CardDone`, *Done*; Idle → de-emphasised surface +
`StateIdle`, *Idle*.

## PrCounts (core, R6)

`PrCounts { open: u8, merged: u8, open_capped: bool, merged_capped: bool }` (non-cross-repository
pull requests on the branch; capped at 100, shown as *100+*). On a card: `Option<PrCounts>`;
`None` → *—* with the unknown reason as tooltip. Not shown at all for a location with no branch
(Default, detached, orphan).

## SessionCounts (core)

`session_counts(sessions) -> (total, running)`: un-archived sessions of the location; running =
905 `session_state == Running`.

## TaskProgress (core, R9)

| Type | Fields |
|---|---|
| `Phase` | `Spec \| Plan \| Tasks \| Implement \| Pr \| Done`, `label()` |
| `Ratio` | `done: u32, total: u32` (`total > 0` always) |
| `FileProgress` | `feature_dir: Option<String>`, `artifact: Option<Phase>`, `ledger: Option<Phase>`, `tasks: Option<Ratio>` |
| `Progress` | `phase: Option<Phase>`, `ratio: Option<Ratio>`, `source: TaskList \| Ledger \| Hooks \| None` |

`phase(ledger, card_state, pr_open, artifact)`: `ledger` → else `Done` if card Done → else `Pr` if
an open pull request → else `artifact`. `Progress` with no phase and no ratio renders the empty
state *No task progress* (FR-017), never a bar.

## HookActivity (core, R10)

`HookActivity { todos: Vec<Todo>, prompt: Option<String> }`, `Todo { text: String, active:
Option<String>, status: Pending | InProgress | Completed }`. Bounds: ≤ 50 todos, text ≤ 200 chars,
prompt = first non-empty line ≤ 200 chars. `current()` → `Option<CurrentActivity { line, ratio:
Option<Ratio> }>`: line = in-progress todo's `active` else its `text`, else the prompt; ratio =
completed / total when `todos` is non-empty. Serde: `Serialize`/`Deserialize` (on the wire, R10),
never `Debug`-logged.

Per card the main session's `hook_activity` is used; when it is `None` the card falls back
(Story 5 scenario 3).

## CardModel (core, built per render)

| Field | Source |
|---|---|
| `location: SessionLocation`, `display_name`, `branch: Option<String>` | sidebar entry |
| `kind: LocationType` | R7 |
| `sessions: (u32, u32)` | SessionCounts |
| `prs: PrDisplay` (`Hidden \| Unknown(reason) \| Counts(PrCounts)`) | R6 |
| `state: CardState`, `reason: CardReason` | R4 |
| `summary: String` | issue title (R8) → branch → display name; Default → *Project root* |
| `progress: Progress` | R9 (feature progress) |
| `activity: Option<CurrentActivity>` | R10 (shown as current activity, Story 5) |
| `activation: Activation` (`OpenSession(id) \| OfferStart(loc) \| RevealOnly(loc)`) | R12 |

`order(cards)`: stable sort by `state` (worst first), ties in the sidebar's order (FR-007).
`attention(cards) -> Option<Attention { state, count }>`: the worst state when NeedsAction or
NeedsAttention, with the number of cards in it; else `None` (FR-012).

## Client `overview::State` (render-free, `features/overview.rs`)

| Field | Meaning |
|---|---|
| `open: bool` | Overview shown (per window; not persisted) |
| `progress: BTreeMap<String /* dir */, FileProgress>` | last file reading per worktree |
| `titles: BTreeMap<u64, TitleLookup>` | `Pending \| Known(String) \| Unavailable` (R8), cleared on project switch |

`Msg`: `Toggled { settings_open }`, `Closed`, `ProgressRead { dir, progress }`, `TitleRead {
number, result }`, `CardActivated(Activation)`. Transitions: `Toggled` flips `open` unless
`settings_open` (R2); `Closed` sets `open = false`; `CardActivated` sets `open = false` and returns
the follow-up (select / start menu / reveal); a project switch clears `progress` and `titles`.
