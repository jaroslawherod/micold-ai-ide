# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Response style: be concise

Skip greetings, preambles, and recaps. Do not open with phrases like "I'd be happy to help",
"Great question", or "Sure, I can do that". Do not close with a restatement of what you just did
unless the user asks for a summary. Answer directly: for a question, give the answer; for a task,
make the change and report only what changed and any follow-up the user needs to take. Output code
without wrapping explanation unless the user asks how or why. Prefer one line over a paragraph, and
a paragraph over a bulleted essay, when either communicates the same information.

## Keep the context small: read with Read, wait without polling

Every tool call re-reads the whole conversation, so call count and output size are what cost.

- **Read files with the Read tool, not `sed -n`, `cat`, `head`, or `tail`.** There is no Grep tool
  in this setup: find the lines with `grep -n` in Bash, then Read with `offset`/`limit`. Session
  history showed ~9,400 `sed -n 'X,Yp'` chunk reads — `main.rs` alone over 300 times — each a
  separate round-trip that Read would have collapsed.
- **Never poll with `sleep` loops or repeated `gh pr checks`.** Run the wait as one Bash call with
  `run_in_background` (an `until …; do sleep 30; done` that exits on the terminal state) or a
  Monitor, and act when its notification arrives. ~7,000 polling calls each re-read the full
  context for no new information.
- **Batch probes into one call.** `git status`, `ls`, a `grep -n` and a `gh pr view` that you would
  run back to back go in one Bash call, and independent Reads go in one message. At a typical
  ~120k context, every call saved is ~120k cache-read tokens saved.
- **One session per unit of work.** Context carried across tasks is re-read on every later call:
  one refactor session made 5,246 calls and alone accounted for 23% of all cache reads. Start a
  fresh session (or subagent) per feature phase or milestone, and hand state over in a file — a
  ledger, `tasks.md`, a commit — not in conversation.

## Put cheap work on a cheaper model, in a fresh context

Session history ran 99% of calls on Opus, subagents included. Move work down a model only where it
starts a **fresh context**: switching the model mid-conversation re-writes the whole cached context
for the new model, which costs more than it saves.

- **Explore** agents (locating code, tracing a call path): pinned to Haiku by `.claude/agents/Explore.md`.
- **Re-review rounds** (round 2+ of a spec, plan, tasks or diff review) and **conformance checks**:
  `model: "sonnet"`. First-round reviews stay on the session model.
- **Forked skills** already set `context: fork` + `model: sonnet` in their frontmatter:
  `visual-pass`, `speckit-analyze`, `speckit-bugfix-verify`, `speckit-tdd-verify`. They see only
  their arguments, so pass paths and what to check. A spec-kit upgrade regenerates the `speckit-*`
  files; re-add those three lines afterwards.
- **Stay on the session model** for writing specs and plans, debugging, architecture, and
  non-trivial Rust.

## Use `mise` tasks, not raw `cargo` commands

Prefer `mise run <task>` over invoking `cargo` directly — the tasks in `mise.toml` are the
canonical way to build, test, and run this project, so use them instead of rediscovering the
right `cargo` invocation each time. `mise tasks` lists every task with its description; the ones
whose use is not obvious from that:

- `mise run gate` — CI's merge gate locally, in CI's order: `cargo fmt --check`, clippy (core, then
  workspace, `-D warnings`), `cargo test --workspace`, then `scripts/tests/*.test.sh`. Run this, not
  `mise run test`, before pushing — CI stops at fmt before any test runs.
- `mise run test-core` — test only the render-free core (`cargo test -p micold-core
  --all-targets`); no GUI, no iced, so it is much faster for logic-only changes.
- `mise run image` — build the `:dev` sandbox image from the working tree; `mise run test-sandbox`
  then runs the real-runtime sandbox suite against it (both crates, release, one at a time). Those
  tests are off by default, so `mise run test` does not need a container runtime installed.
- `mise run app` — on Linux it stages the macOS bundle unsigned, which is what
  `scripts/tests/macos-bundle.test.sh` drives.

The first `mise run <task>` in a fresh worktree/clone requires trusting the repo's `mise.toml`
once via `mise trust` (mise refuses untrusted configs by default).

## One target directory, one build at a time

`mise run` builds share `target-shared/` beside the main checkout (`scripts/build-lock.sh
--print-target-dir`); a bare `cargo` in a worktree builds into a private `target-shared/` beside the
worktree instead. Expect to wait behind another worktree's build (`Blocking waiting for file lock`)
and leave it that way — never point `CARGO_TARGET_DIR` somewhere private. `MICOLD_NO_BUILD_LOCK=1`
skips the git-dir lock for a one-off run. When disk is short, use the `reclaim-disk` skill. Why:
`docs/development/shared-build.md`.
