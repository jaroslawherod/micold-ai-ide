# Contract: the context header (`micold_core::context_header`, `ui/session_area.rs::header`)

## Text rules (core, pure, no I/O, no logging; each row is a core unit test)

### H1. Title (FR-002)

`HeaderText.title == model.title`. For the four cases of feature 1120's `card_title`:

| Model | Title | Tooltip |
|---|---|---|
| issue title "CI red on main: daemon tests", branch `fix/1117_ci-red-on-main`, name `1117_ci-red-on-main` | `CI red on main: daemon tests` | `CI red on main: daemon tests` + newline + `1117_ci-red-on-main` |
| no issue title, branch `feat/x` | `feat/x` | `feat/x` + newline + the display name |
| no issue title, no branch, name `scratch` | `scratch` | `scratch` |
| Default entry | `Default` | `Default` |

### H2. State (FR-003)

`HeaderText.state == model.state` for each of the five card states; the word is
`CardState::label` (*Needs action*, *Needs attention*, *Working*, *Done*, *Idle*), the tooltip is
`model.reason.label()`.

### H3. Branch chip (FR-004)

| Branch | Chip |
|---|---|
| `fix/1117_ci-red-on-main` | text `fix/1117`, tooltip `fix/1117_ci-red-on-main` |
| `main` (no short form) | text `main`, tooltip `main` |
| none (Default, detached HEAD) | `None` |

### H4. Pull request chip (FR-005)

| Input | Chip text |
|---|---|
| none | `None` |
| #1118, `Open { checks: Failing }` | `PR #1118 · Open · checks failing` |
| #7, `Draft { checks: Pending }` | `PR #7 · Draft · checks pending` |
| #7, `Open { checks: None }`, review `ChangesRequested` | `PR #7 · Open · changes requested` |
| #7, `Open { checks: Passing }`, `conflict` | `PR #7 · Open · merge conflict` |
| #7, `Open { checks: None }`, review `None` | `PR #7 · Open` |
| #9, `Merged` | `PR #9 · Merged` |
| #9, `Closed` | `PR #9 · Closed` |
| any, `stale` | same text; tooltip ends with the stale note |

The phrase is `StatusPhrase::of_status`; `of_status` and `of` agree on every
`(checks, review, conflict, state)` combination (one table test over both).

### H5. Changes label (FR-006)

| Counts | Label |
|---|---|
| `Known { files: 3, added: 42, removed: 7 }` | `Changes · 3 files · +42 −7` |
| `Known { files: 1, added: 0, removed: 5 }` | `Changes · 1 file · +0 −5` |
| `Known { files: 0, .. }` | `Changes · no changes` |
| `Unknown` | `Changes` |

`−` is U+2212, as the Changes view's rows write it (`ui/changes.rs:536`).

### H6. Summary (US3 scenario 1)

`ChangeSummary::of` over a list of a text file (+10 −2), a binary file (0, 0) and a renamed file
(+1 −1) is `{ files: 3, added: 11, removed: 3 }`.

## Header behaviour (client)

### H7. Following the location (FR-014)

From `State::update`'s follow step (integration test through `State::update`):

- selecting a session of another location sends one `ReadCounts` for that location, and the
  header's entry is it;
- selecting another session of the same location sends nothing;
- no active session, or its worktree removed: entry `None`, no read;
- an answer for an earlier location (old `seq`) is dropped;
- `Msg::Changed` with the view open on location A and the header on B reads both lists.

### H8. Pressing Changes (FR-007)

| View | Press sends | Button |
|---|---|---|
| closed | `SidebarMsg::ReviewChangesRequested(shown location)` | not selected |
| open on the shown location | `changes::Msg::Closed` | `.selected(true)` |
| open on another location | `SidebarMsg::ReviewChangesRequested(shown location)` | not selected |

The press message is chosen by a pure function (`session_area::changes_press(state, &location)`)
covered by a client unit test.

## Header layout (client glue; geometry gates and visual pass)

### H9. Structure

`column![ row![ title, changes_button ], chips ]`, padding `spacing::LG` horizontally, the title
`Length::Fill`. The child lists of the column and the rows never vary; the chips row holds the state
chip always, then the branch chip and pull request chip when present. Title is `Ellipsized` at
`TypeRole::Title`; each chip is a `Tag` with `.max_width(CHIP_MAX_WIDTH)` (`CHIP_MAX_WIDTH` a new const in `ui/session_area.rs`) in a `Tooltip` with its
full text. The Changes button sits on the title line, so the chips keep the header's whole width at
the smallest window (M3: beside both lines it left the chips no room at 640×480). It is never
narrower than it asks (gate, H10).

### H10. Fit (FR-013, SC-004)

At 640×480 with the sidebar at 300 px, and at 1280×800, light and dark: the title, every chip and the
Changes button lie inside the session area and overlap nothing; the title and chips may be cut with
an ellipsis; the Changes button is laid out at its requested width.
