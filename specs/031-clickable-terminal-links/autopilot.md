# Autopilot ledger — 031-clickable-terminal-links

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: the links shown at terminal or ai cli session should be real links. User should be able to click them and open webside or other related application
- **Kind**: feature
- **Worktree branch**: feat/links-in-terminal-should-be-clickable
- **Started**: 2026-09-14
- **Phase**: 4-milestones
- **Next step**: M5 step 6: wait for `ci complete`, then rebase-merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #336 | Spec | merged | 411711c1 |
| #341 | Design (clarify, plan, tasks, milestones) | merged | 226d3a8b |
| #356 | M1 Link recognition core | merged | 33f6491c |
| #361 | M2 Opening pipeline | merged | 9995dbdb |
| #385 | M3 Clickable web, mail and declared links in the pane | merged | e5bdc804 |
| #393 | M4 Session terminal identity and the FORCE_HYPERLINK opt-in | merged | 59132a50 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T012 | Link recognition core (`micold_core::link`, SC-002 corpus) | #356 | merged |
| M2 | T013–T022 | Opening pipeline: `LinkActivated` through `update_inner` to the system opener | #361 | merged |
| M3 | T082, T083, T037, T023–T034 | Clickable web, mail and declared links in the pane | #385 | merged |
| M4 | T035, T036, T038–T041 | Session terminal identity and the FORCE_HYPERLINK opt-in | #393 | merged |
| M5 | T084, T087, T042–T048, T088, T049–T054 | File links on the host: open, reveal runnables, not-found | — | in progress |
| M6 | T085, T055–T067, T069, T068 | Sandboxed file links, translated and confirmed | — | pending |
| M7 | T086, T070–T077 | Link context menu | — | pending |
| M8 | T078–T081 | Close: SC-005 measurement, full visual walkthrough | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | 1-spec | Spec review still had a MAJOR after 3 rounds: how to proceed? | Run one more fresh review; open PR 1 if clean | decided by user | round-3 findings fixed in spec.md |
| 2 | 1-spec | Round 4 still had 2 MAJOR (fixed): open PR 1 or review again? | Open PR 1 now | decided by user | round-4 findings fixed in spec.md |
| 3 | 1-spec | `specs/030-*` landed on main from another flow (030-settings-rail-slide) during specify: which number? | Renumbered to 031 | agent-resolved | `git ls-tree origin/main specs/` |
| 4 | 2-clarify | Which gesture opens a link? (FR-004) | Ctrl+click on Linux/Windows, Cmd+click on macOS | decided by user | spec.md Clarifications |
| 5 | 2-clarify | Should sessions advertise hyperlink support? (FR-006) | No; document the FORCE_HYPERLINK opt-in | decided by user | Claude Code 2.1.270 detection reads FORCE_HYPERLINK / TERM_PROGRAM list |
| 6 | 2-clarify | May application-specific schemes open? (FR-011) | Never | decided by user | spec.md Clarifications |
| 7 | 2-clarify | Round 2 scan: further critical ambiguities? | None; hover display, notification and confirmation reuse existing tooltip/notification/confirm components (plan-level) | agent-resolved | crates/micold-client/src/ui/confirm_*.rs, features/notifications.rs |
| 8 | 3-design | Plan review round 3 still had 2 MAJOR (fixed): review again or proceed? | Proceed to speckit-tasks | decided by user | round-3 findings fixed in plan/research/data-model/contracts/quickstart |
| 9 | 3-design | US1 is 34 tasks: one milestone or several? | Split into recognition (M1), opening (M2) and pane (M3); M1–M2 unreachable from the UI until M3. Deliberate departure from rule 3: the smallest scenario half (scenarios 1, 2, 4, 5) still needs nearly all of recognition, opening and the pane, so a scenario split would not shrink the first PR; rule 6 keeps M1–M2 unreachable from the UI. M6 kept whole (translation without confirmation would violate FR-018a) | agent-resolved | .claude/skills/speckit-autopilot/references/milestones.md rules 3 and 6; tasks.md ## Milestones |
| 10 | 3-design | Where do outer-loop acceptance tests run, with no GUI end-to-end runner? | `#[cfg(test)] mod acceptance` in shell/links.rs, driving the headless `TerminalPane` through `update_inner` (integration of composed modules); rendered pixels via visual-pass | agent-resolved | .specify/memory/tdd-profile.md (acceptance runner is sandbox-only); crates/micold-client/tests/terminal_size_reporting.rs headless pattern |
| 11 | 3-design | Tasks review round 1: declared-link hover was built in M3 but tested in M4 (tests-after) | Moved T083 (A7–A11) and T037 (U129) into M3 ahead of T028; M4 keeps session identity and the opt-in | agent-resolved | milestones.md rule 7; constitution I |
| 12 | 3-design | Local baseline is red on 2 `micold-daemon` pi tests (exclusivity, pi_launch_wiring): block the loop? | No; recorded as pre-existing, not fixed here — out of flow, and green in CI on main | agent-resolved | tdd/cycle-log.md Baseline; CI run on b27ffe62 success |
| 13 | 3-design | Tasks review round 3 still had 2 MAJOR (fixed): review again or open PR 2? | Open PR 2 now | decided by user | round-3 findings fixed in tasks.md, tdd/test-list.md, research.md R15 |
| 14 | 4-milestones | Which suite is "the full suite" inside an M1 cycle, at ~343 s a workspace run? | The core fast subset (`cargo test -p micold-core --all-targets`) per cycle, since M1 touches only `micold-core`; the workspace suite and `mise run gate` at the milestone's end | agent-resolved | .specify/memory/tdd-profile.md fast-subset note |
| 15 | 4-milestones | U25: a wide char's spacer cell holds a space on the wire (alacritty 0.26 writes `' '`; messages.md calls it meaningless), so `LinkRows` as designed cannot tell it from real text and detection would stop at it | Add `LinkRows::spacer(row, col)`, answered by the client from the style-run flags; the logical line skips spacers in its text and covers them with the char before them on their row. Contract §1 and L5, data-model §1 and T028 updated | agent-resolved | alacritty_terminal 0.26 `Term::input`; specs/010 contracts/messages.md §Wide characters; crates/micold-daemon/src/framer.rs:362,368 |
| 16 | 4-milestones | M1 review A: a detected address nested in one whose start lies past the top cut (`?next=https://…`) was offered, since L7 dropped only a candidate at column 0 | Widen L7's upper edge: drop a candidate when only address characters precede it on the logical line (U151); contract L7 updated | agent-resolved | contract L7 "A truncated address is never recognised (FR-003)"; research R4 "safer than recognising a truncation" |
| 17 | 4-milestones | M1 review A round 2: `detect` rescanned the rest of the line for every candidate that failed, 3.4 s on a capped line of `mailto:` repeated. The suggested fix (skip ahead to the failed scan's end) would stop finding an address nested in a rejected one, which the contract keeps | Precompute each start's stop, the next `'`, `@` and `/`, and the trailing punctuation run in one pass, so each candidate costs O(1) plus its authority (U152); checked against the old `detect` on 200,000 random texts | agent-resolved | research R4 "bounds the work on pathological output"; contract §3 nested addresses |
| 18 | 4-milestones | M1 review A round 3: alacritty writes the end-of-row padding before a wrapping wide char with the cursor template's hyperlink, so hovering it built a one-cell declared link over the plain char before it | `link_at` reads the hyperlink of the lead cell of the char under the pointer (U153); contract L5 says a spacer's own `hyperlink` is not read | agent-resolved | contract L5 "its spacer cell belongs to the same link as its lead cell" |
| 19 | 4-milestones | T021 has `shell/links.rs` run the session reducer itself, but `tests/feature_registration_cost.rs::only_the_root_drives_a_feature` forbids any caller of a feature reducer outside `app.rs` | Add `State::update_session_for_effects` in `app.rs`: it runs the reducer, drains every outcome but `ClipboardWrite` and `OpenLink`, and returns those two for `shell/links.rs` to perform | agent-resolved | tests/feature_registration_cost.rs SC-002, FR-002 |
| 20 | 4-milestones | The user asked that the pointer change only when a link is clickable: on plain hover or only with the link modifier held? | Only while Ctrl (Cmd on macOS) is held; underline and hint still show on plain hover. Spec clarification 2026-09-16; FR-007, US1 scenario 1, research R7, data-model T1, T025, T030, U123, quickstart B.1 updated before M3 | user | user, 2026-09-16: "only while Ctrl is held" |
| 21 | 4-milestones | The hover key in data-model §3 has no scroll position: scrolling the view under a resting pointer would reuse a link from other rows | Add `display_offset` to `HoverKey`/`HoverCache`, so scrolling re-resolves | agent-resolved | terminal_pane.rs `hover_refresh`; U156 `rows_follow_the_scrollback_offset` |
| 22 | 4-milestones | The acceptance tests cannot reach `ui::material` (`pub(crate)`) from the bin crate | Build the pane through the public `ui::terminal::pane`, the same element the app renders | agent-resolved | shell/links.rs `mod acceptance` |
| 23 | 4-milestones | Hint colours | `surface_container_highest` container, `on_surface` text, as `TermPalette::hint_container`/`hint_content` | agent-resolved | ui/terminal.rs |
| 24 | 4-milestones | The hover change needs a repaint, but `idle_requests_no_frames.rs` allows one `request_redraw` in the rendering layer | Edge-triggered `shell.invalidate_widgets()`, as `select.rs` does | agent-resolved | gate failure recorded in tdd/cycle-log.md "Gate fixes: M3" |
| 25 | 4-milestones | Rebase onto main: BUG-007/008 added `press_cell` (a click without a drag selects nothing) on the plain local press, in conflict with the link press arm | Keep the link gesture; the non-link local press sets `press_cell` as main does. A link press dragged off its cell starts its selection already off the press cell, so it needs none | agent-resolved | terminal_pane.rs left-press arm; review B round 2 checked it |
| 26 | 4-milestones | M3 review A round 2 (low): a hover off the grid read no rows, so a resize bringing text under the resting pointer never re-resolved | `hover_refresh` re-resolves on any grid move when the cached hover read no rows (U161) | agent-resolved | tdd/cycle-log.md Cycle 56 |
| 27 | 4-milestones | T038 asks the Windows builders to be built on every OS; should the Unix `bash` builder be too? | No: `#[cfg(not(windows))]`, since `tests/background_spawns_hide_console.rs` recognises only whole-Windows exclusions and the task asks it only of the PowerShell builders | agent-resolved | tdd/cycle-log.md Cycle 60 |
| 28 | 4-milestones | Where does the daemon test's session get a program that dumps its environment, with no real CLI or shell config? | A stand-in script (`claude`, `claude.cmd` on Windows) first on `PATH` for `spawn_ai_cli`, and as `SHELL`/`COMSPEC` for `spawn_shell`, on a re-executed test child | agent-resolved | tests/session_identity_env.rs; review B F3: its Windows arm is first proven by CI on the M4 PR |
| 29 | 4-milestones | M4 review A / B F1: the daemon strip read only the builder's UTF-8 entries | Match the daemon's `vars_os()` lossily plus the builder's entries (U162) | agent-resolved | tdd/cycle-log.md Cycle 63 |
| 30 | 4-milestones | M4 review B F2: contract §2's "inherited and set by the script" row had no real bash diff | Unix re-exec test through `resolve` (U163), guarded by a mutant | agent-resolved | tdd/cycle-log.md Cycle 64 |
| 32 | 4-milestones | M4 CI red on windows-latest: the `.cmd` stand-in cannot be spawned (os error 193), the arm review B F3 flagged | Compile the stand-in with `rustc` into `claude.exe` per test process; a batch file is not an executable image and neither spawn can wrap it in `cmd.exe /C` | agent-resolved | tdd/cycle-log.md Cycle 65; run 35766841670 |
| 31 | 4-milestones | Quickstart §B.17's second half (a *declared* link with `FORCE_HYPERLINK=1`) could not be observed: Claude Code 2.1.280 showed no declared run, and that run showed no hover feedback at all, so the two cannot be told apart. Block M4? | No. FR-006 is what micold removes and documents (Decision 5), which §B.14 shows; `osc8_passthrough.rs` shows a declared link reaching the grid on all three CI OSes, and §B.7 (M3) shows the pane opening one. Recorded as a follow-up to re-run when an AI CLI is known to declare links | agent-resolved | visual-pass.md §"Milestone M4" |
| 33 | 4-milestones | T049 asks for `app::State.host_names`, but `tests/root_state_is_shared.rs` (G2) refuses a flat root field that is neither a feature's `State` nor declared in `SHARED`, and this one is written by no feature | It lives on `features::session::State`, with the sessions whose panes show the links, and boot fills it in `shell/startup.rs` (where `core` is built) rather than in `main.rs`; `main.rs` already writes `core.session.*` at boot the same way | agent-resolved | tests/root_state_is_shared.rs `every_root_field_is_a_feature_struct_or_a_declared_shared_member`; `shell/startup.rs::boot` already writes `core.session.default_ai_cli`, `core.session.pi_activity_component` and `core.settings.theme_pref` at boot the same way |
| 34 | 4-milestones | The acceptance harness ran each published message through `update_inner` but dropped what its task produced, so A17's notification (raised on `LinkOpenFinished`, a second round) could never be observed | `Session::send` drains the queue to a fixed point, as iced's runtime does | agent-resolved | tdd/cycle-log.md Cycle 73 |
| 35 | 4-milestones | T031 built the pane's context inline in `ui/mod.rs` through `material::local_link_context()`; T050 needs `state` and the sandbox | One `ui::terminal::link_context(state, sandbox)`, called by `ui::view` and by the acceptance tests, so A12/A14 cover the glue rather than a copy of it; `local_link_context` stays as `TerminalPane`'s own default | agent-resolved | tdd/test-list.md U134/U136; `shell/links.rs::acceptance::a_file_link_in_a_sandboxed_session_reaches_nothing_and_says_why` covers the sandboxed arm |
| 36 | 4-milestones | M5 review A finding 1: on Windows a `file://` link to `tool.exe.` or `tool.exe%20` carried no runnable extension (`rsplit_once('.')`), so it was handed to the system opener — and Win32 strips trailing dots and spaces, so the opener ran the real `tool.exe` | `action_for` trims trailing dots, spaces and tabs from the name on `HostPlatform::Windows` before it reads the extension | agent-resolved | tdd/cycle-log.md Cycle 75; `runnable.rs::tests::on_windows_a_trailing_dot_or_space_never_hides_a_runnable_extension` |
| 37 | 4-milestones | M5 review A finding 2: C10 says an escape that cannot be decoded is not followable, but `u8::from_str_radix` also reads a sign, so `%+A` decoded to a newline. Should a decoded control character be a path at all? | Both digits must be hex digits, and a decoded path holding any control character is `NotFollowable`: no real path needs one, and a decoded newline would forge a second line in the hover hint | agent-resolved | address.rs `percent_decode`; U32 rows `%+A`, `%-1`, `%00`, `%0A` |
| 38 | 4-milestones | M5 review A finding 5: `link_activated` ignored `needs_confirmation`, which no resolver sets before M6 | The `HostPath` arm refuses a link that needs one, so M6's confirmation cannot be bypassed the day the flag is first set (O3) | agent-resolved | tests/features_session_links.rs `a_host_path_that_needs_confirmation_opens_nothing_yet`, proved by a mutant |
| 39 | 4-milestones | M5 review B finding 1: U136 was recorded as covered by A12, but A12 sets the names itself and nothing called `boot()` | The boot read is now `shell::startup::host_names_at_boot`, with its own test against `gethostname` (red: `left: []`); Cycle 74's wrong "no mutant was needed" note and its swapped U134/U136 ids are corrected | agent-resolved | tdd/cycle-log.md Cycles 74–75 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | B | F3 (MINOR): scrollback arrival does not bump the grid `seq`, so a resting hover over fetched lines is stale until the pointer moves | Kept as a follow-up below: fixing it changes `GridCache::apply_scrollback`'s versioning, shared with other features, for a state that corrects itself on the next pointer move |
| M2 | B | F1 (MINOR): `link_open_finished`'s `NotFound` text ships untested | The arm exists because the match is exhaustive; its behaviour is U77, assigned to M5, whose cycle must show its own red |
| M2 | B | F3 (MINOR): U94's `< 250 ms` bound would pass a shorter debounce | `perform` is `Task::perform` over `spawn_blocking` with no timer in the code; the bound guards against a real delay, and a tighter wall-clock bound would flake on a loaded CI runner |
| M1 | B | F5 (MINOR): contract §3 rows `http://localhost:5173/`, `mailto:team@example.com,`, `file:///home/u/My%20Doc.pdf`, `team@example.com`, `src/main.rs:42`, `data:text/html,x` have no `detect` unit test of their own | Each is a line of the SC-002 corpus (`tests/fixtures/link_corpus.txt`, section "Contract link-recognition §3") checked cell by cell through `link_at`, which calls `detect`; a second table test would pass on arrival and duplicate it |
| M5 | A | Finding 4 (MEDIUM): `container_host_names` assumes the container's hostname is the id's 12-character prefix, which podman does not default to | The sandbox's own `--hostname` is M6's business (C11, T055–T067): in M5 every sandboxed file link is `Unreachable` whatever the host matches, so no M5 behaviour changes. Kept as a follow-up below |
| M5 | A | Finding 8 (LOW): a `file://` path keeps a `?query` or `#fragment`, so such a link reports "the file doesn't exist on this machine" | The spec's C4–C10 do not strip either, and RFC 8089 gives a `file` URL no query component; inventing a rule here would resolve a path the hint did not show. Kept as a follow-up to spec, not to patch inside M5 |
| M5 | A | Finding 9 (LOW, efficiency): `ui::terminal::link_context` is rebuilt every frame and clones up to four `String`s | SC-005 is measured over the hover path in M8 (T078); four small clones per frame are far below the pane's own per-frame work, and caching it adds a second source of truth for the sandbox state. Kept as a follow-up |
| M5 | B | Finding 5 (MINOR): the real-file leg of SC-007 sampled only some of FR-013's extensions | Fixed rather than declined: `every_runnable_kind` now creates one real file per entry of FR-013's list for this platform, plus every macOS bundle extension as a directory |
| M5 | B | Finding 6 (MINOR): FR-013's lists are transcribed twice in one file, so one careless edit changes both | Accepted as is: the test copy is the spec's list and the production copy is the code's, and a single file is what a reviewer can compare. A generator or an `include_str!` of spec.md would couple the crate to the spec's prose |

## Open escalation

None. (M2's block on the Windows install smoke was resolved by #358; see Decisions 20 and git history of this file.)

## Follow-ups not done

- Quickstart §B.17's second half is unconfirmed: no AI CLI on this machine was seen to declare an OSC 8 link with `FORCE_HYPERLINK=1` (Claude Code 2.1.280), and that visual-pass run surfaced no hover feedback at all. Re-run it when a CLI is known to declare links (M4, minor).
- Scrollback lines fetched by `apply_scrollback` do not bump the grid `seq`, so a resting pointer over just-fetched lines refreshes its hover only on the next pointer move or modifier change (M3, minor).
- The sandbox is created with `--name`, never `--hostname`, so `container_host_names`' 12-character-prefix assumption (C11) holds on docker and not on podman. Pass `--hostname <id prefix>` at create time, or add the container name to the list, when M6 translates container paths (M5 review A finding 4, medium).
- A `file://` path keeps a `?query` or `#fragment` and then reports as missing. Decide in the spec whether either is stripped (M5 review A finding 8, low).
- `ui::terminal::link_context` is rebuilt on every frame; cache it on the session state if T078's SC-005 measurement shows it (M5 review A finding 9, low).
