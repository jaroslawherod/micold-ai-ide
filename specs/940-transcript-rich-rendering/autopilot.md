# Autopilot ledger — #940 transcript-rich-rendering

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #940 "Structured session view: markdown, syntax-highlighted code and diffs for Edit/Write" (spec 940; depends on #939, fully merged through M6, PR #1206, b6c1dd92); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #940
- **Worktree branch**: claude/project-thread-x8n8q6
- **Started**: 2026-10-21
- **Phase**: milestone M2
- **Next step**: orchestrator opens the M2 PR (title and body in the session scratchpad); after it merges, milestone M3 (T016–T024)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1187 | design (spec, clarify, plan, tasks, milestones) | merged | a7df5dd37517543447b150ff02d6a5f195c7c392 |
| #1216 | M1 markdown model | merged | 746ef5858a0e565dfdb13924d070ff244c329f3a |
| pending | M2 edit diff | to open (orchestrator, via MCP) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | markdown model in core: settled blocks final, incremental, total (after #939 M6; T001 re-checks #939 names) | #1216 | merged (746ef585) |
| M2 | T013–T015 | full | edit diff in core: recorded edits round-trip, malformed → None | pending | reviews A/B clean, full gate green at 83a70f77; PR to open |
| M3 | T016–T024 | full | window's render-free rich cache: derive/prune, highlight jobs, checked copy, web/mail link opens | — | pending |
| M4 | T025–T034 | full | shared MarkdownView, CodeBlock, InlineDiff, LinkText, DisclosureCard::meta with gates and showcase | — | pending |
| M5 | T035–T042 | full | transcript draws with BlockRenderers::RICH; budget; user guide; Part B | — | pending |

## Decisions

- Plan (2026-10-21): planned against #939's planned names (plan.md, tasks.md, contracts/); #940 implementation starts only after #939 M6 merges, first re-checking those names (research R14). New core deps pulldown-cmark 0.13 and similar 2 (research R1, R6).
- Tasks (2026-10-21): 42 tasks, five milestones as the plan's Delivery (layered like #939's: core, cache, components, renderers last; no half-wired UI). M1 has 12 tasks (setup + re-check + fixtures + model; no split along a scenario leaves a deliverable). speckit-analyze (inline): M-5 moved to markdown_subset as plan names it; `refresh` takes an injected `Grammar { knows, token }` so the render-free cache does not depend on `ui::syntax` (data-model.md updated).
- Clarify round 1 (2026-10-21): FR-020 Edit/MultiEdit/Write cards start folded (#939 FR-010); FR-021 no tool result gets rich rendering (issue #940 scope; sub-agent output is #944). Both agent-resolved, no requirement added; unit ends CLEAN.
- M2 (2026-10-21): Claude Code 2.1.306 has no `MultiEdit` tool, so `edit-turn.jsonl` records a Write, an Edit with `replace_all` and a multi-line Edit; MultiEdit is covered by `tests/edit_diff.rs`'s synthetic set (R13 wanted it recorded; not recordable, no user decision needed). `bytes` = `input_json.len()`. A `replace_all` edit with old == new is `NoChange` (nothing changes). Adding `similar` 2.7.0 made cargo 1.97 re-pick some Windows-only `windows-sys`/`getrandom` edges in Cargo.lock; `cargo update --workspace` gave the same lock, kept as cargo resolved it.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | feee0a64dd03cc16117948539e16660f7f842da3:cee4e71942a66b77ba2d7107e57fd6e433d38b7e | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Review spec | 2 | 754f7fe793b60b5c4e0cd62f61f515be16d06b54:cee4e71942a66b77ba2d7107e57fd6e433d38b7e | CLEAN |
| Review plan | 1 | cbeaa4e165ca7c3e9f2ea7b08be207eb01604534:9a46f37fbf00a575587b6d7999a054954d903020 | CLEAN: 3 MINOR (all fixed, prose only) |
| Review tasks | 1 | df307d2ead90c7a22675e927c76fda956075f993:48dae9c37b7f429569d6b850335973fabda4a734 | CHANGES: 2 MAJOR (MarkdownView copy/large plumbing; CopyExpired timer request unnamed → `Outcome::CopyConfirmTimer`), 1 MINOR (`fills` arity) — all fixed; data-model, research R9, contracts/rich-renderers.md updated |
| Review tasks | 2 | 09be2f38d7faea105eb95db889edb95def0aba7a:48dae9c37b7f429569d6b850335973fabda4a734 | CLEAN: 1 MINOR (double blank line, fixed) |
| Review A M1 (code-review high) | 1 | c52ec378bfd53e2b9e289fa241e1faced7b8f684:4009ce61770beaba9d569a6c263284283ca295fb | 9 findings: nested definitions cut/hid (fixed), image in link lost its link (fixed), whole-text hash per delta (fixed: kept source text, memcmp), container-closed fence stayed open (fixed), empty task item paragraph (fixed), `lines` off on trailing blanks (fixed), duplicated inline prologue (fixed), eager `code_blocks` (declined), lone trailing `\r` (declined) |
| Review B M1 (conformance, sonnet) | 1 | c52ec378bfd53e2b9e289fa241e1faced7b8f684:4009ce61770beaba9d569a6c263284283ca295fb | CHANGES: 2 MAJOR (cycle log untracked → committed; fence red not shown → shown against a stub, logged), 1 MINOR (batched cycles; noted). Verify: subset 18, stream 4, lib 10 passed |
| Review A M1 (code-review, sonnet) | 2 | aa1daa152374351708fc813c0750520c06544c37:57b9034cc9b18594488478b04fa6f02cef99ef3c | CLEAN: 1 MINOR (duplicate-label definition inside a container stays hidden; not fixed) |
| Review B M1 (conformance, sonnet) | 2 | aa1daa152374351708fc813c0750520c06544c37:57b9034cc9b18594488478b04fa6f02cef99ef3c | CLEAN; Verify re-run green |
| Review A M2 (code-review high) | 1 | 91f29aa19fb9dce4c7014317aa0c19aa654c0b06:d3939ee9054b6be493d62dcb528ed054caa5b87d | CLEAN: 1 MINOR (Cargo.lock windows-sys churn; declined) |
| Review B M2 (conformance, sonnet) | 1 | 91f29aa19fb9dce4c7014317aa0c19aa654c0b06:d3939ee9054b6be493d62dcb528ed054caa5b87d | CLEAN: 2 MINOR (lock churn, declined; CRLF-only note unpinned in test, fixed: pinned `None`). Verify: edit_diff 15 passed |

## Declined review findings

- Review A M1 #8 (`code_blocks` builds a Vec, `code` scans): MINOR; ids are few per reply and the renderer reads them from M3's cache, not per frame.
- Review A M1 #9 (lone trailing `\r` read as a complete line ending): a lone `\r` is a CommonMark line ending; the `\n` that may follow only completes the same CRLF ending, so the line before it is complete either way.
- Review A/B M2 (Cargo.lock re-picks Windows-only `windows-sys`/`getrandom` edges): `cargo update -p similar --precise` cannot run before `similar` is in the lock, and `cargo update --workspace` from origin/main's lock gives the same result; hand-editing the lock is not allowed. Windows-only edges, no effect on the Linux/macOS builds.
- Scoped gate M2: `settings_refuses_save_over_failed_read` fails as root (root reads a 0o000 file); pre-existing, unrelated; `scripts/gate.sh` runs it unprivileged (`root_only`).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Markdown model edge cases outside the guarantee (research R2 *Refined in M1*): a definition whose title spans three or more lines can still absorb a paragraph a line later; link and image titles are not shown; a duplicate-label definition inside a quote or list item is not shown (review A M1 r2 MINOR).

- Possible later issue: draw Task (sub-agent) results as markdown and highlight Read results (out of #940 scope, FR-021).
