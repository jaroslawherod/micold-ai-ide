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
- **Next step**: M5 implement (T050–T060)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #927 | design (spec, plan, tasks) | merged | 0e3b543ff7eaf73840eea4e07d00a2f7e93e1248 |
| #930 | M1 | merged | 6cf261efc3ae0a1d4e35b1ad157a63644acf327e |
| #932 | M2 | merged | fc417da8a17206a597e17ab2a697126186c3007d |
| #936 | M3 | merged | d93c9dbc687412cced0a9b7854cf44323ee3930e |
| #951 | M4 | merged | 35b8584fbc0734c15ffe2b14bfb67595ec66d9fe |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T023 | full | Overview grid from the top-bar button, session-derived card states | #930 | merged |
| M2 | T024–T030 | full | PR counts and merge conflict on cards | #932 | merged |
| M3 | T031–T039 | full | Ctrl/Cmd+Shift+O and attention badge | #936 | merged |
| M4 | T040–T049 | full | Card activation and keyboard traversal | #951 | merged |
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
| M2 review A (code-review high) | 1 | 6b1655982fe593202311c406f2f377a670100338:c8a49970814fedf44a35826d742201d0fc715771 | 8 findings, all declined (spec-mandated or pre-existing pattern; see Declined) |
| M2 review B (conformance) | 1 | 25f177cecaf3a5717b2501e8410e56404959bb28:4f5c655a6df6e2bc340fd8b1d358f9ffdba67b6c | CLEAN (sonnet; Verify green; 2 MINOR unfixed) |
| M2 visual pass | 1 | 7f83ac90 | B4, B5 PASS (evidence visual-pass/m2/); showcase lacks PR-off card |
| M1 visual pass | 1 | dd716a91 | B15 states/grid/greyscale PASS, B1 reduced PASS; type word only in tooltip FAIL (FR-002), fixed test-first |
| M3 review A (code-review high) | 1 | 88916a9c:84b25cec | CLEAN |
| M3 review B (conformance, sonnet) | 1 | 88916a9c:84b25cec | CHANGES: F1 MAJOR cycle-log lacked M3 (fixed: M3 entries added, red = compile errors), F2 MINOR toolbar mapping untested (fixed: pure fns + test), F3 MINOR settings_open false (confirmed: Settings replaces the body, no terminal pane exists then) |
| M3 visual pass | - | - | not run: no xdotool/sway in the container; notes in visual-pass/m3/notes.md; geometry pinned by layout snapshot |

## Declined review findings

- Tasks round 1 F4 (MINOR, T056/T057 fold their tests into the implementation task): each task
  already says test first and names the test file; splitting is taste, left to the M5 implementer.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | F5 MINOR empty `hook_activity`/`task_progress` modules | T001 registers them in M1 by design (tasks.md); M5/M6 fill them |
| M1 | A r1 | F6 MINOR `CardModel` clone per layout pass | 30 cards of short strings; SC-007 bound met; not worth an `Rc` API now |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

| M2 | A r1 | `UNKNOWN` mergeable read as no conflict | T024 and W5 specify `UNKNOWN`/missing → false |
| M2 | A r1 | `mergeable` in the shared fragment, `oc`/`mc` 100-node aliases | specified by T027 / plan W5; no new poller (FR-019) |
| M2 | A r1 | capped counts include fork nodes | W5's table: 100 nodes → capped; cross-repo not counted |
| M2 | A r1 | stale counts kept on Passing/RateLimited | mirrors `statuses` handling (T025); same as existing pr_status behaviour |
| M2 | A r1 | MINORs: String clone, tuple threading, no re-render at rate-limit end | not worth the churn |
| M4 review A (code-review high) | 1 | db133badc287a5ad6a886585f659bcabbcc09a75:e1cc1b79a0e3ead06504121b7ce58ba7d82f4c0e | CHANGES: F1 MAJOR collapsed-group row never revealed / stale start-menu anchor (fixed: group expanded on reveal, no menu without bounds, test added), F2 MINOR toggle closes open menu (fixed: dismiss first) |
| M4 review B (conformance, sonnet) | 1 | db133badc287a5ad6a886585f659bcabbcc09a75:e1cc1b79a0e3ead06504121b7ce58ba7d82f4c0e | CLEAN (MINOR: main.rs task chains untestable; B8/B9 need a display) |
| M4 visual pass | - | - | not run: no display in the container; quickstart B8/B9 for a human pass |
| M4 client invariants | - | - | 5 repo-wide invariant tests failed on the first diff; fixed (render-free, write isolation, SC-002, material boundary) |
