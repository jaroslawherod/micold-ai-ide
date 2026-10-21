# Data model: Overview cards, a link to the worktree's issue

## IssueRef, IssueLink, IssueSources (core, `micold_core::issue_link`, new)

Derived per render, never stored (spec *Key Entities*).

| Type | Fields | Meaning |
|---|---|---|
| `IssueRef` | `Github(u64)` \| `Jira(String)` | The ticket of a worktree directory name, classified: all digits → GitHub; the Jira form → the key upper case; anything else → none |
| `IssueSources<'a>` | `github: Option<&'a GithubRepo>`, `github_on: bool`, `jira_base: Option<&'a JiraBaseUrl>`, `jira_on: bool` | What FR-001 checks besides the name |
| `IssueLink` | `label: String`, `address: String` | "Issue #N" / "Issue KEY" and the browser address (FR-002, FR-004) |

`Debug` of `IssueLink` prints the label only (FR-014: addresses are not logged).

## CardModel (core, `micold_core::overview`)

Gains `issue_link: Option<IssueLink>`, set by `crates/micold-client/src/ui/overview.rs::card` from
`issue_link::issue_link(dir, &sources)` for a worktree, `None` for *Default* (FR-001).

## JiraBaseUrl (core, `micold_core::jira`, new)

| Field | Type | Rule |
|---|---|---|
| (inner) | `String` | `https://host[:port][/path]`, host lower case, no trailing `/`, no user info, query or fragment (FR-011) |

Serialized as its string; deserialized through `parse` (strict on the wire; lenient in the settings
file, where a value that does not parse reads as unset). `JiraField::BaseUrl` and `BASE_URL_REFUSED`
name a refusal.

## Settings and wire

| Where | Field | Default |
|---|---|---|
| `micold_core::settings::Settings` | `jira_base_url: Option<JiraBaseUrl>` | `None`; missing or unparsable entry → `None` |
| `protocol::messages::DaemonSettings` | `jira_base_url: Option<JiraBaseUrl>` | — |
| `ClientMsg::SettingsSet` | `jira_base_url: Option<Option<JiraBaseUrl>>` | `None` = unchanged; `Some(None)` = clear |
| `features::settings::State` (client mirror) | `jira_base_url: Option<JiraBaseUrl>` | `None`, set by `adopt_daemon_settings` |
| `features::settings::JiraDraft` | `base_url: String` | the mirror's text, or empty |

Contract: [contracts/wire.md](contracts/wire.md).

## Overview state (client, `features::overview::State`)

| Field | Type | Meaning |
|---|---|---|
| `repo` (new) | `RepoLookup` = `Unknown` \| `Asked` \| `Known(Option<GithubRepo>)` | The project's GitHub repository (R2); `Unknown` after `ProjectSwitched` |

New messages: `Msg::RepoRead { project, repo: Option<GithubRepo> }` (dropped for a project no
longer active, through `read_project`), `Msg::IssueLinkActivated(SessionLocation)`,
`Msg::CardStepped { forward: bool }`, `Msg::LinkMenuRequested(SessionLocation)` (the link's Menu
key; the root opens the menu with origin `KeyboardFromLink`). `MenuOrigin` gains
`KeyboardFromLink`, handled as `Keyboard` in the reducer; `focus_return` then names the link
(FR-009). `repo` is read again on `DaemonMsg::Attached` for the active project while the Overview
is open and `repo` is `Unknown` (R2).

## Expansion (core, `micold_core::overview_expansion`) — additions to feature 1121's model

| Field | Change |
|---|---|
| `focused` | Now also set while the card's **link** holds the keyboard (FR-008) |
| `focus_part` (new) | `Card` \| `Link` \| `Both`: which of the focused card's two stops reported focus last, so the two reports of a move inside one card (either order) are one move |
| `link_held` (new) | `Option<SessionLocation>`: the link whose focus the open card menu took; the loss is deferred to the next focus report ("Held across the menu", FR-009) |

### New events

| Event | Effect |
|---|---|
| `LinkRested(l)` | menu open → none. Else `pointer_on = l`, `pointer_escaped = false`; nothing expands; an expanded `l` stays expanded (FR-007) |
| `LinkFocusGained(l)` | menu open → record only. If `focused == l` (its card holds or just held the keyboard): part becomes `Link`, form and `focus_escaped` unchanged. Else as `FocusGained(l)` |
| `LinkFocusLost(l)` | if `focused == l` and the card part holds the keyboard: part becomes `Card`, nothing else. Else as `FocusLost(l)` |
| `FocusGained(l)` / `FocusLost(l)` | as 1121, with the same "inside one card" rule: a card report while its link holds or just held the keyboard changes the part only |

"Just held": the immediately preceding event was the matching lost report for `l` (forward Tab:
`FocusLost(l)` then `LinkFocusGained(l)`). Any other event in between ends the pairing, and the
lost report's collapse stands (contract K5).

"Held across the menu" (FR-009): `LinkFocusLost(l)` while the menu is open (the link's Menu key
opened it) keeps `focused = l`, `focus_escaped` and part `Link`, and sets `link_held = Some(l)`
(new field). `MenuOpened`, `MenuClosed` and pointer events do not end it. The next focus report
ends it: `LinkFocusGained(l)` is a move inside the card (form and `focus_escaped` unchanged); any
other focus report first applies the deferred loss exactly as `FocusLost(l)` with the menu closed
would, then itself. `WindowBlurred`, and `Listed` without `l`, clear it with `focused`.
