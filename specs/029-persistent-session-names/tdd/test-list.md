# Test list — 029-persistent-session-names, BUG-002 (milestone M1)

029 predates the `tdd` extension, so it has no feature-wide test list. This one covers **BUG-002's
milestone M1 only** (T031–T035), so the `tdd.run` loop that `speckit-implement`'s
`before_implement` hook drives has the list it requires. The behaviours are the ones T031 and T032
specify, and the rule they trace to is
[`contracts/session-name-persistence.md`](../contracts/session-name-persistence.md) C16.1 / C17,
under FR-005 and FR-006.

Inner-loop unit behaviours, `micold-core`. There is no outer acceptance behaviour: the fix changes
which string an existing read returns, and `crates/micold-daemon/tests/session_name_recovery.rs`
already pins both call sites' behaviour and must keep passing unchanged (T035).

| ID | Behaviour | Traces to | Task | State | Test |
|---|---|---|---|---|---|
| B1 | After a `/rename`, `ClaudeProvider::read_title` answers the `custom-title` and not the pre-rename `ai-title` `claude` keeps re-emitting beside it | C16.1, FR-005 | T031, T033 | DONE | `crates/micold-core/tests/ai_cli_provider.rs::a_rename_is_the_current_name_not_the_ai_title_it_replaced` |
| B2 | With no `custom-title`, an `agent-name` is honoured as a name record rather than ignored | C16.1 | T032, T033 | DONE | `…::an_agent_name_is_a_name_record_and_not_a_kind_to_ignore` |
| B3 | With neither kind renamed, `agent-name` and `ai-title` are resolved **by position**: an `ai-title` written after the last `agent-name` wins | C16.1, D4 | T032, T033 | DONE | `…::an_ai_title_written_after_the_last_agent_name_wins_by_position` |
| B4 | The latest `custom-title` wins however early it sits, and a later rename beats an earlier one | C16.1 | T032, T033 | DONE | `…::the_latest_custom_title_wins_however_early_it_sits` |
| B5 | An empty `customTitle`, `agentName` or `aiTitle` is not a name and falls through; a read still never errors on unparsable lines | C17 | T032, T033 | DONE | `…::an_empty_name_in_any_record_kind_is_not_a_name` |
