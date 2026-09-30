# Contract: the issue source in the form, and the Settings section

**Feature**: [spec.md](../spec.md) · Research [R12](../research.md#r12--matching-issues-reuse-micold_coretypeahead-over-one-row-string)–[R14](../research.md#r14--the-settings-editor-composed-from-existing-primitives)

Render glue under `src/ui/` only composes; every decision is in the reducer or `micold-core`
(Principle I's GUI exception, verified by [quickstart §B](../quickstart.md#b-recorded-manual-pass)).

## 1. Source switch (`ui/worktree_form.rs::source_switch`)

Three `ToggleChip`s: **New branch**, **Existing branch**, **GitHub issue**. The third is
`.disabled(true)` unless `form.github` is `Available`. Beneath the row, one muted caption line:

| `form.github` / `form.source` | Caption |
|---|---|
| `Checking` | "Checking for a GitHub remote…" (FR-002) |
| `Unavailable(reason)` | the reason — "This repository has no GitHub remote." or "Couldn't read this repository's remotes: …" (FR-002) |
| `Available(repo)`, source ≠ `Issue` | "GitHub issue reads open issues of **owner/name** from GitHub." — the opt-in notice, shown **before** the choice that starts the load (FR-025, Principle IV) |
| `Available(repo)`, source = `Issue` | none here — the body's notice (§2 item 1) takes over |

`ToggleChip::disabled(bool)` (NEW builder method, shared component): no `on_press` is emitted; the
Material disabled treatment (38% content, 12% container) from existing tokens; no new colour. The
component gallery poses it (`showcase/`), and `material_builder_api.rs` sees a chainable method.

## 2. Issue source body (`BranchSource::Issue`)

Top to bottom:

1. **Notice** (FR-025), always while this source is active: caption
   "Reads open issues of **owner/name** from GitHub." — shown before the load, during it and after.
2. **Picker**, by `IssueList` state:
   - `Loading` → `StageProgress::new("Loading issues from GitHub…")` (FR-006). The switch and the
     inputs below stay live.
   - `Failed` → error caption from `IssueLoadError::message` + **Retry** button (FR-007).
   - `Loaded` with zero issues → caption "owner/name has no open issues." (FR-008).
   - `Loaded` otherwise → `material::Typeahead` (feature 021's component, unchanged) with rows
     `TypeaheadRow::new(issue.row_text, spans)`, `.label("Issue")`,
     `.placeholder("Search by number, title or label")`, `.selected(picked row)`,
     `.empty_message("No open issue matches.")`, `on_pick → IssueRowPicked(index)`, `on_move`, `on_focus`,
     `on_dismiss` — exactly the branch picker's wiring.
   - Under it: when `!listing.complete`, caption "Showing the 1,000 most recently updated of N open
     issues — search also looks on GitHub." (FR-004); `SearchState::Searching` → "Searching GitHub…";
     `SearchState::Failed` → the error + **Retry**.
3. **Type / Ticket / Name** — the very same three controls the New branch source renders (a shared
   private fn `naming_inputs(form, r, focused)`), so a pick visibly fills them and they stay
   editable (FR-011, FR-015; AS4, AS5).
4. **Preview** — unchanged; `preview()` treats `Issue` as `New` (AS4, FR-012).

Submitting runs the **existing** new-branch create path, including branch-conflict pre-flight
(FR-012, AS6). No new create mode exists.

## 3. Shell effects (`crates/micold-client/src/shell/`)

| Trigger | Effect |
|---|---|
| `Msg::Opened` | send `RemoteList` for the active project; when not connected, dispatch `RemotesListed(Err(…))` instead ([remote-list-rpc §3](./remote-list-rpc.md)) |
| `SourceChanged(Issue)` accepted by the reducer (state now `Loading{seq}`) | `Task::perform(spawn_blocking(locate + load_listing))` → `IssuesLoaded{seq, …}` |
| `IssueRetry` accepted | the same, for the load or the search that failed |
| `IssueQueryChanged` leaving `SearchState::Pending{seq}` | `Task::perform(sleep 300 ms)` → `IssueSearchDue{seq}` |
| `IssueSearchDue` accepted (state now `Searching{seq}`) | `Task::perform(spawn_blocking(search_open))` with the `gh` path kept from the load → `IssueSearched{seq, …}` |
| `IssueRowPicked(index)` (view message) | resolve `index` with the pure `State::issue_number_at(index)` (decision logic stays in the reducer module, tested in `issue_source_state.rs`; `None` dispatches nothing), read the mapping from the settings store, dispatch `IssuePicked{number, mapping}` |

Locating `gh` happens inside the load's `spawn_blocking`: the env-include `PATH` for the project
root (cache hit, or resolved there on a miss and returned for the shell to cache —
[github-issue-source §1](./github-issue-source.md)), then process `PATH`, then well-known
directories. `ToolMissing` is reported without spawning `gh`. The `IssueSource` is built from the
located path by the factory in `Capabilities`.

**Only on named events (FR-003).** A gate test,
`crates/micold-client/tests/issues_are_requested_only_on_named_events.rs`, asserts over the shell
source that the issue-source capability is called from exactly the load, retry and search-due arms
— the pattern of `refresh_is_only_on_demand.rs` and `availability_is_asked_only_on_named_events.rs`.

## 4. Settings → GitHub issues (`ui/settings/github.rs`, NEW)

- Rail entry "GitHub issues", `Icon::IssueMapping` (NEW variant, Material Symbols `label` glyph —
  already in the shipped full-coverage font; `icons_font.rs` gate).
- Explanatory text: "When you create a worktree from a GitHub issue, the first entry whose label the
  issue carries sets the worktree's type. Applies to every project."
- One row per entry: `TextField` (label, `FieldId::IssueMappingLabel(i)`, shows the `FieldError`
  when it is the offending one) · `Select<ConventionalType>` · `IconButton(Icon::MoveUp)` (disabled on
  the first row) · `IconButton(Icon::MoveDown)` (disabled on the last) · `IconButton(Icon::Delete)`.
  `MoveUp`/`MoveDown` are NEW `Icon` variants mapped to Material Symbols `keyboard_arrow_up` /
  `keyboard_arrow_down` (`arrow_upward` is already `Icon::NavigateUp`'s codepoint, and no two icons
  share one — `tests/icons.rs`).
- Below: **Add entry**, **Restore defaults** (text buttons).
- Empty mapping: caption "No labels are mapped — picking an issue leaves the type for you to choose."
- Save/Cancel are the Settings view's existing ones (save-together rule).

## 4a. Layout coverage

The layout gates see only states registered in `crates/micold-client/tests/support/covered_states.rs`
(`layout_coverage_registry.rs`). New covered states, with `tests/fixtures/layout_snapshot.txt`
regenerated (`UPDATE_LAYOUT_SNAPSHOT=1`):

| State | Milestone |
|---|---|
| form, GitHub chip disabled with reason | US1 |
| form, Issue source `Loading` | US1 |
| form, Issue source `Failed` + Retry | US1 |
| form, Issue source `Loaded` with rows and the cap caption | US1 |
| form, Issue source `Loaded`, `SearchState::Searching` | US1 AS10 |
| Settings, `GithubIssues` section with three entries and one offending entry | US3 |

## 5. Documentation (Principle VII)

| Page | Section | Milestone |
|---|---|---|
| `docs/user-guide/worktrees-and-sessions.md` | "From a GitHub issue": prerequisites (`gh` + `gh auth login`), which remote is used and that aliased (`insteadOf`) remote URLs are not recognised, what is sent (FR-025), each failure message and its remedy | with the source (US1) |
| same | "Searching beyond the 1,000 loaded issues" | US1 AS10 |
| same | "The issue's labels choose the type" + the default table | US2 |
| `docs/user-guide/settings.md` | "GitHub issues" section | US3 |
| `docs/development/component-library.md` | `ToggleChip::disabled` | US1 |
| `docs/development/architecture.md` | where the issue fetch runs (client, host) and why (R4) | Polish |
