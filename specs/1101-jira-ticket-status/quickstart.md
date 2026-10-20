# Quickstart: Jira Ticket Status

## Part A: automated

1. `mise run test-core` : parse, JQL, board, chip model.
2. `mise run gate` : client tests incl. `jira_is_requested_only_on_named_events`, `jira_token_never_leaks`.

### Test layers

| Requirement | Layer |
|---|---|
| FR-001, 007, 009, 013 | core unit |
| FR-004, 005, 008, 010 | core unit (`StatusBoard`, chip model) |
| FR-002, 003, US1-US3, SC-003 | client integration with `FakeJiraSource` |
| FR-006, 006b, SC-004 | client integration (named events, gate) |
| FR-012 | client integration (`jira_token_never_leaks`) |
| FR-011 | core unit (JQL unchanged) + existing picker tests |
| SC-002 | client integration, injected clock |
| Long name, no-chip layout | geometry gates |

## Part B: visual pass (no human; `visual-pass` skill)

Prerequisite: `cargo run -p micold-client --example fake_jira` serving tickets in To Do, In
Progress, In Review and Done; a project connected to it.

1. Open New worktree, choose Jira: rows show distinct status chips (SC-001).
2. Link worktrees to tickets; sidebar rows show chips beside the key tag.
3. Change a ticket's status in the fake, unfocus, wait >5 min (or shorten the clock in the harness),
   refocus: the chip updates (SC-002).
4. Stop the fake: chips dim; restore: they recover.
5. Overview: cards show the same chips.
6. Long status name: truncated, key and title intact.
Record screenshots under `specs/1101-jira-ticket-status/visual/`.
