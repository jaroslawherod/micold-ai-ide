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

## T035 / T034 — the full gate, and the two verifications the tasks name

- **`mise run gate`**: `GATE_EXIT=0`, 0 failed binaries, run detached at `c572b3b5`
  (fmt → clippy core → clippy workspace `-D warnings` → `cargo test --workspace` →
  `scripts/tests/*.test.sh`, all green). Selected results from that run:

  ```
  Running tests/session_name_recovery.rs
  test recovery_fills_pending_labels_from_the_clis_own_records_and_nothing_else ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

  Running tests/untitled_session_labels.rs
  test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

  Running tests/session_name_persistence.rs
  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```

  `session_name_recovery.rs` passes **unmodified** — the recovery pass's shape, cost and precedence
  are untouched (C14–C20) — and 032's 26 label tests pass too, which is D3 confirmed by the suite
  rather than by argument.
- **No `cfg(target_os)` arm changed** (the whole source diff is inside `ClaudeProvider::parse_title`),
  so no `aarch64-apple-darwin` cross-check was required. Nothing visible changed, so no visual pass.
- **T034 `mise run site-check`**: recorded in the *Post-review* entry below, with the second gate run.

## Post-review — the second gate run, and T034's site checks

- **Review fixes** (4 accepted from reviews A and B, 5 declined — see the ledger): the user-guide
  paragraph, `BUG-002.md`'s *Impact* line, a doc comment's unbalanced quote, and recording the
  evidence above. Only one Rust line changed and it is a doc comment, so no behaviour moved and no
  test changed.
- **`mise run gate`, second run**: `GATE_EXIT=0`, 0 failed binaries, 3504 tests passed, 0 failed.
  `session_name_recovery.rs` passes unmodified in this run too.
- **T034 `mise run site-check`**: `site-check` alone refuses without a build
  (`there is no built site … run without --checks-only first`), so `mise run site-build` was run,
  which ends in the same checks. Three of the four passed:

  ```
  -- media completeness
  -- internal links
  🔍 886 Total (in 33ms) 🔗 443 Unique ✅ 793 OK 🚫 0 Errors 👻 93 Excluded
  links: 25 built page(s) -- every internal link and fragment resolves
  -- media budget
  media-budget: 25 page(s) -- every page inside 1.0 MB of stills, every clip inside 3.0 MB,
  nothing published unreferenced
  ```

  The fourth, `-- page checks`, **could not run here**:
  `Cannot find package 'playwright' imported from site/checks/page-checks.mjs`. There is no
  `site/package.json` and no local install path — `.github/workflows/pages.yml:202` installs it with
  `npx playwright install --with-deps chromium` — so that check is CI-only by construction, not
  something this change broke. `mdbook build` rendered the edited page without a warning, and the
  edit is prose inside an existing section, adding no link, image or heading anchor.
