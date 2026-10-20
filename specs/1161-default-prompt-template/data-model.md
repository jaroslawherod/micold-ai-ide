# Data model: default prompt template

## 1. Stored setting
`Settings.default_prompt_template: String`. Default empty. Missing key reads as empty. Persisted verbatim (newlines kept). Mirrored in `DaemonSettings`.

## 2. PromptTemplate (core, new)
- Built only by `PromptTemplate::parse(&str) -> Result<_, TemplateError>`; holds a list of segments `Literal(String) | Placeholder(Placeholder)`.
- `Placeholder`: `Issue | IssueTitle | IssueBody | Branch | Type | Name`; `PLACEHOLDERS` lists their spellings in the order of the error message.
- `TemplateError`: `Unknown(String)` (the name as written) | `Unbalanced`. `Display` gives `Unknown placeholder {issu}; valid: {issue}, ...` (spec US3 s1).
- `render(&self, &WorktreeFacts) -> Option<String>`: `None` when the result is blank after trim (FR-006).

## 3. WorktreeFacts (core, new)
All `Option<String>`/`String`, empty means absent: `issue`, `issue_title`, `issue_body`, `branch`, `type_`, `name`. No state transitions; never persisted.

## 4. Form state (client)
`WorktreeForm.first_prompt_edited: bool` (default false). Transitions: set on `first_prompt_changed`; cleared on form open. `Issue.body: String` holds the full plain text (not persisted, no Serialize).
