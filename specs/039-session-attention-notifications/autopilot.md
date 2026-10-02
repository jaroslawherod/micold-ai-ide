# Autopilot ledger — 039-session-attention-notifications

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #481: https://github.com/jaroslawherod/micold-ai-ide/issues/481 — Issue title: Notify when a session needs attention and track unread sessions. Issue body: ## Problem — The sidebar already shows each session's activity dot (working / awaiting input), but only while the user is looking at the window. With several agents running in parallel, a session that finished its turn or is waiting for a permission prompt goes unnoticed until the user happens to check it. There is also no way to tell which sessions have new output the user has not looked at yet. ## Proposal — When a session moves to **awaiting input** (turn ended, permission or idle prompt), show an OS desktop notification naming the project, worktree and session. Clicking it focuses the window and switches to that session. — Mark a session **unread** when it produces a turn the user has not viewed since it became active, and clear the mark when the user opens the session. — Show an unread count on the project in the switcher, so attention needed in a background project is visible. — Suppress the notification when the session is the one currently focused. — Settings: on/off for desktop notifications, and optionally per provider. ## Acceptance criteria — A session that reaches awaiting input while another session is focused (or the window is unfocused) raises exactly one desktop notification on Linux and macOS. — Clicking the notification opens that session. — The unread mark appears on the session and its project, and clears when the session is opened. — Nothing is sent when notifications are turned off. — Works in both the host and the sandboxed daemon runtime.
- **Kind**: feature
- **Issue**: #481
- **Worktree branch**: feat/notify-session-needs-attention
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Continue the design unit from *Handover*: review the plan (Plan rubric, round 1), then `speckit-tasks`, milestones, `speckit-analyze`, checklists, PR 2.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #528 | Spec | merged | 824e0a9bc58ad5f977ef56fa813e58217c563ea5 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 188c880b773ff483e9791fdb9842587dcb7f9515:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR ("in view" defined two ways, unread after reconnection undefined, click with no window open undefined), 3 MINOR; all six fixed |
| Spec | 2 | b217dc29a9ef4507039fd9ddace7063a33e1ac63:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN (1 MINOR: FR-019 did not name leaving Settings; fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Design unit handed over at the context cap, after `speckit-plan` (step 1 of `phases/3-design.md`), before its review.

- **Done**: `branch-start.sh 528` (three clarify commits rebased onto `origin/main`). Written and committed, not pushed: `plan.md`, `research.md` (R1–R10), `data-model.md`, `quickstart.md`, `contracts/wire.md`, `contracts/desktop-notification.md`, `contracts/unread-mark.md`. `.specify/extensions.yml` has no `before_plan` or `after_plan` hook. `checklists/requirements.md` has no unchecked item.
- **Next step**: dispatch the Plan review, round 1 (no round is recorded yet; no snapshot taken). Then `speckit-tasks` (its `after_tasks` hooks: `speckit.tdd.plan`, `speckit.docguard.score`, both optional; `before_tasks` hooks not read yet), cut milestones as plan.md *Delivery order* gives them (M1 story 1 `full`, M2 story 2, M3 story 3 `full`, M4 story 4 follows the `tool_server_enabled` pattern, M5 polish), `speckit-analyze`, the tasks review, checklists, PR 2.
- **Open findings**: none yet. Points the reviewer should test, not yet verified by me in code: that a window detaches the project it leaves (`shell/daemon_sync.rs:195`), so "open in a window" equals "attached"; the names `ProjectMsg::Reopened` and `SessionMsg::Selected` as the switch and select messages; `Button`'s builder in `ui/material/button.rs` (the contract adds `.trailing_mark`); where the sandbox keeps the service's `projects.json`. Two facts are marked **unverified** in research R4 and R7 (Windows notification-centre click, Wayland activation through foreign handles) and need a task each.
- **PR**: none opened by this unit.

## Open escalation

None.

## Token usage

## Follow-ups not done

