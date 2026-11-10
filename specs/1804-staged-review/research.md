# Research: staged review

| Decision | Chosen | Rejected |
|---|---|---|
| Facts script language | Python stdlib, beside `brief.py` | Bash + awk (BSD/GNU drift on regex and sed); a Rust crate in the workspace (drags `mise run gate` and a build into tooling) |
| Symbols | Regex over changed lines for `fn|struct|enum|trait|const|static|type|impl|mod` plus the git hunk-header function context, Rust files only | rust-analyzer/LSP (not scriptable, slow to index, not in the repo's required tools); ctags (not required, differs on macOS) |
| Callers | `git grep -nw <name>` over tracked `*.rs`, minus the definition line, capped at 20 per symbol with a count | `rg` (not required); LSP `findReferences` (interactive tool, no script access). Known gap: textual match can over-report common names; the cap and the "textual" label say so |
| Coverage | Read an lcov file (`--lcov` or `AUTOPILOT_LCOV`); a changed line is covered/uncovered by its `DA:` entry, otherwise `unknown` | Run `cargo llvm-cov` in the script (not installed per `.specify/memory/tdd-profile.md`, minutes of build, violates the one-minute goal) |
| Facts file location | `<out-dir>/review-facts-<tree>-<head>.md`, out-dir = scratchpad | A file in the worktree (pollutes the snapshot tree); `/tmp` (rule forbids) |
| Stage dispatch | The unit runs stages 2 to 4 as separate fresh subagents | A nested reviewer spawning workers (reviewer has no `Agent` tool) |
| Stage 2 and 4 agent | `autopilot-reviewer` on the session model, two separate dispatches | One agent for both (it would read the evidence already framed by its own plan: anchoring) |
| Probe agent | `autopilot-worker`, `model: "haiku"`; `"sonnet"` when a probe needs running a test | Reviewer type for probes (it is not a mechanical-task type and inherits the strong model) |
| Shadow tokens | `scripts/autopilot-tokens.py --worktree` and the subagent usage lines the harness reports | A new meter |
| Shadow record | `specs/1804-staged-review/shadow.md`, committed | Ledger columns (ledger format must not change) |
