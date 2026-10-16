# Implementation Plan: Show a GitHub issue's relationships when a worktree is created for it

**Branch**: `claude/project-thread-tw6sph` | **Date**: 2026-10-16 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/837-issue-relationship-metadata/spec.md`

## Summary

One new render-free read in `micold-core`, **issue details**: a single `gh api graphql` query for one
issue's blockers, blocked issues, parent, sub-issues, labels, assignees, milestone, project field
values, and closing and other linked pull requests. Its parser maps each GraphQL error to the kind
whose path it names, so a kind that cannot be read is marked not readable and the rest still shows.
The model (`IssueDetails`) is built so its invariants hold by type: at most 20 items per list, totals
from GitHub's summary counts, titles held only as neutralised `PlainText`, and "blocked" derived from
the open-blocker count. It has two outputs, both in core: the tool result's JSON object and the
form's display lines. The agent tool and the form therefore cannot report different things (spec
Assumptions, 550 FR-016).

- **Agent tool** (`micold-daemon/src/mcp/tools.rs`): a `github_issue` call starts the details read on
  the blocking pool as soon as the remote is resolved. It runs alongside the 550 lookup and the
  create, and is awaited after the create under a 10-second deadline counted from its start. Its
  outcome goes into the result as `issue` (contract
  [contracts/tool-result.md](contracts/tool-result.md)). A refused call drops it. A call without
  `github_issue` never starts it.
- **Form** (`micold-client`): picking a GitHub issue sets a `seq`-tagged `IssueDetailsState::Loading`
  in the reducer. The shell sees that the request changed and runs the read with the `gh` the list
  was read with. The answer applies only while it matches the current pick and `seq`. **Create**
  never looks at the state. A thin view function draws the core's display lines with existing
  `Text` primitives (contract [contracts/form-details.md](contracts/form-details.md)).

The sidebar is unchanged (spec US2 scenario 6, an orchestrator default pending the user's
confirmation). Because `IssueDetails` lives in core and does not depend on the form, a later sidebar
marker could read `IssueDetails::is_blocked()` from wherever it is held, with no rework of the read
or the model. That marker is not planned. Full reasoning: [research.md](research.md).

## Technical Context

**Language/Version**: Rust (workspace toolchain, `rust-toolchain.toml`).

**Primary Dependencies**: existing only. In `micold-core`: `github` (`GhCli`, `GithubRepo`,
`IssueLoadError`, `classify`, the invisible-character rule of `description_from`), `process::run_bounded`,
`serde_json`. In `micold-daemon`: `mcp::tools`, `state`, `tokio`. In `micold-client`:
`features::worktree_form`, `shell::issues`, `shell::capabilities`, `ui::worktree_form`, `ui::material::Text`.
No new crate.

**Storage**: none. The details are held only in the open form and in one tool result (research R9).

**Testing**: `cargo test` through `scripts/gate.sh` (cloud) / `mise run gate`, with `mise run test-core`
for the core model. Core: `crates/micold-core/tests/issue_details.rs`, against recorded GraphQL
answers in `crates/micold-core/tests/fixtures/issue_details/`. Daemon: `crates/micold-daemon/tests/mcp_create_worktree.rs`,
with GitHub stubbed through the injected `FakeIssueSource` (`with_details`, plus a delayed answer)
and a shortened bound. Client: reducer and shell tests in `crates/micold-client/src/main_tests.rs`
and the form's existing test module. View glue: quickstart Part B (`visual-pass`).

**Target Platform**: Linux, macOS, Windows. `gh` is found as today (`locate_gh_on_host`); no shell
stubs; the fake is an in-process trait object.

**Project Type**: desktop app with a daemon (three crates: `micold-core`, `micold-daemon`, `micold-client`).

**Performance Goals**: one `gh` run per read in the common case. The read is abandoned after 10 s
(FR-011): a tool call returns at most 10 s later than without this feature (SC-005), and **Create**
does not wait at all.

**Constraints**: no request to GitHub without a GitHub issue (FR-010, SC-006). Only owner, name and
number leave the machine (FR-009). No credentials are handled. Untrusted text is neutralised (FR-013).

**Scale/Scope**: one core module (about 600 lines with tests), one daemon change, one form state plus
message, one view section, a tool description and two user-guide sections.

## Constitution Check

*GATE: passes before research and after design.*

| Principle | Verdict | Note |
|---|---|---|
| I. Test-First | PASS | The parser, counts, caps, neutralising, JSON shape, display lines, the form's stale-answer rule and the tool's bound all sit in tested core, daemon or reducer code, written test-first. Only the view function that draws ready-made lines uses the GUI-glue exception (quickstart Part B). |
| II. Multi-Session Support | PASS | Nothing is persisted. Each tool call's read belongs to that call; each window's form holds its own state (spec Edge Cases). |
| III. Worktree Integration | PASS | Creation is unchanged: `ops::create_worktree` for both paths. The read never gates or alters it (FR-006). |
| IV. Local-First Storage | PASS | The read runs only when the user picks a GitHub issue or an agent passes `github_issue`, the same per-use opt-in 034/550 established. It sends only owner, name and number, and stores nothing. |
| V. Rust + iced Stack | PASS | `Kind<T>` (read or not readable), `RelatedList<T>` (shown ≤ 20 plus totals), `PlainText` (constructible only through the neutraliser) and `Reference` (local or other repository) make the invalid states unrepresentable. Built with Rust and iced only. |
| VI. Cross-Platform Parity | PASS | Same `GhCli` runner and locator as 034/550, with no OS branching in the new code. Tests run on all three CI hosts. |
| VII. Documentation First-Class | PASS | `docs/user-guide/agent-tools.md` (the `create_worktree` row and section), `docs/user-guide/worktrees-and-sessions.md` ("From a GitHub issue") and the tool description ship in the same change (FR-014). |
| VIII. Reusable UI Component Foundation | PASS | The form section is composed from existing `ui::material::Text` (caption, muted, tinted). No new widget. If one is needed during implementation, it goes into `ui::material` as a builder. |

Complexity Tracking: none; no violation.

## Design

### Core: `micold-core/src/issue_details.rs` (new)

`github.rs` is already 1,800 lines, so the details go in their own module, which reuses that
module's runner and classifier.

- **Model**: see [data-model.md](data-model.md).
- **Query and parse**: see [contracts/details-read.md](contracts/details-read.md). The query is one
  GraphQL document with `first: 20` on every list, plus the summary counts and the first 100
  cross-reference timeline events for linked pull requests. Further timeline pages are read only
  while the deadline allows (research R4). `parse_details(stdout, repo)` returns `Result<IssueDetails, IssueLoadError>`. A whole
  answer with no `data.repository.issue` is an `IssueLoadError`. An `errors[]` entry whose `path`
  starts with a kind's field marks that kind `NotReadable(reason)`.
- **Trait**: `IssueDetailsSource { fn read_details(&self, repo, number, deadline) -> Result<IssueDetails, IssueLoadError> }`.
  `IssueLookup` (daemon side) and `IssueSource` (form side) both take it as a supertrait, so the
  daemon's injected `Arc<dyn IssueLookup>` and the client's `Arc<dyn IssueSource>` can each read
  details with no new factory. Implemented by `GhCli` (runs bounded by the time left before the
  deadline) and by `FakeIssueSource` (`with_details(result)`, `with_details_delay(d)`, `details_calls()`).
- **Outputs**: `IssueDetails::to_json()` (tool) and `IssueDetails::display_lines()` (form); both obey
  "only kinds that are present or not readable".
- **Neutralising**: `PlainText::new(raw)` drops the characters `description_from` drops (control,
  zero-width, direction overrides; joiners kept between visible characters). The shared character
  rule moves into one `pub(crate)` function used by both, with no copied block.

### Daemon: `create_derived_worktree`

`resolve_naming` already resolves the GitHub repo. It now also returns `Option<PendingDetails>`, a
`JoinHandle` started with `spawn_blocking` right after the remote is chosen, before the lookup runs.
After `create_with_optional_session` and `worktree_row`, the handle is awaited with
`tokio::time::timeout` up to `details_bound` (10 s; settable in tests through
`DaemonState::set_details_bound`, as `first_prompt_bound` is). The outcome, or the FR-007 failure
sentence on timeout, is inserted as `row["issue"]`. If any earlier step returns `Err`, the handle is
dropped: the refused call reports no details. A literal or non-`github_issue` derived request never
creates a handle (FR-010).

### Client: form state, shell, view

- `WorktreeForm.issue_details: IssueDetailsState { Idle, Loading { seq, number }, Loaded { number, details }, Failed { number, message } }`,
  data-model §6.
- The pick is set in one place: `WorktreeForm::set_picked_issue(Option<u64>)`, used by `issue_picked`
  and `reset_issues`, and by any other writer of `picked_issue`. It bumps `details_seq` and moves the
  state to `Loading` for a GitHub pick, or to `Idle` for a cleared pick or a Jira pick.
- `State::issue_details_request() -> Option<DetailsRequest { seq, number, repo, gh }>`. The shell
  compares it before and after each form message, as `update_then_follow` does for descriptions, and
  starts `Task::perform` on the blocking pool when it changed.
- `Msg::IssueDetailsLoaded { seq, number, result }` applies only when the state is
  `Loading { seq, number }` with both equal (FR-012). A closed form has no state, so the answer is
  dropped.
- `submitted` is untouched. The details are never consulted (US2-4, FR-006).
- View: `issue_details(form, r)` under the picker. It pushes one `Text` row per display line: a
  caption label, then the items as body text, muted, or tinted with `r.error` and led by the word
  "Blocked" when the line is a warning. A failure is one muted caption. `Loading` shows nothing
  (research R7).

### Documentation (FR-014)

- `create_worktree` description and `github_issue` schema text: the result's `issue` object reports
  the issue's relationships.
- `docs/user-guide/agent-tools.md`: the table row, plus a short "What a `github_issue` call reports"
  subsection.
- `docs/user-guide/worktrees-and-sessions.md` ("From a GitHub issue"): what the form shows after a
  pick, the blocked warning, the 20-item cap, "not readable", and that **Create** never waits.

## Test Strategy

| Layer | Where | What |
|---|---|---|
| Core unit | `crates/micold-core/tests/issue_details.rs` + fixtures | Parsing every kind. Blocked exactly when open blockers > 0. Sub-issue closed/total. Cap 20 with "N more". Hidden count. `owner/name#N` cross-repo references. Closing-and-mentioning PR listed once. Empty project fields left out. Per-kind not readable from an error path. Whole-read errors mapped to `IssueLoadError`. Neutralising. `to_json` omission. `display_lines` content and warning flag. Query arguments carry only owner, name and number. |
| Daemon integration | `crates/micold-daemon/tests/mcp_create_worktree.rs` | US1 1-6 through the real tool path with a fake. US3-1, US3-2, US3-4. A hanging fake with a 200 ms bound: the call returns within bound plus slack and reports the failure. A refused (collision) call has no `issue`. A literal call makes no details call. The 550/532 suites unchanged (SC-004). |
| Client reducer and shell | form tests, `main_tests.rs` | A pick moves to Loading and the shell starts one read. A stale `seq`/number is dropped. A cleared pick goes Idle. A closed form drops the answer. Create during Loading submits at once. A failure shows one line. A Jira pick and the New/Existing sources start no read (SC-006). |
| View | quickstart Part B | The blocked warning reads as a warning. The list layout. The failure line. |

## Project Structure

### Documentation (this feature)

```text
specs/837-issue-relationship-metadata/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── details-read.md      # the gh GraphQL request and its parse
│   ├── tool-result.md       # create_worktree's `issue` object
│   └── form-details.md      # form state, messages, display lines
└── tasks.md                 # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── issue_details.rs         # new: model, query, parse, to_json, display_lines, PlainText
├── github.rs                # IssueDetailsSource supertrait on IssueLookup/IssueSource; GhCli + Fake impls; shared invisible-char rule
└── lib.rs                   # pub mod issue_details
crates/micold-core/tests/
├── issue_details.rs         # new
└── fixtures/issue_details/  # new: recorded GraphQL answers
crates/micold-core/src/mcp/tools.rs     # create_worktree description + github_issue schema text
crates/micold-daemon/src/
├── mcp/tools.rs             # PendingDetails: start, await under bound, insert row["issue"]
└── state.rs                 # details_bound + set_details_bound
crates/micold-daemon/tests/mcp_create_worktree.rs
crates/micold-client/src/
├── features/worktree_form.rs  # IssueDetailsState, set_picked_issue, Msg::IssueDetailsLoaded, issue_details_request
├── shell/issues.rs            # start the read when the request changes
├── ui/worktree_form.rs        # issue_details section (glue)
└── main_tests.rs
docs/user-guide/agent-tools.md
docs/user-guide/worktrees-and-sessions.md
```

**Structure Decision**: the existing three-crate workspace. Logic goes in `micold-core` so the daemon
and the client share one reading, the daemon change stays in its tool module, and the client follows
the existing reducer/shell/view split.

## Complexity Tracking

None.
