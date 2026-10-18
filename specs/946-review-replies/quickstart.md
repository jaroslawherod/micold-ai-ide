# Quickstart: Review Comments the Agent Answers (#946)

**Plan**: [plan.md](./plan.md) | Contracts: [batch-format](./contracts/batch-format.md),
[agent-tool](./contracts/agent-tool.md), [review-wire-delta](./contracts/review-wire-delta.md) |
Data model: [data-model.md](./data-model.md). Builds on 482's quickstart
(`specs/482-worktree-changes-review/quickstart.md`), whose fixture and setup it reuses.

## Prerequisites

- `mise trust` once; `git` on `PATH`.
- For §B: the `visual-pass` skill's private display and data directory, and 482's echoing fake AI
  CLI as the default AI CLI, connected to the app's tool server, so the batch can be read in the
  terminal and reports can be made with `report_review_comment` by hand (the fake CLI's MCP client
  from `crates/micold-daemon/tests/mcp_read_tools.rs`, or `curl` against the tool server endpoint
  with the session's credentials).

## §A — Automated

```sh
mise run test-core   # review::{comment,anchor,prompt,reply,store}, mcp tools catalog and policy
scripts/gate.sh      # daemon review_edit / review_send / review_reports, client features_changes, docs
```

Expected: green. Every behaviour of the plan's test strategy has a test that was seen failing
first (tdd/cycle-log.md).

## §B — Recorded pass on Linux (visual-pass)

Fixture: 482's project P and worktree `wt`, with a session running in `wt`.

1. Open `wt`'s Changes view. The review panel is open on the right, saying there are no comments
   and how to add one (US3 s5).
2. Add comments on `src/a.rs` lines 10–12, on a removed line of `src/a.rs`, and on `src/b.rs`
   line 4. The panel lists them grouped by file, by line, all Draft, counts "3 Draft" (US3 s1).
3. Edit and delete a draft from the panel; the diff card changes alike (US3 s3).
4. **Send review**. The terminal shows one message: the preamble, then one `### Comment C-…`
   block per comment, matching contracts/batch-format.md. All rows turn Sent (US2 s1, s6).
5. Report the first comment `applied` with reply "Renamed as asked" and the second `answered` with
   an explanation through the tool. Within 2 s the cards and rows show Applied / Answered and the
   replies, in a second window too (US1 s1–2, SC-005).
6. Report `answered` with an empty reply → the tool refuses; the comment stays Sent (US1 s3).
7. Mark the third comment Answered by hand with a note (US1 s6).
8. Insert 5 lines above a draft's lines in `src/b.rs` on disk; the card moves down 5 lines and is
   not outdated (US4 s1). Change one of its own lines: it shows Outdated at its last place (US4 s3).
9. Select a row whose file's kind is toggled off: the panel says why instead of jumping (US3 s2).
10. Quit and restart; statuses, replies and positions are as before (US1 s7, SC-007).

Screenshots of steps 2, 5, 8 and 9, light and dark, go in `visual-pass/`.

## §C — macOS and Windows

On each: steps 4, 5 and 10 of §B; the batch bytes equal Linux's (copy from the terminal and diff,
or run the P1 golden test there in CI, SC-003).

### Record (close, 2026-10-18)

- **CI**: the P1 golden test (SC-003) passed on ubuntu, macOS and Windows in the CI of the five
  merged pull requests (#969, #981, #986, #993, #997); each merged after at most one re-run of a
  timing flake (`ssh_real_concurrent` in #969, `history_stop_request` in #981 and #993).
- **Not run**: steps 4, 5 and 10 on a macOS or Windows machine; no such machine was available.
- **Not run**: §B itself (T034, T046, T056, T067, T068). The cloud container that ran every
  milestone has no display driver (xdotool, Xvfb). `visual-pass/m4.md` says what stands in for it.
  Run §B steps 1–10 in both schemes on a machine with a display before release.
