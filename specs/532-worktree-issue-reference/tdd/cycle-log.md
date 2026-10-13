# TDD cycle log: 532 worktree issue reference (M1)

| Behaviour | Test | Red | Green |
|---|---|---|---|
| `initialize` instructions carry the convention and the include-the-issue-reference rule (FR-007) | `mcp_jsonrpc::initialize_declares_tools_without_list_changes_and_names_the_server` | failed: `the instructions must say "include the issue reference"` | `INSTRUCTIONS` sentence added (T007); 13 passed |
| `create_worktree` description and `ticket` input state the convention (FR-001, FR-002, FR-006) | `mcp_tools_catalog::create_worktree_guidance_states_the_issue_reference_convention` | written with the jsonrpc test before any string change; the run stopped at the jsonrpc failure above, so the catalog test's own red was not printed (its phrases are absent from the old strings: `git grep 'include the issue reference' 8ef4099a~1 -- crates/micold-core/src` matches nothing, and the other convention phrases (`${type}/${ticket}_${name}`, `not inside a literal`, `issue filter`) are new text, so the test fails against the pre-change `tools.rs`; verified again at close) | tool descriptions updated (T005, T006); 41 passed |
| Ticket in any form names the worktree as the dialog does, issue tag only when not blank (FR-003, SC-001, SC-002) | `mcp_create_worktree::a_ticket_in_any_form_names_the_worktree_as_the_dialog_does` | none: pins existing behaviour, green at once | 32 passed |
| Literal-branch result has exactly the pre-feature keys (FR-009) | `mcp_create_worktree::literal_calls_are_unchanged` | none: pins existing behaviour | 32 passed |
