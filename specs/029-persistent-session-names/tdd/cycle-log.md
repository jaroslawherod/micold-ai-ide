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

## B2–B5 — the rest of C16.1: position for the other two kinds, and empty is not a name

- **Tests** (all in `crates/micold-core/tests/ai_cli_provider.rs`):
  `an_agent_name_is_a_name_record_and_not_a_kind_to_ignore` (B2),
  `an_ai_title_written_after_the_last_agent_name_wins_by_position` (B3),
  `the_latest_custom_title_wins_however_early_it_sits` (B4),
  `an_empty_name_in_any_record_kind_is_not_a_name` (B5).
- **All four passed on first run** against the B1 code
  (`scripts/build-lock.sh cargo test --test ai_cli_provider`: 22 passed, 0 failed), so the
  playbook's **deliberate-mutant check** was applied. It showed B2's fixture was worthless: it gave
  `ai-title` and `agent-name` the same string, which is what all four real transcripts do, so it
  could not fail against the standing mutant "ignore `agent-name` entirely" — which was literally
  the code at that moment. **B2's fixture was rewritten** to carry the `agent-name` *alone*, with no
  `ai-title` to fall back on. B3 (a later `ai-title` beating an `agent-name`), B4 (an early
  `custom-title` beating every later record) and B5 (an empty value falling through rather than
  masking a name) each fail against the mutant that would break them, so they stand as written.
- **Red** (B2, after the rewrite):
  `scripts/build-lock.sh cargo test --test ai_cli_provider an_agent_name_is_a_name_record_and_not_a_kind_to_ignore -- --exact`

  ```
  assertion `left == right` failed: `agent-name` is a name record like the others, not a record kind to ignore (C16.1)
    left: None
   right: Some("Token usage optimization")
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 21 filtered out
  ```
- **Green**: one match arm — `Some("agent-name") => (&mut positional, "agentName")` — folding that
  kind into the *same* slot as `ai-title`, so the two are resolved by position and neither ranks
  above the other. Suite: 1223 passed, 0 failed, EXIT=0 (baseline 1218 plus the five new tests).
  032's `the_title_and_the_label_are_read_from_the_same_file_and_never_mixed` gate and
  `the_latest_ai_title_record_wins` both still pass untouched, which is what D3 predicted.
- **Refactor**: `latest` renamed to `positional`, which is the rule it now carries rather than a
  restatement of "the last one". Re-run green in the same run as the T035 confirmation below.
- **T035 end-to-end confirmation**: a throwaway `micold-core` test (written, run, then deleted — not
  a deliverable) copied the real renamed transcript
  `~/.claude/projects/…feat-windows-package/c4cc3dca-36e3-4480-b76f-27ad4584181c.jsonl` into a temp
  `<config>/projects/<encoded cwd>/` and read it through `ClaudeProvider::read_title`:

  ```
  PROBE real read_title = Some("Windows package")
  test result: ok. 1 passed; 0 failed
  ```

  On `origin/main` the same read answered `Some("PR 284 macOS feature")` (ledger *Reproduction on
  origin/main*). The reproduction is fixed against the transcript that produced it.
