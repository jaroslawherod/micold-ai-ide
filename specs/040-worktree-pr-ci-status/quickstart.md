# Quickstart: Validating "Pull Request and Check Status for Each Worktree"

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Contracts**:
[pull-request-source.md](./contracts/pull-request-source.md),
[reading-and-wire.md](./contracts/reading-and-wire.md),
[pull-request-ui.md](./contracts/pull-request-ui.md)

**§A** is the automated suite (what CI runs). **§B** is the recorded pass for the render glue in
`src/ui/` and the real `gh` that no test reaches (Principle I's GUI exception), run headless with
the repo's `visual-pass` skill. Every rule with a decision in it is in §A.

---

## Prerequisites

```bash
mise trust                      # once per fresh worktree
gh --version && gh auth status  # §B only: a signed-in GitHub CLI
```

A project for §B: a github.com repository the signed-in account can push to (a scratch repository
is enough), cloned locally, with these branches as worktrees of the project:

| Worktree branch | On GitHub |
|---|---|
| `pr-open-failing` | an open pull request with one failing check (a workflow that runs `false`) |
| `pr-open-passing` | an open, approved pull request whose checks passed |
| `pr-draft` | a draft pull request with no checks |
| `pr-merged` | a merged pull request; the local branch has no later commit |
| `pr-merged-ahead` | a merged pull request; one local commit made after the merge |
| `pr-closed` | a pull request closed without merging |
| `no-pr` | pushed, no pull request |

and, for the quiet cases, `git init /tmp/no-github && git -C /tmp/no-github commit --allow-empty -m init`.

---

## §A — Automated

```bash
mise run test-core     # micold-core: every pure rule
mise run gate          # fmt, clippy, workspace tests, script tests — what CI runs
```

| Area | Test file | Covers |
|---|---|---|
| Query and arguments | `micold-core/tests/pull_request_query.rs` | FR-023, FR-031, FR-006 |
| Answer splitting and parsing against recorded `gh --include` output | `micold-core/tests/pull_request_parse.rs` + `tests/fixtures/gh/pr_*.txt` | FR-002, SC-002, the issue's constraint |
| Which pull request is the branch's | `micold-core/tests/pull_request_select.rs` | FR-004, FR-005, story 3 scenario 6 |
| Check reduction | `micold-core/tests/pull_request_checks.rs` | FR-003, FR-008, story 1 scenarios 5–8 |
| Failure kinds and the rate-limit pause | `micold-core/tests/pull_request_failure.rs` | FR-019, FR-024, FR-025 |
| Staleness | `micold-core/tests/pull_request_stale.rs` | FR-019 |
| Chunks of 50, empty list, fake source | `micold-core/tests/pull_request_source.rs` | FR-023, Edge "very many worktrees" |
| Containment rule | `micold-core/tests/git_containment.rs` | FR-015, FR-017 |
| Setting default and round trip | `micold-core/tests/settings_roundtrip.rs` (extended) | FR-030 |
| Wire | `micold-core/tests/protocol_roundtrip.rs`, `schema_hash.rs` | protocol 21 |
| `MergedBranchCheck` on a real temp repository | `micold-daemon/tests/merged_branch_check.rs` | FR-015, FR-017, FR-018a |
| Setting persisted and broadcast | `micold-daemon/tests/pr_status_setting.rs` (new) | FR-029, FR-030 |
| Schedule reducer | `micold-client/tests/features_pr_status.rs` | story 4, FR-018 to FR-022, FR-024, FR-025, FR-027, FR-029 |
| Row projection and tooltip | `micold-client/tests/features_sidebar.rs` (extended) | FR-001, FR-007, FR-010 to FR-012, FR-015 |
| Row menu | `micold-client/tests/worktree_menu_pull_request.rs` (new) | FR-013, story 2 scenario 9 |
| Opening the address | `micold-client/tests/pr_open.rs` | FR-014 |
| Settings draft | `micold-client/tests/features_settings.rs` (extended) | FR-029 |
| Only on named events | `micold-client/tests/pr_status_is_read_only_on_named_events.rs` | FR-018, FR-026, SC-008 |
| No timer while off | `micold-client/tests/idle_subscriptions.rs` (extended) | FR-026, SC-006 |
| Never serialised, never logged | `micold-client/tests/pr_status_is_never_stored.rs` | FR-032, SC-011 |
| Component, showcase, icons | `material_builder_api.rs`, `showcase_completeness.rs`, `icons_font.rs` | FR-033 |
| Layout | `covered_states.rs` + `layout_snapshot.txt` | FR-001, FR-009, FR-011 |

File names are the plan's intent; a task may fold a small file into a neighbour, never drop a row.

---

## §B — Recorded visual pass

Run with the `visual-pass` skill on a private display. Record results and screenshots under
`specs/040-worktree-pr-ci-status/evidence/`.

| # | Steps | Expected | Covers |
|---|---|---|---|
| B1 | Component showcase → "Pull request indicator", light then dark theme. | 12 poses; each state and check status told apart by shape in a greyscale copy of the screenshot; stale poses dimmer, same shapes. | FR-009, FR-033, story 1 scenario 13 |
| B2 | Fresh settings; open the §B project. | No indicator on any row; Settings → GitHub shows the switch unchecked with its note. | FR-030, story 4 scenario 11 |
| B3 | Check the switch, apply. | Within 10 s: `pr-open-failing` open + failing; `pr-open-passing` open + passing; `pr-draft` draft, no check glyph; `pr-merged` and `pr-merged-ahead` merged; `pr-closed` closed; `no-pr` and "Default" nothing. | SC-001, story 1 scenarios 1–9, story 4 scenario 12 |
| B4 | Hover `pr-open-passing`, then `no-pr`. | First: today's lines, then `Pull request: #n title`, `PR state: open`, `Checks: passing`, `Review: approved`. Second: today's tooltip. Neither covers its row. | FR-010, FR-011, story 2 scenarios 1–4 |
| B5 | Right-click `pr-open-failing` → **Open pull request**. Right-click `no-pr`. | The browser opener is called with that pull request's address (record the address from the opener, a stub `xdg-open` on the pass's `PATH`); selection unchanged. `no-pr` has no such entry. | FR-013, FR-014, SC-009 |
| B6 | Look at `pr-merged` and `pr-merged-ahead`; hover both. | `pr-merged`: chip "can be removed" and the `Cleanup:` line. `pr-merged-ahead`: merged, no chip, no line. | FR-015, FR-017, story 3 scenarios 1, 4 |
| B7 | Right-click `pr-merged` → **Delete**; cancel. | The existing delete confirmation; nothing removed. | FR-016, SC-010 |
| B8 | Close the failing pull request on GitHub (`gh pr close`); press the sidebar's refresh. | The refresh control returns to idle with "Worktree list refreshed."; within 10 s the row shows closed. Selection and scroll unchanged. | story 4 scenarios 3, 4, 6, SC-003 |
| B9 | Make the sidebar as narrow as it goes. | Indicators keep their size; names shorten. | FR-009 |
| B10 | Uncheck the switch, apply. | Every indicator and chip gone at once. | FR-029 |
| B11 | Switch on; put a `gh` that exits 1 with "not logged in" first on `PATH` (or `GH_CONFIG_DIR` to an empty directory); refresh. Then make `gh` unfindable; refresh. | Indicators disappear; no notice, dialog or error line; worktree and session functions work. | FR-025, SC-004, story 1 scenario 10 |
| B12 | Open `/tmp/no-github` with the switch on. | No indicator, no error; `gh` was not run (a wrapper `gh` that logs its calls shows none). | FR-026, story 1 scenario 11 |
| B13 | With the switch on, disconnect the network for one reading (a wrapper `gh` that sleeps 30 s); click and type meanwhile. | The application stays responsive; rows keep their status; no error. | FR-019, FR-021, SC-005 |
| B14 | Linux: start the client from its `.desktop` launcher; repeat B3. | Same indicators as from a terminal launch. | FR-034, 034 FR-026 |
| B15 | Settings → Session service: sandbox placement; repeat B3 and B6. | Same indicators and the same removal suggestion. | Assumption "where the tooling runs", R5 |
| B16 | With the switch on and indicators shown, start a second client on the same project (it is refused: read-only, take-over banner). Then press **Take over** in it. | Before: the second window shows no indicator and the logging wrapper `gh` records no call from it. After: the second window shows the indicators after one reading; the first window's indicators are gone and it makes no further call. | SC-007, Edge "several windows" |

B1, B3, B4, B6 are repeated in the dark theme. macOS and Windows arms of B14 are 034's
`github_locate_desktop_launch.rs`, which this feature reuses unchanged: it adds no platform code
(FR-034).

After the pass, **B17**: search the pass's `settings.json`, every other file under its configuration
and data directories, and its client and daemon logs for the title and the address of each pull
request of the §B project. Expected: 0 matches (FR-032, SC-011).

**Not in §B**: the 5-minute interval, the `again` rule, the rate-limit pause and the stale form's
timing are decided by the reducer and covered in §A with an injected clock; the stale form's look
is B1.
