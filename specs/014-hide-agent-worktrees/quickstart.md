# Quickstart: Hide Agent Worktrees

Validation guide for [plan.md](./plan.md). Part 1 is the automated suite — the primary gate, and
where every decision in this feature is covered. Part 2 is the recorded manual procedure for the
two render-only pieces that the render-free test suite structurally cannot reach (Constitution
Principle I, GUI-wiring exception): the reveal chip's placement in the accordion and the agent
badge on a revealed row.

## Prerequisites

- The repo's `mise.toml` trusted once in this worktree: `mise trust`.
- A throwaway git repo to open in the app (do **not** run the fixture steps against this
  repository — they create real worktrees and branches).

## Part 1 — Automated (the gate)

```bash
mise run test          # cargo test --no-default-features --all-targets
```

Expected: green, including the new coverage.

| Test file | Covers |
|---|---|
| `tests/worktree_owner.rs` | Every row of the truth table in [contracts/agent-worktree-classification.md](./contracts/agent-worktree-classification.md), including the 16/15-character boundary and the case rules (US1, US2, FR-005, FR-006) |
| `tests/sidebar_tree.rs` | Hidden and revealed trees; agent-only project yields zero worktree entries; orphan and missing agent worktrees stay hidden; revealed rows carry `Tag::Agent`; tag filters apply to revealed rows (US1, US3, US4, FR-002/003/007/010b/010d) |
| `tests/app_state.rs` | `show_agent_worktrees` defaults to `false`; the toggle reducer flips only that field; two toggles restore the prior list; a project switch resets it to `false`; `available_tag_filters()` ignores hidden worktrees (FR-010a, FR-010d, FR-010e, R7) |

Also confirm nothing regressed in the existing worktree suite — `worktree_model`,
`worktree_discovery`, `sidebar_state`, `session_lifecycle` — since `Worktree` gains methods but no
fields, and those files should not need edits.

## Part 2 — Manual (GUI wiring)

### Fixture

In a scratch repo (`$REPO`), create a realistic mix: two user worktrees, two agent worktrees
(one of them an orphan directory git does not know), and one decoy whose name starts with the
reserved word but is not machine-generated.

```bash
cd "$REPO"
mkdir -p .claude/worktrees

# User worktrees
git worktree add .claude/worktrees/feat-101-login    -b feat/101-login
git worktree add .claude/worktrees/fix-102-timeout   -b fix/102-timeout

# Decoy — MUST stay visible (FR-006)
git worktree add .claude/worktrees/agent-foo         -b agent/foo

# Agent worktrees — MUST be hidden
git worktree add .claude/worktrees/agent-a885b42dc521fbda1 -b worktree-agent-a885b42dc521fbda1
git worktree add .claude/worktrees/agent-abf6a58b16c3c9e6f -b worktree-agent-abf6a58b16c3c9e6f

# Orphan agent directory — registered with nothing (FR-007)
mkdir -p .claude/worktrees/agent-ae474105b29fbeb68
```

Record the pre-run state so SC-005 can be checked afterwards:

```bash
git worktree list > /tmp/worktrees-before.txt
git branch --list 'worktree-agent-*' > /tmp/branches-before.txt
```

### Run

```bash
mise run run           # cargo run --features gui
```

Open `$REPO` in the app.

### Scenarios

| # | Steps | Expected | Traces |
|---|---|---|---|
| 1 | Look at the sidebar | Exactly three worktree rows: `feat-101-login`, `fix-102-timeout`, `agent-foo`. Neither `agent-a885…` nor `agent-abf6…` nor the orphan `agent-ae47…` appears | US1 #1, US2 #1, FR-002, FR-007, SC-001, SC-002 |
| 2 | Open the filter accordion (funnel icon, sidebar header) | A **"Show agent worktrees"** chip is present, outlined (off), above the tag chips | US4, FR-010, FR-010c |
| 3 | Press the chip | It fills (on); the three agent worktrees join the list, each carrying a muted `agent` chip. The three user rows are unchanged and unmoved | US4 #1, FR-010b, SC-003a |
| 4 | Press it again | The three agent rows disappear; the user rows are untouched | US4 #2 |
| 5 | Activate a tag filter (e.g. `feat`), then toggle reveal on and off | The active tag filter is never cleared by the reveal toggle, and revealed rows obey it | FR-010d |
| 6 | Quit and relaunch with the chip left **on** | The chip is off again and agent worktrees are hidden | US4 #3, FR-010a |
| 6a | With the chip **on**, switch to another project via the project switcher, then switch back | The chip is off in the other project and its agent worktrees are hidden; switching back does not restore it | US4 #4, FR-010e |
| 7 | Hover a revealed agent row (reveal on) | The normal action cluster appears — start-session and delete are available, not suppressed, and the delete confirm carries no extra agent-specific warning | FR-013 |
| 7a | Read every string this feature adds — the chip, the badge, the docs section | All say "agent"; none says "assistant" | spec Terminology |
| 8 | In a second scratch repo whose **only** worktrees are agent-owned, open it | Sidebar shows the "No worktrees yet. Add one to get started." hint — **not** "No worktrees match the filter." | US1 #2, FR-003, R7 |
| 9 | Open the filter accordion in that same agent-only repo | The reveal chip is still present, even though the tag area says "No tags to filter yet." | FR-010c, R4 |
| 10 | With the app open, `git worktree add .claude/worktrees/agent-<new-17-hex> -b worktree-agent-<same>` from a terminal, then trigger a refresh (add or delete a worktree in-app) | No new row appears | US1 #3, FR-009 |
| 11 | In a scratch repo carrying ~50 worktrees (mixed user and agent), open the project, then open and close the filter accordion | The list renders immediately; no pause attributable to classification | SC-004 |

### Post-run check (SC-005, FR-008)

Quit the app, then:

```bash
cd "$REPO"
git worktree list | diff - /tmp/worktrees-before.txt
git branch --list 'worktree-agent-*' | diff - /tmp/branches-before.txt
ls .claude/worktrees/agent-ae474105b29fbeb68
```

Expected: both diffs empty and the orphan directory still present. The app must not have pruned,
removed, renamed, or adopted anything.

### Cleanup

```bash
cd "$REPO"
git worktree remove --force .claude/worktrees/feat-101-login
git worktree remove --force .claude/worktrees/fix-102-timeout
git worktree remove --force .claude/worktrees/agent-foo
git worktree remove --force .claude/worktrees/agent-a885b42dc521fbda1
git worktree remove --force .claude/worktrees/agent-abf6a58b16c3c9e6f
rm -rf .claude/worktrees/agent-ae474105b29fbeb68
git branch -D feat/101-login fix/102-timeout agent/foo \
  worktree-agent-a885b42dc521fbda1 worktree-agent-abf6a58b16c3c9e6f
```

## Documentation check (FR-012, Principle VII)

`docs/user-guide/worktrees-and-sessions.md` must, in the same change, explain that an AI assistant
may create its own worktrees in the project, that the app hides them by default, how to reveal them
from the filter panel, and that the app never cleans them up.
