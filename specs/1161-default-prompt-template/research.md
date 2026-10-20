# Research: default prompt template (#1161)

## R1 Where rendering lives
- **Decision**: new `micold-core::prompt_template` (pure, no iced, no I/O).
- **Rationale**: FR-011 needs one renderer for client and daemon; core is already shared and is where TDD logic goes (constitution I).
- **Rejected**: a renderer in each of client and daemon (drift); a template engine crate (`handlebars`, `tera`): far more syntax than six placeholders, and conditionals are out of scope.

## R2 Syntax and parsing
- **Decision**: one left-to-right scan; `{name}` substitutes, `{{`/`}}` are literal braces, anything else with a brace is a `TemplateError` (unknown name, unbalanced). Values are appended to the output, never re-scanned (US2 s4).
- **Rejected**: sequential `str::replace` per placeholder (re-expands inserted bodies, order-dependent); regex (new dependency, same result).

## R3 Where the template is stored and reaches the daemon
- **Decision**: `Settings.default_prompt_template: String` (`#[serde(default)]`), service-owned like `default_ai_cli`: a new `Option<String>` field on the client-to-daemon set-settings message (`protocol/messages.rs`), a `server.rs` handler arm, `state`/`catalog.set_default_prompt_template`, and a `DaemonSettings` field returned by `settings_wire()` (so an unrelated settings change does not revert it). The client sends it on save.
- **Rationale**: the daemon already holds a copy of the settings it needs and updates it when they change; remote daemons then need no file access (spec edge case).
- **Rejected**: daemon reading the client's file (breaks remote daemons); a new message kind (an optional field on the existing one suffices).

## R4 Validation and bad stored text
- **Decision**: `PromptTemplate::parse(&str) -> Result<PromptTemplate, TemplateError>`; `SettingsDraft::validate` refuses on `Err` (the save path writes only on `Ok`, so the stored value is kept). At load the string is stored untouched; users of it call `parse` and treat `Err` as empty (FR-013). Opening Settings validates the stored text and shows the error.
- **Rejected**: dropping the text at load (loses user data); failing the load (app does not start).

## R5 Issue title and body for MCP
- **Decision**: reuse `IssueLookup` (550). `IssueSnapshot` gains `body_text` and `ISSUE_QUERY` adds `bodyText` (already used by the list query). The facts come from the worktree dir name: `naming::ticket` gives `{issue}`, `naming::parse_tags` the type, the dir name the name, the catalog the branch. The lookup runs under `tokio::time::timeout(5 s)` on the blocking pool; timeout or error renders title and body empty. A non-numeric ticket (Jira key) gives `{issue}` = key and no lookup: Jira title/body are empty over MCP in this feature (FR-014 allows empty when no lookup succeeds).
- **Rejected**: a Jira lookup in the daemon (new credential path, outside the issue); blocking creation on the lookup (FR-012); a persisted copy of title/body (privacy, SC-006 of 034).

## R6 Dialog pre-fill and "edited"
- **Decision**: `WorktreeForm.first_prompt_edited: bool`, set by `first_prompt_changed`, cleared when the form opens and when a CLI is newly chosen from none. `refresh_default_prompt` re-renders into `first_prompt` after a change of CLI, issue, issue details, type or name, only when `!first_prompt_edited` and a CLI is set. Clearing the field counts as an edit (US1 s3). Full issue body comes from a new `Issue.body` (the list already reads `bodyText`; `description` stays the truncated row text).
- **Rejected**: comparing field text to the last rendering (a user retyping the same text then loses it to a refresh); rendering only at submit (user could not see or edit it).

## R7 Settings field widget
- **Decision**: reuse `ui/material/text_area.rs` (wraps iced `text_editor`). The draft holds a `text_editor::Content`; `SettingsDraft::validate` extracts its text. No new widget.

## R8 Explicit prompt wins
- **Decision**: `first_prompt_for(explicit: Option<String>, template, facts)`: a non-blank explicit prompt is returned untouched and the template is not parsed or looked up; otherwise render. A blank explicit prompt falls through to the template (spec US4). Today `prompt: Some("")` counts as given; SC-003 compares only the empty-template case, so unchanged.
