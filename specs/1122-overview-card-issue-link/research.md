# Research: Overview cards, a link to the worktree's issue

Each entry: the decision, why, and what was rejected. Paths are workspace-relative; "new" marks
what this feature adds.

## R1 — Where the ticket and the link's rules live

**Decision**: a new render-free module `crates/micold-core/src/issue_link.rs` (`pub mod issue_link`
in `lib.rs`) holds `IssueRef` (`Github(u64)` / `Jira(String)`, the key upper case), `issue_ref(dir)`
built on the existing `micold_core::naming::ticket` (feature 813, T054), `IssueSources` (the inputs
FR-001 names) and `issue_link(dir, &IssueSources) -> Option<IssueLink>` giving label and address
(contract K1–K3). The Jira key form (a letter, then letters, digits or `_`, `-`, digits) is checked
here; `naming::ticket` is not changed (spec *Out of scope*).

**Why**: FR-016 puts every rule in render-free code; the card model is already built from core
types (`CardModel`, `CardText`), so the link is one more derived field.

**Rejected**: reading the ticket in `crates/micold-client/src/features/overview.rs` beside
`numeric_ticket` (client code, and it would add a second Jira-key reader next to the one GitHub
reader); parsing the branch name (spec Assumptions: the directory name is the source).

## R2 — The project's GitHub repository on the Overview

**Decision**: `features::overview::State` gains `repo: RepoLookup` (`Unknown` / `Asked` /
`Known(Option<GithubRepo>)`), cleared on `Msg::ProjectSwitched`. `shell/issues.rs::on_overview_opened`
sends the existing `PendingOp::TitleRemotes` (`ClientMsg::RemoteList`) whenever the repository is
`Unknown`, not only when titles are wanted, and before the GitHub issues switch check; the answer
(`on_title_remotes`) first records `Msg::RepoRead { project, repo }` from
`micold_core::github::choose_remote`, then goes on with the titles exactly as today (switch-gated).
A window with no route to a daemon reads nothing and shows no GitHub link.

A project switch with the Overview open (user guide *Overview*) clears `repo` on
`ProjectSwitched` while the Overview stays open, so `on_overview_opened` (the closed → open edge)
never runs again. `shell/daemon_sync.rs`'s `DaemonMsg::Attached` arm therefore also calls
`shell::issues::read_repo(app)` (new; `on_overview_opened` calls it too) for the active project
while the Overview is open and `repo` is `Unknown`: the same `PendingOp::TitleRemotes` with no
title numbers, so its answer records `RepoRead` and fetches nothing. `Attached` is the trigger, not
the switch itself: the new project's daemon answers `RemoteList` only once attached, and a
reconnect after a lost route re-reads the same way. Titles keep today's behaviour.

**Why**: the link needs the repository synchronously at render; the client holds it nowhere today
outside the New-worktree form. Reading the remotes regardless of the switch makes FR-012 hold for
the GitHub switch (turning it on shows the link at once, no second read). `RemoteList` is a git
read in the daemon, not a GitHub request: feature 1102 FR-009 already allows it while the switch
is off, and it is what the title reads use today (FR-014: no request to GitHub or Jira).

**Rejected**: taking it from `worktree_form` (`github_repo()` exists only while the dialog is open);
a new daemon op (the existing one answers); re-reading on each render (I/O in `view`); re-running
all of `on_overview_opened` on `Attached` (its title numbers come from the worktree listing, which
still holds the old project's worktrees until the listing after `Attached` arrives).

## R3 — The Jira base URL: type, validation, storage

**Decision**: `micold_core::jira::JiraBaseUrl` (new): `parse(&str) -> Result<Self, FieldProblem>`
accepts `https://host[:port][/path]`, refuses another scheme, no host, user info, a query or a
fragment, and stores it with every trailing `/` removed (contract K4). The authority and host
checks are extracted from `SiteOrigin::parse` into one private helper both call (no copied block,
`mise run duplication`). `JiraField::BaseUrl` (new) and `BASE_URL_REFUSED` carry the reason.
Stored per project (D4, owner's answer), by the route feature 533 already uses for each project's
Jira connection: `Settings.jira_base_urls: BTreeMap<String, JiraBaseUrl>` keyed by
`jira::connection_key(project)`, read leniently entry by entry (an entry that no longer parses is
dropped and reads as unset, never failing the document; a missing map reads as empty, FR-010);
`DaemonSettings.jira_base_urls` (the same map, mirrored to every window); and
`ClientMsg::SettingsSet.jira_base_url: Option<JiraBaseUrlChange>`, one project's change
(`JiraBaseUrlChange { project: String, url: Option<JiraBaseUrl> }`, `url: None` clears, outer `None`
leaves every project unchanged) with `apply(&mut map)` as `JiraConnectionChange` has. The service
merges the one change into the map on disk inside its write (`Catalog::persist_service_settings_with`,
as for `jira_connection`), so two windows saving different projects never overwrite each other.
`PROTOCOL_VERSION` 57 → 58 (contract [wire.md](contracts/wire.md)).

**Why**: FR-011's rules differ from `SiteOrigin` (a path is kept, a debug-build loopback `http`
is not needed for a browse link). The owner chose a value per project, for projects on different
Jira sites. The per-project Jira connection map is the existing per-project, service-owned route:
every window follows it without a restart (FR-012, feature 533 FR-022), its per-project merge keeps
concurrent saves of two projects apart, and its key, field placement and `follow` re-seeding on a
project switch are reused rather than built again. `JiraBaseUrl` (M1) is unchanged: only where it is
stored changes.

**Rejected**: one app-wide value (the default before D4; the owner chose per project); a field of
`JiraConnection` (a browse link would then need a site, key, email and API token, and the
connection's required fields would gate a link that makes no request); the project's own state file
via a new project message, like `ProjectRename` or `SetPaneLayout` (a second per-project Jira
store beside the connections, and a dialog outside Settings for one field); the connection's site
as the base (a separate value was chosen, and the site drops a path); a client-only setting (other
windows would not follow, FR-012); a plain `Option<String>` with `""` meaning clear (an unvalidated
value could reach the wire).

## R4 — Settings page

**Decision**: `features::settings::JiraDraft` gains `base_url: String`, seeded by
`JiraDraft::follow` from the open project's entry of `jira_base_urls` under the same rule as the
connection's fields (re-seeded on a project switch, or while untouched when the reported map
changes). It is validated on save through `JiraBaseUrl::parse` (the refusal marks
`FieldId::SettingsJiraBaseUrl`, new, and nothing is sent); a value equal to the stored one sends
nothing, a changed one sends `Some(JiraBaseUrlChange { project, url })`, an empty field `url: None`.
It is optional and outside `required_fields(SettingsSection::Jira)` and `JiraDraft::validated`, so
a base URL saves with the connection's fields blank, and the connection's validation never blocks
it. `ui/settings/jira.rs` shows a `TextField` labelled **Jira base URL** with supporting text ("Used
to link Jira keys on this project's Overview cards."), after the connection's fields, under the
same condition (`draft.jira.enabled && draft.jira.project.is_some()`, FR-010). `SETTINGS` gains the
entry `tests/settings_sections.rs` checks. The Keep-or-Remove question (switch off) leaves it
untouched (spec edge case).

**Why**: the open project's Jira settings already live in this section (feature 533's connection
form), with the same field primitives (Principle VIII) and the same draft that follows the open
project; a separate per-project dialog would be a second place for a project's Jira settings.

**Rejected**: making it a fifth field of the connection (its required marks and validation would
gate it); a dialog from the project list's row, beside Rename (a second place for a project's Jira
settings, and out of reach while a project is open).

## R5 — The link widget: a shared `TextLink`

**Decision**: a new shared component `crates/micold-client/src/ui/material/text_link.rs`:
`TextLink::new(label, on_press, roles)` with `.role(TypeRole)`, `.id(Id)`, `.on_keyboard_focus(f)`,
`.on_menu_key(m)`, `.on_page(f)`, `.focus_on_press(bool)`, `.focusable(bool)` (a link that is not a
Tab stop, used by the card until its link focus is wired); `impl From<TextLink> for Element`.
It draws its label in `roles.primary`, a 1 px underline in the same colour while the pointer is on
it, the pointing-hand pointer, and the shared focus ring through `TakesTheKeyboard`'s `Indicator`
(contract U1). A click is press and release both on it; Enter and Space activate it while it has
the keyboard. `TakesTheKeyboard` gains `.press_takes_focus(bool)` (default `true`, unchanged for
every other control) so the link can stay out of the keyboard on a click (FR-005). It is added to
the showcase catalogue (`showcase_completeness`, `material_builder_api`).

**Why**: Principle VIII: no link primitive exists (`terminal_pane.rs` underlines a cell link
inside its own canvas); a link is a reusable control. `primary` is the role the theme's
contrast rules already allow on neutral surfaces (`composition_contrast.rs`).

**Rejected**: an iced `button` styled as text (no underline-on-hover, and its press would take the
keyboard); `rich_text` spans with a link (no focus stop, no key activation).

## R6 — Keeping a press on the link off the card: `ControlOverlay`

**Decision**: the card's control chain (ripple, keyboard focus, the transparent button) moves from
around the face to a layer above it, inside a new cdk wrapper
`crates/micold-client/src/ui/cdk/control_overlay.rs`: `ControlOverlay::new(content, control)`.
Layout: the content sizes it, the control fills it (as iced's `Stack` with a base layer). Events go
to the content first; if the content captured the event, the control never sees it; otherwise the
control sees it with the cursor levitated wherever the content answers a non-`None`
`mouse_interaction` (the link). Draw: content, then control (ripple and focus layers over the
face, as today), the control drawn with the same levitated cursor, so it shows no hover layer over
the link. The expanded card's inner scrollbar is not such a region: iced 0.14.2's `Scrollable`
answers `None` over its scrollbar (and while it is grabbed), so the control shows its hover layer
and pointer cursor there, as the card's button does today (its `mouse_interaction` never asks its
content); a press on the scrollbar is still captured by the scrollable first, as today. Focus traversal: control, then content, so Tab
reaches the card, then its link (contract U2). The `ContextArea` stays outermost, so a secondary
click on the link opens the card menu (FR-006).

What follows: a press on the link is captured by `TextLink` → no ripple, no card focus, no
activation (FR-005); a press elsewhere and release on the link → the button sees a levitated cursor
on release → no activation, the ripple fades (FR-005/FR-006); Enter on the focused link is captured
in the content → the card's ripple `centred_on_keys` never runs.

**Why**: the three wrappers today act on any press in the card's bounds, the ripple even before its
child (`ripple.rs` `update`), and the button captures the press too, so no wrapper can tell a link
press from a card press by capture alone. Ordering content first and levitating the control is the
one place that decides it, generic (any interactive child of a card-like control), and needs no
change to `Ripple` or `button`.

**Rejected**: iced `Stack` (it updates the top layer first, so the control would take the press
before the link); teaching `Ripple`, `TakesTheKeyboard` and `button` a "hole" each (three changes to
shared primitives for one region); a twin layer laying the link out over the card (doubles every
card's text layout, and the expanded card's inner scroll would need syncing).

## R7 — Clicking a link leaves keyboard focus where it was

**Decision**: `CardGrid` sends a primary press over a card's link to that card only; the other cards
do not see it, so a focused card or link elsewhere in the grid keeps the keyboard (FR-005, Edge Case
*Window loses focus*). The grid finds each card's link bounds with an `Operation` over that card's
subtree that returns the bounds of the widget whose id is the card's link id (translated by any
scrollable it is inside, as `ui/focus.rs::focused_bounds` does). The link itself never takes the
keyboard on a press (R5). A control outside the grid that held the keyboard loses it as on any
click, which is the app-wide rule (`keyboard_focus.rs`, FR-022a).

**Why**: every `TakesTheKeyboard` drops focus on a press outside itself, in tree order, before the
link can capture; only the grid sits above all the cards.

**Rejected**: re-focusing the previous card after the click (a frame of collapse and re-expand, and
the Escape flag would be lost); making `TakesTheKeyboard` ignore captured presses app-wide (a click
on any button would stop clearing focus elsewhere).

## R8 — Pointer rest on a link (FR-007)

**Decision**: the grid's rest timer (`card_grid.rs::observe_rest`) tracks `(card, on_link)`: a change
of either re-arms it; after `EXPAND_REST` it publishes the card's `on_rest` or, on the link, the new
`on_link_rest`; `on_leave` still only when the card changes. Core: `Event::LinkRested(l)` (new):
with a menu open nothing; else `pointer_on = l`, `pointer_escaped = false`, and nothing expands —
`l` expanded stays expanded, a collapsed `l` stays collapsed, another focus-expanded card is not
re-expanded while the pointer is on `l` (contract K5).

**Why**: a rest on the link counts as a rest on its card for the re-expansion rule (FR-007), but must
not move the link from under the pointer. Moving off the link onto the card re-arms the timer, so
the card expands after a rest there, as 1121 FR-002 defines.

**Rejected**: a hover message from `TextLink` to the app (the grid would still have to re-arm its
timer, and every pointer move across a link would be an app message).

## R9 — Keyboard: Tab stops, arrows skip, focus between card and link (FR-008, FR-009)

**Decision**:

- Tab and Shift+Tab: iced's `focus_next`/`focus_previous` as today; the link is the focusable right
  after its card (R6 traversal order), so no code.
- Arrows: the Overview's arrow listener (`ui/mod.rs::overview_arrow`) sends
  `Message::Overview(Msg::CardStepped { forward })` (new) instead of the root `FocusMoved`; the root
  runs `ui::overview::step_card(forward, cards)` (new), an `Operation` that focuses the next or
  previous **card** id, counting from the focused card or from the card of the focused link
  (contract U4). The root `Message` vocabulary is unchanged (`root_vocabulary_is_cross_cutting`).
- Focus events: the link reports `Event::LinkFocusGained(l)` / `LinkFocusLost(l)` (new) through
  `TextLink::on_keyboard_focus` (shown focus only). `Expansion` tracks which part of the focused
  card holds the keyboard; a move between a card and its own link, in either order of the two
  reports (forward Tab: card lost then link gained; Shift+Tab: card gained then link lost), keeps
  `focus_escaped` and changes no form (contract K5).
- Menu key on the link: the link gets its own message, `Msg::LinkMenuRequested(l)` (new; the card's
  `on_menu_key` stays `CardMenuRequested(l)`, which carries no origin). `main.rs::overview_message`
  handles it in the `CardMenuRequested` arm, with the same open/activating guard and the same
  `focused_bounds_task`, which then measures the focused **link**, so the menu is anchored in the
  link's bounds (`keyboard_anchor`), and opens it with origin `MenuOrigin::KeyboardFromLink` (new).
  The reducer treats `KeyboardFromLink` as `Keyboard` (first entry highlighted, `focus_return` set
  on close) and keeps the origin in `CardMenu`; closing without a choice makes
  `main.rs::card_focus_return` focus `card_link_id(l)` (new), falling back to the card id when the
  link is gone. Tests: a `main_tests.rs` test that the link's Menu key opens the menu with
  `KeyboardFromLink` and the card's with `Keyboard`; a reducer test that dismissing a
  `KeyboardFromLink` menu sets `focus_return` and the return focuses the link id.
- Focus across the menu (FR-009 "counting as a move between a card and its own link"): opening the
  menu takes the keyboard from the link (`LinkFocusLost(l)` while `menu_open`), and the return
  brings `LinkFocusGained(l)` after `MenuClosed`. 1121's `FocusLost` clears `focused` and
  `focus_escaped` even while the menu is open, so the plain pairing rule would re-expand an
  Escape-collapsed card. `Expansion` therefore holds the pairing across the menu (contract K5,
  data-model *Expansion* "Held across the menu"): `LinkFocusLost(l)` while the menu is open keeps
  `focused = l`, `focus_escaped` and part `Link`; the next focus report after `MenuClosed` decides —
  `LinkFocusGained(l)` is a move inside the card (no form change), anything else first applies the
  deferred loss as `FocusLost(l)` would have. The card's own `FocusLost` while the menu is open is
  unchanged (1121's K1 suite stays green).
- Escape: unchanged (`app.rs::on_escape` asks the expansion first; focus stays where it is).
- Page keys: the card passes its `on_page` closure to the link (Story 3 sc. 6), and
  `scroll_focused_into_view` (already chained after focus moves) brings a link in the expanded body
  into view; a client test proves it reaches the inner scrollable.

**Rejected**: keeping arrows on `FocusMoved` (they would stop on links); a `Msg` per part
(`FocusGained { link: bool }`) changing 1121's existing event signatures and callers; a
`from_link: bool` field on `CardMenuRequested` (changes 972's variant and its callers for one
case); keeping `focused`/`focus_escaped` for every focus loss while the menu is open (changes 1121's
behaviour for card-opened menus).

## R10 — Opening the address and failures (FR-004, FR-013, FR-014)

**Decision**: the card's link publishes `Msg::IssueLinkActivated(location)` (new). The root
(`main.rs::overview_message`) recomputes the link from current state (`ui::overview::issue_link_for`)
and hands its address to `shell::links::perform(app, OpenRequest::Url(address))`, as
`shell/pr_status.rs::open_requested` does. A failure comes back as `LinkOpenFinished` and is
reported by feature 031's notification. The Overview state is untouched, so nothing else changes.
Addresses are never logged: the new core module has no logging and the message's `Debug` shows the
location only.

**Rejected**: carrying the address in the message (the decision would be made at render, and a
stale card could open a link the settings have since removed).

## R11 — Card layout (FR-003)

**Decision**: collapsed: the branch line becomes `row![Ellipsized(short branch).shrink(), link]`
(the link at its natural width, the branch giving way first); expanded: `row![wrapped full branch
(Fill), link]` with the link top-aligned after it; with no branch line (title is the branch,
orphan or invalid) the link alone in the branch line's place. The link is drawn at `TypeRole::Body`,
the branch line's role (FR-015).

**Why**: FR-003's ellipsis order and "never clipped" at 280 px: the link text is at most
"Issue " plus a key, short enough for the narrowest card.

**Rejected**: the link on a line of its own below the branch (an extra line on every card with a
ticket, shrinking the collapsed card's room for the title and summary); the link after the title
(the title wraps and ellipsizes by its own rules, so the link would move with title length and be
clipped with it); the link before the branch (the branch, the line's subject, would be the one cut
first by position rather than by FR-003's ellipsis order).

## R12 — Tests, snapshot, showcase, docs

**Decision**: core unit tests for K1–K5 (`mise run test-core`); client in-crate tests for
`TextLink`, `ControlOverlay`, the card (presses, release-off, keys, ripple, focus, secondary click,
widths 280/380/480 both forms) and the grid (press routing, link rest); reducer tests for the new
messages; daemon `tests/jira_base_url_setting.rs` (new, modelled on `jira_setting.rs`'s
per-project connection tests);
`composition_contrast.rs` gains `primary` on the card body; covered state `overview-issue-link`
(new) with a GitHub and a Jira link; the existing `overview-open` anchors move one level because of
`ControlOverlay`, an intended change, re-blessed once; showcase *Surfaces* shows a card with a link,
collapsed and expanded; docs: `docs/user-guide/overview.md` (*Opening a worktree's issue*,
*Keyboard*), `docs/user-guide/settings.md` (Jira section), `docs/daemon.md` (version 58).

**Rejected**: a new geometry gate for the link (the existing text-overflow and containment gates on
the new covered state check "never clipped").
