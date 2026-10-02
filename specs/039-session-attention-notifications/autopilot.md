# Autopilot ledger — 039-session-attention-notifications

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #481: https://github.com/jaroslawherod/micold-ai-ide/issues/481 — Issue title: Notify when a session needs attention and track unread sessions. Issue body: ## Problem — The sidebar already shows each session's activity dot (working / awaiting input), but only while the user is looking at the window. With several agents running in parallel, a session that finished its turn or is waiting for a permission prompt goes unnoticed until the user happens to check it. There is also no way to tell which sessions have new output the user has not looked at yet. ## Proposal — When a session moves to **awaiting input** (turn ended, permission or idle prompt), show an OS desktop notification naming the project, worktree and session. Clicking it focuses the window and switches to that session. — Mark a session **unread** when it produces a turn the user has not viewed since it became active, and clear the mark when the user opens the session. — Show an unread count on the project in the switcher, so attention needed in a background project is visible. — Suppress the notification when the session is the one currently focused. — Settings: on/off for desktop notifications, and optionally per provider. ## Acceptance criteria — A session that reaches awaiting input while another session is focused (or the window is unfocused) raises exactly one desktop notification on Linux and macOS. — Clicking the notification opens that session. — The unread mark appears on the session and its project, and clears when the session is opened. — Nothing is sent when notifications are turned off. — Works in both the host and the sandboxed daemon runtime.
- **Kind**: feature
- **Issue**: #481
- **Worktree branch**: feat/notify-session-needs-attention
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Continue the design unit from *Handover*: `speckit-tdd-plan`, `speckit-analyze`, the tasks review (round 1), checklists, PR 2.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #528 | Spec | merged | 824e0a9bc58ad5f977ef56fa813e58217c563ea5 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | The service knows what is in view and counts attention events (integration test; nothing new on screen) | | pending |
| M2 | T017–T034 | full | One desktop notification on Linux for a session not in view | | pending |
| M3 | T035–T044 | full | The same notification on macOS and Windows | | pending |
| M4 | T045–T061 | full | The unread mark on a session's row, kept across restarts | | pending |
| M5 | T062–T072 | full | Unread counts on the switcher's rows and button | | pending |
| M6 | T073–T090 | full | A click on the notification opens the session | | pending |
| M7 | T091–T100 | full | Keyboard focus from a click on Wayland (probe first) | | pending |
| M8 | T101–T112 | full | The Desktop notifications switch | | pending |
| M9 | T113–T117 | light | Developer docs and the recorded quickstart passes | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which spec number? | 039: 036 is taken by the flow in worktree `fix-issue-430` (not on `main`), 037 and 038 are on `main`, no open PR or worktree holds 039. | agent-resolved | `ls <worktree>/specs` across `git worktree list`; `gh pr list --state open`, 2026-10-02 |
| D2 | spec | The issue names Linux and macOS. Is Windows in scope? | Yes. Principle VI makes parity on all three a condition of done (FR-029). | agent-resolved | `.specify/memory/constitution.md` §VI; spec.md#Assumptions |
| D3 | spec | What is "the session currently focused"? | The selected session of a window with keyboard focus ("in view"). The same condition decides notification and unread. | agent-resolved | spec.md#Terms; issue acceptance criterion 1 ("or the window is unfocused") |
| D4 | spec | Do unread marks depend on the notification setting? | No. The switch stops desktop notifications only (FR-017, FR-027). | agent-resolved | spec.md#User Story 4 |
| D5 | spec | Is a session covered by Settings, or showing a regular terminal tab, in view? | Settings: not in view (the user cannot see it). Regular terminal tab: in view (its indicator is beside it). | agent-resolved | spec.md#Terms; Review Spec round 1 F1 |
| D6 | spec | Does a change missed during a lost connection count? | Yes, with a window open: it is treated as a change at reconnection (FR-006). At application start nothing is notified (FR-005); unread then waits on FR-008. | agent-resolved | spec.md FR-005, FR-006; Review Spec round 1 F2 |
| D7 | clarify | FR-008: a session changes to awaiting input while no window is open. What must happen? | (b) No desktop notification; the session is unread when the application is next opened. Unread state outlives the window and is stored on the user's computer (FR-008, FR-008a, FR-024). | user | Orchestrator relayed the user's answer, 2026-10-02; spec.md#Clarifications |
| D8 | clarify | FR-023: does the switcher's button show unread with its panel closed? | (a) Yes: the total of unread sessions in projects other than the active one, nothing when zero. | user | Orchestrator relayed the user's answer, 2026-10-02; spec.md#Clarifications |
| D9 | clarify | FR-028: a notification switch per AI CLI? | (a) No: one switch for every AI CLI; per-CLI is out of scope. | user | Orchestrator relayed the user's answer, 2026-10-02; spec.md#Clarifications |
| D10 | clarify | What is unread on the first start with the feature, or when the stored unread state cannot be read? | Nothing, and no error is shown: there is no record of what the user viewed (FR-008a). Same fallback as an unreadable settings file. | agent-resolved | spec.md#Edge Cases ("Settings file unreadable"); follows from D7 |
| D11 | design | FR-008: how is a change known that happened with no window open? | The session service keeps `attention_seq` and `unread` per session in its catalog and sets them itself; windows report what is in view (`WindowView`). Within FR-007: the existing connection only. | agent-resolved | research.md R1, R2 |
| D12 | design | Several windows are several client processes: who raises the one notification, and which window does a click open? | A window claims an attention event and the service grants each once (FR-006a); a click is sent to the service, which forwards it to the window holding the project (FR-012). | agent-resolved | research.md R3, R6; `messages.rs:92` `ClientInstance`, `RefusalReason::ProjectBusy` |
| D13 | design | Which notification crates? | Linux `zbus` directly, macOS `mac-usernotifications` 0.3.1, Windows `tauri-winrt-notification` 0.8.1, behind one trait. `notify-rust` rejected: no Wayland activation token, `cc` build on macOS. | agent-resolved | research.md R4 |
| D14 | design | Is the switch a client or a service setting? | Service-owned (`DaemonSettings`), so every window has one value at once; the service refuses grants while off. | agent-resolved | research.md R8; pattern `tool_server_enabled` |
| D15 | design | A notification is clicked after the window that raised it was closed, with another window open. Must the session open? | No: the click is reported to the raising process only, on all three systems, so no window learns of it (FR-015's exception). Nothing changes; the unread mark finds the session. Stated in spec Edge Cases, research R6 *Known limit*, contract N9, quickstart B11a, and the user guide in M3. Sharing ids through the service was rejected: it helps only on Linux services that send the click to every listener. | agent-resolved | spec.md FR-015, Edge Cases; Review Plan round 1 F3; `mac-usernotifications` 0.3.1 `src/delegate.rs`, `tauri-winrt-notification` 0.8.1 `src/lib.rs:485` |
| D17 | design | How are the stories cut into milestones? | Nine: story 1 in three slices (service rules; Linux notification; macOS and Windows), story 2 in two (row mark; switcher counts), story 3 in two (click; Wayland focus, which starts with a probe), story 4, polish. Each story is well past 15 tasks; M1 is observable by its integration test only and its PR takes `docs-not-needed`. The wire is bumped once per milestone that changes it (21 to 26), so the claim pair moved to 22 and the activation token to 25. | agent-resolved | `references/milestones.md` rules 1, 3; tasks.md Milestones; research.md R10 |
| D16 | design | Do the facts part 1 left unconfirmed hold? | Code: all four hold (`switch_daemon_attachment` sends `Detach` then `Attach`, `daemon_sync.rs:198`; `ProjectMsg::Reopened` `features/project.rs:531`; `SessionMsg::Selected` `features/session.rs:1554`; `Button` is a builder with a `leading` slot, `button.rs:95`; the sandboxed service's `projects.json` is the host's state directory mounted at `/var/lib/micold-ai-ide`, `sandbox/mod.rs:367`, `:659`). Wayland: the foreign-handle binding is confirmed in the sources (winit selects `client_system`; `smithay-clipboard` does the same in this application); whether a compositor honours the notification's token stays unverified and has a probe task in M3. Windows: the notification-centre click stays unverified: no primary source found; a task in M3 checks it on an installed build. | agent-resolved | research.md "Where the code is today", R4, R7 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 188c880b773ff483e9791fdb9842587dcb7f9515:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR ("in view" defined two ways, unread after reconnection undefined, click with no window open undefined), 3 MINOR; all six fixed |
| Spec | 2 | b217dc29a9ef4507039fd9ddace7063a33e1ac63:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN (1 MINOR: FR-019 did not name leaving Settings; fixed) |
| Plan | 1 | 7dfdd00bfdb595aa473a2abd3e119814606b521d:21c79c9d9bf6fb59b7945d2b4bd0596a9a9aefe7 | CHANGES: 5 MAJOR (`SessionChanged` is never sent, the carrier is `CatalogChanged`; the Wayland token did not reach the window that is raised; no rule for a click after the raising window closed; the raise decision was untested glue; no test layer for FR-015, FR-015a, FR-017, FR-025, FR-028), 3 MINOR (an event while the switch is off claimable after reconnecting; counts need the session in view; `SettingsSet` field is an `Option`); all eight fixed |
| Plan | 2 | 6f5590e2f6b7bd8ef3510df31aee2a75532299d2:80af04fa661ea74bd8f32172b5dd8b92c2813ecb | CLEAN (1 MINOR: wrong line cited for the second `note_activity` caller; fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Design unit, part 2, handed over at the context cap after `speckit-tasks` (step 2 of `phases/3-design.md`), before `speckit-analyze`.

- **Done**: `branch-start.sh 528` (rebased; nothing pushed). The plan's unconfirmed facts checked (D16). Plan review: round 1 CHANGES, all fixed; round 2 CLEAN. `tasks.md` written with 117 tasks and `## Milestones` M1–M9 (D17); plan.md *Delivery order*, research R10 and `contracts/wire.md` aligned with it (versions 21 to 26) **after** the plan review's last snapshot, so the tasks reviewer should also read those three edits (commit `7f0f72fd`). The `before_tasks` hook (`speckit.docguard.review`, optional) was not run.
- **Next step**: the `after_tasks` hooks — run `speckit-tdd-plan` (it writes `tdd/test-list.md`, which the mandatory `before_implement` hook `speckit.tdd.run` reads; feature 038 has one) and skip `speckit.docguard.score` (optional, a score only). Then `speckit-analyze` and fix what it finds; the Tasks and milestone review, round 1 (no round recorded, no snapshot taken); checklists (`checklists/requirements.md` has no unchecked item; re-check after the spec's new Edge Case line); PR 2 `docs(039): clarify, plan and cut milestones for session attention notifications`, body ending `Refs #481`, local gate `mise run test-scripts`.
- **Open findings**: none. Points for the tasks reviewer, not yet reviewed by anyone: M1 ships nothing a user sees (deliverable is the integration test) — rule 1 of `milestones.md` read with rule 3; M2 leaves macOS and Windows returning `NotifyError::Unsupported` until M3 (Principle VI is met at M3); the spec gained one Edge Case line for D15 without an escalation; T017/T018/T073 share `attention.rs` and carry no `[P]`; M9's tier is `light` although T115 runs the visual pass.
- **PR**: none opened by this unit.

## Open escalation

None.

## Token usage

## Follow-ups not done

