# Contract: issue link rules (`micold-core`)

Every item is a red-first unit test in the module named (FR-016, `mise run test-core`).

## K1 — `issue_link::issue_ref(dir_name: &str) -> Option<IssueRef>`

Built on `naming::ticket(dir_name)`.

| Directory name | Result |
|---|---|
| `feat-1122_overview-issue-link` | `Github(1122)` |
| `feat-mic-87_add-login` | `Jira("MIC-87")` |
| `fix-ABC-123_login` | `Jira("ABC-123")` |
| `feat-ab2-9_x` (digits after the first letter) | `Jira("AB2-9")` |
| `feat_login`, `plain`, `feat-login` (no ticket) | `None` |
| a ticket neither all digits nor the Jira form (`abc`, `87-mic`, `mic-`, `mic-8a`) | `None` |
| an all-digit ticket that overflows `u64` | `None` |

## K2 — `issue_link::issue_link(dir_name, &IssueSources) -> Option<IssueLink>` (FR-001, FR-002, FR-004)

- `Github(n)` with `github: Some(o/r)` and `github_on` → label `Issue #n`, address
  `https://github.com/o/r/issues/n`. `github: None` or `!github_on` → `None`.
- `Jira(k)` with `jira_base: Some(b)` and `jira_on` → label `Issue K`, address `<b>/browse/K`.
  `jira_base: None` or `!jira_on` → `None`.
- A GitHub ticket never uses the Jira base, a Jira key never the repository.
- `Debug` of `IssueLink` does not contain the address (FR-014).

## K3 — Addresses have exactly one `/` before `browse` (SC-002, Story 2 sc. 5)

Base saved as `https://example.atlassian.net/` → `https://example.atlassian.net/browse/MIC-87`;
`https://jira.example.com/jira` → `https://jira.example.com/jira/browse/MIC-87`.

## K4 — `jira::JiraBaseUrl::parse(&str) -> Result<JiraBaseUrl, FieldProblem>` (FR-011)

- Accepts, trimmed: `https://example.atlassian.net`, `https://example.atlassian.net/`,
  `https://jira.example.com/jira/`, `https://jira.example.com:8443/jira`, `HTTPS://Example.Com`;
  stores without trailing `/`, scheme and host lower case, path as written.
- Refuses with `JiraField::BaseUrl` and `BASE_URL_REFUSED`: `http://…` (in every build), `ftp://…`,
  no scheme, `https://` (no host), `https://user@host`, `https://host?x=1`, `https://host#a`,
  `https://host/path?x`, a host with an empty or invalid label, a port of 0 or non-digits.
- `SiteOrigin::parse`'s existing tests stay green after the shared helper is extracted.
- Settings file: an entry that does not parse reads as `None` and the rest of the document loads;
  a missing entry reads as `None` (FR-010).

## K5 — `overview_expansion::Expansion` additions (FR-007, FR-008, FR-009)

Each line is a test, on top of 1121's K1 suite, which stays green unchanged:

- `LinkRested(A)` with A collapsed → nothing expanded; then `PointerRested(A)` → A (Pointer).
- A expanded (Pointer), `LinkRested(A)` → A stays expanded.
- Focus A (expanded), rest B → B; leave B; `LinkRested(C)` → none expanded (A not re-expanded
  while the pointer rests on C's link); `PointerLeft(C)` → A again.
- Menu open: `LinkRested(B)` → no change.
- Forward Tab: focus A (expanded), `FocusLost(A)`, `LinkFocusGained(A)` → A expanded, focused A.
- Same after Escape: focus A, `Escape` (collapsed), `FocusLost(A)`, `LinkFocusGained(A)` → none
  expanded, `focus_escaped` still true (FR-008: no re-expansion by a move inside the card).
- Shift+Tab: `LinkFocusGained(A)` (escaped as above), `FocusGained(A)`, `LinkFocusLost(A)` → none
  expanded; focused A.
- Link to the next card: `LinkFocusLost(A)`, `FocusGained(B)` → B expanded, A collapsed.
- `FocusLost(A)` followed by any other event, then `LinkFocusGained(A)` → treated as a new focus
  (A expands, escaped cleared) — the pairing covers only the two reports of one move.
- Menu from the link, after Escape: `LinkFocusGained(A)`, `Escape` (collapsed), `MenuOpened`,
  `LinkFocusLost(A)`, `MenuClosed`, `LinkFocusGained(A)` → none expanded, `focus_escaped` still
  true, focused A (FR-009).
- Menu from the link, no Escape: A expanded by focus, `MenuOpened`, `LinkFocusLost(A)`,
  `MenuClosed`, `LinkFocusGained(A)` → A stays expanded, focused A.
- Menu from the link, a choice moves focus: A expanded, `MenuOpened`, `LinkFocusLost(A)`,
  `MenuClosed`, `FocusGained(B)` → B expanded, A collapsed, focused B.
- Card menu unchanged: `FocusGained(A)`, `Escape`, `MenuOpened`, `FocusLost(A)`, `MenuClosed`,
  `FocusGained(A)` → as 1121 today (the hold applies to the link's loss only).
- `Listed` without A while A's link holds the keyboard → `focused` cleared (Edge Case *Card goes
  away*).
