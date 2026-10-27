# Implementation Plan: Readable worktree titles without a GitHub issue title

**Branch**: `claude/project-thread-t2a600` | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1504-readable-worktree-titles/spec.md`

## Summary

Today a worktree's location title is `overview_text::card_title(issue_title, branch,
display_name)`, with the issue title read only for a numeric ticket in the folder name, asked once
per run. This feature replaces it with one pure chooser in core, `location_title::choose`, over
eight sources in FR-001's order, fed from state the window already holds (GitHub titles, Jira
board, pull request reading, session list, worktree list) plus one new off-thread read per worktree
listing (spec heading, own commit subject). The title ticket also comes from the branch name; Jira
summaries ride the existing status request; failed GitHub reads are retried on existing occasions
after 5 minutes. The source is named in the row tooltip, header tooltip and expanded card. See
[research.md](research.md), [data-model.md](data-model.md),
[contracts/location-title.md](contracts/location-title.md).

## Technical Context

**Language/Version**: Rust stable (mise-managed), workspace edition.

**Primary Dependencies**: iced 0.14 (unchanged use); existing `micold_core::{naming, task_progress,
git::Git, review::base, jira_status, occupancy, overview_text, context_header, pull_request}`;
`micold-client` `features::{overview, sidebar, jira_status, worktree, pr_status}`,
`shell::{issues, jira_http, jira_refresh}`. No new crate.

**Storage**: N/A. In-memory per-window state only; nothing persisted.

**Testing**: `cargo test` core units (`location_title`, `jira_status`, `git` with a temp repo);
client reducer/feature tests (`tests/features_overview.rs`, `tests/overview.rs`,
`tests/features_sidebar.rs`, `tests/features_jira_status.rs`, `tests/jira_status_sidebar.rs`,
`src/main_tests.rs` for the shell's answer mapping); caller-count tests unchanged; quickstart §B
visual pass.

**Target Platform**: Linux, macOS, Windows.

**Project Type**: desktop app (iced), crates `micold-core` (render-free), `micold-client`.

**Performance Goals**: no UI-thread I/O for titles (SC-005); the per-listing read is one bounded
`git log` and one small file read per worktree.

**Constraints**: offline-capable (every source but the two issue reads is local); no new polling,
timer or protocol message (FR-005, SC-004); titles never logged (FR-011).

**Scale/Scope**: one new core module, one Git trait method, one extended board, one new client
message, one new shell module; three tooltip surfaces.

## Constitution Check

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision is in core (`location_title`, `jira_status`, `Git`) or a render-free reducer (`features::overview`, `features::jira_status`), each test-first (Test strategy). The shell glue (`shell/location_titles.rs`, the `main.rs` listing hook) only starts the tested reads. |
| II. Multi-Session | PASS | No session state added; the session source only reads the main session's label. Title state is per window and per project, cleared on switch. |
| III. Worktree Integration | PASS | Spec and commit reads run against each worktree's own path (`Worktree.path`); no git write; Default keeps today's title. |
| IV. Local-First | PASS | Spec, commit, PR, session, branch sources are local or already-held data. The only network reads are the existing GitHub title read (now also for branch tickets, under the GitHub issues switch) and the existing Jira status request (one more field, branch-only keys, under the Jira switch and disclosure). Fully usable offline: the chooser falls through. |
| V. Rust + iced | PASS | Rust only. `TitleTicket` and `TitleLookup::Failed { at }` make "a ticket that is neither a number nor a key" and "a retry without a failure time" unrepresentable. |
| VI. Cross-Platform Parity | PASS | Paths via `Path::join`; git via the existing `GitCli` (bounded, cross-platform); ticket parsing is string-only; CRLF/BOM handled in `first_line`. CI builds all three. |
| VII. Documentation | PASS | `docs/user-guide/overview.md` *Title* section and `docs/user-guide/worktrees-and-sessions.md` row title text updated in the same change; 813 FR-015 and 1120 US2 amended per FR-013. |
| VIII. Reusable UI Components | PASS | No new widget: the title text and tooltip strings feed the existing `TreeItem::row_tooltip`, the card's expanded text and the header tooltip. |

No violations; Complexity Tracking empty. Re-checked after Phase 1 design: unchanged.

## Design

### D1 Chooser (FR-001, FR-007, FR-012)
New `crates/micold-core/src/location_title.rs` (registered in `lib.rs`): `TitleTicket`,
`TitleSource`, `TitleInputs`, `LocationTitle`, `choose`, `first_line`, `spec_title`,
`title_ticket`, `read_spec_title` (data-model). `overview_text::card_title` is removed and its tests
move to `choose`. Client helper `features::overview::location_title(app, worktree, display_name)`
gathers inputs (R8, R9, `State::issue_title`, `State.facts`) and is the one caller: `ui/overview.rs::card`
(worktree places; Default keeps `choose` with only `display_name`, giving `Folder`),
`features::sidebar::row_title` (custom name still wins), and through `CardModel.title` the header.
`CardModel` gains `title_source`.

### D2 Title ticket (FR-002, FR-003)
`location_title::title_ticket` (research R2). `features::overview::numeric_ticket` callers that
mean *title* (`request_titles`, `known_title`) switch to it; those that mean the *linked* ticket
(progress `feature_dir`, `worktree_tags`, `linked_keys`, issue link) stay on the folder.
`issues::worktrees_fingerprint` hashes `(dir_name, branch)` so a branch change re-asks.

### D3 GitHub title reads and retry (FR-003, FR-009)
`TitleLookup` gains `NoIssue` and `Failed { at }` (R3). `request_titles(app, worktrees)` takes the
worktrees' GitHub title tickets; seeds from the form's held issues as today; `titles_to_fetch(...,
now)` adds due retries. `on_title_remotes` and `read_title` map answers per the R3 table, stamping
`Failed` with `features::jira_status::now()`'s seconds (the shared Unix clock). No new occasion.

### D4 Jira summaries (FR-004)
`jira_http::read_chunk` asks `["status", "summary"]`; `parse_statuses` → `KeyedIssue`;
`StatusBoard` keeps summaries (R4). `features::jira_status::State::plan` adds
`branch_title_keys(worktrees, project)` as summary-only keys. Chip and status set unchanged.
Contract: L6. The existing `jira_http_adapter` body test asserts `summary` in `fields`.

### D5 Spec and commit facts (FR-006, SC-005)
`features::worktree::State.listing` bumped in `app::State::set_worktrees`. In `main.rs`'s
after-update block (beside the fingerprint check) a changed `listing` calls new
`shell/location_titles.rs::read_facts(app)`: per valid worktree, one `Task::perform` +
`spawn_blocking` (deadline as `shell/changes.rs` `READ_DEADLINE`) running
`location_title::read_spec_title(&path, naming::ticket(dir))` and
`git.own_commit_subject(&path)` (skipped without a branch), answering
`Msg::TitleFactsRead { project, dir, listing, facts }`. No local git → no read. Reducer drops
stale/removed answers (R5, L7). New `Git::own_commit_subject` in `crates/micold-core/src/git.rs`
(`GitCli` via `review/git.rs` `read` + `review_base`; `FakeGit::with_own_commit`).

### D6 Tooltips (FR-008, US2.6)
`TitleSource::words` (L3). `features::sidebar::with_title_line(node, &LocationTitle, tooltip)`
inserts the words after the title line (and keeps today's tooltip for `Folder`);
`context_header::HeaderText::of` adds the words line; `overview_text::ExpandedText` gains `source`,
drawn in `ui/material/overview_card.rs` beside the display-name line (research R6).

### D7 Privacy (FR-011)
Hand-written `Debug` without text for `LocationTitle`, `TitleFacts`, `KeyedIssue`, and the board's
summaries; `title_failure_line` unchanged (number + fixed reason); facts read logs no text.

### D8 Spec amendments (FR-013)
`specs/813-worktree-overview/spec.md` FR-015 and `specs/1120-overview-card-readability/spec.md`
US2 scenarios 1–2 get an *Amended by 1504* note pointing to FR-001/FR-008 here.

## Requirement map

| FR / SC | Design | Contract | Data model |
|---|---|---|---|
| FR-001 | D1 | L1 | `choose`, `TitleInputs` |
| FR-002 | D2 | L2 | `TitleTicket`, `title_ticket` |
| FR-003 | D2, D3 | L5 | `TitleLookup` |
| FR-004 | D4 | L6 | `KeyedIssue`, `StatusBoard` |
| FR-005 | D1 (R8, R9) | L8 | `TitleInputs` |
| FR-006 | D5 | L7 | `TitleFacts`, `listing`, `own_commit_subject` |
| FR-007 | D1 | L1 | `location_title` helper |
| FR-008 | D6 | L3 | `TitleSource::words` |
| FR-009 | D3 | L5 | `TitleLookup::Failed` |
| FR-010 | D1 (derived per render), D5 | L1, L7 | — |
| FR-011 | D7 | L9 | `Debug` impls |
| FR-012 | D1 | L4 | `first_line`, `spec_title` |
| FR-013 | D8 | — | — |
| SC-001..003 | D1–D3 | L1, L2, L5 | — |
| SC-004 | D3, D4 | L8 | — |
| SC-005 | D5 | L7 | — |

## Test strategy

| Requirement | Layer |
|---|---|
| FR-001, FR-012, L1, L4 | core unit: `location_title::choose` one test per source and per skip (empty, whitespace, multi-line, BOM, CRLF); `spec_title` heading/prefix/placeholder |
| FR-002, L2 | core unit: `title_ticket` table (L2 rows); client: Jira chip and `linked_keys` unchanged for a branch-only key (`tests/jira_status_sidebar.rs`) |
| FR-003, FR-009, L5 | client reducer (`tests/overview.rs`, `tests/features_overview.rs`): `titles_to_fetch` with branch tickets, `Failed` before/after 300 s, `NoIssue` never re-asked, `Pending` never re-asked; `src/main_tests.rs`: `on_title_remotes(None)` → `Failed`, no GitHub remote → `NoIssue`, `read_title` `Ok(None)` → `NoIssue`, `Err(Load)` → `Failed` |
| FR-004, L6 | core unit: `parse_statuses` reads summary, `StatusBoard` summary-only keys get no status, failure keeps summaries until retired; client: `jira_status::State::plan` asks branch-only keys; `tests/jira_http_adapter.rs`: request fields include `summary` |
| FR-005, SC-004, L8 | `tests/issues_are_requested_only_on_named_events.rs`, `tests/jira_is_requested_only_on_named_events.rs` unchanged and green |
| FR-006, SC-005, L7 | core: `GitCli::own_commit_subject` on a temp repo (own commit, fresh branch, detached, no default branch); `read_spec_title` on a temp dir; client reducer: `TitleFactsRead` stale/removed/other-project dropped, `set_worktrees` bumps `listing` |
| FR-007 | client: card model title == sidebar `row_title` for each source (`tests/features_sidebar.rs` + `ui/overview.rs` unit) |
| FR-008, L3 | core unit: `TitleSource::words`; client: `with_title_line`, `HeaderText::of`, `ExpandedText::of` lines |
| FR-010 | client reducer: PR opened / session archived changes the derived title with no other message |
| FR-011, L9 | core + client unit: `Debug` of `LocationTitle`, `TitleFacts`, `KeyedIssue`, board omit the text |
| Layout | geometry gate `tests/layout_snapshot.rs` if the expanded card's new line changes its measured height; showcase overview section poses a card per source |
| Visual | quickstart §B (all three platforms) |

## Documentation

`docs/user-guide/overview.md` *Title*: the source order, branch tickets, Jira summaries, retry
after 5 minutes, where the source is shown. `docs/user-guide/worktrees-and-sessions.md`: the row's
first line and tooltip. 813/1120 amendment notes (D8).

## Project Structure

### Documentation (this feature)

```text
specs/1504-readable-worktree-titles/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
├── contracts/location-title.md
└── tasks.md                      # tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── location_title.rs     # NEW: TitleTicket, TitleSource, LocationTitle, choose, spec_title, title_ticket
├── lib.rs                # register module
├── overview_text.rs      # card_title removed; ExpandedText.source
├── overview.rs           # CardModel.title_source
├── context_header.rs     # title tooltip with source line
├── task_progress.rs      # feature_dir/read_text reused (read_text pub(crate))
├── git.rs                # Git::own_commit_subject (+ FakeGit)
├── review/git.rs         # GitCli impl via review_base + read
└── jira_status.rs        # KeyedIssue, summaries, begin(status, summary keys)
crates/micold-client/src/
├── features/overview.rs  # TitleLookup states, TitleFacts, Msg::TitleFactsRead, titles_to_fetch(now), location_title helper
├── features/sidebar.rs   # row_title / with_title_line take LocationTitle
├── features/jira_status.rs # branch_title_keys, plan
├── features/worktree.rs  # State.listing
├── app.rs                # set_worktrees bumps listing
├── shell/issues.rs       # request_titles by title ticket, answer mapping, fingerprint with branch
├── shell/jira_http.rs    # fields + summary
├── shell/location_titles.rs # NEW: read_facts
├── main.rs               # listing hook
├── ui/overview.rs        # card uses location_title
├── ui/sidebar.rs         # row uses location_title
└── ui/material/overview_card.rs # expanded source line
docs/user-guide/overview.md, docs/user-guide/worktrees-and-sessions.md
specs/813-worktree-overview/spec.md, specs/1120-overview-card-readability/spec.md (amendment notes)
```

**Structure Decision**: extend the existing overview/sidebar/jira modules; one new core module and
one new shell module. No protocol change, no new dependency.

## Complexity Tracking

None.
