# Contract: template syntax and call sites

## Syntax
| Text | Result |
|---|---|
| `{issue}` | tracker id without `#` (GitHub number, Jira key) |
| `{issue_title}`, `{issue_body}` | title; plain-text body |
| `{branch}`, `{type}`, `{name}` | branch, conventional type, worktree name |
| `{{`, `}}` | `{`, `}` |
| any other `{...}`, lone `{` or `}` | `TemplateError` |
A missing value renders empty. Inserted values are not re-scanned. Blank result after trim means no prompt.

## Error text
`Unknown placeholder {issu}; valid: {issue}, {issue_title}, {issue_body}, {branch}, {type}, {name}`; unbalanced: `Unbalanced brace in the template; use {{ and }} for literal braces`.

## Dialog
On CLI/issue/details/type/name change and `!first_prompt_edited`: `first_prompt = render(...)` (empty when `None`). Submit sends `first_prompt` as typed.

## MCP (`create_session`, `create_worktree` with `ai_cli`/`prompt`)
`prompt` non-blank: used as is. Else: template rendered with facts from the worktree; `None` or a template error: no prompt, reply as today (`prompt_delivered` absent). Reply schema unchanged. `create_worktree` with neither `ai_cli` nor `prompt` creates no session. Tool descriptions in `micold-core/src/mcp/tools.rs` mention the default.
