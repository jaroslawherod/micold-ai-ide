# Feature Specification: Structured Session — Usage, Context Meter, Subagents and Background Tasks

**Feature Branch**: `claude/project-thread-lllxzt`

**Created**: 2026-10-21

**Status**: Draft

**Input**: GitHub issue #944 "Structured session: usage, context meter, subagents and background
tasks". "Part of the structured-session (stream-json) UI epic. Keeps the PTY/TUI session mode as
the fallback throughout. Show token/cost/context usage from `result` and usage events, plan-usage
integration, and nested subagent (Task) transcripts as expandable child threads. Acceptance:
Context meter and per-turn cost visible. Subagent activity nested under the parent tool call. Feeds
the existing plan_usage feature."

## In short (for non-technical readers)

A structured Claude session (feature 938) shows Claude's work as a readable conversation (#939).
This feature adds the numbers and the hidden work around that conversation. A small meter shows how
full Claude's working memory (its context) is, so the user sees a long conversation approaching
the point where Claude starts summarising older parts. Each answer shows how many tokens it used,
next to the cost and duration #939 already shows, and the session shows its running total. When
Claude hands part of a job to a helper (a subagent), the helper's own messages and tool uses appear
folded under the step that started it, and open with one press, instead of being mixed into the
main conversation or missing. Work Claude left running in the background (a long command, a helper
working on its own) is listed with whether it is still running or how it ended. And the plan-usage
indicator the application already has (feature 490) also learns from structured sessions, which
otherwise could not feed it. Terminal sessions are unchanged and remain the default.

## Context: where this sits in epic #945

| Issue | Delivers | State |
|---|---|---|
| #938 | Session kind, event model, daemon side, wire, resume | shipped (`specs/938-structured-session-mode`) |
| #939 | Read-only transcript view: assistant text, thinking, tool-call cards, a per-turn footer with cost and duration | specced in parallel, not on main |
| #940 | Markdown, highlighted code, Edit/Write diffs in that view | specced in parallel |
| #941 | Prompt input, interrupt, queued messages | specced in parallel |
| #942 | Permission prompts | specced in parallel |
| #943 | Slash commands, model switching, falling back to the terminal mode | specced in parallel |
| **#944 (this)** | Usage and context meter, token figures, plan-usage feed, nested subagent threads, background tasks | — |

This feature is built on #939's transcript view and does not re-specify it. In particular, #939
owns the transcript, the tool-call card, and the per-turn footer with the turn's cost and
duration; this feature **extends** that footer with the turn's token figures and adds what is new
here (context meter, session totals, plan-usage feed, subagent threads, background tasks).

## Terms

- **Structured session, event, event history, turn result, Save terminal history**: as defined in
  feature 938's spec.
- **Transcript view, tool-call card, turn footer**: as defined by #939. The **turn footer** is the
  line #939 shows at the end of each turn with that turn's cost and duration.
- **Token figures** of a turn: the tokens the CLI reports for it, split into input, output, and
  the input tokens served from or written to the prompt cache.
- **Context**: what the model is given on each request in the conversation; its size is measured
  in tokens. **Context used** is the size of the context as of the latest model response the CLI
  reported; **context window** is the most the model accepts; the **compaction threshold** is the
  size at which the CLI starts summarising older parts of the conversation automatically, when the
  CLI reports one.
- **Context meter**: the compact display, in a structured session's view, of context used against
  the context window, with the compaction threshold marked when known.
- **Session totals**: the sum over the session's turns of cost and of each token figure.
- **Plan usage, usage reading, limit window**: as defined in feature 490's spec. A **plan-limit
  event** is the event in which the CLI reports, in structured mode, the account's limit windows
  with their used share and reset time.
- **Subagent**: a helper Claude conversation the CLI starts for one tool call of the main
  conversation (the task/agent tool). The CLI marks every message and partial delta of a subagent
  with the identity of the tool call that started it (the **parent tool call**).
- **Subagent thread**: the subagent's messages, thinking, tool calls and results, shown folded
  under the parent tool call's card.
- **Background task**: work the CLI starts and leaves running while the conversation goes on — a
  shell command run in the background, or a subagent started in the background — and later reports
  as finished, failed or stopped.

## Clarifications

### Session 2026-10-21

- Q: How is cost shown for an account on a subscription plan, for which the CLI still reports a
  list-price cost? (FR-026) → A: Shown for every account, never hidden, and always labelled as an
  estimate: the figure is Claude Code's own estimate at API list prices, not what the account is
  billed, and the cost details say so in words ("estimated at API list prices; a subscription plan
  does not bill this amount"). The application does not detect the plan type for this.
  _(agent-resolved: issue #944 acceptance "per-turn cost visible";
  https://code.claude.com/docs/en/agent-sdk/cost-tracking — `total_cost_usd` and `costUSD` "are
  client-side estimates, not authoritative billing data", computed "from a price table bundled at
  build time"; https://code.claude.com/docs/en/agent-sdk/typescript#sdkusagereport — "an estimate
  and not what your plan bills")_
- Q: May the CLI's plan-limit event feed the plan-usage indicator (FR-010, amending feature 490's
  FR-012)? → A: Yes. Feature 490's FR-012 is amended to also accept the CLI's documented plan-limit
  event (`rate_limit_event`, `SDKRateLimitEvent` in
  https://code.claude.com/docs/en/agent-sdk/typescript#sdkratelimitevent) from structured sessions.
  Its documented fields carry no window name, and CLI 2.1.302 writes no top-level `utilization`,
  so the per-window figures are read, tolerantly, from the event's `unifiedWindows` (`five_hour`
  and `seven_day`, each a used share and a reset time). A window or event of any other shape yields
  no reading and no error. _(decided by user (jaro, 2026-10-21))_
- Q: Are the CLI's cost and per-model figures per turn? (FR-006, FR-007, FR-009) → A: No. In the
  streaming-input mode structured sessions use, a turn result's `total_cost_usd` and per-model
  figures (`modelUsage`) are the running totals of the CLI's session — counting subagents, and
  including the totals restored on resume — while its `usage` covers only that turn's main
  conversation, without subagents. A turn's cost and per-model figures are therefore the
  difference from the previous turn result of the same count. _(agent-resolved:
  https://code.claude.com/docs/en/agent-sdk/cost-tracking#track-costs-in-streaming-input-mode and
  #accumulate-costs-across-multiple-calls; crates/micold-core/tests/fixtures/stream-json/
  resume-init.jsonl, whose result reports 0.0954 USD and 20 output tokens for claude-sonnet-5-5
  after text-turn.jsonl's 0.0371 USD and 14)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See how full the context is (Priority: P1)

A user working in a long structured session sees, at all times, how much of Claude's context the
conversation now fills, and how close it is to the point where the CLI will compact it, so they can
decide to start a new session or compact on purpose before Claude loses detail.

**Why this priority**: It is the issue's first acceptance item, and the one figure a user cannot
get any other way from a structured session (the terminal mode's own status display does not exist
in it).

**Independent Test**: Feed the client's view model a recorded fixture's events; the meter shows
the context used and window taken from the fixture, its share as a percentage, and the compaction
threshold where the fixture reports one; feed a later response with a larger context and the
meter rises.

**Acceptance Scenarios**:

1. **Given** a structured session whose CLI has reported at least one model response, **When** the
   user views the session, **Then** the context meter shows the context used, the context window,
   and the share used as a whole percentage.
2. **Given** the CLI reported a compaction threshold, **When** the meter is shown, **Then** the
   threshold is marked on it and its details name it.
3. **Given** context used reaches the compaction threshold or 90% of the window when no threshold
   is known, **When** the meter is shown, **Then** it shows a warning look that differs from the
   normal look by an icon or text and not by colour alone.
4. **Given** a new response reports a different context size (it grew, or the CLI compacted the
   conversation and it shrank), **When** the event arrives, **Then** the meter shows the new size
   within one second.
5. **Given** a session in which no model response has been reported yet, **When** the user views
   it, **Then** the meter is shown empty with a "no data yet" note, never as 0% of an assumed
   window.
6. **Given** a response from a model whose context window the CLI did not report, **When** the
   meter is shown, **Then** it shows the context used in tokens without a percentage or bar.

---

### User Story 2 - See what each turn and the session used (Priority: P1)

A user sees, at the end of each turn, the tokens the turn used next to the cost and duration #939
already shows, and sees the session's running totals, so they can tell expensive turns from cheap
ones and know what the session has consumed.

**Why this priority**: The acceptance names per-turn cost; #939 shows the cost, and this story
adds the token figures and the totals that make it useful.

**Independent Test**: Replay a fixture with two turns into the view model; each turn footer shows
that turn's token figures, and the session totals equal the sum of the turns' cost and figures.

**Acceptance Scenarios**:

1. **Given** a turn that ended with a turn result reporting usage, **When** the turn footer is
   shown, **Then** it shows, beside #939's cost and duration, the turn's input and output tokens,
   with the cached input tokens in its details.
2. **Given** a session with several finished turns, **When** the user views the session's usage
   details, **Then** they show the session totals (cost and each token figure) and the number of
   turns counted.
3. **Given** a turn whose result reports no usage, or a turn that ended without a result (feature
   938's "ended without a result" note), **When** its footer is shown, **Then** the token figures
   are absent rather than shown as zero, and the totals say they do not include that turn.
4. **Given** the CLI reports cost and tokens per model (for example a main model and a smaller one
   used for housekeeping), **When** the user opens a turn's or the session's usage details,
   **Then** they list each model with its tokens and cost.
5. **Given** earlier events were evicted from the event history (feature 938, FR-018), **When** the
   totals are shown, **Then** they are marked as covering only the turns still held.

---

### User Story 3 - Follow a subagent's work under the step that started it (Priority: P1)

When Claude starts a subagent through a tool call, the subagent's own messages, tool calls and
results appear as a thread folded under that tool call's card, and the user expands it to follow
what the helper did, while the main conversation stays readable.

**Why this priority**: The acceptance names it. Without it, a subagent's events either clutter the
main transcript or are invisible, and a session that delegates work cannot be followed.

**Independent Test**: Replay a recorded fixture of a turn that starts a subagent into the view
model; every event marked with the parent tool call's identity lands in that call's thread and
none in the main transcript; the thread is folded by default and expands to show them in order.

**Acceptance Scenarios**:

1. **Given** a tool call that started a subagent, **When** the transcript shows its card, **Then**
   the card offers a folded thread with a one-line summary: the subagent's state (running,
   finished, failed), the number of its tool calls, and its latest activity while running.
2. **Given** a folded subagent thread, **When** the user expands it, **Then** it shows the
   subagent's text, thinking, tool calls and their results in the order the CLI wrote them, with
   the same cards and rendering the main transcript uses, visibly indented under the parent card.
3. **Given** a running subagent, **When** its events arrive interleaved with the main
   conversation's, **Then** each goes to the thread of its parent tool call and none to the main
   transcript, and an expanded thread updates live.
4. **Given** a subagent that itself starts a subagent, **When** the transcript is shown, **Then**
   the inner thread is folded under the inner tool call's card inside the outer thread, at any
   depth.
5. **Given** several subagents running at once, **When** their events interleave, **Then** each
   thread receives exactly its own events.
6. **Given** a subagent's parent tool call was evicted from the event history, or an event names a
   parent tool call the session never showed, **When** the event arrives, **Then** it is shown in a
   thread under a placeholder that names the unknown parent, never dropped and never placed in the
   main transcript.
7. **Given** a user who expanded a thread, **When** new events arrive or another window views the
   same session, **Then** the expansion is kept in that window only and does not change what other
   windows show.

---

### User Story 4 - Structured sessions feed the plan-usage indicator (Priority: P2)

A user who runs only structured sessions still sees the plan-usage indicator of feature 490 kept up
to date, because each structured session passes the account's limit windows the CLI reports in
structured mode to that feature, as terminal sessions do through their status line.

**Why this priority**: The acceptance names it. A structured session has no status line, so
without this a user who switches to structured sessions loses the plan-usage indicator.

**Independent Test**: With **Show Claude plan usage** on, run the daemon against the stand-in CLI
replaying a fixture that contains a plan-limit event; a test client receives a usage reading with
the fixture's limit windows, percentages and reset times. With the switch off, it receives none.

**Acceptance Scenarios**:

1. **Given** **Show Claude plan usage** is on, **When** a structured session's CLI writes a
   plan-limit event, **Then** the application takes a usage reading from it, with each limit
   window's name, used percentage and reset time, and every window shows it within 10 seconds, as
   feature 490 requires of a terminal session's reading.
2. **Given** readings from terminal and structured sessions, **When** a newer one arrives from
   either kind, **Then** it replaces the current one (feature 490, FR-019: newest wins).
3. **Given** **Show Claude plan usage** is off, **When** a plan-limit event arrives, **Then** no
   reading is taken, kept or shown; the event still stays in the session's event history.
4. **Given** a plan-limit event the application cannot read (missing or unexpected fields), **When**
   it arrives, **Then** no reading is taken from it, nothing is shown about it, and the session and
   the current reading are unaffected (feature 490, FR-015, FR-016).
5. **Given** a structured session restored after a daemon restart with its saved event history,
   **When** the history is restored, **Then** plan-limit events from before the restart do not
   produce a new reading; only events the running CLI writes do.

---

### User Story 5 - See background tasks and how they ended (Priority: P3)

When Claude leaves work running in the background — a shell command or a subagent — the user sees
a list of the session's background tasks with each one's description and state, and learns when one
finishes, fails or is stopped, without searching the transcript.

**Why this priority**: The issue title names it, but the acceptance does not; it is the smallest
slice of value and is built on the same event attribution as US3.

**Independent Test**: Replay a recorded fixture in which the CLI starts a background command and
later reports it finished; the session's background-task list shows the task as running, then as
finished, and the transcript's tool-call card that started it links to the list entry.

**Acceptance Scenarios**:

1. **Given** the CLI starts a background task, **When** the event arrives, **Then** the session's
   background-task list shows it with its description, its kind (command or subagent) and the
   state running, and the session shows a count of running background tasks.
2. **Given** a running background task, **When** the CLI reports it finished, failed or stopped,
   **Then** its entry shows that state and, where the CLI reports one, its outcome summary; the
   running count drops.
3. **Given** a background task started by a tool call, **When** the user views that tool call's
   card, **Then** the card shows the task's current state.
4. **Given** the session's CLI exits or is restarted while background tasks were running, **When**
   the session's state changes, **Then** those tasks are shown as ended without a report, not as
   still running.
5. **Given** a session with no background tasks, **When** the user views it, **Then** no
   background-task list or count is shown.

---

### Edge Cases

- **No data yet**: a session that has not finished a turn shows an empty meter with a note, no
  totals and no background-task list; nothing shows 0 as if it were measured.
- **Missing or malformed figures**: a turn result or plan-limit event missing a field, or with a
  value of the wrong kind, leaves that figure absent; it never fails the session, the view, or the
  plan-usage reading already shown (feature 938, FR-012: such an event may arrive as unrecognised).
- **Unknown models**: a model whose context window is not reported shows tokens only (US1 s6).
- **Compaction**: after the CLI compacts the conversation, context used drops; the meter follows
  the latest response and does not keep the old maximum.
- **Model change** (#943): after the session's model changes, the meter uses the window reported
  for the model of the latest response.
- **Resume and restart**: a restored session's meter, totals, threads and background-task list are
  rebuilt from its event history, so a window opening it after a daemon restart (with **Save
  terminal history** on) sees what it saw before; with the setting off they start from the resume.
  Totals that the CLI reports cumulatively per process are never double-counted across a restart.
- **Eviction**: when the event history has evicted early events, totals are marked partial (US2
  s5), and subagent events whose parent card was evicted go under a placeholder (US3 s6).
- **Late and multiple windows** (Principle II): every window viewing a session shows the same
  meter, totals, threads and background tasks, built from the same events; fold and unfold state
  is per window.
- **Several sessions at once** (Principle II): each session's meter, totals, threads and background
  tasks come only from its own events; plan-usage readings from all sessions feed the one shared
  indicator, newest wins, as in feature 490.
- **Long or deep subagent threads**: a thread with many events, or nesting several levels deep,
  stays usable; folded threads cost nothing to show until expanded.
- **Subagent that never ends**: if the CLI exits or the turn ends without a result while a
  subagent is running, its thread shows it as ended without a result, not as running.
- **Background task reported for an unknown start** (the start was evicted, or never written):
  the report still creates an entry, with the description the report carries.
- **Terminal sessions**: nothing in this feature changes a terminal session; its plan usage keeps
  coming from its status line (feature 490).
- **Cross-platform** (Principle VI): figures are formatted the same on Linux, macOS and Windows,
  with the user's locale digit grouping for token counts, cost in US dollars with at least two and
  at most four decimals, and reset times in local time as feature 490 formats them.

## Requirements *(mandatory)*

### Functional Requirements

**Context meter**

- **FR-001**: A structured session's view MUST show a context meter with the context used as of
  the latest model response the CLI reported, the context window of that response's model, and the
  share used as a whole percentage.
- **FR-002**: When the CLI reports a compaction threshold, the meter MUST mark it and its details
  MUST name it.
- **FR-003**: The meter MUST show a warning look when context used is at or above the compaction
  threshold, or at or above 90% of the window when no threshold is reported; the warning MUST
  differ from the normal look by an icon or text, not by colour alone.
- **FR-004**: Before any model response is reported, the meter MUST show that no data exists yet;
  when the window is unknown, it MUST show tokens only. It MUST NOT show a figure the CLI did not
  report.
- **FR-005**: The meter MUST follow each new response, including a smaller context after
  compaction, within one second of the event reaching the client.

**Turn and session usage**

- **FR-006**: The turn footer #939 shows MUST be extended with the turn's input and output tokens
  (the main conversation's, as the turn result's usage reports them); its details MUST show the
  cached input tokens and, when the CLI reports them per model, each model's tokens and cost for
  the turn, subagents included.
- **FR-007**: A structured session MUST show its session totals — cost and each token figure — and
  the number of turns they cover, in the session's usage details.
- **FR-008**: A turn without reported usage MUST show no token figures, and the totals MUST say
  that they leave it out. Totals MUST be marked partial when the event history has evicted earlier
  events.
- **FR-009**: Totals MUST be correct across process restarts and resumes of the session: no turn
  is counted twice and no turn held in the history is left out. The CLI reports cost and per-model
  figures as running totals of its session (restored on resume): a turn's cost and per-model
  figures MUST be its result's figures minus those of the previous turn result held for the
  session, or its result's figures as they are when none is held or when a figure is lower than the
  previous one (the CLI started counting again). A turn whose previous result was evicted MUST show
  no per-turn cost or per-model figures and is left out of the totals as FR-008 says.

**Plan-usage feed**

- **FR-010**: While **Show Claude plan usage** is on, each plan-limit event a structured session's
  running CLI writes MUST yield a usage reading for feature 490, with each limit window's name, used
  percentage and reset time, shown within 10 seconds. This **amends feature 490's FR-012**: besides
  the status-line input, the CLI's documented plan-limit event in structured mode is a usage source.
  The windows are read from the event's `unifiedWindows` (`five_hour`, `seven_day`: used share and
  reset time), which the CLI writes but does not document; they are read tolerantly, so a window
  missing or of another shape is left out and an event with no readable window yields no reading
  (FR-012). The source reads no credential and makes no request of its own (feature 490, FR-013,
  FR-014 hold).
- **FR-011**: While the switch is off, plan-limit events MUST NOT yield, keep or show a reading.
- **FR-012**: A plan-limit event that cannot be read MUST yield no reading and show nothing, and
  MUST NOT change the current reading or affect the session; failures follow feature 490's logging
  rule (FR-016).
- **FR-013**: Events replayed from a restored event history MUST NOT yield readings.

**Subagent threads**

- **FR-014**: Every message, block and partial delta the CLI marks with a parent tool call MUST be
  shown in that tool call's subagent thread and MUST NOT be shown in the main transcript.
- **FR-015**: A subagent thread MUST be folded by default under its parent tool-call card, show a
  one-line summary (state, number of tool calls, latest activity while running), and expand and
  fold on one press, in the same order and with the same cards the main transcript uses.
- **FR-016**: Threads MUST nest to any depth, each under the tool call that started it.
- **FR-017**: An event whose parent tool call is not held (evicted or never written) MUST be shown
  under a placeholder naming that parent, never dropped and never shown in the main transcript.
- **FR-018**: A subagent whose session's CLI exited or whose turn ended without a result while it
  ran MUST be shown as ended without a result.
- **FR-019**: Fold state MUST be kept per window and MUST NOT change what other windows show.

**Background tasks**

- **FR-020**: A structured session MUST list its background tasks, each with its description, kind
  (command or subagent), state (running, finished, failed, stopped, ended without a report) and,
  when reported, outcome summary, and MUST show the number still running; with no background tasks
  it MUST show neither.
- **FR-021**: The tool-call card that started a background task MUST show that task's current
  state.
- **FR-022**: When the session's CLI exits or restarts, background tasks still running MUST be shown
  as ended without a report.

**Common**

- **FR-023**: Everything this feature shows MUST be derived from the session's events, so every
  window viewing a session, and a window opening it later or after a daemon restart with its event
  history restored, shows the same figures, threads and tasks.
- **FR-024**: No event this feature reads MAY fail the session or the view: a missing or malformed
  figure is left out, as feature 938's tolerant parsing requires.
- **FR-025**: Terminal sessions MUST be unchanged by this feature.
- **FR-026**: Every cost this feature shows (the turn details of FR-006 and the session totals of
  FR-007) MUST be shown for every account and labelled as an estimate, and the cost details MUST
  say in words that it is Claude Code's estimate at API list prices and not what a subscription
  plan bills. Cost MUST NOT be hidden by plan type, and the application MUST NOT detect the plan
  type for this. The cost figure in #939's footer stays #939's; this feature adds the estimate
  note to the footer's details.
- **FR-027**: The user guide MUST describe the context meter, the token figures and totals, how
  structured sessions feed the plan-usage indicator, subagent threads and the background-task list,
  in the same change that ships each; feature 490's Settings note and guide page MUST say that
  structured sessions are a usage source too.

**Testing**

- **FR-028**: The figures, the plan-limit reading, subagent attribution and background-task states
  MUST be tested against recorded fixtures captured from a real CLI (feature 938's capture
  procedure), covering at least: a turn with a subagent, a subagent starting a subagent, a
  background command that finishes, and a plan-limit event; the plan-usage feed MUST be covered by
  a daemon test against the stand-in CLI.

### Key Entities

- **Turn usage**: the token figures, cost and per-model breakdown one turn result reports; absent
  figures stay absent.
- **Context reading**: context used, context window and compaction threshold as of the latest model
  response; derived per session.
- **Session totals**: the sums over the turns held (each turn's cost and per-model figures derived as
  FR-009 says), with the count of turns covered and whether the
  history was evicted.
- **Subagent thread**: the parent tool-call identity, the ordered events attributed to it, its state
  and its child threads.
- **Background task**: identity, description, kind, state, outcome summary, and the tool call that
  started it when known.
- **Usage reading** (feature 490): unchanged; structured sessions are a new producer of it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every recorded fixture, the context meter's figures equal those in the fixture's
  latest model response, and each turn footer's token figures equal its turn result's, in 100% of
  test runs.
- **SC-002**: Session totals equal the sum of the turns held, for sessions of 1 to 50 turns and
  across a restart and a resume, with no turn counted twice.
- **SC-003**: In every subagent fixture, 100% of the events marked with a parent tool call appear
  in that call's thread and 0 appear in the main transcript, at every nesting depth.
- **SC-004**: With **Show Claude plan usage** on, a plan-limit event yields a reading in every
  window within 10 seconds; with it off, none, in 100% of test runs.
- **SC-005**: A user can tell from a structured session's view, without opening any menu, how full
  the context is and whether it is near compaction, in under 5 seconds.
- **SC-006**: A fixture with malformed usage, plan-limit and background-task events produces zero
  session or view failures.
- **SC-007**: Every existing terminal-session and plan-usage test passes unchanged.

## Out of Scope

- The transcript view, tool-call cards, and the per-turn cost and duration footer (#939; extended
  here, not re-specified).
- Markdown and diffs (#940), input, interrupt and queued messages (#941), permission prompts (#942),
  slash commands, model switching and falling back to the terminal mode (#943).
- Stopping or controlling a background task or a subagent from the application (the user does it
  through the conversation; #941/#943 own input and commands).
- Budgets, spending limits or alerts on cost; usage reporting across sessions or over time.
- Context and usage figures for terminal sessions or for other AI CLIs.
- Changing feature 490's indicator, warning threshold or settings. The one change to feature 490
  in scope is the added usage source (FR-010) and its Settings note and guide text
  (FR-027).

## Assumptions

- **Dependency on #939**: this feature needs #939's transcript view, tool-call cards and turn
  footer, and ships after #939. Where #939 places the footer and cards, this feature adds to them.
  #939 leaves every event with a parent tool call out of the main transcript, counts it on that
  card (or on a single "sub-agent messages not shown" notice when the card is not held) and keeps
  it in the event history; this feature builds each subagent thread through #939's nesting seam,
  `Transcript::subagent_events(id)` and `Transcript::nested(id)`
  (specs/939-structured-transcript-view/contracts/transcript-model.md), so a thread uses the main
  transcript's rules and cards (FR-015), and replaces #939's notice with the placeholders of
  FR-017.
- The CLI's structured output reports, as in feature 938's recorded fixtures (Claude Code 2.1.302):
  per-response and per-turn token usage, a running session cost and a running per-model breakdown
  with each model's context window (FR-009); an automatic-compaction state with its threshold; a plan-limit event with each
  limit window's used share and reset time; a short "what is happening now" activity summary event;
  and, on every message and delta, a parent tool-call field, which is empty in all of those
  fixtures because none of them starts a subagent.
- **New recordings are needed** (FR-028): no existing fixture holds a subagent, a nested subagent
  or a background task. US3's and US5's independent tests and SC-003 run on those new recordings.
  The spec assumes, as the CLI's field implies, that it fills the parent tool-call field for every
  subagent event at every depth; if the recordings show it does not for nested subagents, the
  events it leaves unmarked are attributed to the nearest marked ancestor, and FR-016 is revised in
  the plan. The background-task states of FR-020 (stopped, ended without a report) and the kinds
  (command, subagent) are likewise settled by the recordings: whatever the CLI does not report is
  left out rather than guessed.
- The "latest activity" of a running subagent (US3 s1, FR-015) is the CLI's activity summary event
  where it reports one for the subagent, and otherwise the name of the subagent's latest tool call.
- A structured session has no status line, so it does not feed feature 490 through that path; the
  plan-limit event is the CLI's own output, read locally, which keeps feature 490's rules: no
  credential read, no request made, nothing leaves the machine.
- Plan-limit events carry no account identity, so feature 490's newest-wins rule applies unchanged.
- The context used is taken from the latest model response's own usage figures (the input it was
  given, including cached input, plus its output), which is how the CLI itself measures context.
- Event histories, their bound and their saving are feature 938's; this feature adds no storage of
  its own.
