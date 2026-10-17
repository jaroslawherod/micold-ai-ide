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
- **Next step**: M1 full gate, push, PR

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
| M1 review A (code-review high) | 1 | a788a747a73dad0b4fd79f50fe1e929c9fa9b067:a9d57c2caafecd4e3dbdade759c913e7ede04768 | CHANGES (F1 MAJOR nested scrollable, F2 MAJOR other main-area openers leave Overview open, F3/F4 MINOR fixed; F5 F6 MINOR declined) |
| M1 review A (code-review high) | 2 | a788a747a73dad0b4fd79f50fe1e929c9fa9b067:a9d57c2caafecd4e3dbdade759c913e7ede04768 → c75653a03639e42b0f8ff27ed47da14a6f605c1e:dd716a91e83a5147a1ed8428014219cbdce9dedb | CLEAN (sonnet, scoped; 1 MINOR: Toggled unguarded, for M3 shortcut) |
| M1 review B (conformance) | 1 | c75653a03639e42b0f8ff27ed47da14a6f605c1e:dd716a91e83a5147a1ed8428014219cbdce9dedb | CHANGES (F1/F2 MAJOR missing assertion reds in cycle-log, fixed by mutation reds; F3/F4 MINOR log headings and T002 run, fixed) |
| M1 visual pass | 1 | dd716a91 | B15 states/grid/greyscale PASS, B1 reduced PASS; type word only in tooltip FAIL (FR-002), fixed test-first |

## Declined review findings

- Tasks round 1 F4 (MINOR, T056/T057 fold their tests into the implementation task): each task
  already says test first and names the test file; splitting is taste, left to the M5 implementer.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | F5 MINOR empty `hook_activity`/`task_progress` modules | T001 registers them in M1 by design (tasks.md); M5/M6 fill them |
| M1 | A r1 | F6 MINOR `CardModel` clone per layout pass | 30 cards of short strings; SC-007 bound met; not worth an `Rc` API now |

## Handover

M1 unit 2. All T001–T023 ticked; review A (2 rounds, CLEAN), review B (round 1, fixed), visual pass
(fixed) done. Full gate (`scripts/gate.sh`) was red twice for repo invariants (material_boundary
`.size(` in a test; root_is_routing_only: Overview close moved to `Outcome::MainAreaTaken`), both
fixed; third full gate running on the committed tree. Next: if green, push the branch, write the PR
title/body (draft in the unit scratchpad `pr-body.md`, `GATE_LINE` placeholder), set this section
to None. If red: fix, rerun `scripts/gate.sh`. No PR open yet.

## Open escalation

None.

## Follow-ups not done

None yet.
