# Autopilot ledger — #1259 markdown-italic-face

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: "start work on all tickets that are not yet in progress use speckit autopilot. Check if the other sessions don't work on them yet" — this run: issue #1259 "Markdown italic renders upright: no italic font face bundled"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1259
- **Worktree branch**: claude/project-thread-wxdcto
- **Started**: 2026-10-24
- **Phase**: M2 gate green, pushed; reviews A/B and visual-pass B5 pending (orchestrator)
- **Next step**: orchestrator dispatches review A and B (and visual-pass B5), then opens the M2 PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1343 | design (spec, plan, tasks) | merged | 56515707ebd3c2d9e078e059a5e791e16982a6c3 |
| #1377 | M1 | merged | dbe13ae2425575a7b112d3c9eceaffb6ceb58f27 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | emphasis draws in the bundled Roboto italic at the surrounding weight; bold italic as heavy as bold; code in emphasis upright; showcase shows both | #1377 | PR open |
| M2 | T019–T022 | light | `pin_roboto_italics`: a host Roboto italic or off-weight upright never replaces the bundled faces | — | implemented (T019-T021); T022 B5 pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Bundle italic faces or drop the italic mapping (issue's two options)? | Bundle italic faces; keep the mapping (FR-010) | agent-resolved | spec.md#Context; orchestrator prompt prefers option 1 |
| D2 | spec | Which italic weights? Issue suggests Italic + Medium Italic | 400, 500 and 600: one italic twin per shipped upright weight, so bold (resolves to 600) + italic keeps its weight (FR-002/FR-003) | agent-resolved | link_text.rs `painted` sets `Weight::Bold`; assets/fonts ships 400/500/600 only |
| D3 | spec | Upstream reachable from the sandbox? | Yes: raw.githubusercontent.com google/fonts `ofl/roboto/Roboto-Italic[wdth,wght].ttf` returns 200; `uv` present for fonttools | agent-resolved | curl -I 2026-10-24 |
| D4 | clarify | FR-009 sample: extend shared core fixture or showcase-only sample? | Left to the plan; spec requires only a bold-italic run | agent-resolved | spec review F2 |
| D5 | plan | FR-009 sample location | Shared `markdown-subset.md` gains `***bold italic***`; core test asserts bold+italic style | agent-resolved | research R5; showcase draws MARKDOWN_SUBSET |
| D6 | plan | How to make emphasis beat a host Roboto Italic (FR-006) | `install_roboto` loads six faces, `pin_roboto_italics` removes non-Binary non-normal Roboto faces only (upright host faces kept for FR-004) | agent-resolved | research R2; mono.rs precedent |
| D7 | plan | Code inside emphasis | `painted` sets no slant on code runs | agent-resolved | research R4; spec Edge: inline code |
| D8 | plan review 1 | F1: host Roboto Bold 700 makes upright bold heavier than bold-italic (600). Pin off-weight host faces (a) or narrow FR-003 (b)? | (a): the pin also removes non-Binary upright Roboto faces whose weight is not 400/500/600; FR-004 gains that one exception; C-2 gains weight-700-upright row and a `roboto.rs` case with a file-loaded Bold 700 | agent-resolved | every client upright query is 400/500/600 or 700 (`Weight::Bold`); fontdb picks nearest heavier for a missing 700, so a host Black 900 would also win; (b) leaves US2's defect on hosts with fonts-roboto |
| D9 | tasks | Milestone cut: US2 its own milestone (milestones rule 2)? | No: US1+US2 in M1, US3 (the pin) M2. FR-002 needs the 600 italic in M1 for headings/bold roles, which already makes bold italic 600; US2 alone would be one fixture line and one assertion, and shipping US1 without the 600 italic would put US2's lighter-bold-italic defect on main. M1's 18 tasks stay one milestone: apart from the binary fonts the diff is small, and any split leaves a deliverable without the italic faces | agent-resolved | spec US2 'Why this priority'; contract C-2 weight-700 rows |
| D10 | M1 fix (visual pass T018) | Bold (700) and bold italic drew in DejaVu, not Roboto 600: cosmic-text 0.15 takes the requested family only at the exact requested weight (`FontFallbackIter::default_font_match_key` filters `font_weight_diff == 0`), so a 700 request skipped Roboto 600 for any host 700 face. Same on `origin/main` for upright bold (upright registration unchanged). Ship a real 700 face, map bold to 600 in `painted`, or alias? | Alias: `install_roboto` pushes each bundled 600 face again as a 700 `FaceInfo` over the same bytes (`alias_bold`). One place, covers every `Weight::Bold` on `ROBOTO` (also the picker's emphasis), keeps the spec's "bold resolves to 600" (no new face, no weight change; spec Out of scope "changing which weight bold resolves to" holds). Contract C-3 updated. Resolution tests now shape through cosmic-text (`test_support::resolves_to_bundled`), red first | agent-resolved | cosmic-text-0.15.0 src/font/fallback/mod.rs:279-287, 305-340; red: "Normal ROBOTO at Bold is not drawn in the bundled Roboto-SemiBold" |
| D11 | M1 fix (visual pass T018 B6) | Italic 漢字 drew boxes: cosmic-text falls back only to faces of the requested style (`Attrs::matches`: style equal, emoji excepted) and hosts ship no italic CJK | `LinkText` splits emphasis per grapheme where Roboto's italic has no glyph and draws those pieces upright (`upright_where_italic_lacks`, `roboto::italic_has_glyph`); they then fall back as upright text does, which is the spec's Edge (coverage). Link kept per piece, so one focus stop. No synthesised slant (spec Out of scope). Contract C-4 updated | agent-resolved | cosmic-text-0.15.0 src/attrs.rs:323-327; red: `[("漢字 and 🙂", true)]` vs the split |
| D12 | M1 rebase onto #1262 | #1262 (a4f2edd2) moved `text.rs`, `link_text.rs`, `mono.rs`, `test_support.rs` into `crates/micold-material`. Where does `roboto.rs` go? | `crates/micold-material/src/roboto.rs`, beside `mono.rs` (its twin): `link_text` (now in the crate) calls `roboto::italic_has_glyph` and `text.rs` names `ROBOTO`, and the crate cannot depend on the client. It names no `micold_core` module, so `tests/no_domain_types.rs` holds. `pub mod roboto` + re-exports in the crate's `lib.rs`; the client's `ui::material` glob re-export and `ui/mod.rs` keep every `ui::ROBOTO*` path. `unicode-segmentation` moves from the client's Cargo.toml to the crate's (only `link_text` uses it); `include_bytes!` paths shortened. No behaviour change; contract, plan, tasks, research, data-model paths follow | agent-resolved | full gate (scripts/gate.sh) green at fd518b5d after the rebase onto 78c2fc22; git show a4f2edd2; 41415bd7 (1127 precedent) |
| D13 | M2 | Does the alias (D10) make the weight-700 pin unnecessary, or C-2 need adjusting? | No change: the alias is a Binary face at 700 beside a host Roboto Bold 700; fontdb breaks the tie for the first-loaded (host) face and cosmic-text matches the requested weight exactly, so a host 700 would still win bold. The pin keeps removing host non-Binary Roboto faces at unshipped weights; it runs after `alias_bold` (the alias is Binary, so it survives). C-2 as written; test seeds a host 700 | agent-resolved | `pin_drops_host_italics_and_off_weight_faces_only` green |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 6721080f0a781ff07679652e804cc693d92581c9:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CLEAN, 2 MINOR (run by orchestrator): F1 FR-005/FR-008 implementation detail vs checklist, F2 FR-009 bold-italic vs shared fixture; both applied in clarify |
| Clarify | 1 | spec.md | 3 questions, all agent-resolved (record existing rules); no new requirement, unit ends |
| Plan review | 1 | e209d8637cc99c1c00d5b3b330033eabd3b06074:7bcd080cc6c6f537fd45f68b69b97148f3485dca | CHANGES (run by orchestrator): F1 MAJOR host Roboto Bold 700 breaks FR-003/SC-002 → option (a), D8; F2 MINOR R6/R7/R8 lacked rejected alternatives → added. Both applied |
| Plan review | 2 | b530e8ce4e792f8d5efd2595d56a740898e653ed:7dc4a597b0b322dedf551f73f0d16810cf578500 | CLEAN (orchestrator, sonnet); 1 MINOR: plan.md Test strategy roboto.rs row not updated for D8 → fixed, prose only |
| Tasks review | 1 | 73cdce2dbb41cb29369c478080c14350bb320212:d20f1b50e27026933493e4e4d962f14aadbb3a33 | CLEAN (orchestrator; also covered speckit-analyze: all FR/SC covered, paths verified); 3 MINOR applied: F1 C-3 pin joins in M2 + T007 host-Roboto note, F2 D9 M1-size note, F3 Verify adds `mise run gate` |
| M1 review A + B | 1 | 9d4252ee3fb689d755244756853ea3ec0e6e73fa:e4c4ba50ea9a87862bd9f149d94fd1a3abbc9655 | A (code-review high): CHANGES — F1 MAJOR site/stage.sh OFL text names only the uprights → fixed (lists every `Roboto-*.ttf`); F2 MINOR roboto_font.rs guard ignores `ui::ROBOTO_FACES` → fixed (new test over `ROBOTO_FACES`, family from `ROBOTO_FAMILY`); F3 MINOR layout.rs dead constants, wrong doc, three locks → fixed; F4 MINOR roboto.rs host-dependent test, copied query helper → fixed (skips when the host has its own Roboto; shared `test_support::query`). B (conformance): CHANGES — F1 MAJOR no tdd/cycle-log.md → added (red not recorded in unit 1 says so; red shown by mutation in unit 2); F2 MINOR fixture README → fixed |
| M1 visual pass T018 | 1 | e4c4ba50 (worker, evidence in visual-pass/) | FAIL B1, B3 (bold italic in DejaVu Sans Mono Bold Oblique), B6 (italic 漢字 boxes); B2, B4, B7 PASS → fixed by D10, D11; re-run by the M1 unit: B1/B3 bold italic and B6 PASS (visual-pass/results.md *Re-run*) |
| M1 review A | 2 | 9596267dc307630eff98da28bee82557759a16d3:468b460804a33c78d42ce9d7f39bb150035773de | CLEAN (scoped re-review of A and B, sonnet): all round-1 findings fixed; D10 and D11 confirmed against cosmic-text 0.15.0 source. 2 MINOR declined (see below); no code change, so no new gate. Full gate (scripts/gate.sh) green at 468b4608; layout snapshot unchanged |
| Review A M2 | 1 | 9edcbec0b4cb30468e08cba30782b1764f2a0625:2983ec92f3edaae8514bd95d544336d1b38eb71a | CLEAN; F1 MINOR fixed (`remove_host_faces` shared with `pin_monospace`), F2 MINOR declined |
| Review B M2 | 1 | same | CHANGES: F1 MAJOR fixed (M2 cycle-log with red by mutation), F2 MINOR fixed (Verify names micold-material); visual pass B5 PASS |

## Declined review findings

- M2 A-F2 (rename `pin_roboto_italics`): declined; the name is the contract C-2 / plan name and the doc comment states the off-weight upright removal.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A round 2 | F1 MINOR: the italic split runs on every view build | Cost is small; revisit only if profiling shows it |
| M1 | A round 2 | F2 MINOR: whitespace the italic cmap lacks could split into upright fragments | Harmless visually |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Issue #1259 names `ui/transcript/rich.rs` for `MarkdownView`; the component lives in `crates/micold-client/src/ui/material/markdown_view.rs`, and the italic slant is set in `link_text.rs` (`painted`).
