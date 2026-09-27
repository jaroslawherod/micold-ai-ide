---
feature: 033-directory-aware-start-affordance
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 10 # US1 1–5, US2 1–2, US3 1–3
planned_at: 7cca563c
updated_at: 7cca563c
suite_baseline: pending # measured by the first M1 cycle on its own base; see cycle-log.md
---

# Test List: The start affordance answers for its own directory

## Outer loop: acceptance behaviors

The profile's acceptance runner (`sandbox_real_*`) drives the container runtime and cannot see a
sidebar row. So these are **client integration tests over the real update loop**. They run in
`crates/micold-client/src/main_tests.rs`: messages go through `update_inner` with the outbox harness
(`connected_with_outbox`, `availability_asked_for`), and results are read through the same
render-free readers the sidebar calls. That is the highest entry point the repository can test
without a display. The rendered half is quickstart §B, run by `visual-pass` (T028). Every test is
named with `availability` so the milestone Verify filter selects it.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | Home answers `[claude]` and `P` answers `[claude, pi]`, with no Settings or list opened. Once connected and answered, `P`'s Default and worktree rows offer the choice, and `P`'s start list includes Pi | US1-1, FR-001, FR-006, SC-001 | example | TODO | `availability_a_project_whose_script_adds_a_cli_offers_it_on_its_rows` |
| A2  | Open `Q` (answers `[claude]`): its rows offer no choice, and the primary press yields `Start(ClaudeCode)` | US1-2, FR-001 | example | TODO | `availability_a_project_without_the_script_offers_no_choice` |
| A3  | With `P`'s rows offering Pi, `Settings(Opened)` and its home answer `[claude]` leave `P`'s rows offering the choice | US1-3, FR-002, FR-008 | example | TODO | `availability_opening_settings_never_replaces_a_rows_answer` |
| A4  | After a reconnect, `P`'s rows are re-asked with no user action, and offer Pi again once the answer arrives | US1-4, FR-011 | example | TODO | `availability_a_reconnect_reasks_every_row_and_restores_its_answer` |
| A5  | With default `Pi` and only `P` resolving it, the primary press on `P`'s row yields `Start(Pi)` with no list | US1-5, FR-010 | example | TODO | `availability_the_primary_press_starts_a_default_only_the_row_provides` |
| A6  | Worktree A answers `[claude, pi]` and B `[claude]`. After A's list is opened, answered and closed, B offers no choice and its primary press yields `Start(ClaudeCode)` | US2-1, FR-002 | example | TODO | `availability_opening_one_rows_list_does_not_change_another_row` |
| A7  | Same worktrees: opening B's list leaves A offering the choice | US2-2, FR-002 | example | TODO | `availability_opening_the_other_rows_list_leaves_the_first_alone` |
| A8  | With `P` offering Pi through the script, saving env-include off in this window re-asks every row, and `P`'s rows stop offering Pi once the answers arrive | US3-1, FR-004, SC-005 | example | TODO | `availability_saving_env_include_refreshes_every_row` |
| A9  | With every row answered, unrelated messages (hover, scroll, a redraw's `view`) send no availability request, and no source site outside contract C1 asks | US3-2, FR-006, SC-003 | example | TODO | `availability_nothing_is_asked_while_nothing_changes` + U37 |
| A10 | A row whose answer is held is re-asked when its start list opens, and a newly installed CLI in the new answer is offered there | US3-3, FR-004 | example | TODO | `availability_opening_a_rows_list_refreshes_its_answer` |

## Inner loop: unit behaviors

### `crates/micold-client/src/features/session.rs`: `AvailabilityAnswers`

Tests: `crates/micold-client/tests/directory_availability.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | `asked(req, Dir(d))` then `answered(req, a)` stores `a` for `d` and returns `true` | FR-003, R1 | example | DONE | `directory_availability.rs::an_answer_is_filed_under_the_directory_its_request_named` |
| U2  | Same directory: `answered(older_req)` after `asked(newer_req)` is dropped and returns `false`, and `answered(newer_req)` is kept (both sides of the boundary) | FR-009 | example | DONE | `directory_availability.rs::a_late_answer_to_an_older_request_for_the_same_directory_is_dropped` |
| U3  | Two directories answered in either order are both held | FR-009, FR-002 | example | DONE | `directory_availability.rs::answers_for_different_directories_are_all_kept_in_any_order` |
| U4  | `answered(req)` for a `req` never asked is dropped and returns `false` | FR-011, R4 | example | DONE | `directory_availability.rs::an_answer_to_an_unknown_request_is_dropped` |
| U5  | A `Home` answer changes no `Dir` answer, and a `Dir` answer does not change `home()` | FR-002, FR-008 | example | DONE | `directory_availability.rs::home_and_directory_answers_never_replace_each_other` |
| U6  | `for_dir(d)`: `d`'s answer when held, the home answer when not, `None` when neither | FR-005 | example | DONE | `directory_availability.rs::a_directory_reads_its_own_answer_and_falls_back_to_home` |
| U7  | `retain(wanted)` drops unwanted `Dir` answers and requests and keeps `Home`. A request to a pruned directory answered later is dropped | FR-003, FR-012 | example | DONE | `directory_availability.rs::retain_drops_answers_and_requests_for_directories_no_row_has` |
| U8  | `unasked(wanted)` lists each wanted directory once, excluding ones held or in flight | FR-006 | example | DONE | `directory_availability.rs::unasked_lists_only_directories_neither_held_nor_asked` |
| U9  | `clear()` drops every answer (home too) and every in-flight request | FR-011 | example | DONE | `directory_availability.rs::clear_forgets_every_answer_and_request` |
| U10 | `env_include_changed(s)`: `false` when `s == asked_under`, `true` and recorded when different or unset | FR-004, R6 | example | DONE | `directory_availability.rs::env_include_changed_reports_only_a_real_change` |

### `crates/micold-client/src/features/session.rs`: readers, and `app.rs` `location_dir`

Tests: `crates/micold-client/tests/features_session.rs` (extended) and `directory_availability.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U11 | `start_affordance_offers_a_choice(d)` is false with one CLI in `d`'s answer and true with two (boundary 1/2), whatever home holds | FR-001, 026 FR-006 | example | TODO | `the_chevron_follows_the_rows_own_answer` |
| U12 | `start_intent(Primary, d)` is `Start(default)` when `d`'s answer lists the default and home does not | FR-001, FR-010 | example | TODO | `the_primary_press_reads_the_rows_answer` |
| U13 | `start_intent(Primary, d)` is `OfferChoice { unavailable_default: Some(default) }` when `d`'s answer lacks the default and home has it | FR-010 | example | TODO | `a_default_missing_in_the_row_opens_its_list_marked` |
| U14 | A held **empty** answer for `d` yields `NothingAvailable` and does not fall back to home. Nothing held anywhere also yields `NothingAvailable` | FR-005, 027 FR-023b | example | TODO | `an_empty_answer_is_an_answer_not_a_fallback` |
| U15 | `offered_providers(None)` reads home only, with directory answers held | FR-008 | example | TODO | `settings_reads_home_only` |
| U16 | `location_dir`: `Default` → the active root; `Worktree(d)` → `root/.claude/worktrees/d`; no active project → `None` | FR-007 | example | DONE | `directory_availability.rs::a_row_is_answered_for_the_directory_its_session_would_run_in` |

### `crates/micold-client/src/features/session.rs`: `wanted_availability_dirs`

Tests: `crates/micold-client/tests/directory_availability.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U17 | The wanted set is the active root plus each visible valid worktree's `location_dir` | FR-006, FR-007 | example | DONE | `directory_availability.rs::the_wanted_set_is_the_rows_on_screen` |
| U18 | A hidden agent worktree is excluded, and is included once the reveal control is on | FR-003, SC-004 | example | DONE | `directory_availability.rs::hidden_agent_worktrees_are_not_asked_about_until_revealed` |
| U19 | A `Missing` worktree is excluded | FR-004 (deleted) | example | DONE | `directory_availability.rs::a_worktree_whose_directory_is_gone_is_not_asked_about` |
| U20 | An included worktree is keyed by `location_dir`, not `Worktree::path` | FR-007, R3 | example | DONE | `directory_availability.rs::an_included_worktree_is_keyed_like_its_reader` |
| U21 | No active project yields an empty set | FR-012 | example | DONE | `directory_availability.rs::no_project_no_wanted_directories` |

### `crates/micold-client/src/shell/daemon_sync.rs`, `shell/workspace.rs`, `shell/persist.rs`, `main.rs`

Tests: `crates/micold-client/src/main_tests.rs`, each named with `availability`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U22 | Connect asks `cwd: None` once, and once for each of the D wanted directories, and nothing for hidden or `Missing` worktrees | FR-006, SC-004, C1 A1 | example | TODO | `availability_connect_asks_home_and_each_row_directory_once` |
| U23 | Reconnect clears held answers and re-asks, and an answer to a pre-reconnect request is dropped | FR-011 | example | TODO | `availability_reconnect_drops_answers_and_reasks` |
| U24 | `Settings(Opened)` asks `cwd: None` only | FR-008, C1 A2 | example | TODO | `availability_settings_asks_for_home_only` |
| U25 | `StartMenuOpened` asks for the row's `location_dir`, also when that directory's answer is held | FR-004, C1 A3 | example | TODO | `availability_a_start_list_asks_for_its_own_directory` |
| U26 | Opening a project asks once per new wanted directory, and opening it again asks nothing | FR-006, C1 A4 | example | TODO | `availability_opening_a_project_asks_once_per_directory` |
| U27 | A `CatalogChanged` adding one worktree asks for exactly that directory | FR-004, C1 A5 | example | TODO | `availability_a_new_worktree_is_asked_about_once` |
| U28 | Switching to another project drops the previous project's answers, and switching back asks again | FR-012, FR-003 | example | TODO | `availability_switching_projects_drops_and_reasks` |
| U29 | `ForgetConfirmed` drops the forgotten project's answers | FR-012, C1 A6 | example | TODO | `availability_forgetting_a_project_drops_its_answers` |
| U30 | A `CatalogChanged` turning a worktree `Missing` drops its answer, and one making it valid again asks for it | FR-004 (deleted/recreated) | example | TODO | `availability_a_deleted_then_recreated_worktree_is_asked_again` |
| U31 | `ShowAgentWorktreesToggled` on asks for the revealed agent worktrees, and off drops them | FR-003, C1 A5b | example | TODO | `availability_revealing_agent_worktrees_asks_hiding_drops` |
| U32 | This window's save changing env-include, then its `SettingsChanged` echo, re-asks home and every wanted directory, and held answers stay readable until replaced | FR-004, SC-005, R6 | example | TODO | `availability_this_windows_env_include_save_refreshes_every_row` |
| U33 | Another window's `SettingsChanged` with changed env-include re-asks the same set | FR-004, 029 FR-011 | example | TODO | `availability_another_windows_env_include_change_refreshes` |
| U34 | A `SettingsChanged` changing only `scrollback_lines` asks nothing | FR-004 ("only these events") | example | TODO | `availability_an_unrelated_settings_change_asks_nothing` |

### `crates/micold-client/src/ui/mod.rs`, `ui/sidebar.rs` (rendered surfaces)

Tests: `crates/micold-client/tests/provider_choice_surfaces.rs`,
`missing_cli_is_reported_where_it_is_chosen.rs` (extended).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U35 | The rendered Settings default select lists the home answer, not a held directory answer | FR-008 | example | TODO | `the_settings_select_lists_the_home_answer` |
| U36 | The rendered start list for `P`'s row lists `P`'s answer (Pi included) while home lacks Pi, and the missing-default notice judges by the list's directory | FR-001, FR-008 | example | TODO | `the_start_list_lists_its_rows_answer` |

### Structural (`crates/micold-client/tests/`)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U37 | Every non-test line under `src/` naming `ask_cli_availability`, `sync_cli_availability`, `refresh_cli_availability` or `ClientMsg::AiCliAvailabilityRequest` is a C1 site on the allowlist. None is under `ui/`, and a stale entry fails | SC-003, FR-006, FR-004 | example | TODO | `availability_is_asked_only_on_named_events.rs` (new) |
| U38 | The idle application runs no subscription or timer | SC-003 | example | DONE | `crates/micold-client/tests/idle_subscriptions.rs` (existing; re-run, not rewritten) |

## Invariants and edge cases still to place

None. "Sessions in a container" (027) and "Platforms" (Principle VI) need no new test: the key is
`SessionLocation::cwd` and the service's answer is unchanged. A5's `source` stamping keeps its
existing test in `cli_availability_comes_from_the_service.rs`.

## Out of scope

- The daemon's availability decision and its environment-include cache, including its missing
  single-flight (spec Out of Scope, research R11).
- Observing CLI installs as they happen (spec Out of Scope).
- A per-project default CLI (spec Out of Scope).

## Verification commands

From `.specify/memory/tdd-profile.md`, run from the workspace root:

- single: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- file: `scripts/build-lock.sh cargo test --test {file}`
- suite: `scripts/build-lock.sh cargo test --workspace`
- binary-module tests (`main_tests.rs`): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide availability`
