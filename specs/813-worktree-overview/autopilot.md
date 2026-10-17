# Autopilot ledger — #813 worktree-overview

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #813 "feat: worktree overview with state-aware cards (sessions, PRs, task
  progress)" (title and body as given to the spec unit); labels: enhancement, effort:high,
  flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #813
- **Worktree branch**: claude/project-thread-2lmupk
- **Started**: 2026-10-17
- **Phase**: milestones
- **Next step**: M1 implement (T001–T023)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #927 | design (spec, plan, tasks) | merged | 0e3b543ff7eaf73840eea4e07d00a2f7e93e1248 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T023 | full | Overview grid from the top-bar button, session-derived card states | — | in progress |
| M2 | T024–T030 | full | PR counts and merge conflict on cards | — | pending |
| M3 | T031–T039 | full | Ctrl/Cmd+Shift+O and attention badge | — | pending |
| M4 | T040–T049 | full | Card activation and keyboard traversal | — | pending |
| M5 | T050–T060 | full | Task summary and progress on cards | — | pending |
| M6 | T061–T069 | full | Hook-fed agent activity (protocol 49) | — | pending |

M1 holds 18 US1 tasks plus setup and foundational: kept whole because no smaller cut has an
observable deliverable (the grid needs the rules, card, grid, view and button together); US1's PR
counts are split off as M2 along acceptance scenario 3. Polish T070–T071 changes no code: close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Definition of "main session" (issue open question) | Feature 776's main session: the un-archived session marked main, else the most recently active | agent-resolved | crates/micold-core/src/occupancy.rs `main_session`; specs/776-one-session-per-worktree |
| D2 | spec | Is the LLM summary (option 3) in the first iteration? | No: out of scope; options 1 and 2 ship (issue's recommendation) | agent-resolved | issue #813 "Recommended: 1 as the baseline and 2 where hooks are configured" |
| D3 | spec | Card state vs feature 905's location state | Reuse 905's state; Blocked: needs permission → Needs action, Blocked: failed/resumable → Needs attention | agent-resolved | crates/micold-core/src/location_state.rs |
| D4 | clarify | FR-023: Overview in the main pane or as a window overlay? | Main pane, like Changes/Compare (482/483); sidebar and top bar stay; a sidebar selection closes it | agent-resolved | crates/micold-client/src/ui/mod.rs `view`; spec FR-009/FR-012 need the top bar visible |
| D5 | clarify | Overview shortcut while Settings is open | Deferred to plan: follow how Settings yields to other main-pane views | agent-resolved | crates/micold-client/src/ui/mod.rs; feature 027 FR-026 |
| D6 | plan | D5: shortcut / button while Settings is open | Nothing happens; the draft is kept; the button is drawn disabled (the top bar stays above Settings) | agent-resolved | research R2; crates/micold-client/src/ui/mod.rs `view_with` (toolbar above every body); features/settings.rs `saved`/`cancelled` |
| D7 | plan | Red / yellow / green from theme roles (FR-021) | Add `warning*` and `success*` role families as fixed palette tones per scheme, like feature 482's diff roles | agent-resolved | research R11; crates/micold-core/src/tokens/mod.rs `diff_added`; ui/material/style.rs notes no warning role |
| D8 | plan | Issue title source | One-shot cached `GhCli::read_issue` per numeric ticket on Overview open when GitHub issue tooling is available; else branch name | agent-resolved | research R8; spec Assumptions (no fetch only when lookup off/offline); no client title cache exists |
| D9 | plan | PR counts without a new poller | Light `oc`/`mc` aliases (first 100, non-fork nodes) on the existing status query; `mergeable` for conflicts | agent-resolved | research R6; crates/micold-core/src/pull_request.rs `status_query`, `select_pull_request` drops forks |
| D10 | plan | Ledger phase words → card phase | spec/clarify/bug → Spec, plan → Plan, tasks → Tasks, milestones/bugfix/chore → Implement, close → PR, done → Done | agent-resolved | research R9; .claude/skills/speckit-autopilot/templates/autopilot-ledger.md |
| D11 | tasks | Does the start menu anchor to its row when a card opens it? | No as planned (`start_menu_toggled` uses the last row press, stale or (0,0)); `OfferStart` focuses the row's start control, scrolls it into view, measures its bounds with a read-only focus operation and sends `StartMenuAnchored` before `StartMenuOpened` (T042, T046); plan fixed in research R12 | agent-resolved | features/session.rs `start_menu_toggled`; ui/material/split_action.rs anchor rule; ui/focus.rs `into_view` |
| D12 | tasks | Does `on_disconnected` keep the sessions so cards keep their state? | Yes: it removes only `daemon_catalogs[daemon]`, read only by `register_projects`; sessions stay in core state; PR reading is `Released` (unknown); research R14 updated, pinned by T010 | agent-resolved | shell/daemon_sync.rs `on_disconnected`, `register_projects` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c54be34533424e39665e73004ef6ce8f0cadc4e2:992e129808513fd43ef6b1982b4cb1881cdb0921 | CLEAN (3 MINOR: F1 F2 F3 fixed, prose only) |
| Plan | 1 | a87de0fcfc292f91a22bf5eed48bbb38ce5f826e:2b103a91d9ac9fc083447d12fbfc68629dc01877 | CHANGES (F1 MAJOR: alternatives missing in R7/R12/R13/R14, F2 MINOR: ProgressBar vs StageProgress; both fixed, prose only) |
| Tasks | 1 | 0434f4af333fc87f0250ee05e67d0bda684de551:86d6baaac0c3f6bc0d2544cb19bad7681f594131 | CHANGES (F1 MAJOR: M5 Verify gave cargo two filters, fixed; F2 MINOR checklist note, F3 MINOR [P] on T007/T008, fixed; F4 MINOR declined) |
| Tasks | 2 | 5f969cc13bac9d5706f57bb7d0cc3b1de8c919c0:86d6baaac0c3f6bc0d2544cb19bad7681f594131 | CLEAN (sonnet, scoped) |

## Declined review findings

- Tasks round 1 F4 (MINOR, T056/T057 fold their tests into the implementation task): each task
  already says test first and names the test file; splitting is taste, left to the M5 implementer.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 1 (context cap). Branch reset to origin/main 0e3b543; work committed, not pushed, no PR.
- Done and green: T001, T002/T003 (core side), T006–T008 + T015 (`micold-core/src/overview.rs`;
  `location_type(location, included, status)`, `CurrentPr::from_status` (conflict false until M2)).
  Cycle log: `specs/813-worktree-overview/tdd/cycle-log.md`.
- Written, not yet compiled or run (client build was in progress): T004/T005 icons (dashboard
  e871, priority_high e645, visibility e8f4, check_circle e86c, lightbulb e0f0, bug_report e868,
  build e869, folder e2c7, link e157, folder_off eb83) in `src/icons.rs`, `tests/icons.rs`,
  `tests/icons_font.rs`; T009/T016 `Prompt::session` (test `a_queued_prompt_names_the_calling_session`
  in `shell/daemon_sync.rs`; test literals use `SessionId(Default::default())`); T002 client
  contrast test. Next: run `cargo test -p micold-client --lib composition_contrast`, `--test icons
  --test icons_font --test features_agent_confirm`, the bin's daemon_sync test; fix any exhaustive
  `match Icon` / docs icons gate; tick T004, T005, T009, T016; then T010–T014, T017–T023.
- Card sources found: `sidebar_entries()` (Default, Worktree, Group → `RunNode.row`
  `RunRow::Worktree`), `session.active_sessions()`, `pr_status.{enabled,statuses,removable,read_at}`
  (Unknown when `!enabled` or `read_at.is_none()`), `agent_confirm.pending`. `terminal_focused`
  app.rs:431, `main_area_taken` app.rs:461.

## Open escalation

None.

## Follow-ups not done

None yet.
