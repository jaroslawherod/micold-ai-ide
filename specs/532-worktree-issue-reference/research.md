# Research: Agent-created worktrees carry the issue reference

## D1. Fix by guidance, not by behaviour

**Decision**: change the description, the `ticket` schema text, the `initialize` instructions and
the user guide only.
**Rejected**: (a) refuse a literal `branch` without an issue tag: breaks existing callers, spec
out-of-scope and FR-004. (b) add a hint or field to the result when a literal branch has no tag:
clarified out, FR-009. (c) infer the ticket from the caller's session or branch: guesses, and the
tool cannot know the issue.

## D2. Where the rule is stated

**Decision**: three places a session reads without a human: tool `description` (read at every tool
choice), the `ticket` input description (read when filling arguments), `INSTRUCTIONS` (read once at
connect). Put the convention string in the first and third; put the consequence (tag and filter) in
the second.
**Rejected**: instructions only (callers skim instructions, the incident happened with them);
description only (not present before the first tool list).

## D3. Re-order the description

**Decision**: lead with the derived form as the way to name a worktree for an issue, keep the
literal branch as the alternative for existing branches and modes. The literal text is a new
sentence added, not a rewrite of the alternatives list, so the existing `description` assertions
(catalog and policy tests) keep holding; the implementation greps them before editing.
**Rejected**: leaving the order and appending a sentence: the incident came from the form being the
"second of two alternatives".

## D4. Pin the guidance with tests on the strings

**Decision**: assert that stable phrases (the two convention strings, "issue reference", `ticket`,
`github_issue`) appear; not whole-string equality.
**Rejected**: snapshot of the whole description: breaks on every wording change.

## D5. Parity test against `naming::derive`

**Decision**: the dialog derives names with `micold_core::naming::derive` (`worktree_form.rs`), and
the tool calls the same function (`micold-daemon/src/mcp/tools.rs`). The parity test compares the
created worktree with `derive` for a table of inputs; the client form itself is not driven (needs
iced state; `issue_naming_parity.rs` already covers the form's issue pick).
**Rejected**: driving the client form from a daemon test: crate dependency inversion.

## D6. No existing worktrees are retagged

Out of scope in the spec; recorded so the four 2026-10-02 worktrees are not touched.
