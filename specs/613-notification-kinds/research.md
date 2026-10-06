# Research: Notification kinds, per-kind settings and icons

**Feature**: 613 | **Spec**: [spec.md](./spec.md) | **Builds on**: `specs/039-session-attention-notifications`
(research R3 claim/grant, R4 backends, R10 protocol versioning)

No NEEDS CLARIFICATION remained in the Technical Context. Each decision below names what was
chosen, why, and what was rejected.

## R1 — The service classifies, at the moment the session changes

**Decision.** The kind of an attention event is decided by the session service inside
`SharedState::note_activity` (`crates/micold-daemon/src/state.rs`), in the same step that 039 uses
to count the event (`began_waiting`), and the kind of an error ending in the paths that end a
session. Each live session carries a `TurnClock` (`micold_core::attention`, render-free) fed from
the activity events the service already receives.

**Rationale.** Only the service watches every session all the time, with or without a window
(039 FR-008), and only it sees the hook that caused the change (`Stop` or `Notification`); the
client only sees the resulting `ActivitySignal` in a snapshot, which is the same for both causes. A
client that lost its connection would mis-time a turn; the service did not lose it. Classifying in
one place gives every window the same kind (spec Edge Cases, "Several windows").

**Alternatives rejected.**
- *Classify in the client from snapshots*: cannot tell `Stop` from `Notification`, and a turn's
  duration is wrong across a reconnect.
- *Add the cause to `ActivitySignal` on the wire*: widens a shared wire enum every badge reads, for
  a fact only notifications need.

## R2 — Turn timing and the long-task threshold

**Decision.** `TurnClock` keeps `NotInTurn | Working { since } | Paused { since }` over a
monotonic `micold_core::clock::Uptime`. A prompt submitted starts a turn at that moment; any other
sign of work starts one when the session is not in a turn (the first moment the service knew it
was working) and resumes a paused one; a `Notification` while working pauses it and is **Needs
permission**; a `Stop` ends it and is **Long task finished** when `now − since ≥ threshold`, **Turn
finished** otherwise, waits included. A change into awaiting input with no turn known (`Unknown →
AwaitingInput`) is **Turn finished**. The threshold is
`pub const LONG_TASK_THRESHOLD: Duration = Duration::from_secs(60)` in `micold_core::attention`,
the only place the number appears; `TurnClock::change` takes the threshold as an argument, and the
service passes the constant.

**Rationale.** Matches the spec's Terms (turn, turn duration) and Edge Cases (unknown start,
exactly at the threshold counts as long). Taking the threshold as an argument means a later
adjustable threshold (D4 is provisional) is a new `Settings` field read where the constant is
passed today, with no change to the classification or its tests. A monotonic clock is immune to
wall-clock changes.

**Alternatives rejected.**
- *Measure from the last `Working` signal*: excludes the waiting time FR-003 requires.
- *Store the threshold in `Settings` now*: the clarified spec (D4) fixes it; a setting with no UI
  is dead configuration.
- *Wall-clock timestamps*: jump with NTP or a manual clock change.

## R3 — Which signal is "in the middle of a turn"

**Decision.** `HookKind::Notification` (Claude Code's `Notification` hook; Copilot's
`permission.requested`) received while the `TurnClock` is `Working` is **Needs permission**. The
payload's `notification_type` is not read. A `Notification` when the session already awaits input
(Claude's idle reminder after a turn) changes no signal, so 039 counts nothing (FR-003). A user
prompt submitted while paused (the user interrupted and typed anew) starts a new turn; a tool call
while paused resumes the old one.

**Rationale.** The turn state already says whether the session was mid-turn, on every CLI that has
a `Notification` mapping, without depending on a payload field one CLI added recently. Pi has no
such event: its sessions never raise **Needs permission**, which the spec accepts (Assumptions).

**`SubagentStop` is ignored (FR-024).** `hooks.rs::classify_hook` maps Claude's `"SubagentStop"`
to `HookKind::Stop` today (`hooks.rs:239`). A Task subagent finishing in the middle of a turn
therefore moves the session to `AwaitingInput` (`activity.rs` Stop arm) while the main agent works
on; the following `PostToolUse` is a no-op, so the session shows as waiting until the next
`PreToolUse`, and when the main agent ends without another tool call the real `Stop` changes nothing
and the turn's real end is never notified. Under this feature it would also end the turn clock early
and split one long task into short ones. The mapping was never a deliberate behaviour: 010 BUG-001
registered the hook only because the `"Stop" | "SubagentStop"` arm already existed
(010 `contracts/hooks.md` lines 98–100).

From this feature on `classify_hook("SubagentStop")` is `HookClass::Ignored` (answered 200, no
`ActivityEvent`), and `settings_json` stops registering it, so `claude` no longer sends it. The
arm stays explicit so a session still running under an older settings file is ignored too.

Consumers of the old mapping, and what each sees now:

| Consumer | Old (`SubagentStop` = `Stop`) | New (ignored) |
|---|---|---|
| Activity FSM (`activity.rs`, 010 H4) | `Working → AwaitingInput` mid-turn | unchanged state |
| 039 `began_waiting` → `mark_attention` (`attention_seq`, unread) | a mid-turn attention event, session unread | none (FR-024, FR-018's exception) |
| 039 desktop notification claim | a "waiting" notification mid-turn | none |
| Sidebar/status badge (activity signal) | "waiting" while the agent works | stays working |
| `TurnClock` (new) | would end the turn: `Finished` | no `TurnChange`; a `Paused` turn stays paused (the user is still needed) |
| `SubagentStop` after the main `Stop` (background subagent) | no change (already `AwaitingInput`) | no change |
| Tests | `hooks.rs::classifies_hook_event_names` (no `SubagentStop` case today); `settings_json_embeds_the_url_and_bearer_token` lists `"SubagentStop"` | assert `ev("SubagentStop") == Ignored`; drop it from the settings list and assert it is absent |
| Docs | 010 `contracts/hooks.md`: settings example (line 85) and the BUG-001 note (lines 98–100) | example without `SubagentStop`; note says it is ignored since 613 |

No other code names `SubagentStop` (`grep -rn SubagentStop crates` hits only `hooks.rs`).

**Alternatives rejected.**
- *Keep `SubagentStop` as `Stop` and ignore it only in the clock*: the signal would still change
  into awaiting input, which is an attention event that FR-001 must classify, with no kind that fits.
- *Map it to `PostToolUse`*: no FSM change, but the clock treats it as work, so a subagent finishing
  while the main turn waits on a permission would resume a `Paused` turn although the user is still
  needed.
- *Leave 010/039 behaviour untouched and accept the split*: keeps the spurious mid-turn waiting
  mark and unread, loses the real turn end's notification, and makes FR-002/FR-003 wrong for every
  Claude session that uses subagents.
- *A new `HookKind::SubagentStop` that the FSM and clock ignore*: the same behaviour as `Ignored`
  with a variant every match must handle; nothing consumes it.
- *Parse Claude's `notification_type`*: Claude-only, version-dependent, and adds nothing the turn
  state does not already say.

## R4 — Delivering a Session error: one notice to one window

**Decision.** On an error ending the service sends `DaemonMsg::SessionErrorNotice { project,
session }` to exactly one connected window, when desktop notifications are on, **Session error** is
on, and no window has the session in view. The window is `Views::error_notice_target`: the one that
last reported keyboard focus, else the window connection with the lowest id. Window connections are
those that have sent a `WindowView` (039 W1.1). With no window, nothing is sent and nothing is kept.

**Rationale.** FR-007 asks for one notification per ending, none on start or reconnect, none for an
ending while no window was open. A push at the moment of the ending satisfies all three by
construction. 039's claim/grant exists because unread marks need a persistent sequence and a
reconnecting client must judge what it missed; an error ending deliberately does neither (it does
not make the session unread, and a missed one is never notified), so a claim protocol would only
add a sequence, a claim and a grant to be told "no".

**Alternatives rejected.**
- *A second sequence (`error_seq`) with claim/grant*: persistent state and two messages to deliver
  what one message delivers; the reconnect rule would have to be "never claim", which is the push.
- *Broadcast to every window and let them race*: violates "one per ending across windows".

## R5 — Error endings

**Decision.** **Session error** is: the crash-loop give-up (`SupervisionAction::GiveUp` in the
supervision tick, where every abnormal exit the service cannot recover ends, a failed respawn
included, since that is counted as another crash), and a CLI-reported error ending
(`ActivityEvent::Ended { error: true }`, produced by `copilot_event` for `session.error`). A clean
exit (`SupervisionAction::Stop`), a Copilot `session.shutdown`, Pi's `session_shutdown`, and a
session the user stops or closes are not. A single abnormal exit that the service restarts raises
nothing (spec Edge Cases).

**Rationale.** These are exactly the spec's three error causes as the service observes them today.
`ActivityEvent::Ended` gains an `error: bool` so the Copilot mapping can say which it saw without
string-matching a reason.

**Alternatives rejected.**
- *Infer error from the `reason` text*: free text from another tool.
- *Notify on every abnormal exit*: the spec says repeated crashes the service recovers from raise
  nothing until it gives up.

## R6 — Where the switches live and how they apply

**Decision.** `Settings::notification_kinds: NotificationKinds` — four named booleans, each with
its own `#[serde(default = …)]` (three `true`, `turn_finished` `false`), the struct itself
`#[serde(default)]` — service-owned beside `desktop_notifications`, sent in `DaemonSettings` and set
through `SettingsSet { notification_kinds: Option<NotificationKinds> }`. The service applies them
where 039 applies the master switch: `Views::note_event` records an event as already granted when
its kind is off or the master is off, and `Views::grant` refuses a claim when either is off at claim
time. Turning the master or a kind on marks every current `attention_seq` as granted, as 039's
`set_desktop_notifications` does, so nothing that happened while off is notified afterwards.

**Rationale.** FR-010's three default cases fall out of serde: a fresh file and an unreadable file
use `Settings::default()`, a pre-feature file lacks the key. Service ownership makes every window
follow a change at once (039 W4.1), and live reads give FR-013 without a restart. Named fields, not
a map, make a kind without a switch unrepresentable (Principle V).

**Alternatives rejected.**
- *Client-side filtering after the grant*: the grant would be consumed for a window that shows
  nothing, and two windows with different stale settings could disagree.
- *`HashMap<NotificationKind, bool>`*: a missing key must be defaulted by hand; a misspelt key is
  silently kept.
- *Replace the master switch*: the spec keeps it (D3).

## R7 — Kind icons: one glyph, rasterised by the client for the OS

**Decision.** Four new `Icon` variants drawn from the bundled `MaterialSymbolsOutlined.ttf` (full
coverage): **Needs permission** `pan_tool` (a raised hand), **Session error** `error` (a circle with
`!`), **Long task finished** `task_alt` (a circled tick), **Turn finished** `chat_bubble` (a speech
bubble). Codepoints are pinned in `icons.rs` and checked against the font by `tests/icons_font.rs`.
For the OS, `micold_client::notification_icon` rasterises the same glyph with `ttf-parser`
(outline) and `tiny-skia` (fill) as a white glyph centred on a rounded square tile whose colour has
relative luminance in [0.10, 0.30] — contrast at least 3:1 against white and against black, and the
white glyph at least 3:1 against it. Each kind's tile colour comes from the design tokens. The shell
renders the four once per run: Linux passes the pixels in the `image-data` hint (no file);
Windows and macOS need a file, so the four are encoded as PNG (`tiny-skia`'s `png-format`) into
`<data dir>/notification-icons/<kind>.png` and passed to `Toast::icon(path, IconCrop::Square, …)`
(placement `appLogoOverride`) and `Notification::image_path` (an attachment).

**Rationale.** One source for the Settings glyph and the notification image, so they cannot drift
(FR-015). Rasterising from the font at run time keeps binary images out of the repository and lets
`tests/` check FR-017 directly on the pixels: tile contrast, and pairwise distinctness of the four
single-colour masks at 16×16. The tile is what gives 3:1 against both light and dark notification
backgrounds; a bare glyph in one colour cannot. `tiny-skia` and `ttf-parser` are already in the
build (iced, and a client dev-dependency); `png-format` adds only the `png` crate (image-rs,
MIT/Apache-2.0, actively maintained, the de facto Rust PNG encoder).

**Alternatives rejected.**
- *Commit pre-rendered PNGs with a Python generator* (as `assets/icon/generate.py` does for the app
  icon): a second copy of each glyph that can drift from `icons.rs`, and a test still needs to decode
  PNG to check contrast.
- *A hand-written PNG encoder*: bespoke code where a vetted crate exists.
- *Linux `app_icon` / `image-path` with a file*: `image-data` has the highest priority in the
  freedesktop spec and needs no file the notification server must be able to read.
- *macOS app icon only*: macOS always shows the bundle icon at the left and offers no per-
  notification replacement; the attachment is the only app-supplied image, and the title carries the
  kind wherever the attachment is not shown (FR-016).

## R8 — Notification text

**Decision.** `notification_text(kind, project, worktree, session)` returns the titles of spec US3
scenario 3: `<session> needs permission`, `<session> stopped with an error`, `<session> finished a
long task`, `<session> finished its turn`; the body stays `<project> — <worktree>` (039 FR-004). No
CLI text, no error reason.

**Rationale.** FR-008; the title alone tells the kind where no icon is shown (FR-016).

## R9 — Settings row: the shared Checkbox with an icon

**Decision.** `Checkbox` gains `.icon(Icon)`, a chainable builder method that draws the glyph in
the checkbox's label role between box and label. The kind rows are `Checkbox::new(kind.name(), on,
roles).icon(kind.icon())` wrapped in `field_note(…, Some(kind.description()))`, listed under
**Desktop notifications** in `NotificationKind::ALL` order and indented one step. While the master
switch is off a row gets no `on_toggle` (the Checkbox's existing disabled form) and keeps its
stored value.

**Rationale.** Principle VIII: extend the primitive the master switch already uses rather than add
a widget. Rows stay consistent with the other Environment settings.

**Alternatives rejected.**
- *A new `KindSwitch` widget*: a fork of `Checkbox` for one leading glyph.
- *`IconLabel` beside a bare checkbox*: two focus targets and a label that is not the checkbox's.

## R10 — Protocol version

**Decision.** One bump, 29 → 30, for the four wire changes (contract [wire.md](./contracts/wire.md)),
in the first milestone that changes the wire; a later milestone that changes it again takes the
next number (039 R10). No `#[serde(default)]` on wire types.

**Rationale.** 039 R10's rule: client and service of different versions refuse each other at the
handshake rather than misreading a message.
