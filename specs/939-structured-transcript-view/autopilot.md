# Autopilot ledger — #939 structured-transcript-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 939 — GitHub issue #939 "Structured session view: read-only transcript (assistant text, thinking, tool calls)"; coordinator scope note: design per-block rendering behind a small seam (one renderer per block kind) that #940 can swap; #940's rendering out of scope. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #939
- **Worktree branch**: claude/project-thread-61i2k8
- **Started**: 2026-10-21
- **Phase**: done
- **Next step**: handoff — the close PR (orchestrator opens it from the pushed branch); the run ends when it merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1183 | Design: spec, clarify, plan, research, contracts, tasks | merged (rebase) | cee4e71942a66b77ba2d7107e57fd6e433d38b7e |
| #1189 | M1: transcript model for finished events | merged (rebase) | 1348549cf021492216ffa03a63f1ae9faee0e03d |
| #1192 | M2: streaming, supersede, bound and re-sync | merged (rebase) | 061d8805a1f11b934d72e4c5f606e8e972738e8a |
| #1193 | M3: window's per-session transcript state | merged (rebase) | 965949081cfe2b986593b96eed026e2122f82ece |
| #1198 | M4: shared DisclosureCard, AccordionSection slots, Text::monospace, icons | merged (rebase) | dd7b2946270f8b493b08004bc3089512fb147e15 |
| #1203 | M5: transcript view behind the renderer seam, lazy keyed entries, SC-004 budget | merged (rebase) | 21cc8efa9b2e994f1c6adef594658bd4d9077630 |
| #1206 | M6: pane shows the transcript; plain log removed; user guide; Part B | merged (rebase) | b6c1dd92621146110ef078c83576956cd3c94b7a |
| (close) | Close: #944's recordings in `RECORDED`, tdd-verify, test-strength fixes, spec closed | pending (orchestrator opens) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Transcript model for finished events (entries, cards, pairing, footers, raw rows, notices, summaries, figures, sub-agent counts) | #1189 | merged |
| M2 | T016–T019 | full | Streaming, supersede, bound and re-sync; G1–G4 on every fixture | #1192 | merged |
| M3 | T020–T023 | full | Window's per-session transcript state (fold, follow, TranscriptMsg, scroll tasks) beside the plain log | #1193 | merged |
| M4 | T024–T030 | full | Shared DisclosureCard, AccordionSection slots, Text::monospace, icons, showcase | #1198 | merged |
| M5 | T031–T037 | full | ui::transcript::view behind the renderer seam, lazy entries, SC-004 budget (showcase only) | #1203 | merged |
| M6 | T038–T043 | full | Pane shows the transcript (single, tiled); plain log removed; user guide; Part B | #1206 | merged |

## Decisions

- speckit-analyze (2026-10-21): 0 CRITICAL/HIGH; 1 MEDIUM + 6 LOW fixed in tasks.md (T032 green at T037, T017/T019 → US3, T039 file named, FR-001/FR-004 cited, T002 dependency noted).
- Tasks (2026-10-21): milestones follow the plan's reviewed layered delivery (core → state → components → view → pane) rather than one milestone per story, because the UI of US1/US2 cannot finish before M6 (milestones.md rule 6); M1 carries both P1 stories at the model (15 tasks, kept whole: splitting it splits one build.rs rule table across PRs with no new observable).
- M2 (2026-10-21): the bound runs after each pushed event, not once per message, so a result whose tool use was dropped earlier in the same message is an `OrphanResult` (FR-020) rather than filling a card that then leaves; `Transcript::held()` added for G4. A `content_block_start` of a block type the transcript does not stream is absorbed but keeps its position in the stream, so its finished block (a `Raw { UnknownBlock }` row) and later blocks stay matched; empty deltas change nothing (no revision).
- M3 (2026-10-21): `TranscriptMsg` rides in `session::Msg::Transcript`, but the root routes it to the transcript feature (`app::State::update_transcript`), never the session reducer (allowlisted in `feature_registration_cost.rs`); the shell (`shell/transcript.rs`) performs the returned `Scroll` as `snap_to_end` / `scroll_to`. The wire's events and the two forget paths go through `app::State::structured_events` / `forget_structured`, so the log and the transcript cannot drift. T021's binary-side half (wire routing, scroll tasks via `Task::units`, close path) is in `src/main_tests.rs::structured_transcript`, since the shell lives in the binary. `transcript_scroll_id` is in `ui/transcript.rs` (a file, not yet a directory: `material_boundary.rs` needs a new `ui/` directory registered; M5 does that when the view arrives). Fold state is pruned after a rebuild and once the bound may have dropped entries.
- M4 (2026-10-21): `build`, `check_circle` and `error` are already `TypeChore`'s, `CardDone`'s and `SessionError`'s codepoints and `tests/icons.rs` forbids two variants on one, so `ToolCall`, `ToolDone`, `ToolFailed` take the font's alias codepoints for the same glyphs (`U+F8CD`, `U+E92D`, `U+E001`), the precedent `ActiveMarker`/`Unavailable` set; research R11 notes it. A failed `DisclosureCard` is an outlined card (error outline, no shadow) so the outline is its one edge; the failure's words and icon are the caller's trailing `IconLabel`. `AccordionSection` without slots draws its pre-939 header row unchanged (`plain`), so no layout fixture moved.
- Cross-issue, with #944 (2026-10-21): in streaming-input mode a result's `total_cost_usd` is a running total for the whole CLI session, not per turn. #939's turn footer shows it labelled as the session total, with no per-turn math; the per-turn delta belongs to #944. M4 touches no footer wording or cost text; M5 applies it (footer label, and the spec/contract/plan wording that says "per-turn cost").
- M5 (2026-10-21): the view's message type is `TranscriptMsg` (`blocks::Block = Element<'static, TranscriptMsg>`); the pane maps it in M6. `RenderCtx` carries the `scheme` beside `roles`, so the entries' lazy key holds it (contract updated). A folded card's body is not built, so its renderers do not run. The footer's cost reads `"session total $…"` (`footer_figures`, per the #944 decision; spec FR-005, research R14, data model, contract and quickstart reworded). SC-004 first failed on deltas (178 ms): at the bound the column shifted every entry a tree slot and every `lazy` rebuilt; `ui/transcript/keyed.rs` keeps slots by `EntryId` (iced's `keyed_column` matches by index at an unchanged length): open 180 ms, slowest delta 5 ms. Not R12's fallback (no windowed list, no measured heights), so no escalation. The transcript sample is a live pose of the `DisclosureCard` showcase entry, since the catalogue lists only library components.
- M6 (2026-10-21): a tiled structured primary draws its transcript in `ui/panes.rs`, and the single-pane check moved below the tiled one in `ui/terminal.rs`, so a tiled layout tiles even when the active session is structured (it showed only the log before). `SplitView` had no `operate`, so no operation reached a tiled pane: it forwards to every pane now, or the shell's `snap_to`/`scroll_to` on `transcript-<session>` would miss a tiled transcript. A transcript pane focuses on a press as an empty pane does (no terminal widget to do it). The window's `BlockCache` is one shared empty static in `ui/terminal.rs` (`window_block_cache`): #939's renderers read none, and `app::State` takes only feature structs (`root_state_is_shared.rs`), so #940, which fills it, moves it into the window's state. `containment.rs` gains `TRANSCRIPT_CONTENT`, scoped to `session-transcript-pane`'s bottom edge like `SETTINGS_PAGE_CONTENT`. Part B ran on the CPU rasteriser (no `claude` CLI, display or xdotool in the container) over recorded fixtures, so evidence is in `visual/` beside M4/M5's (tasks and quickstart said `visual-pass/`; reworded); live streaming, wheel/drag and a second window are covered by `transcript_follow_layout.rs`, `features_structured_transcript.rs` and per-window state, not by a live run.
- Clarify (2026-10-21, 2 rounds, agent-resolved): sub-agent events (parent tool use set) are kept out of the main transcript and counted per parent tool-call card (FR-024; orphans on one notice); deltas not counted. #944 adds nesting on top. #944 treats the per-turn cost in #939's turn footer as #939's; #944 adds context meter, plan_usage feed and sub-agent nesting.
- Close (2026-10-21): `speckit-converge` not run. Review B is `CLEAN` for M2–M6; M1's B was `CHANGES` only for an environment BLOCKER (the reviewer's sandbox refused cargo, so Verify went unobserved), resolved by Verify inside the full gate with no conformance finding, so B checked every milestone's tasks and scenarios against its diff.
- Close (2026-10-21): #944's four recordings joined `RECORDED`; the transcript model needed no change. They exposed two gaps in the tests, not the model: G1 did not account sub-agent events (FR-024: counted on the card, else on the one notice, now asserted as a total) or the absorbed `signature_delta` (contract table already absorbs it). G2's exact order cannot hold where an unmodelled event (`thinking_tokens`) arrives while a block streams: the streamed entry opened at its first partial event, the finished one after the raw row. G2 now compares the conversation and the raw rows each in order over all nine, and #938's five (SC-002's partial-message fixture among them) stay equal entry for entry; contract G2 says so. A nested sub-agent's messages name a tool use inside the sub-agent, so with no card for it in the main transcript they count on the notice (FR-024 as written).

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
| Review A M4 | 1 | 386c7ecc49536f04c8c3c035f105d300be0c0eba:ec28ea0bba7fe12b7af23d361571adc18b5fdc67 | CLEAN (code-review high, `claude -p --agent autopilot-reviewer`): no findings |
| Visual pass M4 | — | ec28ea0b | PASS (CPU rasteriser, the gate's renderer; no xdotool/lavapipe in the container): showcase entry light/dark/greyscale — icons distinct, no overlap, summary one line and cut, failed = outline + `Failed` + `error` icon, readable in greyscale (SC-005); evidence `visual/m4-disclosure-card-{light,dark}.png`. Note: an unmounted iced button draws `Disabled` until its first redraw, so a render must tick once first |
| Review B M4 | 1 | 27b813ac12ca641ae8609118b028ec90d09ed68d:73d4bb564058b032d2e9877bae0286d27ee20abd | CLEAN (conformance, sonnet; Verify 722 lib + 6/11/73/11/43 integration passed, supplied by the unit); full gate (`scripts/gate.sh`) green after rebase onto c61380e1 |
| Review A M5 | 1 | 7d15db943c685856b67ec516b20e1d8492e64e3f:087d4ecb6126db407c28da6262c4b0521307562b | CLEAN (code-review high, `claude -p --agent autopilot-reviewer`): F1 MINOR fixed (prose only — `view`'s doc says the lazy key holds no renderer set; #940 adds one if it swaps at run time), F2 MINOR fixed (doc rewrapped) |
| Review B M5 | 1 | 7d15db943c685856b67ec516b20e1d8492e64e3f:087d4ecb6126db407c28da6262c4b0521307562b | CLEAN (conformance, sonnet; Verify supplied from the gate and the release run): F1 MINOR fixed (scratch visual test removed before commit), F2 MINOR fixed (contract names `view`'s `ctx` argument), F3 MINOR fixed (comments rewrapped) |
| Visual pass M5 | — | 087d4ecb | PASS after one fix (CPU rasteriser, the gate's renderer): transcript of `tool-turn` open light/dark, folded + detached with "Jump to latest" floated bottom-centre, `error-result` in greyscale (Failed = label + icon + error text, SC-005). Found: tabs in a `Read` result drawn as missing glyphs — fixed by `plain_text` (tab stops). Evidence `visual/m5-transcript-{open-light-dark,failed-greyscale,jump-to-latest}.png` |
| Full gate M5 | — | bef0e450 | GATE_EXIT=0 (`scripts/gate.sh`; the first run hit a commit-signing server 503 in two `micold-daemon` github_issue tests that `git commit` in a temp repo, rerun with `commit.gpgsign=false` in the env: green). Release budget: open 180 ms, slowest delta 5 ms |
| Review A M6 | 1 | 921845aebe61fae028d8db80f70ecb5578b8f059:57c509763839e190957b2a11c85eb690219602a9 | CLEAN (code-review high, `claude -p --agent autopilot-reviewer`; src pass, then a tests pass after the first run was denied a compound read): no findings |
| Review B M6 | 1 | 921845aebe61fae028d8db80f70ecb5578b8f059:57c509763839e190957b2a11c85eb690219602a9 | CLEAN (conformance, sonnet; Verify supplied by the unit): F1 MINOR fixed, prose only (cycle log says why the single-pane red fails on `origin/main`) |
| Review A M6 | 2 | (fix diff HEAD~1..HEAD, panes.rs) | CLEAN (sonnet, scoped): the source-shape test now pins `draws_a_terminal` and its `!is_structured_primary(` decision; not counted (forced by a red gate) |
| Visual pass M6 | — | 57c50976 | PASS (CPU rasteriser, inline; no `claude`/display in the container): detached pane light/dark with "Jump to latest" bottom-centre clear of the bar, one card open; `error-result` greyscale (Failed = label + icon + error text, SC-005); mid-stream `partial-deltas`; tiled two structured sessions each with its own transcript and fold state; terminal pane beside a structured one unchanged ("Starting…"). Evidence `visual/m6-*.png` |
| Full gate M6 | — | 59281b0d | GATE_EXIT=0 (`scripts/gate.sh`, `commit.gpgsign=false` in the env); the first run (57c50976) failed `ui::panes::tests::an_empty_pane_tile_is_pressable_and_a_terminal_pane_is_not`, a source-shape test, fixed |
| TDD verify smell pass (close) | 1 | (fresh `autopilot-reviewer`, `claude -p`) | 1 HIGH (judged MED: every arm asserts, mutants caught), 5 MED, 4 LOW; F3–F9 fixed by sonnet workers (core, client), F2 declined (SC-004 is a time budget), F10–F11 LOW kept; 9 deliberate mutants caught; verdict FAIL on test-first evidence only (`tdd/verification.md`) |
| Close review | 1 | 637d0a5977ee88745b36caaec06fb5c3fa9816b7:4134d89024f2b03e41d67681912d4c5ac1034537 | CLEAN (`claude -p --agent autopilot-reviewer`): F1 MINOR fixed, prose only (T046 sentence); F2 MINOR fixed (a doc comment back above `opened_by`) |
| Full gate close | — | 14c91cf5 | GATE_EXIT=0 (`scripts/gate.sh`, `commit.gpgsign=false` in the env; 9190 passed, 0 failed) |

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

- ~~#944's recordings outside `RECORDED`~~ done in the close PR (see Decisions).
- TDD verification (close): history cannot show test-first order per behaviour (one commit per milestone), and four review/visual fixes were test-after; recorded in `tdd/verification.md`, each now mutant-checked.

