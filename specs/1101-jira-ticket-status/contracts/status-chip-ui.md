# Contract: status chip and refresh events

**Feature**: [spec.md](../spec.md) | [research R3, R4, R7, R8](../research.md)

## Chip (`ui/material/status_chip.rs`)

```rust
StatusChip::new(&TicketStatus).stale(bool)   // -> Element via .into()
StatusChip::accessible_text(&TicketStatus, stale: bool) -> String // "In Review" / "In Review, may be out of date"
```
- Built on `Tag`; tint by category: ToDo = secondary role, InProgress = primary, Done = tertiary;
  no category = neutral. Stale = dimmed. Text is always the status name (not by tint alone).
- Truncates with an ellipsis (`Ellipsized`) within a max width; never pushes key/title.
- `None` status draws nothing and adds no spacing (FR-009).
- Accessible text is also the tooltip (research R7).

## Surfaces

| Surface | Placement | Source |
|---|---|---|
| New worktree picker row | beside the key | `Issue::status` |
| Sidebar worktree row | right of the `Tag::Issue` tag | `StatusBoard::status_of(key)` |
| Overview card | in the card header beside the ticket | same board |

## Refresh events

| Event | Condition | Action |
|---|---|---|
| Project opened (NEW call `shell::jira_refresh::on_project_changed(app)` in the project-change branch of `shell/daemon_sync.rs` beside `Overview::Msg::ProjectSwitched`, and once after startup restores the project, `shell/startup.rs`) | gate passes, >=1 linked Jira key | `clear()` (new epoch) then `begin` + one request; the board is app-wide, so a second window attaching to the same project does not fire it |
| `WindowFocusChanged(true)` | gate passes, board `due(Focus)` | same |
| Any other | | none (FR-006) |
| Switch off / connection removed / project switch | | `clear()` |

Gate = `jira_request(app)` (switch, connection, token, source). Result message
`JiraStatusesRead { keys, result }` applies `StatusBoard::apply`. The disclosure in Settings > Jira
and the picker states that linked tickets' keys are sent with the project key and credentials, also
with the dialog closed (FR-006b).
