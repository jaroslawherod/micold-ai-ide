# Contract: what the user sees

**Feature**: [spec.md](../spec.md) | **Research**: [R12–R15](../research.md) |
**Data**: [data-model §4](../data-model.md)

Wording in this file is the wording shipped. A change to it is a change to this contract.

---

## 1. `PullRequestIndicator` (shared component)

`crates/micold-client/src/ui/material/pull_request_indicator.rs`, builder form, re-exported from
`ui::material`:

```rust
PullRequestIndicator::new(state: PrMark, roles: &Roles)
    .checks(CheckMark)        // default: none
    .stale(bool)              // default: false
    -> Element
```

`PrMark` (`Open | Draft | Merged | Closed`) and `CheckMark` (`Pending | Passing | Failing`) are the
component's own enums; the sidebar maps `PrState` / `CheckStatus` to them, so `ui::material` does
not depend on the pull request module.

| Meaning | `Icon` variant | Material Symbol | Role (current) |
|---|---|---|---|
| open | `PrOpen` | `call_split` | `primary` |
| draft | `PrDraft` | `edit` | `on_surface_variant` |
| merged | `PrMerged` | `call_merge` | `tertiary` |
| closed | `PrClosed` | `block` | `on_surface_variant` |
| checks passing | `ChecksPassing` | `check` | `primary` |
| checks pending | `ChecksPending` | `schedule` | `on_surface_variant` |
| checks failing | `ChecksFailing` | `close` | `error` |

- **Layout**: the state glyph, then the check glyph when there is one, 16 px each, 2 px apart, in a
  box of fixed height. Width is 16 px without a check status and 34 px with one; it never depends
  on the sidebar's width (FR-009).
- **Without colour** (FR-009): the seven glyphs are seven different shapes; colour only repeats
  what the shape says.
- **Stale form** (FR-019): both glyphs take the `outline` role; shapes are unchanged.
- **Not interactive**: no press, no hover state, no tooltip of its own; the row's tooltip speaks
  for it.
- **Icons**: the eight new `Icon` variants (the seven above and `OpenInBrowser` for §4's menu
  entry, unless an existing variant already draws `open_in_new`) are codepoints of the shipped
  font, pinned by `tests/icons_font.rs`.
- **Showcase** (FR-033): one catalogue entry, "Pull request indicator", posing the 4 states without
  checks, open and draft with each of the 3 check statuses, and the stale form of open-passing and
  merged — 12 poses. `tests/showcase_completeness.rs` and `material_builder_api.rs` hold the entry
  and the builder form.

---

## 2. The worktree row

`ui/sidebar.rs::build_items`, for a row whose projection has `Some(RowPullRequest)`:

- The indicator is the first child of the row's **trailing** element, left of the hover action
  cluster. The name gives way to it (it is the name that shrinks or is cut, as for the row's
  existing trailing marks).
- A row with `removable` also carries a label-only chip **can be removed** in the row's existing
  chip slot, in the style of "outside this app". The chip has no press action (FR-016).
- A row with `None` builds exactly the element tree it builds today (FR-001): a layout test
  compares the two.
- Session rows and the "Default" row are never given either mark (FR-007).

---

## 3. Tooltip lines

`features::sidebar::worktree_tooltip` gains one argument, `Option<RowPullRequest>` plus `now`
through it; with `None` its output is **byte-identical** to today's (FR-011, test against today's
expected strings). With `Some`, these lines follow today's lines, in this order:

| # | Line | When |
|---|---|---|
| 1 | `Pull request: #<number> <title>` | always |
| 2 | `PR state: open` \| `draft` \| `merged` \| `closed` | always |
| 3 | `Checks: passing` \| `pending` \| `failing` | open or draft with a check status |
| 4 | `Review: approved` \| `changes requested` \| `review required` | GitHub reports a decision |
| 5 | `Read: <n> min ago` | stale (`is_stale`) |
| 6 | `Cleanup: merged — this worktree can be removed (right-click, Delete)` | `removable` |

- **Title**: control characters and line breaks become spaces; cut to 72 characters, the 72nd
  being `…`, so the number, which comes first, always shows (story 2 scenario 6). The tooltip's
  existing bounded width wraps the rest (029 FR-009).
- **`<n>`** is `(now − read_at) / 60`, rounded down; from 120 minutes on the line reads
  `Read: <h> h ago`.
- **No link, no control** (FR-013): the tooltip stays one string; no address appears in it.
- **No clock inside**: `now` is an argument; the function reads no disk and sends nothing
  (FR-012, SC-008).

---

## 4. Opening the pull request

- `ui/mod.rs::worktree_menu_items` gains **Open pull request**, placed directly above **Delete**,
  only for a row with `Some(RowPullRequest)`. A row without one has the menu of today, entry for
  entry (story 2 scenario 9).
- Choosing it sends `WorktreeMsg::PullRequestOpenRequested(dir)`. The shell looks up the row's
  branch in `statuses` and hands `url` to the existing `LinkOpener::open`.
- The address is opened **only when it starts with `https://github.com/`**; otherwise nothing
  happens. It is the address GitHub reported, never one built from the number (FR-014).
- The entry changes nothing in the application's state: no selection, no session, no sidebar
  change, no notice on success (FR-014). When the row lost its status between opening the menu and
  choosing the entry, nothing happens.
- A failure of the opener is reported the way the application already reports a link that could
  not be opened (feature 031); that is not an error "about pull requests" in the sense of FR-025,
  which concerns readings.
- Two actions from row to browser: right-click, choose (SC-009). No confirmation dialog (R15).

---

## 5. The Settings control

`ui/settings/github.rs`, at the top of the section, above the issue mapping. The section's title
changes from **GitHub issues** to **GitHub** (also in `features/settings.rs`, the rail label and
`tests/settings_sections.rs`).

`Checkbox`, label:

> Show pull request status on worktrees

`field_note` beneath it:

> Reads the pull requests of the open project's GitHub repository with your GitHub CLI sign-in:
> when the project opens, every 5 minutes, and when you refresh the worktree list. Only the
> repository's name and your worktrees' branch names are sent.

- Unchecked on a first start (FR-030).
- The note states the three things FR-029 asks for: what is read, how often, what is sent.
- Follows the Settings draft rules of the section's other controls: `Msg::PrStatusToggled(bool)`
  on the draft, sent as `SettingsSet { pr_status_enabled: Some(v) }` when the draft is applied.
- The control is always enabled: it does not depend on `gh` being installed (the switch is consent,
  not capability; with `gh` missing it is on and nothing is shown, FR-025).

---

## 6. Covered layout states

Added to `tests/support/covered_states.rs`, with `tests/fixtures/layout_snapshot.txt` regenerated:

| State | Holds |
|---|---|
| worktree row with an indicator (open, failing) | the indicator is inside the row, left of the action cluster; the name does not overlap it |
| row with indicator and the "can be removed" chip (merged) | chip and indicator inside the row, no overlap |
| row with a stale indicator | same geometry as the current form |
| the three above at the narrowest sidebar width | the indicator's width is unchanged; the name is what shrank |
| row without a pull request, switch on | geometry equal to the row with the switch off |
| Settings → GitHub with the switch | checkbox, note and the issue mapping below it, none clipped |
| showcase "Pull request indicator" | 12 poses, none clipped |

Colour, the glyphs' legibility in both themes and the real `gh` are quickstart §B's.
