# Contract: the link on the card (`micold-client`)

In-crate tests (`ui::material::test_support::Mounted`) unless a file is named.

## U1 — `TextLink` (shared component, `ui/material/text_link.rs`, new)

Builder: `TextLink::new(label, on_press, roles)`, `.role(TypeRole)` (default `Body`), `.id(Id)`,
`.on_keyboard_focus(Fn(bool) -> M)`, `.on_menu_key(M)`, `.on_page(Fn(bool) -> Option<M>)`,
`.focus_on_press(bool)` (default `true`); `impl From<TextLink> for Element`.

- Press and release on it → `on_press` once; the press and the release are captured.
- Press on it, release off it → nothing published, release not captured.
- Press off it, release on it → nothing published, release not captured.
- Enter and Space while it has the keyboard → `on_press`; while it does not → nothing.
- Menu key / Shift+F10 while focused → `on_menu_key`; Page Up/Down → `on_page` when it gives one.
- `focus_on_press(false)`: a press on it leaves it without the keyboard (`TakesTheKeyboard`
  `.press_takes_focus(false)`, new; every existing caller keeps `true`).
- `mouse_interaction` is `Pointer` over it, `None` elsewhere. Drawn in `roles.primary`; an
  underline in the same colour only while the cursor is over it; the focus ring (`roles.secondary`)
  only for shown keyboard focus.
- Showcase catalogue entry; `tests/material_builder_api.rs` and `tests/showcase_completeness.rs`
  pass.

## U2 — `ControlOverlay` (`ui/cdk/control_overlay.rs`, new)

`ControlOverlay::new(content, control)`; the content sizes the node, the control is laid out at
that size.

- An event the content captures never reaches the control.
- Otherwise the control gets it, with the cursor levitated where the content's
  `mouse_interaction` is not `None`.
- `draw`: content, then control. `mouse_interaction`: the content's where it is not `None`, else
  the control's. `operate`: control, then content (focus order).

## U3 — `OverviewCard` with a link (`ui/material/overview_card.rs`)

Builders: `.on_issue_link(M)` (the card shows `model.issue_link` only with it), `.link_id(Id)`,
`.on_link_keyboard_focus(Fn(bool) -> M)`, `.on_link_rest(M)` (read by the grid like `on_rest`).
The link reuses the card's `on_menu_key` and `on_page`. Tests, each on the collapsed and the
expanded card:

- Click on the link → only the link's message; no ripple started; the card does not take the
  keyboard; `on_press` not published (FR-005, SC-003).
- Press elsewhere on the card, release on the link → nothing published; the ripple fades (FR-005).
- Press on the link, release elsewhere on the card → nothing published (Story 4 sc. 5).
- Click on the card outside the link → `on_press` with its ripple, as before (Story 4 sc. 1).
- Secondary press on the link → `on_secondary_press` (FR-006).
- Enter on the focused link → link message, no card ripple; Enter on the focused card → card.
- Tab from the card focuses its link; Tab again leaves the card; Shift+Tab reverses (FR-008).
- At 280, 380 and 480 px, with a 120-character branch: the link's text is whole and inside the
  card, the short branch ends in an ellipsis (FR-003, SC-004); with no branch line the link sits
  alone in its place.
- A card whose model has no `issue_link` lays out exactly as before (no extra node in the body).

## U4 — Arrows skip links: `ui::overview::step_card(forward, &[SessionLocation])`

An `Operation` that focuses the next (or previous) card id after the focused card, or after the
card whose link is focused; it never focuses a link id. From the first card backward and from the
last forward it does what `focus_previous`/`focus_next` did from them before (wraps as iced does).
Tested with `Mounted` on a three-card grid where the middle card has a link.

## U5 — `CardGrid` (`ui/material/card_grid.rs`)

- A primary press over a card's link is delivered to that card only; another card holding the
  keyboard keeps it (R7).
- The rest timer re-arms when the pointer crosses between a card's link and the rest of the card;
  after `EXPAND_REST` on the link it publishes `on_link_rest`, on the rest of the card `on_rest`;
  `on_leave` only when the card under the pointer changes (R8). No new `motion::wake_at` caller.

## U6 — Wiring (`ui/overview.rs`, `main.rs`)

- `ui::overview::cards` sets `CardModel.issue_link` from `IssueSources { github: overview.repo,
  github_on: settings.github_issues_enabled, jira_base: settings.jira_base_url.as_ref(),
  jira_on: settings.jira_enabled }`.
- `Msg::IssueLinkActivated(l)` → `shell::links::perform(OpenRequest::Url(address))` with the address
  recomputed from state; nothing when the link is gone; Overview state unchanged (FR-004, FR-013).
- `MenuOrigin::KeyboardFromLink` → closing the menu focuses `card_link_id(l)` (FR-009).
- Reducer tests (`app.rs` / `features::overview`): `RepoRead` recorded and dropped for another
  project; `ProjectSwitched` clears `repo`; `IssueLinkActivated` changes no state.
