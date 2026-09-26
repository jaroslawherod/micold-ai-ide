# Cycle log — 029-persistent-session-names, BUG-002 (milestone M1)

Append only. One entry per red-green-refactor cycle, with the red evidence verbatim.

Commands are the `.specify/memory/tdd-profile.md` rust stack's, run detached to the session
scratchpad because the shared build lock can queue a run behind another worktree's build. The suite
command used per cycle is the profile's `fast_subset`
(`scripts/build-lock.sh cargo test -p micold-core --all-targets`), not `suite`: the change is
confined to `micold-core`, and the full-workspace suite runs once at the end as `mise run gate`
(T035). **Baseline** before the first cycle: 1218 passed, 0 failed, EXIT=0.

## B1 — a `/rename` is the current name, not the `ai-title` it replaced

- **Test**: `crates/micold-core/tests/ai_cli_provider.rs::a_rename_is_the_current_name_not_the_ai_title_it_replaced`,
  with the `renamed_turn()` fixture in the record order the real transcript writes
  (`custom-title`, the pre-rename `ai-title`, `agent-name`), repeated over two turns.
- **Red**: `scripts/build-lock.sh cargo test --test ai_cli_provider a_rename_is_the_current_name_not_the_ai_title_it_replaced -- --exact`

  ```
  assertion `left == right` failed: the user's `/rename` is the conversation's current name; the pre-rename `ai-title` beside it is stale by construction and must not come back (C16.1)
    left: Some("PR 284 macOS feature")
   right: Some("Windows package")
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 17 filtered out
  ```

  This is the failure BUG-002 and T031 predicted, value for value.
- **Green**: `ClaudeProvider::parse_title` now tracks the latest non-empty `custom-title` in a second
  slot and answers `custom.or(latest)`. Suite: 1219 passed, 0 failed, EXIT=0.
- **Refactor**: the two per-kind branches were collapsed into one `(slot, field)` match rather than
  two duplicated `if` arms, so adding `agent-name` in the next cycle is one line. Re-run green.
- **Notes**: `agent-name` is deliberately not handled yet — that is B2/B3.
