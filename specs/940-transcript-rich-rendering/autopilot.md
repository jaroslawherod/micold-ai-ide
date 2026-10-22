# Autopilot ledger — #940 transcript-rich-rendering

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #940 "Structured session view: markdown, syntax-highlighted code and diffs for Edit/Write" (spec 940; depends on #939, fully merged through M6, PR #1206, b6c1dd92); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #940
- **Worktree branch**: claude/project-thread-x8n8q6
- **Started**: 2026-10-21
- **Phase**: milestone M5
- **Next step**: M5 PR #1234: CI attempt 2 after the red-CI fix (pinned test monospace face), then merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1187 | design (spec, clarify, plan, tasks, milestones) | merged | a7df5dd37517543447b150ff02d6a5f195c7c392 |
| #1216 | M1 markdown model | merged | 746ef5858a0e565dfdb13924d070ff244c329f3a |
| #1220 | M2 edit diff (+ orchestrator commit 5bcb6f83: Windows "Report rust-cache hit" CI step failed on every cache miss — find/du on a missing target/ under pipefail) | merged | 528209c8d15fd0e5f109ae49e0202fdaba01b842 |
| #1229 | M3 rich cache | merged | 7ce164cfa2e5323ab82807db03e80e35bacd9af2 |
| #1232 | M4 shared components | merged | 1a6250835d61c5cf2c800e8ce131f35051f63215 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | markdown model in core: settled blocks final, incremental, total (after #939 M6; T001 re-checks #939 names) | #1216 | merged (746ef585) |
| M2 | T013–T015 | full | edit diff in core: recorded edits round-trip, malformed → None | #1220 | merged (528209c8) |
| M3 | T016–T024 | full | window's render-free rich cache: derive/prune, highlight jobs, checked copy, web/mail link opens | #1229 | merged (7ce164cf) |
| M4 | T025–T034 | full | shared MarkdownView, CodeBlock, InlineDiff, LinkText, DisclosureCard::meta with gates and showcase | #1232 | merged (1a625083) |
| M5 | T035–T042 | full | transcript draws with BlockRenderers::RICH; budget; user guide; Part B | #1234 (draft) | CI attempt 1 red (layout_snapshot, host monospace); fixed test-only, full gate green |

## Decisions

- Plan (2026-10-21): planned against #939's planned names (plan.md, tasks.md, contracts/); #940 implementation starts only after #939 M6 merges, first re-checking those names (research R14). New core deps pulldown-cmark 0.13 and similar 2 (research R1, R6).
- Tasks (2026-10-21): 42 tasks, five milestones as the plan's Delivery (layered like #939's: core, cache, components, renderers last; no half-wired UI). M1 has 12 tasks (setup + re-check + fixtures + model; no split along a scenario leaves a deliverable). speckit-analyze (inline): M-5 moved to markdown_subset as plan names it; `refresh` takes an injected `Grammar { knows, token }` so the render-free cache does not depend on `ui::syntax` (data-model.md updated).
- Clarify round 1 (2026-10-21): FR-020 Edit/MultiEdit/Write cards start folded (#939 FR-010); FR-021 no tool result gets rich rendering (issue #940 scope; sub-agent output is #944). Both agent-resolved, no requirement added; unit ends CLEAN.
- M2 (2026-10-21): Claude Code 2.1.306 has no `MultiEdit` tool, so `edit-turn.jsonl` records a Write, an Edit with `replace_all` and a multi-line Edit; MultiEdit is covered by `tests/edit_diff.rs`'s synthetic set (R13 wanted it recorded; not recordable, no user decision needed). `bytes` = `input_json.len()`. A `replace_all` edit with old == new is `NoChange` (nothing changes). Adding `similar` 2.7.0 made cargo 1.97 re-pick some Windows-only `windows-sys`/`getrandom` edges in Cargo.lock; `cargo update --workspace` gave the same lock, kept as cargo resolved it.
- M3 (2026-10-21): `ui::syntax::knows` reads two-face's grammar set (the one iced_highlighter builds with; iced keeps its set private) as a new direct dependency of micold-client at iced_highlighter's version and features, so no crate is added to the build (Cargo.lock: one dependency edge). Rich messages route through `structured_transcript::State::update(&mut RichCache, msg) -> Effect { scroll, outcomes, jobs }` (allowlisted in feature_registration_cost); `App::structured_events` takes the `Grammar` and returns the jobs. `highlighted(session, key, part, spans)` takes the message's fields (the message carries no lines); it stores only for an issued, still-present part whose line count matches. A part's spans sit in `Spans`' `new` side, both schemes. The view now reads the window's `transcript_rich` as its `BlockCache` (still drawn with `PLAIN`).
- M4 (2026-10-21): the four components own what they draw (`From<…> for Element<'b, M>` with `M: 'static`), so `BlockRenderers::RICH` (M5) can return them as its `'static` blocks; `MarkdownView` gains `.stamp(u64)` (the entry's `Derived.stamp`), and a settled block's lazy key also hashes the block itself (`Hash` derived on core's markdown types), so a document parsed afresh never shows an old block at the same index. One rule for the size limit, `review::limits::exceeds(lines, bytes)`, now shared by the cache and `MarkdownView`. `Ticket::posed(n)` makes copy states for the gallery and tests. Task-list marks use two new icons, `TaskDone`/`TaskOpen` (`check_box`, `check_box_outline_blank`). Inside a horizontal-only viewport no row can fill a width (iced compresses it to nothing), so `InlineDiff` pads each hunk's monospace lines to the widest one so their tints end together. `LinkText` focus: one `TakesTheKeyboard` per link around the one paragraph, so the ring is drawn round the paragraph, not the link (iced's `rich_text` exposes no span bounds). "Edit n of m" is shown only when a call holds several edits; "No newline at end" only when the old and new text differ in it, as a row marked on the side that lacks it.
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
| Review A M3 (code-review high) | 1 | 44fb4d110ddc2e383c9fc7dacf9676ecb10dd479:59cb951d0d56f3b805f85c3d82ad77e3a993a0d0 | CLEAN: 3 MINOR — settle cloned+hashed every settled block per delta (fixed: hashed in place); unbounded parallel highlight tasks (fixed: one blocking task per batch, jobs in turn); stale late spans (declined) |
| Review B M3 (conformance, sonnet) | 1 | 151e2ec884c299f9a0f025d2ccf1a268dcb973d0:a7ce3e85dce237449f36f8f13e3e2af35a07526d | CLEAN: 1 MINOR (T019/T020 test-after; T020 red shown afterwards and logged, T019 left disclosed). Visual pass: not applicable (still `PLAIN`) |
| Review A M4 (code-review high) | 1 | 2df6462e12a4d095de132ab6078cddffe1de467a:9478b70f8fb881a801b067a833257004120e7349 | 10 findings: a link split across styles gave one stop per span (fixed: consecutive spans of one link collapse); fixed 28 dp list gutter overflowed by long numbers (fixed: `number_gutter`); `Ticket::posed` public (fixed: `#[doc(hidden)]`); paragraph-wide focus ring (declined, follow-up); per-view clones, inert-link colour, pad counts chars, `ink` duplication, marker gate poses (MINOR, declined) |
| Review A M4 (code-review, sonnet) | 2 | 20dc1895118dfc8c4f13d0aaa25f5a488aacca72:9478b70f8fb881a801b067a833257004120e7349 | CLEAN |
| Review B M4 (conformance, sonnet) | 1 | 20dc1895118dfc8c4f13d0aaa25f5a488aacca72:9478b70f8fb881a801b067a833257004120e7349 | CLEAN: 1 MINOR (CodeBlock/LinkText test-after and T026 never red; disclosed in the cycle log, accepted) |
| Visual pass M4 (showcase, tiny-skia on Xvfb) | 1 | — | 1 finding fixed: InlineDiff's "No newline at end" was drawn as a monospace `+` line, indistinguishable from a file line (now a marker plus a caption); MarkdownView, CodeBlock (idle/copied/failed/large), LinkText, the Edit card's `+12 −3` meta and InlineDiff in light and dark otherwise as specified; italic is drawn upright (no italic face bundled: follow-up). Evidence: `visual-pass/m4-*.png` |
| Review A M5 (code-review high) | 1 | 2af3c1c3b4a551349a6a7230788a8079790e7751:b3b61e65db682b853a82c0692ec312f20044bdf9 | CLEAN: 1 MAJOR not real (T042 unfinished: it was in progress, done in this unit), 6 MINOR — header counts ignored the revision (fixed), two `EDIT_TOOLS` (fixed), empty block drew `Space` on a miss (fixed), 130-char guide line (fixed), fallback tests test-after (red shown by mutation, logged), rich budget without spans (declined) |
| Visual pass M5 (real window headless, tiny-skia, recorded fixtures) | 1 | — | No findings: Part B steps 1, 2, 5, 7 pass; 3, 4, 8 not driven (no input/no second window), 6 thinking empty in fixture. Card titles faint = iced first-frame `Status::Disabled` (headless artefact). Evidence: `visual-pass/m5-record.md`, `m5-*.png` |
| Review B M5 (conformance, sonnet) | 1 | 73eb296a5316cb2a2ab7913435cc3a31b23e46e1:7d5a14c06382d17351b077801690349100c73c11 | CHANGES: 1 MAJOR (T042 ticked with steps 3, 4, 6, 8 not seen; fixed: thinking rendered from a synthetic event, two-window same-document assertion added, record names the tests for the undriven steps), 2 MINOR (budget has no red: disclosed; table gate red not shown: disclosed). Verify: layout_snapshot 79/79, seam 2/2, rich renderers 7/7 |
| Review B M5 (conformance, sonnet) | 2 | 1af36e37ec1e7d684293330b8e3a31e3d25ca17c:adfb12ea8c69558c9747585bec8f452610472ed5 | CLEAN: 1 MINOR (steps 3, 4, 8 and the sideways scroll still not driven on screen; recorded with their tests). Verify: features_transcript_rich 16, transcript_rich_renderers 7 passed |
| Red CI M5 attempt 1 (ubuntu) | 1 | — | `layout_snapshot` `session-transcript-rich`: code lines 0.3–1.1px wider on CI. Root cause: the app ships no monospace face, so `Font::MONOSPACE` resolved on the host (Liberation Mono 0.600 em locally, a 0.602 em face on CI, DejaVu Sans Mono by its widths; macOS/Windows passed because they resolved Courier New, metric-compatible with Liberation Mono at 1229/2048 em); no earlier covered state drew code. Fix, test-only: `tests/support/layout.rs` loads a committed Fira Mono (`tests/fixtures/FiraMono-Regular.ttf`, OFL, provenance in `FONT-PROVENANCE.md`) as the monospace family; guard `the_monospace_face_parses_and_is_the_face_that_measures` (red with the pin removed); fixture regenerated (2 lines, 343.3→343.2). No non-test code, so no review A |

## Declined review findings

- Review A M3 F2 (a late answer of an older job for a part whose content changed under the same `CodeId` can overwrite the newer spans): MINOR; same-key content changes only on a rebuild from a different history, the line count must still match, and the job carries no fingerprint by the data model (`HighlightJob { session, key, part, token, lines }`, `Highlighted { session, key, part, spans }`).
- Review A M1 #8 (`code_blocks` builds a Vec, `code` scans): MINOR; ids are few per reply and the renderer reads them from M3's cache, not per frame.
- Review A M1 #9 (lone trailing `\r` read as a complete line ending): a lone `\r` is a CommonMark line ending; the `\n` that may follow only completes the same CRLF ending, so the line before it is complete either way.
- Review A/B M2 (Cargo.lock re-picks Windows-only `windows-sys`/`getrandom` edges): `cargo update -p similar --precise` cannot run before `similar` is in the lock, and `cargo update --workspace` from origin/main's lock gives the same result; hand-editing the lock is not allowed. Windows-only edges, no effect on the Linux/macOS builds.
- Review A M4 F2 (LinkText's focus ring wraps the paragraph, so with several links the focused one is not identifiable): a per-link ring needs a custom text widget drawing from `Paragraph::span_bounds` (iced `rich_text` exposes none); out of M4's tasks, recorded under *Follow-ups not done*.
- Review A M4 F3/F7 (`MarkdownView::view` and the `spans` builders clone spans and settled blocks per view): MINOR; M5's budget task measures the transcript's render cost and owns any `Rc` change.
- Review A M4 F4 (links drawn in `primary` with no `on_link`): MINOR; the transcript always sets `on_link`, and the showcase's inert entry says so.
- Review A M4 F6 (the tint pad counts chars, not display cells): MINOR, part of the recorded tint limit (Decisions M4).
- Review A M4 F9/F10 (`ink` duplicates `Text`'s muted colour; the marker gate poses no missing-final-newline edit): MINOR; the row is unit-tested in `inline_diff.rs`.
- Scoped gate M2: `settings_refuses_save_over_failed_read` fails as root (root reads a 0o000 file); pre-existing, unrelated; `scripts/gate.sh` runs it unprivileged (`root_only`).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M5 | A r1 | F4 rich budget measured with no highlight spans applied | MINOR: spans split each line into coloured runs of the same text; the measured open is 118 ms against the 1 s budget, so the margin is wide. Follow-up recorded. |
| M5 | B r1 | F3 table gate red not shown | MINOR, disclosed in the cycle log. |
| M5 | A r1 | F1 T042 unfinished | Not real at the reviewed tree's next step: T042 was the unit's next task, now done (`visual-pass/m5-record.md`). |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Markdown model edge cases outside the guarantee (research R2 *Refined in M1*): a definition whose title spans three or more lines can still absorb a paragraph a line later; link and image titles are not shown; a duplicate-label definition inside a quote or list item is not shown (review A M1 r2 MINOR).

- Italic emphasis is drawn upright: the bundled fonts are Roboto Regular and Medium only, so `Style::Italic` finds no face (visual pass M4). Needs a Roboto Italic asset with its provenance entry (FR-003); not in M4's tasks.

- LinkText per-link focus ring (review A M4 F2): draw the ring round the focused link's spans (`Paragraph::span_bounds`) instead of the paragraph; fix before #940 closes or file an issue.

- Rich budget with highlight spans applied (review A M5 F4, declined as MINOR): feed the jobs back in `transcript_open_budget.rs` before the timed frames.

- Possible later issue: draw Task (sub-agent) results as markdown and highlight Read results (out of #940 scope, FR-021).
