# Contract: the dialog, the notice and the Settings section

**Feature**: [spec.md](../spec.md) | Research [R9](../research.md#r9--the-switchs-surfaces-and-the-notice)

Render glue under `src/ui/` only composes; every decision is in a reducer or `micold-core`.

## 1. Gating

`jira_visible(settings, project) -> JiraAvailability`: `Connected` only when `jira_enabled` and
`jira_connections` holds an entry for the open project. With `Off`, the form renders exactly the
034 form (US2 AS1, FR-003): the chip row, caption and body are byte-identical to the layout snapshot
before this feature (a covered-state test pins it).

## 2. Create-worktree dialog

| Situation | Chip row | Under the chips |
|---|---|---|
| Jira `Off` | as 034 | as 034 |
| GitHub remote + Jira `Connected` | New, Existing, **Issue** | a second `ToggleChip` row **GitHub / Jira** (the source choice), then the provider's notice |
| no GitHub remote + Jira `Connected` | New, Existing, **Issue** (enabled) | the Jira notice; no source choice (US4 AS3) |

The chip is labelled "GitHub issue" while Jira is `Off` (unchanged text) and "Issue" while
`Connected`. The Jira body is the 034 body: Notice, then Loading (`StageProgress`
"Loading issues from {host}..."), `Failed` caption + **Retry**, zero issues "{KEY} has no open
issues.", or the `Typeahead` (label "Issue", placeholder "Search by key, summary or label"), the
cap caption "Showing the 1,000 most recently updated issues - search also looks on {host}", then
Type/Ticket/Name via the shared `naming_inputs`, and the preview.

**Notice (FR-005b)**, shown before, during and after a load: "Reads open issues of project {KEY}
from {origin}. Sends the project key, your search text and your credentials."

## 3. Settings -> Jira (`ui/settings/jira.rs`)

- Rail entry "Jira" (always present; while the switch is off the page holds only the switch, which
  is FR-001's "its own switch"; the shared Integrations area stays out of scope). Always shows: the switch **Enable Jira integration** with the line "Lets
  Micold read open issues of a Jira Cloud project to start worktrees from. Off: nothing is sent to
  Jira." (FR-001).
- Switch on: a status line "Enabled. Connected to {site} - {KEY}." or "Enabled. This project has no
  Jira connection yet." or "Open a project to connect it to Jira."; fields **Site URL**, **Project
  key**, **Account email**, **API token** (masked; placeholder "A token is stored" when
  `token_stored`), the data-use line of FR-005b, and **Remove connection** when a connection exists.
- Field errors are shown under their field (`FieldError`), from `validate`.
- Switch off with a stored connection: an inline prompt "Keep the Jira connection for {project}?"
  with **Keep** and **Remove**.
- Save/Cancel are the Settings view's existing ones.

## 4. Layout coverage

New covered states in `tests/support/covered_states.rs`, `layout_snapshot.txt` regenerated:
dialog with source choice (GitHub shown); dialog Jira `Loading`; `Failed` + Retry (credentials
refused); `Loaded` with cap caption; Settings Jira off; on with no connection; on with connection
and a field error; keep-or-remove prompt.

## 5. Documentation (Principle VII, FR-024)

| Page | Section | Milestone |
|---|---|---|
| `docs/user-guide/settings.md` | "Jira": the switch, connecting a project, the token file location per OS, removing | slice 1 |
| `docs/user-guide/worktrees-and-sessions.md` | "From a Jira issue": prerequisites (Cloud, API token link), what is sent, the source choice, each notice and its remedy | slices 2-3 |
| `docs/user-guide/settings.md` | GitHub issues section: the "also applies to Jira" sentence | slice 2 |
| `docs/development/architecture.md` | where the Jira request runs (client), why `ureq`, the two-owner settings rule | Polish |
