# Research: Worktree State Icon

**Feature**: 905 | **Plan**: [plan.md](./plan.md) | **Spec**: [spec.md](./spec.md)

The spec has no open `NEEDS CLARIFICATION`. Each entry below is a design decision with the
alternatives rejected.

## R1 — Where the mid-turn / end-of-turn distinction comes from

- **Decision**: from the service's existing per-session turn clock. `TurnClock`
  (`crates/micold-core/src/attention.rs`) is in `TurnState::Paused` exactly while a session that
  was working stopped to ask the user (`TurnChange::AskedUser`, from the `Notification` hook) and
  has neither resumed (`Working`, `PromptSubmitted`) nor finished (`Finished`, from `Stop`). Add
  `pub fn paused_on_user(&self) -> bool` to `TurnClock`, and carry its value outward on the wire
  as `SessionSummary::paused_on_user` (R2).
- **Rationale**: this is the very state that makes feature 613 raise *Needs permission* instead of
  *Turn finished* (`TurnClock::change`, `(Working, AskedUser) → NeedsPermission`), so the icon and
  the notification cannot disagree about which stop is which (spec Clarifications Q1). Today it
  lives only in the daemon's `LiveSession.turn` (`crates/micold-daemon/src/state.rs`); the client
  sees `ActivitySignal::AwaitingInput` for both stops (`crates/micold-daemon/src/activity.rs`
  `apply`: `Stop | Notification → AwaitingInput`). The spec's Assumptions make carrying it part of
  this feature.
- **Alternatives rejected**:
  - *Infer it in the client from the hook order*: the client never sees hooks, only the derived
    signal; a guess would contradict the notification.
  - *Split `ActivitySignal::AwaitingInput` into two variants (or give it a cause field)*: every
    match on the signal changes (`ActivityBadge::emphasis`, `attention.rs`'s `awaiting`, the
    service's `began_waiting` comparison, `mcp/tools.rs`'s `"awaiting_input"`), and the
    attention-event rule in `note_activity` compares signals by `==`, so a permission followed by
    the `Stop` of the same turn would become a second attention event. The unread counting and
    notifications of features 039/575/613 would change behaviour, which the spec puts out of scope.
  - *Use the unread flag or the notification kind*: both exist only for stops while no window had
    the session in view (`began_waiting && !is_in_view`), so a session in view would never read
    Blocked.

## R2 — The wire change

- **Decision**: `SessionSummary` gains `#[serde(default)] pub paused_on_user: bool`, set by
  `overlay_live_summaries` from the live session's turn clock, `false` for a session with no live
  entry (`catalog.rs` `session_summary`). `PROTOCOL_VERSION` 47 → 48 with a doc line in
  `crates/micold-core/src/protocol/version.rs`; `SCHEMA_HASH` follows from `build.rs`.
- **Rationale**: the established shape for a runtime-only summary field (`activity`,
  `input_serial`, `live_shells` are overlaid the same way; `main` was added with `serde(default)`
  and a version bump in 43 → 44). A `bool` beside `activity` rather than a type that forbids
  `paused_on_user && activity != AwaitingInput`: the derivation (R4, rule 5) reads the flag only
  under `AwaitingInput`, so the meaningless combination has one defined answer and no caller has to
  handle it.
- **Alternatives rejected**: a new `DaemonMsg` per change (the client already reconciles whole
  catalog snapshots on every activity change; a second channel could reorder against it); storing
  it in the durable catalog (it is meaningless after a restart: the session is `resumable` then,
  which is Blocked on its own).

## R3 — Live updates when only the turn changes

- **Decision**: `State::note_activity` (`crates/micold-daemon/src/state.rs`) returns `true` — the
  caller then broadcasts the catalog — when the signal changed **or** `paused_on_user` changed.
- **Rationale**: a `Notification` (permission) followed by the turn's `Stop` leaves the signal at
  `AwaitingInput` (`changed == false` today), yet the row must go Blocked → Idle (FR-006). The turn
  clock already sees every event before the attention decision (feature 613, C2), so the flag is
  known at that point.
- **Alternatives rejected**: a periodic re-broadcast (latency and traffic, and FR-006's 1 s is a
  hard bound); broadcasting on every hook (needless traffic for `PostToolUse`).

## R4 — The derivation lives in the core, as one pure function per level

- **Decision**: new module `crates/micold-core/src/location_state.rs` with `LocationState`,
  `BlockedReason`, `session_state(&Session) -> LocationState` (the spec's ordered rules 1–7) and
  `location_state<'a>(impl IntoIterator<Item = &'a Session>) -> LocationState` (FR-004 precedence,
  archived skipped, none → Idle). The client's `WorktreeNode::state()` / `DefaultNode::state()`
  (`crates/micold-client/src/features/sidebar.rs`) call it over `sessions`, which already exclude
  archived ones.
- **Rationale**: Principle I's glue exception requires every decision in tested pure code; SC-004
  asks for an exhaustive pass over lifecycle × activity, which is a core unit test over a closed
  set. The spec's term is *worktree state*; the type is named for the *location* because the
  Default entry has one too (FR-002).
- **Alternatives rejected**: deriving in `ui/sidebar.rs` (untestable glue); in the client's
  `features/` only (the rule is about sessions, not about the sidebar, and the core already holds
  the neighbouring rules `occupancy::main_session` and `attention::counts_as_unread`).

## R5 — Which blocked reason a row names when several sessions are blocked

- **Decision**: `BlockedReason` is ordered `NeedsPermission` > `Failed` > `Resumable`, and the row
  takes the highest among its blocked sessions.
- **Rationale**: the spec fixes the state precedence (FR-004) but the tooltip names one reason
  (FR-009). A permission or question is answered in the session as it stands; a failed session
  needs a restart; a resumable one is waiting quietly since a service restart. The most immediate
  call on the user is named first. The icon is the same for all three, so this changes words only.
- **Alternatives rejected**: listing every reason with counts (a tooltip line per reason breaks
  FR-009's "a line of its own"); first session in list order (depends on the order sessions were
  created, not on what the user must do).

## R6 — Glyphs

- **Decision**: three new `Icon` variants in `crates/micold-client/src/icons.rs`, from the shipped
  Material Symbols Outlined font (no new icon source — spec Assumptions):

  | State | `Icon` | Material name | Codepoint | Shape | Role |
  |---|---|---|---|---|---|
  | Running | `StateRunning` | `play_arrow` | `U+E037` | right-pointing triangle | `primary` |
  | Blocked | `StateBlocked` | `report` | `U+F052` | octagon with `!` | `error` |
  | Idle / waiting | `StateIdle` | `pause` | `U+E034` | two vertical bars | `on_surface_variant` |

  All three codepoints are mapped by `assets/fonts/MaterialSymbolsOutlined.ttf` (checked against
  its `cmap` while planning); the implementing task confirms each name against the upstream
  `.codepoints` file named in `assets/fonts/PROVENANCE.md` and pins them in `tests/icons.rs`.
- **Rationale**: three outlines that differ in silhouette, not in fill — the shipped font is
  static at `FILL=0` (the comment on `Icon::ActivityWorking`), so a filled-vs-hollow pair would
  not survive greyscale reliably. None is used by another mark a row shows (FR-008): the PR
  indicator uses `call_split`, `edit_note`, `call_merge`, `block`, `check`, `schedule`, `cancel`;
  the session activity dot `radio_button_checked`/`unchecked`; the house `home`; the unread mark is
  a drawn filled circle (`unread_mark.rs`). `tests/icons.rs`
  `all_covers_every_variant_without_duplicates` already refuses a shared codepoint.
- **Alternatives rejected**: `pan_tool` for Blocked (feature 613's *Needs permission* glyph; Blocked
  also means failed and resumable, so the hand would say the wrong thing for two of three);
  `block` (the PR *closed* glyph, FR-008); `error` (the *Session error* and unavailable glyph);
  `progress_activity` / `sync` / `autorenew` for Running (a ring like the activity dot, and
  `Refresh`'s research ruled out arrows that promise something happens by itself);
  `hourglass_empty` for Idle (reads as "busy, wait"); `bedtime` (a crescent, too close to
  `DarkMode`'s moon).

## R7 — Where the icon sits in the row, and why a new slot

- **Decision**: a new optional slot on the shared `TreeItem`
  (`crates/micold-client/src/ui/material/tree_view.rs`), `.state_icon(StateIcon)`, drawn in a fixed
  `label_role.size()`-wide box right after the twisty slot and before the leading icon; its width
  enters the second line's `tag_indent` arithmetic as the leading icon's does.
- **Rationale**: a worktree row always has tags (the daemon chip is pushed unconditionally in
  `ui/sidebar.rs` `build_items`), and `TreeView` forbids a `badge` on a row with tags
  (`debug_assert!(!(item.badge.is_some() && has_tags))`) because a badge's width is not a token.
  Taking the component itself (not an `Element`) lets the host size it to the row's label role, so
  the width is a token and the tag line still aligns. Before the leading icon, so the state icons
  of worktree rows and of the Default row (which leads with its house) form one column.
- **Alternatives rejected**: `badge` (the assertion above, and the slot is the session dot's);
  the leading `icon` slot (tinted with the row's tint, and the Default row already uses it for the
  house, FR-002); the trailing cluster (it fades with hover and is shared with the PR indicator;
  the issue asks for the state "on the row without opening it", read down one column).

## R8 — Tooltip line order

- **Decision**: `with_state_line(tooltip, state)` in `features/sidebar.rs` appends the state words
  on a line of its own; the caller applies it before `with_unread_line`, so the unread line stays
  last as feature 575 made it.
- **Alternatives rejected**: first line (feature 029 made the name the first line, §2.3).

## R9 — No accessibility tree

iced 0.14 exposes none (feature 575 plan, *Known limitation*); the tooltip line carries the words
(FR-009), as for the unread indicator.
