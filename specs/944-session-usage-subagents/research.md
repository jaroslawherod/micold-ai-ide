# Research: Structured Session — Usage, Context Meter, Subagents and Background Tasks (#944)

Sources: the spec and its clarifications; #939's merged design and its core module on main
(`micold_core::structured::transcript`, milestones M1–M2); feature 490's `micold_core::plan_usage`;
the recorded fixtures in `crates/micold-core/tests/fixtures/stream-json/` (Claude Code 2.1.302);
the Agent SDK reference (code.claude.com/docs/en/agent-sdk/typescript, /headless,
/agent-sdk/cost-tracking); and one probe run of Claude Code **2.1.306** in the scratchpad (a
background `Bash` command plus a foreground `Agent` subagent, with
`CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`), whose stream is summarised in R6 and R7. The probe is not a
fixture: M1 records the fixtures with the capture script (R11).

## R1. Where the logic lives: one render-free core module, fed like the transcript

- **Decision**: a new render-free module `micold_core::structured::insights` holds every rule this
  feature adds: turn usage and session totals (R3), the context reading (R2), the task table
  behind background tasks and subagent state (R6), and the subagent thread index (R7). Its entry
  point, `SessionInsights`, takes the same `DaemonMsg::StructuredEvents` message the transcript
  takes (`apply(omitted, &[SequencedEvent], live)`) with the same replace-or-continue rule and the
  same `MAX_EVENTS` bound, so it can be built and tested on fixtures before any of #939's client
  code is on main. The replace-or-continue decision is extracted from `Transcript::apply`
  (`crates/micold-core/src/structured/transcript/mod.rs`) into a small shared
  `structured::sync::Continuity` that both use, rather than copied (the duplication budget).
  The client keeps one `SessionInsights` per structured session per window beside #939's
  `SessionTranscript` (R10).
- **Rationale**: Principle I (decisions in tested core), the unit prompt (early milestones land
  before #939 finishes), and FR-023 (everything derived from events: every window that applies the
  same messages holds the same state).
- **Alternatives**: put the rules into #939's `Transcript` (new fields on `TurnFooter`, task state
  on `ToolCard`) — rejected: it couples #944's milestones to #939's in-flight ones and grows a
  module #940–#943 also extend. Compute from `Transcript`'s held events on every draw — rejected:
  O(5,000) per frame, and the events are private to `Transcript` (only `subagent_events` is
  public). A daemon-side aggregate sent on the wire — rejected: a protocol change, and FR-023 is
  already met client-side.

## R2. Context reading: latest main response's usage; window from the result; threshold from `autocompact_state`

- **Decision**:
  - **Context used** = `input_tokens + cache_creation_input_tokens + cache_read_input_tokens +
    output_tokens` of `message.usage` in the latest **main-scope** `Assistant` event (no
    `parent_tool_use_id`), read from its `raw` (the typed `Assistant` variant has no usage). The
    CLI writes one `assistant` line per content block, each with the message's usage, so a repeat
    changes nothing. `message.model` names the model.
  - **Context window** = `modelUsage[<that model>].contextWindow` of the latest held `Result`
    listing that model. Until one is held, the window is unknown and the meter shows tokens only
    (FR-004, US1 s6).
  - **Compaction threshold** = `value.threshold` of the latest `autocompact_state` line when
    `value.enabled` is `true`; none when disabled, absent or malformed.
  - **Warning** (FR-003): used ≥ threshold; with no threshold, used × 10 ≥ window × 9.
  - Subagent responses never move the meter (their context is the subagent's, not the session's).
  - Compaction (`system`/`compact_boundary`) needs no rule: the next response reports the smaller
    size and the meter follows the latest response (spec Edge Cases).
- **Evidence**: `tool-turn.jsonl` — assistant usage `2 + 8515 + 28624 + 16`; result
  `modelUsage["claude-sonnet-5-5"].contextWindow = 1000000`; `autocompact_state.value =
  {enabled: true, effective_window: 480000, threshold: 384000}`.
- **Alternatives**: the window from `autocompact_state.effective_window` (480,000) — rejected: the
  spec defines the window as the most the model accepts; the effective window is the CLI's own cap,
  and its threshold is what the meter marks. The `message_start` stream event's usage for an earlier
  reading — rejected: its `output_tokens` is a placeholder until the message ends, and the finished
  `assistant` line follows within the same message (FR-005's one second). A fixed per-model window
  table — rejected: FR-004 forbids figures the CLI did not report.

## R3. Turn usage and session totals: per-turn `usage`, cumulative cost and `modelUsage` (FR-006–FR-009)

- **Decision**: for each held `Result` (main scope) at `seq`:
  - **Tokens**: the typed `usage` (`input_tokens`, `output_tokens`, `cache_read_input_tokens`,
    `cache_creation_input_tokens`) as reported — per turn, main conversation only (D5). All four
    absent → the turn has no token figures.
  - **Cumulative figures**: `total_cost_usd` and `modelUsage` (per model: input, output, cache read,
    cache creation tokens, `costUSD`), read tolerantly from `raw`.
  - **Turn cost and per-model figures** (FR-009): the cumulative figures minus those of the
    previous held `Result`. When none is held and nothing was omitted (`omitted == 0`), the
    figures as they are. When none is held because earlier events were omitted or evicted, absent.
    When any cumulative figure is lower than the previous one (a fresh CLI count, e.g. a restart
    without resume), the figures as they are (a *restart*).
  - **Session totals** (FR-007, FR-008): the sum over held turns that are **counted** — a `Result`
    with token figures and a derivable turn cost; every other turn (no usage, cost not derivable,
    or `TurnEndedWithoutResult`) is **left out** and counted in `left_out`. `partial` is true when
    `omitted > 0`. Per-model totals are the sums of the counted turns' per-model figures.
  - The turn's own cost is drawn in #939's footer beside #939's "session total" (D7).
- **Evidence**: `text-turn.jsonl` result `0.0371` USD, sonnet output 14; `resume-init.jsonl` (a
  `--resume` of it) `0.0954` USD, sonnet output 20 while its `usage.output_tokens` is 6 — the
  cumulative/turn split of D5; cost-tracking docs, *Track costs in streaming input mode*.
- **Known limit**: a resumed session whose earlier results are not held (history off and a
  daemon restart, or a fresh window on a service with no saved history) shows its first result's
  figures as they are, which include the resumed totals; FR-009 prescribes exactly that ("as they
  are when none is held"), and the turn is not double-counted because nothing before it is held.
- **Alternatives**: per-turn cost from `modelUsage` sums — rejected: also cumulative. Totals as
  the latest cumulative figure — rejected: wrong after a restart, and FR-008 needs the counted/
  left-out split.

## R4. The plan-limit event as a feature 490 source (FR-010–FR-013, D4)

- **Decision**: a new core parser `micold_core::plan_usage::rate_limit_event::parse(raw: &str,
  obtained_at) -> Option<UsageReading>` reads `rate_limit_info.unifiedWindows`: each entry whose
  value is an object with a finite, non-negative numeric `utilization` (a share, `0.01` = 1%) and
  an optional `resetsAt` becomes a `LimitWindow { key, used_tenths: round(utilization × 1000),
  resets_at }`; no readable window → `None`. The per-window rules (non-negative, finite, reset
  after `obtained_at` and in range) are the ones `status_line::window` applies today; they move
  into one shared `plan_usage::limit_window(key, used_percent, resets_at, obtained_at)` that both
  parsers call.
- **Daemon**: the reader of a structured session's stdout is the only place a reading is taken
  (`StructuredSession::spawn`'s `on_event`, called for lines the running CLI writes). A restored
  history is built by `EventStream::restored` and never passes through `on_event`, so FR-013 holds
  by construction. For each `Unrecognised` event whose `raw` has `"type":"rate_limit_event"`, the
  closure parses it on the reader thread and hands `(session, reading)` to the service's plan-usage
  feed: a `tokio::sync::mpsc::UnboundedSender` held by `DaemonState`, drained by a task spawned at
  service start with an `Arc<DaemonState>`, which calls a new `DaemonState::offer_plan_usage(session,
  reading)`. `offer_plan_usage` is extracted from `hooks.rs`'s status handler (switch check,
  `note_plan_usage`, `PlanUsageChanged` broadcast, the once-per-answer debug log of FR-016) so both
  sources share it.
- **Rationale**: feature 490's R1 made the reading source-neutral; newest-wins and repeat
  suppression (`note_plan_usage`) apply unchanged (US4 s2). The event stays in the history as
  before (US4 s3).
- **Alternatives**: a typed `StructuredEvent::RateLimit` variant — rejected: changes #938's event
  model and the wire schema for one consumer. Reading the event in the client — rejected: feature
  490's reading lives in the daemon and every window gets it from there. Capturing `Arc<DaemonState>`
  in the `on_event` closure — acceptable if `spawn_structured`'s callers already hold one; the
  channel is the default because `spawn_structured(&self, …)` has only `&self` today.

## R5. Subagent text needs `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`

- **Decision**: the Claude provider's structured launch sets the environment variable
  `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1` (a new `AiCliProvider::structured_env()`, empty by default,
  applied by `StructuredSession::spawn` after `apply_env_hygiene` and before `spec.env`). The seven
  structured arguments of #938's FR-005 stay as they are; the capture script sets the same
  variable so fixtures match what the service runs.
- **Evidence**: /headless *Follow subagent messages*: by default a foreground subagent's stream
  carries only its first `user` message and its `tool_use`/`tool_result` blocks; with
  `--forward-subagent-text` or `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT` "the run's text and thinking
  blocks too", at every nesting depth (v2.1.219+). The 2.1.306 probe with the variable carried the
  subagent's `thinking` and `text` messages with `parent_tool_use_id` set.
- **Alternatives**: the `--forward-subagent-text` flag — rejected: an older CLI refuses an unknown
  flag and the session would not start, while an unknown variable is ignored (FR-024's spirit:
  degrade, never fail). Without either — rejected: US3 s2 shows the subagent's text and thinking.

## R6. One task table for background tasks and subagent state (FR-018, FR-020–FR-022)

- **Decision**: `insights::tasks` keeps a table keyed by `task_id`, fed by the CLI's `system` lines
  (read tolerantly from `Unrecognised.raw`):
  - `task_started` → entry `{ task_id, tool_use_id?, description, kind, background:
    is_backgrounded == true, state: Running }`. `task_type` `local_bash` → `Command`,
    `local_agent` → `Subagent`, anything else → kind absent (shown without a kind; FR-020 leaves
    out what the CLI does not report).
  - `task_updated` → `patch.status` `completed`/`failed`/`killed` → `Finished`/`Failed`/`Stopped`;
    `patch.is_backgrounded: true` → `background = true`; `patch.description` replaces it.
  - `task_notification` → `status` `completed`/`failed`/`stopped` → state, `summary` → outcome
    summary, `tool_use_id` fills a missing one. A notification for an unknown `task_id` creates the
    entry from the notification (spec Edge Cases), with `background = true`.
  - `task_progress` → `latest activity` of the task: its `summary` when present, else
    `description`, else `last_tool_name` (spec Assumptions).
  - `background_tasks_changed` → its `tasks` array is the CLI's live set: an entry listed there and
    unknown is created (`background = true`); a held `Running` background entry absent from it is
    `EndedWithoutReport`.
  - `Init` (the CLI process started or restarted) → every `Running` entry becomes
    `EndedWithoutReport` (FR-022; only an `init` with `startup_timing`: the CLI writes an `init`
    at the start of every turn, and 2.1.306 adds `startup_timing` only to the process's first, as
    `background-command`'s two inits show (ledger D16); /typescript *SDKBackgroundTasksChangedMessage*: "reset … whenever
    the session's CLI process starts or restarts"). `TurnEndedWithoutResult` → every `Running`
    foreground entry becomes `EndedWithoutReport` (FR-018).
  - While the session's CLI is not running (the client's catalog lifecycle), the view shows every
    `Running` entry as `EndedWithoutReport` — a view-time overlay, not a change to the table, so a
    window applying the same events still holds the same table (FR-023).
  - **Background tasks** (FR-020) are the entries with `background = true`; the running count is
    those still `Running`. The tool-call card of `tool_use_id` shows the entry's state (FR-021).
  - **Subagent state** (US3 s1, FR-018) is the state of the `Subagent` entry whose `tool_use_id`
    is the thread's parent; when there is none (an older CLI), the parent card's `ToolStatus`
    (`Running` → running, `Done` → finished, `Failed` → failed, `NoResult` → ended without a result).
- **Evidence** (2.1.306 probe): the background `Bash` call wrote `background_tasks_changed` (one
  task), `task_started {task_type: local_bash, is_backgrounded: true, tool_use_id}`, later
  `task_updated {patch: {status: completed}}`, `task_notification {status: completed, summary,
  tool_use_id}` and an empty `background_tasks_changed`; the foreground `Agent` call wrote
  `task_started {task_type: local_agent, is_backgrounded: false, spawn_depth: 1, tool_use_id}`,
  two `task_progress` (`description`, `last_tool_name`), then `task_updated` and
  `task_notification`. The tool result of a background command is a placeholder ("Command running
  in background with ID: …"), so the card alone cannot tell the task's state.
- **Alternatives**: pair `task_started` with `task_notification` only — rejected: the docs say to
  trust `background_tasks_changed`, and an event missed while evicted would leave a task running
  forever. Match by `run_id` — rejected for this release: `task_id` is stable across runs and the
  view shows a task, not its runs; a resumed task's second `task_started` reopens the same entry.

## R7. Subagent threads: an attribution index in core, the content through #939's seam

- **Decision**:
  - `insights::subagents` keeps, per parent tool use id `P` that any held `Assistant`/`User` event
    names: the count of its messages, the ids and names of the tool calls in its `Assistant`
    events (`ToolUse` blocks), its first `seq`, and its latest tool name. It also keeps a reverse
    map *tool use id → the thread whose message contains it*, so a nested thread finds its owner.
  - **Placement** (`owner(P)`): `Main` when the main transcript holds the card
    (`Transcript::entry(&EntryKey::Tool(P))` is some); `Thread(Q)` when thread `Q` contains tool
    use `P`; otherwise `Unknown` — shown under a placeholder naming `P` in place of #939's single
    `SubagentNotice` (FR-017). Depth is unbounded: each thread is placed by the same rule (FR-016).
    The ownership test takes the main transcript as an argument, so the core rule needs no client.
  - **Summary line** (FR-015): state (R6), tool-call count, latest activity while running (the
    task's activity, else the thread's latest tool name).
  - **Content** (FR-015, US3 s2): the thread's entries are those of `Transcript::nested(P)` built
    from `Transcript::subagent_events(P)` of the main transcript — #939's seam, the same rules and
    cards (research R7 of #939). The client builds a nested transcript **only while the thread is
    open** in that window (folded threads cost nothing, spec Edge Cases), appends each later live
    event naming `P` to it, and rebuilds it from `subagent_events(P)` once per applied message when
    the main transcript rebuilt or evicted any event (its `rebuilds()` or `omitted_notice()`
    changed). A nested thread's own nested threads open the same way, keyed by their own parent id.
  - Subagent `StreamDelta`s: none in the probe (the CLI does not stream subagent partials); if a
    later CLI writes them, #939's nested scope already streams them.
- **Evidence**: probe — every subagent `user`/`assistant` line carried `parent_tool_use_id` = the
  `Agent` call's id and `agent_id` = the task's `task_id`; the first was the subagent's prompt as a
  `user` text message. /headless: nested subagents' messages "carry the ID of the Agent or Skill
  tool call that started it". The probe could not show nesting (the subagent had no Agent tool);
  M1 records a nested fixture (R11). If it contradicts this, the spec's fallback (attribute to the
  nearest marked ancestor) is planned in M3's tasks.
- **Alternatives**: one nested `Transcript` per thread kept always — rejected: every subagent event
  would be applied twice per window even when nobody looks. Group by `agent_id` — rejected:
  `parent_tool_use_id` is what #939's seam scopes by and is present on every CLI version this
  feature supports; `agent_id` (SDK v0.3.292+) only matters for resumed background subagents,
  whose later messages keep the first call's parent id and so still land in the first thread.

## R8. Incremental, yet equal to a fresh build (FR-023)

- **Decision**: `SessionInsights` keeps a **digest log**: for each held event that matters to it
  (main `Assistant` usage, `Result`, `TurnEndedWithoutResult`, `Init`, `autocompact_state`,
  `task_*` and `background_tasks_changed` lines, and one compact record per parented message), a
  small digest with its `seq` — not the event. Pushing an event updates the derived state
  incrementally; when the bound evicts a digest, the derived state is refolded from the digests
  still held (O(digests held), at most once per applied message). Guarantee **I1**: after any
  sequence of messages, `SessionInsights` equals one built by a single `live: false` message of
  the held events with the same `omitted`.
- **Rationale**: FR-009's rule depends on which results are held, so eviction changes earlier
  turns' figures; refolding a few hundred digests is cheap, while parsing JSON again is not.
- **Alternatives**: hold every event (as `Transcript` does) — rejected: a second copy of up to
  5,000 events of several MiB per session per window. Ignore eviction — rejected: a window opened
  later would disagree (FR-023).

## R9. Formatting

- **Decision**: token counts grouped by thousands with the application's one count format —
  `thousands` in `features/worktree_form.rs` moves to a core `micold_core::format::group_digits`
  and both use it (`37,141`). Cost uses #939's `format_cost` (four decimals below $1, two from $1
  on), with the estimate label of FR-026. Percent is a whole number rounded half up, as feature
  490's `percent_label`. Reset times are feature 490's (`format_reset`), untouched.
- **Spec amendment**: the spec's cross-platform edge case said "the user's locale digit grouping".
  The application has no locale layer (no crate reads the locale's number format; `micold-core`
  formats every count with `,`), and adding one (CLDR data through `icu`/`num-format`) for one
  figure would make it the only localised number in the app. The bullet now reads "grouped by
  thousands as the application formats every count" (D9). Behaviour is identical on all three
  platforms (Principle VI).

## R10. Client: a render-free feature module beside #939's

- **Decision**: `crates/micold-client/src/features/structured_insights.rs` (render-free): per
  window, `HashMap<SessionId, SessionExtras>` with the session's `SessionInsights`, its open
  threads (`HashMap<String, Transcript>` — nested transcripts, only while open) and their fold set,
  and whether the background-task list is open. `apply` is called from the same place #939 routes
  `StructuredEvents` to `structured_transcript` (`shell/daemon_sync.rs`, #939 T022), after the
  transcript, because thread rebuilds read `subagent_events` from it. Messages: `InsightsMsg {
  ToggleThread { session, parent }, ToggleBackgroundTasks { session } }`. `forget(session)` follows
  #939's forget paths.
- **Rationale**: Principle I (decisions in render-free code), FR-019 (fold state per window, never
  on the wire).
- **Alternatives**: fields inside #939's `SessionTranscript` — rejected: ties this module to #939's
  M3 internals and to #940–#943, which also extend that struct.

## R11. Recorded fixtures (FR-028)

- **Decision**: M1 records, with `scripts/capture-stream-json.sh` (which gains
  `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`, R5), under Claude Code 2.1.306 or later:
  `subagent-turn.jsonl` (a foreground `Agent` call that reads a file), `nested-subagent.jsonl` (a
  subagent allowed the `Agent` tool through `--agents`, which starts its own),
  `background-command.jsonl` (a `run_in_background` `Bash` command that finishes, with its
  notification), and `background-subagent.jsonl` (an `Agent` call with `run_in_background`). The
  plan-limit event is already in every recorded fixture. A synthetic `insights-tolerance.jsonl`
  holds malformed usage, `modelUsage`, `autocompact_state`, `rate_limit_event` and task lines
  (SC-006). The README's table gains the new rows. Recording needs a logged-in `claude`; the dev
  container has one (the probe ran there).
- **Alternatives**: synthetic subagent fixtures — rejected by FR-028.

## R12. UI composition (Principle VIII)

- **Decision**:
  - **Context meter**: the shared `material::ProgressBar` gains two builder steps, `.marker(Ratio)`
    (a 2 px tick across the track at the threshold) and `.warning(bool)` (fill in the `error`
    role); the warning look also shows `Icon::Warning` and the text "Near compaction" beside the
    bar (FR-003: not colour alone). With the window unknown, no bar: the token figure only.
  - **Session bar**: one row at the top of the transcript pane (`ui/transcript/session_bar.rs`):
    meter + "37,141 / 1,000,000 · 4%", session totals ("$0.14 est. · 3 turns"), and while any
    exist, "2 background tasks running" — composed from `ProgressBar`, `IconLabel`, `Text` and the
    shared `Tooltip` for details (cached tokens, per-model rows, "estimated at API list prices; a
    subscription plan does not bill this amount", "covers the turns still held" when partial).
  - **Background-task list**: an `AccordionSection` under the bar (folded by default), one row per
    task: kind icon, description, state label + icon, outcome summary.
  - **Footer**: #939's footer row gains the turn's cost ("$0.0052 est.") and tokens ("in 4 · out
    106"), with cached input and per-model rows in its details tooltip.
  - **Tool-call card**: a card with a thread shows, inside its body, a nested `DisclosureCard`
    (#939 M4) headed "Subagent · running · 3 tool calls · Reading notes.txt", whose body draws the
    nested transcript's entries through #939's per-entry draw functions and `BlockRenderers`,
    indented by the shared spacing token. A card that started a background task shows the task's
    state as a trailing `IconLabel` (FR-021).
  - **Placeholders** (FR-017): where #939 draws its `SubagentNotice`, one `DisclosureCard` per
    `Unknown` thread, headed "Subagent of an earlier step (toolu_…)".
  - Icons: the warning look reuses feature 490's `Icon::UsageWarning` (`\u{e002}`); new
    `Icon::Subagent` and `Icon::BackgroundTask` join `crates/micold-client/src/icons.rs` and its
    codepoint test.
- **Alternatives**: a new `ContextMeter` widget — rejected: `ProgressBar` already paints the
  track and fill this needs; extending it keeps one bar family. A modal for usage details —
  rejected: SC-005 asks for the figure without opening anything; details are a tooltip like
  feature 490's indicator.
