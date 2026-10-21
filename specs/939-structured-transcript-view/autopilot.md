# Autopilot ledger — #939 structured-transcript-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 939 — GitHub issue #939 "Structured session view: read-only transcript (assistant text, thinking, tool calls)"; coordinator scope note: design per-block rendering behind a small seam (one renderer per block kind) that #940 can swap; #940's rendering out of scope. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #939
- **Worktree branch**: claude/project-thread-61i2k8
- **Started**: 2026-10-21
- **Phase**: milestone M3
- **Next step**: M3 PR — orchestrator opens it from the pushed branch, waits on CI and merges; then M4

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1183 | Design: spec, clarify, plan, research, contracts, tasks | merged (rebase) | cee4e71942a66b77ba2d7107e57fd6e433d38b7e |
| #1189 | M1: transcript model for finished events | merged (rebase) | 1348549cf021492216ffa03a63f1ae9faee0e03d |
| #1192 | M2: streaming, supersede, bound and re-sync | merged (rebase) | 061d8805a1f11b934d72e4c5f606e8e972738e8a |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Transcript model for finished events (entries, cards, pairing, footers, raw rows, notices, summaries, figures, sub-agent counts) | #1189 | merged |
| M2 | T016–T019 | full | Streaming, supersede, bound and re-sync; G1–G4 on every fixture | pending (orchestrator opens) | verified, gate green |
| M3 | T020–T023 | full | Window's per-session transcript state (fold, follow, TranscriptMsg, scroll tasks) beside the plain log | — | pending |
| M4 | T024–T030 | full | Shared DisclosureCard, AccordionSection slots, Text::monospace, icons, showcase | — | pending |
| M5 | T031–T037 | full | ui::transcript::view behind the renderer seam, lazy entries, SC-004 budget (showcase only) | — | pending |
| M6 | T038–T043 | full | Pane shows the transcript (single, tiled); plain log removed; user guide; Part B | — | pending |

## Decisions

- speckit-analyze (2026-10-21): 0 CRITICAL/HIGH; 1 MEDIUM + 6 LOW fixed in tasks.md (T032 green at T037, T017/T019 → US3, T039 file named, FR-001/FR-004 cited, T002 dependency noted).
- Tasks (2026-10-21): milestones follow the plan's reviewed layered delivery (core → state → components → view → pane) rather than one milestone per story, because the UI of US1/US2 cannot finish before M6 (milestones.md rule 6); M1 carries both P1 stories at the model (15 tasks, kept whole: splitting it splits one build.rs rule table across PRs with no new observable).
- M2 (2026-10-21): the bound runs after each pushed event, not once per message, so a result whose tool use was dropped earlier in the same message is an `OrphanResult` (FR-020) rather than filling a card that then leaves; `Transcript::held()` added for G4. A `content_block_start` of a block type the transcript does not stream is absorbed but keeps its position in the stream, so its finished block (a `Raw { UnknownBlock }` row) and later blocks stay matched; empty deltas change nothing (no revision).
- M3 (2026-10-21): `TranscriptMsg` rides in `session::Msg::Transcript`, but the root routes it to the transcript feature (`app::State::update_transcript`), never the session reducer (allowlisted in `feature_registration_cost.rs`); the shell (`shell/transcript.rs`) performs the returned `Scroll` as `snap_to_end` / `scroll_to`. The wire's events and the two forget paths go through `app::State::structured_events` / `forget_structured`, so the log and the transcript cannot drift. T021's binary-side half (wire routing, scroll tasks via `Task::units`, close path) is in `src/main_tests.rs::structured_transcript`, since the shell lives in the binary. `transcript_scroll_id` is in `ui/transcript.rs` (a file, not yet a directory: `material_boundary.rs` needs a new `ui/` directory registered; M5 does that when the view arrives). Fold state is pruned after a rebuild and once the bound may have dropped entries.
- Clarify (2026-10-21, 2 rounds, agent-resolved): sub-agent events (parent tool use set) are kept out of the main transcript and counted per parent tool-call card (FR-024; orphans on one notice); deltas not counted. #944 adds nesting on top. #944 treats the per-turn cost in #939's turn footer as #939's; #944 adds context meter, plan_usage feed and sub-agent nesting.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | be36b8772c7a0c2eb6acdfe0feb6de62a0ce7f56:9ad0504870e12886d671ea3f4124162315386f78 | CLEAN: 3 MINOR (F1 fixed, prose only) |
| Review plan | 1 | d0912186067579c21f5295b44c37fdcac714e0bd:dac0eb5478f3f39151b5deba2fcffc6b02a82619 | CHANGES: F1 MAJOR (no geometry-gate layer) fixed — geometry layer + DisclosureCard outline/ripple/contrast gates, header-fits and jump-control gates, layout_snapshot transcript state; F2 MINOR fixed (Shown from on_viewport_resize, R9); F3 MINOR fixed (Transcript::nested scope for #944, R7); F4 MINOR fixed (renderer-seam behavioural test) |
| Review plan | 2 | 8156502f4d59c5450a15b614c9c4f442756c771e:dad297fe3e0f1334887ccfe7eda90b93e46ac3c6 | CLEAN: 1 MINOR (F1 fixed, prose only) |
| Review tasks | 1 | cdfaee5481a594e3449ee2b0253893520d5cc0ad:76cdeb51e56a37bb1c3ee20d2dcd18ea98d4e3d8 | CHANGES: F1 MAJOR (stale checklist item: no NEEDS CLARIFICATION marker remains) fixed, prose only — ticked, note rewritten; F2 MINOR fixed (T007 [P] dropped) |
| Review A M1 | 1 | 49a92f67248f43164448d157e66563380ac30900:4f9eaad7133c0dd7c2d931414b6b606d607ae996 | CLEAN (code-review high; no findings) |
| Review B M1 | 1 | 49a92f67248f43164448d157e66563380ac30900:4f9eaad7133c0dd7c2d931414b6b606d607ae996 | CHANGES: F1 BLOCKER (reviewer sandbox refused cargo, Verify unobserved) — resolved by the unit running Verify inside the full gate's `cargo test --workspace` (see PR body); no code change. F2 MINOR (red/green batched per milestone in cycle-log) declined |
| Review A M2 | 1 | 37ffadcfa313d6388da4bdc2aa304eeb518885b6:6207562240b355ad65362d26555f33b64132b762 | CLEAN (code-review high): F1 MINOR fixed — `close` now stops every still-streaming block, one superseded by another kind included; test added |
| Review B M2 | 1 | c258083c60c69b7307e90eaecd795751949f21c6:ec670f5809ec67f239a3539be72727a08ed0b46a | CLEAN (conformance, sonnet; Verify 37 passed, supplied from the gate) |
| Review A M3 | 1 | 20804f72471f0a52c405e4fc95df918068531d3b:fa9c54a1a89302f558a8bfc52bb6b95f9fd4e8ad | CLEAN (code-review high): F1 MINOR fixed — open cards the bound dropped are pruned; F2 MINOR fixed — a toggle of a card not in the transcript opens nothing; tests added |
| Review B M3 | 1 | 20804f72471f0a52c405e4fc95df918068531d3b:fa9c54a1a89302f558a8bfc52bb6b95f9fd4e8ad | CLEAN (conformance, sonnet; Verify 17 passed, supplied by the unit): F1 MINOR declined |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | B | F2 MINOR: red/green recorded once per milestone, not per behaviour | cycle-log records the batched red run before green (part 1); MINOR, no rule violated by the milestone's tests |
| M3 | B | F1 MINOR: `app::State::update` drops the scroll `update_transcript` returns; make the arm `unreachable!` | the drop is the documented split: the root applies the message (lib tests drive it through `State::update`), only the shell can scroll and it routes the message to `shell/transcript.rs` first; `unreachable!` would panic a pure caller |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

