# Autopilot ledger — 039-session-attention-notifications

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #481: https://github.com/jaroslawherod/micold-ai-ide/issues/481 — Issue title: Notify when a session needs attention and track unread sessions. Issue body: ## Problem — The sidebar already shows each session's activity dot (working / awaiting input), but only while the user is looking at the window. With several agents running in parallel, a session that finished its turn or is waiting for a permission prompt goes unnoticed until the user happens to check it. There is also no way to tell which sessions have new output the user has not looked at yet. ## Proposal — When a session moves to **awaiting input** (turn ended, permission or idle prompt), show an OS desktop notification naming the project, worktree and session. Clicking it focuses the window and switches to that session. — Mark a session **unread** when it produces a turn the user has not viewed since it became active, and clear the mark when the user opens the session. — Show an unread count on the project in the switcher, so attention needed in a background project is visible. — Suppress the notification when the session is the one currently focused. — Settings: on/off for desktop notifications, and optionally per provider. ## Acceptance criteria — A session that reaches awaiting input while another session is focused (or the window is unfocused) raises exactly one desktop notification on Linux and macOS. — Clicking the notification opens that session. — The unread mark appears on the session and its project, and clears when the session is opened. — Nothing is sent when notifications are turned off. — Works in both the host and the sandboxed daemon runtime.
- **Kind**: feature
- **Issue**: #481
- **Worktree branch**: feat/notify-session-needs-attention
- **Started**: 2026-10-02
- **Phase**: 2-clarify
- **Next step**: Ask the user the three questions under *Open escalation*, then continue clarify round 1 with the answers: record them in spec.md `## Clarifications` as `_(decided by user)_`, replace the three markers (FR-008, FR-023, FR-028), re-validate `checklists/requirements.md`, commit (no push; it ships in PR 2).

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 188c880b773ff483e9791fdb9842587dcb7f9515:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR ("in view" defined two ways, unread after reconnection undefined, click with no window open undefined), 3 MINOR; all six fixed |
| Spec | 2 | b217dc29a9ef4507039fd9ddace7063a33e1ac63:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN (1 MINOR: FR-019 did not name leaving Settings; fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, 2026-10-02. `speckit-clarify` raised three questions, the spec's three markers; the
scan found no other critical ambiguity. All three are category 1 (product or scope decision the repo
does not settle). Nothing is written to spec.md until they are answered.

1. **FR-008 — a session changes to awaiting input while no window is open. What must happen?**
   - (b) *(Recommended)* No desktop notification; the session is shown as unread when the
     application is next opened. Evidence: the issue's "a turn the user has not viewed"; user story
     2 ("comes back from a meeting"); `specs/010-daemon-session-persistence/spec.md` user story 1
     (close the UI, reopen, find the session where it got to). Cost: the unread state must outlive
     the window, stored locally (Principle IV), and FR-005's "nothing is notified at start" stays.
   - (c) Neither: notifications and unread marks exist only for changes seen while a window is
     open. Smallest; a session that finished while the application was closed shows only its
     activity indicator.
   - (a) Notify even with no window. Not workable inside this spec: the client is one
     `iced::application` process that ends with its window
     (`crates/micold-client/src/shell/startup.rs:83`), so only the session service could raise it,
     and FR-007 and the Assumptions say the container takes no part; a tray or background process
     is under Out of Scope.
2. **FR-023 — must the switcher's button in the top bar show, with its panel closed, that other
   projects hold unread sessions?**
   - (a) *(Recommended)* Yes: the button carries the total number of unread sessions in projects
     other than the active one, and nothing when that is zero. Evidence: the issue's own reason for
     the count, "so attention needed in a background project is visible"; today the button shows
     only the active project's name (`crates/micold-client/src/ui/toolbar.rs:63-77`), so a count on
     the panel's rows alone is seen only after opening it. With notifications off (story 4) it
     would be the only standing sign of a background project.
   - (b) Yes, but a mark without a number.
   - (c) No: the count on the rows of the open panel is all.
3. **FR-028 — is a separate notification switch per AI CLI (Claude Code, GitHub Copilot, Pi) part
   of this feature?**
   - (a) *(Recommended)* No: one switch; per-CLI is left for a later request. Evidence: the issue
     says "optionally per provider" and no acceptance criterion needs it; `Settings` has no per-CLI
     table today (`crates/micold-core/src/settings.rs:110`), only `default_ai_cli` and
     `pi_activity_component`; Pi can already be silenced by **Show activity for Pi sessions** (029
     FR-012e).
   - (b) Yes: one master switch plus one switch per AI CLI, all on by default.

## Token usage

## Follow-ups not done

