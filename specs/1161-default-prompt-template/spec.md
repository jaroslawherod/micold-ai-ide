# Feature Specification: Default prompt template for a new worktree's session

**Feature Branch**: `feat/1161_default-prompt-template-in-settings-for-a-new`

**Created**: 2026-10-20

**Status**: Draft

**Input**: Issue #1161 — a setting that defines a default prompt template, rendered into the initial prompt of the session started with a new worktree (create-worktree dialog, MCP `create_worktree` / `create_session`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A standing default prompt in the create-worktree dialog (Priority: P1)

The user stores a template such as `/speckit-autopilot #{issue}` in Settings. When they create a worktree for an issue and choose an AI CLI, the dialog's first-prompt field is pre-filled with the template rendered for that worktree, and they may edit it before creating.

**Why this priority**: it is the whole value of the issue; everything else extends it.

**Independent Test**: set the template, open the dialog, pick an issue and a CLI, read the first-prompt field, create the worktree, and read the text the session received.

**Acceptance Scenarios**:

1. **Given** the template `/speckit-autopilot #{issue}` and an issue #42 picked, **When** a CLI is chosen, **Then** the first-prompt field shows `/speckit-autopilot #42`.
2. **Given** the pre-filled prompt, **When** the user edits it and creates the worktree, **Then** the session receives the edited text, not the template.
3. **Given** the user cleared the field, **When** they create the worktree, **Then** the session starts with no prompt (the template is not re-applied).
4. **Given** an empty template, **When** the dialog opens, **Then** the field is empty, as today.
5. **Given** the user typed a prompt and then changes the issue, type or name, **When** the form updates, **Then** the typed text is kept; only a field the user has not edited follows the template.

---

### User Story 2 - Placeholders and a worktree without an issue (Priority: P1)

The template may use `{issue}`, `{issue_title}`, `{issue_body}`, `{branch}`, `{type}` and `{name}` (the worktree name). A placeholder with no value for this worktree renders as empty.

**Why this priority**: the template is useless without them, and a no-issue worktree is common.

**Independent Test**: render one template against an issue worktree and a worktree with no issue.

**Acceptance Scenarios**:

1. **Given** the template `Fix {issue_title} on {branch} ({type})`, **When** rendered for issue #42 "Crash on save", type `fix`, branch `fix/42_crash`, **Then** the result is `Fix Crash on save on fix/42_crash (fix)`.
2. **Given** a worktree with no issue, **When** the template `/speckit-autopilot #{issue}` renders, **Then** `{issue}` is empty and the result is `/speckit-autopilot #`. A template whose rendering is blank after trimming starts no prompt.
3. **Given** a template containing `{{` and `}}`, **When** rendered, **Then** they produce literal `{` and `}`.
4. **Given** an issue body containing `{issue}`, **When** rendered, **Then** the body is inserted verbatim and not expanded again.

---

### User Story 3 - Validation in Settings (Priority: P2)

Saving a template with an unknown placeholder, or an unbalanced brace, is rejected with a message naming the placeholder and listing the valid ones. The previous stored value stays.

**Why this priority**: prevents silent empty or garbled prompts.

**Independent Test**: type `{issu}` in the setting and observe the message; type a valid one and observe it saved.

**Acceptance Scenarios**:

1. **Given** the template `Do {issu}`, **When** the user saves, **Then** the setting shows `Unknown placeholder {issu}; valid: {issue}, {issue_title}, {issue_body}, {branch}, {type}, {name}` and nothing is stored.
2. **Given** a stored template that a later version no longer accepts, **When** settings load, **Then** the app starts, the template is treated as empty for rendering, and Settings shows the error.

---

### User Story 4 - MCP `create_worktree` and `create_session` (Priority: P2)

When a session is started through MCP without an explicit prompt, the template renders into its first prompt. An explicit non-blank `prompt` always wins; the template is not rendered at all.

**Why this priority**: agents create worktrees too, and the issue names these tools.

**Independent Test**: call `create_session` on an issue worktree with and without `prompt`, and read `read_session_output`.

**Acceptance Scenarios**:

1. **Given** a template and a worktree for issue #42, **When** `create_session` is called without `prompt`, **Then** the session receives the rendered template.
2. **Given** a template, **When** `create_session` is called with `prompt: "hi"`, **Then** the session receives `hi`.
3. **Given** a template, **When** `create_worktree` is called with `ai_cli` and no `prompt`, **Then** the session receives the rendered template.
4. **Given** an empty template, **When** `create_session` is called without `prompt`, **Then** behaviour is unchanged (no prompt typed).
5. **Given** a template, **When** `create_worktree` is called with neither `ai_cli` nor `prompt`, **Then** no session is created, as today.
6. **Given** an issue worktree whose title and body cannot be obtained, **When** `create_session` renders the template `Fix {issue_title} (#{issue})`, **Then** the session receives `Fix  (#42)`; the lookup is abandoned after 5 s and title and body render empty.
7. **Given** a worktree for Jira issue ABC-12, **When** `{issue}` renders, **Then** it is `ABC-12`.

### Edge Cases

- Template empty or whitespace only: no initial prompt.
- Deliberate trade-off: a placeholder with no value renders empty even if the template then reads oddly (`#{issue}` without an issue yields `#`); the user sees and may edit this in the dialog, and MCP sends it as rendered.
- Issue title or body unavailable (offline, fetch failed, Jira or GitHub issue not yet loaded): the placeholder renders empty; the worktree is still created. The dialog re-renders when the details arrive, unless the user already edited the field.
- Very long issue body: inserted whole; the existing prompt-delivery path owns any length limit. The rendered text is never executed or shell-interpreted.
- Multi-line templates and bodies keep their newlines.
- Two sessions or windows: the template is read from the shared settings at render time; a change applies to worktrees created afterwards, never to an existing session.
- Remote daemons and all platforms: rendering is a pure function of the template and worktree facts, identical on every platform and daemon location.
- Concurrent MCP calls render independently; no shared mutable state.
- A worktree attached to an existing branch: `{branch}` is that branch; `{type}` is empty when none applies.

## Clarifications

### Session 2026-10-20

- Round 1: coverage scan found no critical ambiguities; storage location, tracker lookup mechanics and the dialog's edited-field tracking are plan-level. No questions asked.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Settings MUST hold one optional default-prompt template, persisted with the other settings; a settings file without it reads as empty.
- **FR-002**: Settings MUST offer a multi-line field to edit the template and list the valid placeholders.
- **FR-003**: Rendering MUST substitute `{issue}` (the tracker's identifier without `#`: the GitHub number, or the Jira key), `{issue_title}`, `{issue_body}` (plain text), `{branch}`, `{type}` and `{name}`; `{{` and `}}` MUST render as literal braces; substituted values MUST NOT be re-scanned.
- **FR-004**: A placeholder with no value for the worktree MUST render as empty.
- **FR-005**: An unknown placeholder or unbalanced brace MUST be rejected on save with a message naming it and listing the valid placeholders; the stored value MUST NOT change.
- **FR-006**: A template that is empty, or renders blank after trimming, MUST yield no initial prompt.
- **FR-007**: The create-worktree dialog MUST pre-fill the first-prompt field with the rendered template whenever a CLI is chosen and the user has not edited the field, MUST keep text the user typed, and MUST send exactly what the field holds at submit.
- **FR-008**: MCP `create_session` without a non-blank `prompt` MUST type the rendered template as the first input; with a non-blank `prompt` it MUST use that and ignore the template.
- **FR-009**: MCP `create_worktree` MUST apply FR-008 to the session it starts for `ai_cli`/`prompt`, and MUST NOT start a session solely because a template exists.
- **FR-010**: One template applies to every AI CLI and worktree type; `{type}` lets a template vary text. Per-CLI or per-type templates are out of scope.
- **FR-011**: The dialog and the MCP tools MUST render identically for the same template and facts.
- **FR-012**: Rendering MUST NOT block or fail worktree or session creation; any failure yields no prompt.
- **FR-013**: A stored template that fails validation at load MUST NOT stop the app: it renders as empty, the file keeps its text, and Settings shows the validation error.
- **FR-014**: For MCP, the facts are the worktree's recorded issue reference, branch, type and name; title and body are looked up from the issue tracker when the reference is known, and are empty when the lookup fails or the worktree has no issue.

### Key Entities

- **Prompt template**: user text with placeholders, stored in Settings.
- **Worktree facts**: issue identifier, title, body, branch, type, name — each optional.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a template set, creating an issue worktree needs no typing for the session to start with the rendered prompt (0 keystrokes after choosing issue and CLI).
- **SC-002**: Each of the six placeholders renders its value, and each unknown name is rejected with that name in the message, in the dialog and in MCP.
- **SC-003**: With an empty template, dialog and MCP behaviour is byte-identical to today's in the existing tests.
- **SC-004**: An explicit prompt is delivered unchanged in 100% of tested dialog and MCP cases.

## Assumptions

- One global template; no per-CLI or per-type variants.
- Undefined placeholders render empty rather than skipping the template (so `#{issue}` yields `#` without an issue); users who dislike this keep the template empty.
- The dialog's first prompt still requires a chosen CLI (feature 776).

## Out of Scope

- Per-CLI and per-type templates, conditional blocks, and placeholders beyond the six.
- Changing the prompt of an already-running session.
- A new MCP argument to disable the template.
