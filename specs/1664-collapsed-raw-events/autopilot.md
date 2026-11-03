# Autopilot ledger — #1664 <slug> [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1664 (issue #1664: Structured view: raw events are shown inline in the transcript; present them as collapsed summaries); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1664
- **Worktree branch**: feat/1664_structured-view-raw-events-are-shown-inline-in-the
- **Started**: 2026-11-03
- **Phase**: milestone M2 (implementing)
- **Next step**: implement M2 T009-T025, review, gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1667 | Design | merged | 64ed549a8651dba5cfa433fc833d4d3540904298 |
| #1668 | M1 collapsed summary lines (PR head branch feat/1664_collapsed-raw-events-m1; local worktree branch tracks it) | merged | 4198d80c53a5109eee89f11b0b4cfa8356ad9e36 |
| #… | M2 toggle (PR head branch feat/1664_collapsed-raw-events-m2; push with explicit refspec) | in progress | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T008 | full | Collapsed summary lines | #1668 | merged |
| M2 | T009–T025 | full | Show raw events toggle, persisted, with hidden notice (17 tasks, no shippable split) | #… (head feat/1664_collapsed-raw-events-m2) | in progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 2479fb2a15da66eb0accabf88bb3df2733cb7cd1:606c5bf85706c584f474b0ce2ff8fd50666a5ca5 | CHANGES: 2 MAJOR (fixed: facts match entry fields; error visibility raised as FR-013 marker); MINORs fixed, prose only |
| Plan | 1 | 4000e8699dd9e09f40708486875fbad22d9dbf29:a5e229e469e371ec6969d16d430bbeaf68f9c9b6 | CHANGES: 3 MAJOR (selection units, Settings sites, stale selection), fixed in plan.md and ui-contract.md |
| Plan | 2 | f10850fee75acb8a1900cc086b032d92e3ef5c23:19c23230602f2f7baa0900ae3630e275d1a4d8d3 | CLEAN (1 MINOR into_settings wording, fixed, prose only) |
| Tasks | 1 | 7f490e14daa4fd3665d83223e6898ac8e0ecb4b8:dacfed8994dc737a731872b83cf8f1a69ec11bc9 | CHANGES: 1 MAJOR ([P] tasks sharing a file), 3 MINOR; fixed, prose only; analyze F6 fixed by merging notice into M2 |
| Spec | 2 | a7d89dffe96efd9eed2b34554c01e3d784f407d0:2305bb236cec7443b911cbc0c9b526ac9dd8870a | CLEAN (2 MINOR: FR order fixed; error-footer scenario deferred to FR-013 resolution in clarify) |
| M1 A+B | 1 | 29927781b4b54b1a221637c30fcac1e03b3efb4d:64ed549a8651dba5cfa433fc833d4d3540904298 | A: CHANGES 1 MAJOR (footer details tooltip dropped; now an open-detail unit [4]), fixed; B: CLEAN; MINOR stale doc fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M2 implementation, unit 1 (context cap hit). Branch already reset to origin/main by branch-start.sh (M1 merged #1668); local branch name stays feat/1664_structured-view-raw-events-are-shown-inline-in-the, push with explicit refspec `git push -u origin HEAD:refs/heads/feat/1664_collapsed-raw-events-m2` (never force). M2 PR row is in the PR table (number to fill).

Done (uncommitted-or-WIP-committed in the worktree, `mise run check --all-targets` last green apart from the items below):
- Core: `visibility.rs` (LowSignal, Entry::low_signal, shown, Transcript::hidden_count + 6 unit tests, pass), `Settings.show_raw_events` (+mirror, Default, settings_issue_mapping key list, new roundtrip test in settings_roundtrip.rs). (T009 T010 T014 T015 done; roundtrip test not yet run.)
- Client: settings State.show_raw_events + Msg::ShowRawEventsToggled + into_settings default; persist.rs persist_show_raw_events, window_settings copy, set_changed `show_raw_events: _`, test literal; startup.rs load; main.rs routes TranscriptMsg::ShowRawEvents -> SettingsMsg + selection.visibility_changed() + persist; TranscriptMsg::ShowRawEvents; TranscriptView.show_raw + entries() filter used by rows()/entries_in; State::visibility_changed (clears all selections); app.rs transcript_view_of takes show_raw (self.settings.show_raw_events); RenderCtx.show_raw (tests/showcase set true, terminal.rs real); ui rows(transcript, show_raw), Row::Hidden, RowKey::Hidden, hidden_notice (text + Show button), header toggle raw_events_toggle (LabelledToggle Icon::Filter + Tooltip) in view_full; shown.rs hidden_text/SHOW_HIDDEN/SHOW_RAW_EVENTS(+_TIP).
- Tests added and passing: transcript_entry_view.rs (T011, T022 partly, T023), features_transcript_selection.rs (T012: hidden_entries_are_neither_rows_nor_copied, the_toggle_clears_the_selection).

Next (in order):
1. T013: add reducer tests in `crates/micold-client/src/main_tests.rs` (send Message::Session(SessionMsg::Transcript(TranscriptMsg::ShowRawEvents(true))) via the root update; assert core.settings.show_raw_events, store.saves() last has it true (FakeSettingsStore::loaded + `.with_settings(Arc)` as in main_tests ~l.9250), a fresh startup from that store starts on, two sessions' `app.core.transcript_view(s).show_raw`/rows equal in one step, failing store (`FakeSettingsStore::failing_save(io::ErrorKind::...)`) shows "Couldn't save your settings" yet value applies) and in `shell/persist.rs` tests (persist_show_raw_events writes only that key; set_changed with saved.show_raw_events != baseline leaves target as is; ValidSettings::into_settings emits false). T022: test the Show button message if feasible (else the painted "Show" text is already asserted).
2. Run: `scripts/build-lock.sh cargo test -p micold-core --test settings_roundtrip --test settings_issue_mapping`; `mise run test-core`; `cargo test -p micold-client --test main` (targeted), `--test transcript_selection_parity`, check `tests/feature_write_isolation.rs`, `tests/transcript_follow_layout.rs`, layout_snapshot fixture (header toggle changes the bar: regenerate per repo convention if a snapshot test fails).
3. T021/T025: docs/user-guide/structured-transcript.md sections (toggle off by default, errors stay visible, remembered per user; hidden-events notice; resumed sessions). Docs only text.
4. Tick T009-T025 in specs/1664-collapsed-raw-events/tasks.md; ledger Phase/Next; commit `feat(1664): ...`.
5. tasks/verify.md, then review A (code-review high) with scoped gate, review B (sonnet, conformance), visual pass (quickstart §B steps 2-5), full `mise run gate`, push (explicit refspec above), `gh pr create --base main --head feat/1664_collapsed-raw-events-m2`, title `feat(1664): ... (#1664)`, body ends `Refs #1664`. Fill the PR number in the ledger table.

## Open escalation

None. FR-013 answered by the user (error footers/orphans stay visible); applied to spec.md.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
