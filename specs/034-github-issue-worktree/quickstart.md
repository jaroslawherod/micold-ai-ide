# Quickstart: Validating "Create a Worktree from a GitHub Issue"

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Contracts**:
[github-issue-source.md](./contracts/github-issue-source.md),
[remote-list-rpc.md](./contracts/remote-list-rpc.md),
[issue-naming-and-typing.md](./contracts/issue-naming-and-typing.md),
[issue-picker-ui.md](./contracts/issue-picker-ui.md)

**§A** is the automated suite (what CI runs). **§B** is the recorded manual pass for render glue in
`src/ui/` that no test reaches (Principle I's GUI exception), runnable headless with the repo's
`visual-pass` skill. Every rule with a decision in it is in §A.

---

## Prerequisites

```bash
mise trust                      # once per fresh worktree
gh --version && gh auth status  # §B only: a signed-in GitHub CLI
```

A project for §B: a clone of any public github.com repository with open issues, e.g.

```bash
git clone --depth 1 https://github.com/cli/cli /tmp/issue-demo     # thousands of open issues → exercises the cap
git init /tmp/no-github && git -C /tmp/no-github commit --allow-empty -m init   # no GitHub remote
```

---

## §A — Automated

```bash
mise run test-core                                              # micold-core: all pure rules
mise run gate                                                   # fmt, clippy, workspace tests, script tests
scripts/build-lock.sh cargo test --release -p micold-core --test typeahead_budget    # SC-003 budget (CI runs it)
```

| Area | Test file | Covers |
|---|---|---|
| Remote parsing and choice | `micold-core/tests/github_remote.rs`, `git_remotes.rs` | FR-002, Edge "several remotes" |
| `RemoteList` RPC | `micold-daemon/tests/remote_list.rs` | FR-002 |
| Locating `gh` | `micold-core/tests/github_locate.rs` | FR-026, Edge "desktop launch", "tooling not installed" |
| Desktop launch on the real host (every CI OS) | `micold-core/tests/github_locate_desktop_launch.rs` | FR-026, §B10 macOS and Windows arms |
| Paging, cap, completeness | `micold-core/tests/github_load.rs` | FR-004, FR-005a (completeness) |
| JSON parsing | `micold-core/tests/github_parse.rs` | FR-004 (fields, PR exclusion by type) |
| Failure classification + messages | `micold-core/tests/github_classify.rs` + `tests/fixtures/gh/` | FR-007, FR-022, Edge "rate-limited", "not signed in", "public repo" |
| Bounded run | `micold-core/tests/process_run_bounded.rs` | FR-007 (10 s), SC-004 |
| Search merge + matching | `micold-core/tests/github_search.rs` | FR-005, FR-005a |
| Rank budget (release) | `micold-core/tests/typeahead_budget.rs` (new 1,000-issue case) | SC-003 |
| Name from title | `micold-core/tests/naming_from_title.rs` | FR-010, Edge "slugs to nothing", "very long title" |
| Label → type | `micold-core/tests/issue_types.rs` | FR-013, FR-014, FR-017, FR-019, FR-021 |
| Settings persistence | `micold-core/tests/settings_issue_mapping.rs` | FR-016, FR-020, FR-021, SC-006 |
| Form reducer | `micold-client/tests/issue_source_state.rs` | US1 AS2–AS10, US2 AS1–AS6, FR-003, FR-006, FR-007a, FR-008, FR-010a, FR-014a, FR-023, Edges "switch mid-load", "form closed", "switch back", "retry" |
| Settings reducer | `micold-client/tests/features_settings.rs` (extended) | US3 AS1–AS6, FR-018, FR-019 |
| Only on named events | `micold-client/tests/issues_are_requested_only_on_named_events.rs` | FR-003, FR-024 |
| Capabilities gate | `micold-client/tests/no_concrete_implementations.rs` (existing) | `GhCli` named only in `Capabilities::real()` |
| Offline creation unaffected | existing worktree-form and create tests stay green | FR-024, SC-004 |

---

## §B — Recorded manual pass

Run the client against `/tmp/issue-demo` (and `/tmp/no-github` where stated). Record the result and
screenshots under `specs/034-github-issue-worktree/evidence/`.

| # | Steps | Expected | Covers |
|---|---|---|---|
| B1 | Open `/tmp/no-github`; open New worktree. | Three chips; **GitHub issue** disabled; caption "This repository has no GitHub remote." | FR-001, FR-002 |
| B2 | Open `/tmp/issue-demo`; New worktree; read the caption under the source switch, then choose **GitHub issue**. | Before choosing, the caption reads "GitHub issue reads open issues of cli/cli from GitHub." and no request has been made. After choosing, the body's notice names `cli/cli`; "Loading issues from GitHub…" then a list, newest-updated first, rows `#n title · labels`; the Type/Ticket/Name fields stay below. | AS1, AS2, AS7 (US2), AS9, FR-006 |
| B3 | Type part of a title, then a label name, then a number. Use ↑/↓ and Enter. | List narrows per keystroke with emphasis; Enter picks without leaving the field. | AS3, FR-005 |
| B4 | Pick an issue labelled `bug`. | Ticket = number, Name = title (shortened if long), Type = `fix`; preview shows `fix-<n>_<slug>`. | AS4, US2 AS1 |
| B5 | Edit the name, create. | Worktree created with the edited name; a second attempt on the same issue shows the existing reuse/overwrite prompt. | AS5, AS6 |
| B6 | Type the number of an issue beyond the 1,000 loaded (take one from `gh issue list -R cli/cli --state open --search "sort:updated-asc" -L 1`). | "Showing the 1,000 most recently updated of N…"; "Searching GitHub…"; the issue appears and is pickable. | AS10, FR-004, FR-005a |
| B7 | Disconnect the network; choose GitHub issue (or Retry). | Within 10 s: "Couldn't reach GitHub…" + Retry; **New branch** still creates a worktree. | FR-007, FR-024, SC-004 |
| B8 | `gh auth logout`, retry; then `gh auth login` again. | "you're not signed in to GitHub…" + Retry. | FR-022, Edge "not signed in" |
| B9 | Temporarily rename `gh` off every search dir; retry. | "the GitHub CLI (`gh`) isn't installed…". | Edge "tooling not installed" |
| B10 | Linux: launch the app from its `.desktop` launcher, repeat B2. macOS and Windows: the CI test `github_locate_desktop_launch` (below). | Same list as from a terminal launch. | FR-026, Edge "desktop launch" |
| B11 | Switch Settings → Session service to the sandbox placement, repeat B2. | Same list. | FR-026, Edge "session sandbox" |
| B12 | Settings → GitHub issues: add `question → chore`, move it to the top, save; pick an issue labelled `question` in any open project. Restart; reopen Settings. | Type `chore`; entry persisted. Blank or duplicate label → save refused with the entry marked. Restore defaults → three default entries. | US3 AS1–AS6, SC-005 |
| B13 | Both colour schemes: the disabled chip, the loading line, the error + Retry, the issue rows, the Settings rows. | Legible, themed, nothing clipped at 520 px. | Principle VIII |

B10 must hold on each of Linux, macOS and Windows before the feature closes (Principle VI). The
Linux arm is recorded by hand, under the `visual-pass` skill. The macOS and Windows arms are
`crates/micold-core/tests/github_locate_desktop_launch.rs`, which the CI `build + test` matrix runs
on all three OSes: it resolves `gh` over the runner's real files with the `PATH` a Dock/Finder or
Start-menu launch hands the app and no environment-include `PATH`, and runs the `gh` it finds. It
skips only where `gh` is installed nowhere the lookup knows, and fails instead of skipping on CI.
