# Feature Specification: Show Claude Plan Usage and the Next Limit Reset

**Feature Branch**: `claude/project-thread-u1pay5`

**Created**: 2026-10-10

**Status**: Draft

**Input**: User description: "Implement GitHub issue #490: Show Claude plan usage and next limit reset. Problem: Users on Claude subscription plans hit usage limits and have no view of how close they are, or when the limit resets, without leaving the app. Proposal: Show current usage and the next limit reset time for the signed-in Claude account, in the status area or Settings. Warn when usage passes a configurable threshold. Show nothing, and log nothing alarming, when the data is not available. Open questions: Which source is stable enough to rely on: the CLI's own status output, a documented API, or local usage logs. Undocumented endpoints should not be used. Whether the same can be offered for other providers. Acceptance criteria: Works offline without errors (the indicator is simply hidden). No credentials are read beyond what the chosen source requires, and none leave the machine except to the provider."

## What already exists, and the gap

The application runs Claude Code sessions (and sessions of other AI CLIs) in terminals, each signed in with the user's own Claude account. A Claude subscription plan limits how much the account can use in a rolling window (for example a few hours) and over a longer period (for example a week); when a limit is reached, every Claude Code session on that account stops working until the limit resets. Today the application shows nothing about this. The user learns they are close to a limit only when a session refuses to continue, and learns when it resets only by asking the CLI in a session or leaving the app.

The application already has an opt-in pattern for information fetched with the user's own sign-in: **Show pull request status on worktrees** (Settings → GitHub) is off until the user turns it on, shows nothing and raises no error when its tool is missing or signed out, and documents exactly what is read and sent. This feature follows the same pattern.

## Terms

- **Plan usage**: how much of a Claude subscription plan's allowance the signed-in account has used, per limit window, as a share of that window's allowance (a percentage).
- **Limit window**: one of the plan's usage limits, each with its own allowance and its own reset time — for example the short rolling window and the weekly window. A plan may have more than one at a time, and which ones exist is the plan's, not the application's, decision.
- **Reset time**: the moment a limit window's usage goes back to zero (or its allowance renews).
- **Next limit reset**: the reset time of the limit window that is the closest to its allowance (highest percentage); on a tie, the one that resets first.
- **Usage source**: where the application obtains plan usage. Which one is chosen is open (see FR-012).
- **Usage indicator**: the compact display of plan usage in the application window.
- **Warning threshold**: the percentage of a limit window at or above which the usage indicator warns.
- **Usage reading**: one set of plan-usage values obtained from the usage source, with the time it was obtained.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See how close I am to my plan limit and when it resets (Priority: P1)

A developer on a Claude subscription plan works with several Claude Code sessions. Without leaving the application, they glance at the window and see how much of their plan's current limit they have used and when that limit resets, so they can decide whether to start another long task now or wait.

**Why this priority**: This is the problem the issue states. Without it nothing else in the feature has a value to warn about.

**Independent Test**: With plan usage turned on, a signed-in Claude subscription account and a usage source that returns a reading, open the application: the usage indicator shows the highest usage percentage and the next limit reset; opening its details shows every limit window with its own percentage and reset time.

**Acceptance Scenarios**:

1. **Given** plan usage is turned on and the usage source returns a reading with a short window at 42% resetting at 15:30 and a weekly window at 18% resetting on Monday, **When** the user looks at the application window, **Then** the usage indicator shows 42% and a reset at 15:30, in the user's local time.
2. **Given** the same reading, **When** the user hovers or opens the usage indicator, **Then** they see both limit windows, each with its name, its percentage and its reset time, and the time the reading was obtained.
3. **Given** the indicator is shown, **When** the user's usage changes and a newer reading is obtained, **Then** the indicator shows the new values without the user doing anything, at the latest within the refresh interval (FR-007).
4. **Given** the reset time of the shown window passes and no newer reading has been obtained, **When** the user looks at the indicator, **Then** it no longer presents the old percentage as current (FR-009).

---

### User Story 2 - Be warned before I hit the limit (Priority: P2)

The developer does not watch the indicator all the time. When their usage of any limit window reaches a level they chose, the indicator changes to a warning look, so they notice before a session stops.

**Why this priority**: The issue asks for it, and it is what turns a number into something that saves the user from a stalled session. It needs User Story 1's reading first.

**Independent Test**: Set the warning threshold to 80%; with a reading where one window is at 79%, the indicator is in its normal look; with a reading where that window is at 80%, the indicator is in its warning look and its details name the window that crossed the threshold.

**Acceptance Scenarios**:

1. **Given** the warning threshold is 80% and every limit window is below 80%, **When** the indicator is shown, **Then** it has its normal look.
2. **Given** the warning threshold is 80%, **When** a reading shows any limit window at 80% or more, **Then** the indicator shows its warning look, distinguishable without relying on colour alone (an icon or text, not only a colour change).
3. **Given** the indicator is in its warning look, **When** a newer reading shows every window below the threshold (for example after a reset), **Then** the indicator returns to its normal look.
4. **Given** the user changes the warning threshold in Settings and saves, **When** the indicator is next drawn, **Then** it applies the new threshold to the current reading without waiting for a new reading.

---

### User Story 3 - Nothing appears and nothing goes wrong when the data is not available (Priority: P1)

The developer works offline, or is not signed in to Claude, or uses an API key rather than a subscription plan, or uses only other AI CLIs. The application shows no usage indicator, raises no error, shows no notice and writes nothing alarming to its log.

**Why this priority**: This is the issue's first acceptance criterion and Principle IV (the application is fully functional offline). Getting it wrong would turn the feature into noise for every user who cannot use it.

**Independent Test**: With plan usage turned on, disconnect the network (or sign out of Claude, or make the usage source unavailable): the usage indicator is not shown, no dialog, notice or desktop notification appears, and the application log has no entry at warning or error level about plan usage.

**Acceptance Scenarios**:

1. **Given** plan usage is turned on and the machine is offline at start-up, **When** the application window opens, **Then** no usage indicator is shown and no error, notice or warning-level log entry is produced about it.
2. **Given** the indicator is shown, **When** the machine goes offline and the next reading cannot be obtained, **Then** the indicator keeps the last reading until it is no longer current (FR-009) and is then hidden, with no error shown.
3. **Given** the machine comes back online, **When** the next reading is obtained, **Then** the indicator reappears without the user doing anything.
4. **Given** no Claude account is signed in, or the signed-in account has no subscription plan limits (for example an API key), **When** the application runs, **Then** no usage indicator is shown.
5. **Given** plan usage is turned off, **When** the application runs, **Then** no usage indicator is shown and the usage source is never consulted.
6. **Given** the indicator shows a reading for one account, **When** the user signs in to a different Claude account and the application sees the change, **Then** the old reading is no longer shown, and the indicator reappears only with a reading for the new account (FR-019).

---

### User Story 4 - Choose whether plan usage is shown, and the warning threshold (Priority: P3)

The developer turns plan usage on or off and sets the warning threshold in Settings, with a plain statement of what is read and what, if anything, leaves the machine.

**Why this priority**: Required for consent (Principle IV) and for the configurable threshold, but the defaults make Stories 1–3 usable without touching it.

**Independent Test**: Open Settings, find the plan-usage switch and threshold, change them and save; the indicator appears or disappears and warns at the new threshold, and the choice survives a restart.

**Acceptance Scenarios**:

1. **Given** a fresh installation, **When** the user opens Settings, **Then** the plan-usage switch is in its default state (FR-001) and the warning threshold shows 80%.
2. **Given** the user enters a threshold outside the allowed range (FR-005), **When** they try to save, **Then** the save is refused with a message naming the allowed range, as for other out-of-range settings.
3. **Given** the user changes either setting and saves, **When** another open window of the application is looked at, **Then** it applies the change too, without a restart.
4. **Given** the Settings entry, **When** the user reads it, **Then** it states which source is read, which credentials (if any) that needs, and what is sent where.

---

### Edge Cases

- **Empty or partial reading**: the source answers but has no limit windows, or a window without a percentage or reset time. A reading with no usable window hides the indicator; a window missing its reset time is shown without one; a window missing its percentage is left out.
- **Values out of range**: a percentage above 100% (usage over the allowance) is shown as reported and counts as above any threshold; a negative percentage or a reset time in the past at the moment the reading is obtained is treated as unusable for that window.
- **Failure of the source**: a timeout, a refusal, an expired sign-in, or an answer the application cannot read hides the indicator once the last reading is no longer current; none of them is shown as an error. Repeated failures do not cause the source to be consulted more often than the refresh interval.
- **Many sessions at once (Principle II)**: plan usage belongs to the account, not to a session. Ten running Claude Code sessions produce one indicator and do not make the source be consulted ten times as often. Sessions of other AI CLIs do not affect it.
- **Several windows open**: every open window shows the same reading and the same look; they do not each consult the source on their own schedule.
- **Account changes**: the user signs in to a different Claude account while the application runs. The next reading is for the new account; the old account's values are not shown as the new account's once the change is seen.
- **Session service in a container**: when sessions run in the container with **AI CLI sign-in** not shared, the container has no Claude sign-in; the indicator reflects only what the chosen source can see on the host (or nothing), and the feature never shares the sign-in with the container itself.
- **Clock and time zones**: reset times are shown in the user's local time zone; a reset more than a day away also shows its day. A system clock that is wrong does not make the indicator claim a reset already happened when the reading says otherwise.
- **Cross-platform (Principle VI)**: Linux, macOS and Windows keep the Claude sign-in in different places (a file or the system's credential store). The feature behaves the same on all three; where the chosen source needs no credentials it reads none on any of them, and where it needs them it reads only those, through the platform's normal means.
- **Plan changes**: the plan gains or loses a limit window (for example a model-specific weekly window). The indicator shows whatever windows the latest reading contains.

## Requirements *(mandatory)*

### Functional Requirements

**Turning it on, and consent**

- **FR-001**: Settings MUST have a switch, **Show Claude plan usage**, that turns the feature on and off for every window. Its default is [NEEDS CLARIFICATION: on or off by default? Off follows the opt-in rule for anything read with the user's sign-in (as **Show pull request status on worktrees**); on is acceptable only if the chosen source reads no credentials and sends nothing off the machine].
- **FR-002**: While the switch is off, the application MUST NOT consult the usage source, read any credential for this feature, or show a usage indicator.
- **FR-003**: The Settings entry MUST state, in plain words, which source is read, which credentials (if any) that requires, and what is sent where; the user guide MUST say the same (Principle VII).

**Showing usage**

- **FR-004**: While the switch is on and a current reading exists, the application window MUST show a usage indicator in a place visible without opening any dialog, menu or Settings, showing the highest percentage among the reading's limit windows and that window's reset time (the next limit reset).
- **FR-005**: Settings MUST let the user set the warning threshold as a whole percentage, 80% by default, allowed from 50% to 100%; a stored value outside the range MUST be clamped when read, and a value outside the range MUST be refused when saved, with a message naming the range.
- **FR-006**: The usage indicator MUST offer details (on hover or press) listing every limit window of the reading with its name, percentage and reset time, and the time the reading was obtained.
- **FR-007**: While the switch is on, the application MUST obtain a new reading when the switch is turned on, when the application starts, and then at a fixed refresh interval of 5 minutes (the plan may choose a shorter one, not under 1 minute), and MUST NOT consult the source more often than that, however many sessions or windows exist.
- **FR-008**: Reset times MUST be shown in the user's local time zone, as a time of day when the reset is within 24 hours and with its day otherwise.
- **FR-009**: A reading MUST stop being shown as current when the reset time of the window it shows has passed or when it is older than three refresh intervals, whichever comes first; the indicator is then hidden until a new reading is obtained.

**Warning**

- **FR-010**: When any limit window in the current reading is at or above the warning threshold, the usage indicator MUST show its warning look, which MUST differ from the normal look by an icon or text and not by colour alone, and its details MUST name the window or windows at or above the threshold. Below the threshold in every window, it MUST show its normal look.
- **FR-011**: The warning MUST be shown only in the usage indicator: it raises no dialog and no desktop notification.

**Source and credentials**

- **FR-012**: Plan usage MUST be obtained from one usage source that is either documented by its provider or is the Claude CLI's own documented output; undocumented endpoints MUST NOT be used. [NEEDS CLARIFICATION: which source — the Claude CLI's own documented status output, a documented Anthropic API, or local usage logs? Local logs give a local estimate but not the plan's own percentage or reset time; the other two must be confirmed to report plan limits for subscription accounts.]
- **FR-013**: The feature MUST read no credential beyond what the chosen source requires, and MUST read it only while the switch is on.
- **FR-014**: Nothing obtained or read for this feature (usage values, credentials, account identity) MUST leave the machine, except a request to the provider (Anthropic) that the chosen source itself requires. Readings MUST NOT be written anywhere other than the application's own local state, and credentials MUST NOT be written anywhere at all by this feature.

**Unavailability**

- **FR-015**: When no current reading exists — offline, signed out, no subscription plan limits, the source missing, failing, timing out or answering in a form the application cannot read — the application MUST hide the usage indicator and MUST NOT show any error, dialog, notice or desktop notification about it.
- **FR-016**: Failures to obtain a reading MUST be logged, if at all, below warning level, and an unchanged failure MUST be logged at most once until a reading succeeds again.
- **FR-017**: Obtaining a reading MUST NOT block or slow the application's window or any session; a source that does not answer within 10 seconds counts as a failure for that refresh.

**Many windows**

- **FR-018**: Every open window MUST show the same current reading, warning look and settings; a settings change saved in one window MUST apply in the others without a restart.

**Account changes**

- **FR-019**: When the application sees that the signed-in Claude account has changed, it MUST discard the current reading at once (hiding the indicator) and show only readings obtained for the new account.

### Key Entities

- **Usage reading**: the account's plan usage at one moment — a list of limit windows and the time it was obtained. Kept only as long as it is current (FR-009).
- **Limit window**: a name (as the source reports it, for example "5-hour" or "weekly"), a percentage used, and a reset time.
- **Plan-usage settings**: the on/off switch and the warning threshold, kept with the user's other settings.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the feature on and a reading available, a user can tell their highest usage percentage and its reset time from the main window in one glance, without opening any dialog or menu.
- **SC-002**: The shown percentage matches what the provider itself reports for the same account, at the same moment, to within one percentage point, and the shown reset time matches to within one minute.
- **SC-003**: With the machine offline, signed out, or on an account without plan limits, a full working session of the application produces zero error messages, notices or warning-level log entries about plan usage, and no indicator.
- **SC-004**: With ten sessions running, the source is consulted no more often than with one session.
- **SC-005**: When usage crosses the warning threshold, the warning look appears within one refresh interval (at most 5 minutes) of the provider reporting it.
- **SC-006**: An audit of what the feature reads and sends finds no credential read beyond what the chosen source needs and no data sent anywhere except to the provider.
- **SC-007**: With the feature off, the source is never consulted and no credential is read for it.

## Out of Scope

- Plan usage for other AI CLIs (GitHub Copilot, Codex, OpenCode, Pi Coding Agent). The issue asks whether this can be offered; this feature answers for Claude only and records other providers as a later request.
- API-key billing, spend or token cost reporting, and usage history or charts over time.
- Per-session or per-project usage breakdowns.
- Desktop notifications or sounds for the warning (FR-011).
- Changing the plan, buying extra usage, or any action on the account.
- Undocumented endpoints, scraping web pages, and reading another program's private storage beyond what the chosen source requires.
- Sharing the Claude sign-in with the containerised session service for this feature.

## Assumptions

- "Status area" in the issue means a place in the main window visible at all times (the top toolbar is the application's only always-visible bar); "or Settings" is satisfied by the settings that control the feature. The exact placement is a design decision for the plan.
- "Next limit reset" is the reset of the window closest to its allowance, because that is the one that will stop the user first; the details show the others.
- The warning threshold applies to every limit window alike; a per-window threshold is a later request.
- 80% is the default threshold: high enough not to warn during normal use, low enough to leave time to finish work. 50% is the lowest allowed value because a lower one warns for most of every window; 100% warns only at the limit.
- Signed-in identity comes from the user's own Claude Code sign-in; the application has no Claude sign-in of its own and does not add one.
- **Dependencies**: Settings and its save, clamp and refuse rules; the settings shared across open windows; the top toolbar; the shared component set and showcase (Principle VIII); the user guide's settings chapter (Principle VII); the Claude Code provider and its sign-in.
