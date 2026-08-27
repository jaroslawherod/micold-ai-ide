# Feature Specification: Choose which AI CLI a session runs on

**Feature Branch**: `feat/add-support-for-other-ai-cli`

**Created**: 2026-08-14

**Status**: Closed 2026-08-27 — implemented and shipped; all 119 tasks in [tasks.md](./tasks.md) are
done. Quickstart §A's gate table was walked row by row (T083) and every gate names a green test.
§B B1–B8 ran 2026-08-25 against a real GitHub Copilot CLI 1.0.80 with `COPILOT_HOME` pointed at a
scratch directory — headlessly, on Xvfb + lavapipe, via the repository's `visual-pass` skill rather
than by a person at a display. B1, B2, B3, B5, B6 and B8 pass outright; B4 passes in its label, bar,
register and title halves and failed on the badge's one-second claim. Four defects the pass found
became tasks T086–T089 and are fixed: the activity badge never moved until some other broadcast ran,
a failed resume was computed and dropped, the reason for a failed start was displayed nowhere, and
the session-start list opened at the window origin instead of at the press. One finding is recorded
as an open defect rather than fixed — [BUG-001](./bugs/BUG-001.md), B7's wording gap: an unavailable
stored default offers the available CLIs and starts nothing, correctly, but never says the default
is missing. `mise run test` is green on Linux, macOS and Windows (T082), and no CI runner has
`copilot` installed, which is the shape the feature was built to. Two claims are recorded as reasoned
rather than run: Copilot's Windows base directory was verified from the CLI's own shipped bundle on
this disk, not on a Windows host (T081), and frame pacing on real hardware is out of reach of a
software rasteriser.

**Input**: User description: "Multi-provider AI CLI sessions: generalise the existing AiCliProvider seam so a session records which AI coding CLI backs it, the user can choose that CLI when creating a session, and the choice persists across restarts — with GitHub Copilot CLI landed as the second provider alongside the existing Claude Code provider."

## Clarifications

### Session 2026-08-14

- Q: GitHub Copilot CLI has no lifecycle hooks — nothing like the HTTP hooks that are the *only*
  reliable busy/idle signal for Claude Code sessions (research R4 measured PTY scraping and it does
  not work). What should a Copilot session's activity badge do? → A: Best-effort, derived from
  Copilot's own on-disk session store. The badge is not a decoration — it is how the user knows
  which of several concurrent sessions is waiting for them, and a session row that never reports is
  a session the user has to open to check. Copilot records each turn in its own store as it runs, so
  the signal exists; it is just pulled rather than pushed, and therefore approximate and slightly
  late. The application MUST NOT block the rest of the feature on it: if the derivation cannot be
  made to work, the badge degrades to absent for Copilot sessions and everything else here still
  ships. *(Superseded in part on 2026-08-16: research R5 found a structured per-turn event log, so
  the signal is reported rather than inferred. The requirement that it not be presented as more
  certain than it is — which cited FR-016 in error, and belonged to FR-018 — no longer applies. See
  the 2026-08-16 session below and FR-018.)*
- Q: Where does the user pick the CLI? Starting a session is a single click today, with no options
  dialog. → A: A default in Settings, plus a per-session override at the point of creation. The
  common case — a user who has settled on one CLI — costs nothing, while the case this feature
  exists for (running two CLIs side by side on the same project) does not require a trip to
  Settings between each. The initial default is Claude Code, so a user who never opens Settings
  sees exactly today's behaviour.
- Q: Should the application discover Copilot sessions that were started outside it, the way it
  already discovers Claude Code ones (FR-020b reconciliation)? → A: Yes. Reconciliation exists so
  the sidebar tells the truth about what is running in a worktree; a second CLI that is invisible to
  it would reintroduce exactly the gap that check was added to close. The *mechanism* differs
  between the two CLIs, but that is the plan's problem, not a reason to narrow the behaviour.
  *(Corrected on 2026-08-16: this originally asserted that Copilot's storage is not organised by
  working directory. Research R3 disproved it — Copilot keeps a per-working-directory index of
  session ids. Both CLIs partition by working directory; only the shape differs. See FR-021.)*

### Session 2026-08-16

- Q: Where does the per-session override live, given SC-001's click budget? Starting a session is one
  click on the row's `+` today, and a CLI menu on that button would make every start two clicks —
  including for the majority who have one CLI and never override. → A: A split affordance. Pressing
  the existing control starts the default, unchanged and in one interaction; an adjacent secondary
  control opens the list of available CLIs. The override costs one extra interaction and nothing
  else does. When only one CLI is available the secondary control is absent entirely (FR-006), so a
  single-CLI user sees exactly today's interface.
- Q: Resuming a session whose conversation the CLI no longer has was specified as "a clearly-reported
  failure *or* a fresh session" — which is it? → A: Report, and start nothing. The session is marked
  failed with a message saying that CLI no longer has the conversation. This reuses the reporting
  path a missing CLI already takes (FR-010) rather than inventing one, and it never leaves a row
  whose recorded title describes a conversation that is not behind it. Starting fresh silently under
  the same session identity is explicitly rejected.
- Q: FR-018 required the observed signal not to be "presented as more authoritative than the reported
  signal". Research R5 changed the premise — Copilot writes a structured per-turn event log, not a
  database to be inferred from — so does the badge still need a visual distinction? → A: No. Both
  badges are presented identically. The clause was written when the only known mechanism was polling
  and inference; the signal is now reported by the CLI, in the same vocabulary, just via a file
  rather than a call. A badge deliberately styled as less trustworthy than it is would mislead in
  the opposite direction. FR-018 is amended below rather than left standing on a premise that no
  longer holds.
- Q: FR-019's "no measurable resources while nothing is happening" and SC-006's "indistinguishable
  from today's" are not measurable as written. What does a test actually assert? → A: Structurally,
  that the observation is purely event-driven: no polling timer, no periodic wakeup, and no work
  scheduled per idle session. The test asserts the *absence* of a timer rather than measuring one,
  which is the only form of this claim that is deterministic on all three platforms in CI. A numeric
  wakeup budget was rejected as inherently flaky on shared runners, and a frame-count assertion as
  measuring the wrong process.
- Q: FR-004's fourth scenario rules out silently substituting a different CLI when the default is not
  installed, but not what happens instead. → A: Tell the user, and offer the available CLIs to pick
  from at that moment — the same list the per-session override shows. Nothing starts until they
  choose, and the stored default is left untouched so a temporary `PATH` problem cannot erase it.
  Refusing outright was rejected as a dead end that sends the user to Settings to do what they were
  already trying to do; creating a session that immediately fails was rejected as leaving a row that
  was never going to run.
- Q: FR-019 forbids any polling timer, but the application has no way to be told a file changed —
  and the plan claims the feature adds no new dependency. Which gives? → A: None of the
  requirements. The application adopts a cross-platform filesystem-watch facility so observation can
  be genuinely event-driven, and the plan's no-new-dependency claim is corrected and the addition
  justified where dependency decisions belong. Falling back to a bounded poll was rejected because
  it reintroduces the "how cheap is cheap" vagueness that FR-019 was just rewritten to remove;
  hand-writing three platform backends was rejected as the most platform-specific code this codebase
  would own, on its least-exercised path; and dropping the badge was rejected while a working
  mechanism is available.
- Q: Is the sidebar the only place a session's CLI needs to be identifiable? → A (user-directed): No.
  The terminal bar's AI-CLI mode toggle — the sparkle button at the bar's bottom-right — must carry
  the CLI's name as its own label, so the session you are looking at says which CLI it is running
  without a glance back at the sidebar. Recorded as FR-016a. *(The control named here is gone —
  feature 027 deleted the mode toggle. The answer's requirement is not: FR-016a records how it was
  re-homed onto the pinned AI tab that took the same corner.)*
- Q: A user who has run `copilot` by hand in a worktree for months has a lot of recorded history —
  253 session directories on the development machine, from incidental use. How much of it does a
  first project-open pull in? → A: All of it, on identical rules for both CLIs. Whatever a CLI
  recorded for that working directory is listed, and the durable close action (FR-015) is the answer
  to volume. Hiding history older than the application's first sight of the project, and capping the
  count per location, were both rejected for the same reason: they leave a session that exists and a
  sidebar that denies it, which is the gap reconciliation was added to close.
- Q: FR-016 requires a sidebar row to identify its CLI but not by what means, and colour is the
  cheapest thing to reach for in a dense row. → A: A short text label per row ("Claude",
  "Copilot"). Identification never depends on colour or on recognising a mark, it works at any
  contrast setting, and it keeps one vocabulary across both surfaces now that FR-016a puts the name
  on the terminal bar. A glyph with the name in a tooltip was rejected because a tooltip is not
  "from the sidebar alone" (SC-004); colour with a glyph was rejected because the colour then
  carries nothing the glyph does not, at the cost of failing alone.
- Q: SC-005's five-second bound was chosen when the mechanism was assumed to be polling a database.
  Against an event-driven watch it is a ceiling so far above the real behaviour that it would pass
  even if the watch were broken. → A: Tighten it to one second, on both CLIs — still generous
  against a file notification and a pushed hook, tight enough that a regression to polling fails it.
  **Not configurable in this feature**: the bound is a fixed one second here, and making it a user
  preference alongside the environment-include script timeout is a later change, out of scope.
- Q: The Assumptions still allow the Copilot badge to "degrade to absent", while FR-018 now requires
  a badge for every supported CLI. Both cannot hold. → A: Close the hatch. The badge is no longer
  the droppable slice: research R5 found the event log and this session committed to a watch
  facility to read it, so the mechanism is known to exist and known to be affordable. FR-018 stands
  unqualified. Dropping the badge from here would be a spec change made deliberately, with a reason
  recorded — not a fallback taken quietly during implementation.

### Session 2026-08-18

- Q: FR-014 lists sessions a CLI recorded outside this application. What happens when the user selects
  one whose CLI is still attached to that conversation in another terminal — a `copilot` left running
  in a tmux pane — since resuming would put two processes on one conversation store? → A: Attempt the
  resume and let the CLI decide. If it refuses or exits immediately, report that by the same route a
  missing CLI is reported (FR-010): start nothing, say why. The application performs **no liveness
  detection of its own** — neither CLI offers a lock or a liveness marker research verified, so any
  check we wrote would be a guess, and a guess that wrongly reports "in use" blocks a session the
  user can legitimately resume.
- Q: FR-018 requires an activity badge for sessions of every supported CLI, but a session discovered
  under FR-014 has no process this application spawned — and may be running right now under a terminal
  it does not own. Does the badge cover it? → A: No. The badge is a claim about a session this
  application is **supervising**; anything else reads as `Unknown`, which already means "no signal
  yet" and which FR-018's own conservatism clause forbids rendering as idle. Tailing an event log per
  discovered session would pay a watcher for each of the hundreds a long-lived worktree can hold, for
  a session the user may never open — which is the cost SC-006 exists to prevent.
- Q: FR-015 says a closed session is not rediscovered "on a later open", implying discovery runs at
  project open — but not whether that means *every* open. On a worktree with hundreds of recorded
  conversations the answer decides both cost and behaviour. → A: Every project open, and every
  reopen, for each of the project's locations. It is the only timing under which a session started
  outside the application appears without the user asking for it — including the second such session,
  which a first-open-only rule would never show. The cost is one index read or one directory listing
  per location, not work per conversation, so it does not grow with history.
- Q: FR-016 requires "a short text label" on the row and FR-016a the CLI's name on the bar's
  trailing control, but neither says what the text is — and the row's width budget makes the string length a constraint
  rather than a detail. → A: The **command name** — `claude`, `copilot`. It is what the user types,
  what their process list shows, and the shortest form that is still exact. This governs the two
  identification surfaces only; where the application *offers* a choice or *names a failure* — the
  Settings list, the override list, the missing-CLI message — the human-readable name ("Claude Code",
  "GitHub Copilot") is what belongs there, because those are sentences and menus rather than labels
  in a width budget.
- Q: The "long CLI history" edge case says the sidebar "must stay usable" at hundreds of recorded
  conversations — the last unquantified adjective in the spec, and one FR-014's uncapped rule plus
  per-open discovery makes routine rather than exceptional. → A: Make it **structural**, the way
  SC-006 already is. Building and rendering the tree costs no more per row at any count than it does
  today: no per-session I/O, no per-session watcher, nothing that grows faster than the list itself.
  Verified by test rather than by stopwatch — a timed ceiling would need a latency budget this
  repository has no precedent for measuring, and would fail on a slow runner for reasons that have
  nothing to do with the code.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a session on the CLI I choose (Priority: P1)

I have both Claude Code and GitHub Copilot CLI installed. I want to start a session in a worktree
and say which one of them runs in it — because they are good at different things, and because I am
evaluating one against the other.

Today every session is a `claude` session. There is no choice, and nothing in the application
acknowledges that another AI CLI exists.

**Why this priority**: It is the feature. Without it there is no second provider, only a seam that
nothing uses.

**Independent Test**: With both CLIs installed, start a session choosing Copilot. The session's
terminal shows a running Copilot CLI, in the right working directory, ready for a prompt. Start
another choosing Claude Code, in the same project, and both run at once without interfering.

**Acceptance Scenarios**:

1. **Given** both CLIs are installed and my default is Claude Code,
   **When** I start a session without choosing anything,
   **Then** a Claude Code session starts — unchanged from today.
2. **Given** both CLIs are installed,
   **When** I start a session and choose GitHub Copilot for it,
   **Then** a Copilot session starts in that location, and my default is unchanged.
3. **Given** I have set my default AI CLI to GitHub Copilot in Settings,
   **When** I start a session without choosing anything,
   **Then** a Copilot session starts.
4. **Given** a Claude Code session and a Copilot session are both running in the same project,
   **When** I switch between them,
   **Then** each shows its own CLI's output and neither has affected the other's conversation,
   working directory, or configuration.

---

### User Story 2 - The choice sticks (Priority: P1)

A session is a Copilot session for as long as it exists. When I quit the application and open it
again, that session is still a Copilot session — it resumes the Copilot conversation I was having,
not a fresh one, and not a Claude Code one.

**Why this priority**: A choice the application forgets is not a choice. Sessions are persisted and
restorable across restarts (Principle II); "which CLI" is now part of what makes a session
restorable at all, because resuming with the wrong CLI does not resume anything — it starts a
stranger in my worktree.

**Independent Test**: Start a Copilot session, have a short conversation in it, quit the
application, open it again, and select that session. The Copilot conversation is there.

**Acceptance Scenarios**:

1. **Given** a Copilot session with a recorded conversation,
   **When** I quit and reopen the application and select that session,
   **Then** it resumes as a Copilot session with that conversation, not a new one.
2. **Given** sessions that existed before this feature was installed,
   **When** the application opens,
   **Then** they are Claude Code sessions and behave exactly as they did before — none are lost,
   relabelled, or restarted on a different CLI.
3. **Given** a Copilot session was started in a worktree outside this application,
   **When** I open that project,
   **Then** the application lists it as a Copilot session in that worktree, the same way it already
   discovers Claude Code sessions started elsewhere.
4. **Given** I closed a Copilot session,
   **When** I reopen the project,
   **Then** it stays closed and does not reappear — the same durable suppression Claude Code
   sessions already have.

---

### User Story 3 - I can see which CLI a session is running (Priority: P2)

Looking at the sidebar, I want to know at a glance which of my sessions is Claude Code and which is
Copilot, without opening each one — and once one is open, I want the session itself to say which CLI
I am talking to.

**Why this priority**: The point of running two is telling them apart. It is not required to *use*
the feature — hence P2, not P1 — but without it a user with mixed sessions is guessing.

**Independent Test**: With one session of each kind in a project, look at the sidebar. Each row
identifies its CLI, and the two are distinguishable without hovering, opening, or reading a tooltip.
Open each in turn: its terminal bar names the CLI it is running.

**Acceptance Scenarios**:

1. **Given** a project with a Claude Code session and a Copilot session,
   **When** I look at the sidebar,
   **Then** each row identifies which CLI backs it.
2. **Given** a Copilot session whose CLI has recorded a title for the conversation,
   **When** I look at the sidebar,
   **Then** the row is labelled with that title, the same way a Claude Code row is.
3. **Given** a Copilot session that is mid-response,
   **When** I look at the sidebar,
   **Then** its activity badge reflects that it is working, within a second of it starting to.
4. **Given** a Copilot session I have open,
   **When** I look at its terminal bar,
   **Then** the pinned AI tab reads `copilot` beside its glyph — and a Claude session's reads
   `claude` — whichever pane the session is showing.

---

### User Story 4 - Sensible behaviour when a CLI is not there (Priority: P2)

I only have Claude Code installed. The application should not offer me a CLI I cannot run, and if a
CLI disappears after I have sessions on it, it should say so rather than fail obscurely.

**Why this priority**: Most users will have exactly one of these installed. The feature must not
degrade their experience — an unusable menu entry, or a session that dies silently at launch, would
both be worse than today.

**Independent Test**: With only one CLI installed, open the session-creation surface and Settings.
The missing CLI is either absent or clearly marked unavailable, and cannot be chosen by accident.

**Acceptance Scenarios**:

1. **Given** only Claude Code is installed,
   **When** I start a session or open Settings,
   **Then** GitHub Copilot is not offered as a choice I can make, and Claude Code is my default.
2. **Given** neither CLI is installed,
   **When** I try to start a session,
   **Then** the application says which CLI it could not find and what to do about it, rather than
   opening a session whose process is already dead.
3. **Given** an existing Copilot session and Copilot CLI has since been uninstalled,
   **When** I open the project,
   **Then** the session is still listed and identified as a Copilot session, and selecting it
   reports that its CLI is missing rather than appearing to start.
4. **Given** my default AI CLI in Settings names a CLI that is no longer installed,
   **When** I start a session,
   **Then** the application tells me and offers the CLIs that are available so I can pick one; it
   does not silently substitute a different CLI, nothing starts until I choose, and my stored
   default is left as it was.

---

### Edge Cases

- **A worktree Copilot has not been trusted for.** GitHub Copilot CLI keeps its own list of trusted
  folders and prompts interactively the first time it runs somewhere new. Every worktree this
  application creates is somewhere new. The session must not appear broken or hung while that
  prompt is waiting — the prompt is the CLI's, shown in the session's own terminal, and the user
  answers it there.
- **Two storage layouts, one meaning.** Both CLIs partition by working directory, in different
  shapes: Claude Code files each conversation under a directory named for the one it ran in, while
  Copilot keeps a per-working-directory index listing session ids and files each conversation in its
  own directory elsewhere. Any behaviour phrased as "the sessions for this worktree" — discovery,
  reconciliation, the durable closed marker — has to keep meaning the same thing under both layouts.
- **A working directory with a long CLI history.** A user who has run a CLI by hand in a worktree
  for months may have hundreds of recorded conversations there. All of them are listed, on every open
  (FR-014), so this is a routine size rather than an outlier. The sidebar must cost no more per row
  at that size than at any other (SC-009), and closing sessions must remain the way the list gets
  shorter.
- **A session id one CLI knows and the other does not.** Both CLIs accept an application-chosen
  session id, so the same id could in principle exist in both stores. Nothing may resume, label, or
  archive a session using the wrong CLI's store just because an id matches.
- **The default is changed while sessions are running.** Changing the default AI CLI in Settings
  must affect only sessions started afterwards. No running or existing session changes CLI.
- **A discovered session that is running somewhere else.** It is listed (FR-014) and identified by
  its CLI (FR-016), and its activity reads as unknown, because this application is not supervising it
  and does not watch its storage to find out (FR-018). Unknown is never rendered as idle.
- **Two CLIs, two activity sources.** Claude Code pushes its turn events; Copilot's are read from a
  log it writes. A project with both must not have one starve, mislead, or throttle the other, and
  neither source may keep the machine busy when nothing is happening.
- **A conversation another process is already attached to.** A user may have left a CLI running in
  a terminal outside this application, on a conversation this application then lists (FR-014).
  Selecting it attempts the resume like any other; if the CLI refuses or exits, that is reported and
  nothing starts (FR-008). The application does not test for the conflict beforehand — neither CLI
  exposes a lock or a liveness marker to test against, and a false "in use" would block a resume the
  user is entitled to.
- **A Copilot session whose store entry was removed.** Resuming a conversation the CLI no longer has
  must report that, by the same route a missing CLI is reported (FR-010), and must start nothing —
  not a silent empty terminal, and not a fresh conversation under the old session's identity.
- **An in-flight upgrade.** A user upgrades with sessions already open; the persisted records carry
  no CLI. They are Claude Code sessions, and nothing about them is re-derived from disk in a way
  that could change that.

## Requirements *(mandatory)*

### Functional Requirements

**Choosing**

- **FR-001**: A session MUST record which AI CLI backs it, fixed when the session is created and
  unchanged for the session's lifetime.
- **FR-002**: Users MUST be able to set a default AI CLI, which applies to every session started
  without an explicit choice. When the stored default is not installed, starting a session MUST say
  so and offer the CLIs that are available; nothing starts until the user chooses, and the stored
  default MUST NOT be rewritten — a CLI missing from `PATH` today may be back tomorrow.
- **FR-003**: The initial default MUST be Claude Code, so a user who changes nothing sees no change
  in behaviour.
- **FR-004**: Users MUST be able to override the default for a single session at the point of
  starting it, without changing the default. Starting a session on the default MUST remain a single
  interaction, unchanged from before this feature; the override MUST be a distinct, adjacent
  secondary control, so no cost is imposed on the user who never overrides (SC-001).
- **FR-005**: Changing the default MUST NOT affect any session that already exists.
- **FR-006**: The application MUST NOT offer an AI CLI it cannot find on the system as a selectable
  choice. Where the application offers a choice — the Settings default, the per-session override — the
  CLIs MUST be named by their human-readable names, not by the command-name form FR-016 uses for row
  and bar labels. When fewer than two CLIs are available there is nothing to choose between, so the override
  control MUST be absent rather than present-and-empty.

**Running**

- **FR-007**: Starting a session MUST launch the CLI that session records, in the session's own
  working directory, with a session identity the application owns — so the same session can later be
  resumed by that CLI.
- **FR-008**: Resuming a session MUST resume that session's own conversation in the CLI it records,
  and MUST NOT start a fresh conversation when a recorded one exists. When the CLI refuses the resume
  or exits immediately — including because another process is already attached to that conversation —
  the application MUST report that by the same route FR-010 uses and start nothing. It MUST NOT
  attempt to detect, on its own, whether a conversation is in use elsewhere.
- **FR-009**: Sessions backed by different CLIs MUST run concurrently and remain fully isolated from
  one another — no shared conversation, configuration, working directory, or terminal state.
- **FR-010**: When a session's CLI cannot be found at launch, the application MUST report which CLI
  is missing by its human-readable name ("Claude Code", "GitHub Copilot"), and MUST NOT present the
  session as started. The command-name form FR-016 uses is for labels in a width budget; a message is
  a sentence.
- **FR-011**: The application MUST NOT modify either CLI's user-level configuration; any per-session
  configuration it needs MUST be supplied to that launch alone.

**Persisting and rediscovering**

- **FR-012**: A session's recorded CLI MUST survive application restart, daemon restart, and machine
  reboot.
- **FR-013**: Sessions persisted before this feature existed MUST load as Claude Code sessions,
  with no loss, relabelling, or restart.
- **FR-014**: For each supported CLI, the application MUST discover sessions that CLI has recorded
  for a location but which the application has no record of, and list them as sessions of that CLI.
  Discovery MUST run on **every** project open and reopen, for each of the project's locations, so a
  session started outside the application appears without the user asking for it. The work MUST be
  proportional to the number of locations, not to the number of conversations recorded in them.
  Discovery MUST NOT be bounded by count or by age, and MUST apply identical rules to every CLI: a
  conversation that exists for that working directory is listed, however long ago it was recorded
  and however many there are. Suppressing volume is the user's call, through the durable close
  action (FR-015), not the application's through a hidden cap.
- **FR-015**: Closing or removing a session MUST be durably suppressed for every supported CLI, so a
  closed session is not rediscovered by FR-014 on a later open.

**Showing**

- **FR-016**: The sidebar MUST identify which AI CLI backs each session, by a short text label on the
  row carrying the CLI's **command name** — `claude`, `copilot`. Identification MUST NOT depend on
  colour alone, on a glyph alone, or on a tooltip — it MUST be readable from the sidebar as rendered,
  at any contrast setting (SC-004).
- **FR-016a**: The open session's own terminal bar MUST name its AI CLI by the same command name
  FR-016 uses, as text beside the existing glyph on the bar's **pinned AI tab** — so the session in
  front of the user says which CLI it is running without a glance back at the sidebar. The name is
  drawn whichever pane the session is showing, because the tab is (feature 027 FR-007).

  ~~*as the text of the AI-CLI mode toggle beside that control's existing icon … The name appears
  only while the session is in AI CLI mode; in regular-terminal mode there is no CLI to name and the
  control is unchanged.*~~ — **amended**. The clarification this requirement came from named the
  control at the bar's bottom-right, which was the AI-CLI/Regular mode toggle on the `main` this
  feature branched from. Feature `027-tabs-only-switching` deleted that toggle (its FR-001) and the
  pinned AI tab took the corner, so the name is re-homed onto the tab rather than retired: what the
  clarification asked for is the *session naming its own CLI*, and the control it named was simply
  the one standing there at the time. 027's "and no control MUST replace it" is not violated —
  nothing is added to the bar. The tab was already there, already unconditional, and 027's own
  reasoning for preferring tabs to a toggle is that *a tab names where it goes*; a glyph-only AI tab
  says only that it goes to an assistant, not which one. The mode condition goes with the toggle: a
  tab that came and went with the pane would break 027 FR-007 (feature 023 FR-008a).
- **FR-017**: For each supported CLI, a session's sidebar label MUST use the title that CLI has
  recorded for the conversation, when it has recorded one; a missing or unreadable title MUST fall
  back to the existing label and MUST NOT fail the session.
- **FR-018**: The sidebar MUST show a busy/idle activity badge for sessions of every supported CLI,
  presented identically for all of them. The badge covers sessions this application is **supervising**;
  a session it has discovered (FR-014) but is not running MUST read as unknown rather than as idle or
  working, and MUST NOT cause the application to observe that session's storage. Starting such a
  session here makes it an ordinary session, badge included. The signal MUST be conservative — it may lag, but it MUST
  NOT claim a session is idle while it is working. It MUST NOT be styled to look less certain than
  it is: the original requirement to mark a derived signal as less authoritative was written when
  the only known mechanism was inference from a database, and no longer applies to a CLI that
  reports its own turn events (Clarifications, 2026-08-16).
- **FR-019**: Observing a CLI's storage for activity MUST be purely event-driven **in this
  application's own scheduling**: the application MUST NOT set a polling timer, a periodic wakeup, or
  any per-idle-session work of its own — it waits to be told a file changed. Observation MUST NOT
  degrade the responsiveness of sessions on the other CLI. Being told MUST work equivalently on all
  three supported platforms, behind one abstraction rather than three platform-specific code paths
  (Principle VI). Where the underlying platform or filesystem offers no native notification (network
  mounts, some container filesystems), the watch facility's own fallback is acceptable: latency
  degrades, correctness does not, and the application still schedules nothing itself.

**Keeping the seam honest**

- **FR-020**: Every CLI-specific detail — the command, its arguments, where it stores conversations,
  how a title is read, how activity is observed, how a closed session is marked — MUST be reached
  through the single provider seam, so that adding a third CLI requires adding one implementation
  and touching no session, storage, sidebar, or terminal code.
- **FR-021**: The seam MUST NOT assume any one CLI's storage layout. The rationale first written
  here — that one of the two CLIs does not organise storage by working directory — was **wrong**,
  and research R3 corrects it: both CLIs partition by working directory, but in different shapes
  (a directory of conversation files against an index file listing ids). The requirement stands on
  the corrected ground: the seam must not assume the *shape*, and must provide no default that
  encodes one.
- **FR-022**: A verifiable check MUST exist that no code outside the seam depends on a specific AI
  CLI, so the property in FR-020 cannot silently erode.

### Key Entities

- **AI CLI provider**: One supported AI coding CLI. Identified stably enough to be written to disk
  and read back. Knows how to be launched fresh or resumed for a given session, where its
  conversations live, how to read a conversation's title, how to tell whether a conversation was
  recorded, how to mark one as closed, and how to observe whether it is working.
- **Session**: Gains one attribute — which provider backs it — set at creation, persisted, and
  never changed thereafter. Everything else about a session is unchanged.
- **Provider availability**: Whether a given CLI can be found on this machine. Consulted when
  offering choices and when launching; not persisted.
- **Default AI CLI setting**: The user's chosen provider for new sessions. Persisted with the
  application's other settings, independent of any project or session.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with both CLIs installed can start a session on either one in a single
  interaction, with no visit to Settings and no more clicks than starting a session takes today.
- **SC-002**: 100% of sessions resume on the CLI they were created with, across application restart,
  daemon restart, and machine reboot.
- **SC-003**: 100% of sessions that existed before the upgrade continue to work unchanged after it —
  none lost, relabelled, or started on a different CLI.
- **SC-004**: Given a project with sessions on both CLIs, a user can tell which is which from the
  sidebar alone — by a command name they recognise, not a code they have to learn — without opening a
  session — and, with a session open, can name its CLI from the
  terminal bar alone, without looking back at the sidebar.
- **SC-005**: A session that starts working is shown as working within 1 second, on either CLI. The
  bound is fixed in this feature; exposing it as a user preference — as the environment-include
  script timeout already is — is a later change and explicitly out of scope here.
- **SC-006**: With sessions idle, the application itself schedules no work to observe a CLI's
  storage: zero polling timers and zero periodic wakeups in its own code, verified structurally
  rather than by measurement. A project holding hundreds of discovered-but-unsupervised sessions
  (FR-014) schedules no observation work at all, however many there are. A watch facility's internal fallback on a filesystem without native
  notification is out of that scope.
- **SC-007**: Adding a third AI CLI later requires changes in one place only, demonstrated by an
  automated check that fails if session, storage, sidebar, or terminal code names a specific CLI.
- **SC-008**: A user with only one CLI installed is never offered, and never lands in, a state that
  requires the other.
- **SC-009**: A project holding hundreds of discovered sessions builds and renders its sidebar at the
  same cost per row as one holding a handful: no per-session I/O, no per-session watcher, and nothing
  that grows faster than the list itself. Verified structurally rather than by measurement, for the
  same reason SC-006 is.

## Assumptions

- **Both CLIs accept an application-chosen session identity.** Verified against GitHub Copilot CLI
  1.0.62, which accepts a caller-supplied session id for a new session and can resume an existing
  one by id — matching what the application already relies on from Claude Code. If a future CLI does
  not, that is that CLI's feature to solve, not this one's.
- **Copilot CLI's own storage is readable.** Its conversation records live on the local filesystem
  under the user's own home directory, so reading them for titles, discovery, and activity needs no
  network and no credential (Principle IV).
- **Copilot reports its own turn events.** It writes them to a log as a session runs, so the signal
  is read rather than inferred, and the application is told when the log changes rather than asking
  (FR-019). It may still lag by the time a write takes to be noticed. This was originally written as
  the one droppable part of the scope, on the assumption that the signal would have to be inferred
  from a database; that assumption was disproved and the escape hatch is withdrawn — see
  Clarifications, 2026-08-16.
- **Provider choice is per session, not per worktree or per project.** Two sessions in the same
  worktree may be backed by different CLIs.
- **No migration path between CLIs.** A conversation cannot be moved from one CLI to another, and
  this feature does not attempt it.
- **Exactly two providers in this feature.** Claude Code and GitHub Copilot CLI. Others (Codex,
  Gemini) are each a later, thin feature of their own, which is the point of FR-020.
- **The provider seam already exists** and is the starting point, not something to be invented — but
  it was written against a single implementation and is expected to change shape here (FR-021).
- **Authentication is each CLI's own business.** The application does not log a user in to either
  CLI, store credentials for either, or surface either one's auth state beyond reporting a launch
  that did not succeed.
