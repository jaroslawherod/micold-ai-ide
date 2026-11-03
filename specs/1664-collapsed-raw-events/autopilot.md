# Autopilot ledger — #1664 <slug> [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1664 (issue #1664: Structured view: raw events are shown inline in the transcript; present them as collapsed summaries); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1664
- **Worktree branch**: feat/1664_structured-view-raw-events-are-shown-inline-in-the
- **Started**: 2026-11-03
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1667 | Design | merged | 64ed549a8651dba5cfa433fc833d4d3540904298 |
| #1668 | M1 collapsed summary lines (PR head branch feat/1664_collapsed-raw-events-m1; local worktree branch tracks it) | merged | 4198d80c53a5109eee89f11b0b4cfa8356ad9e36 |
| #1677 | Close (PR head branch feat/1664_collapsed-raw-events-close) | open | - |
| #1672 | M2 toggle (PR head branch feat/1664_collapsed-raw-events-m2; push with explicit refspec) | merged | 9912461a10c757d84371f39827785ca31d0c7494 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T008 | full | Collapsed summary lines | #1668 | merged |
| M2 | T009–T025 | full | Show raw events toggle, persisted, with hidden notice (17 tasks, no shippable split) | #1672 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Close tdd-verify | 1 | HEAD 9667c5f0d | FAIL: F1/F2 survived mutants (toggle press, Show button), fixed with press tests; F3 Verify commands, F5 title assertion fixed; F4/F6 low, accepted. Converge not run: M2 B was CHANGES (fixed), covered instead by tdd-verify's traceability audit |
| Spec | 1 | 2479fb2a15da66eb0accabf88bb3df2733cb7cd1:606c5bf85706c584f474b0ce2ff8fd50666a5ca5 | CHANGES: 2 MAJOR (fixed: facts match entry fields; error visibility raised as FR-013 marker); MINORs fixed, prose only |
| Plan | 1 | 4000e8699dd9e09f40708486875fbad22d9dbf29:a5e229e469e371ec6969d16d430bbeaf68f9c9b6 | CHANGES: 3 MAJOR (selection units, Settings sites, stale selection), fixed in plan.md and ui-contract.md |
| Plan | 2 | f10850fee75acb8a1900cc086b032d92e3ef5c23:19c23230602f2f7baa0900ae3630e275d1a4d8d3 | CLEAN (1 MINOR into_settings wording, fixed, prose only) |
| Tasks | 1 | 7f490e14daa4fd3665d83223e6898ac8e0ecb4b8:dacfed8994dc737a731872b83cf8f1a69ec11bc9 | CHANGES: 1 MAJOR ([P] tasks sharing a file), 3 MINOR; fixed, prose only; analyze F6 fixed by merging notice into M2 |
| Spec | 2 | a7d89dffe96efd9eed2b34554c01e3d784f407d0:2305bb236cec7443b911cbc0c9b526ac9dd8870a | CLEAN (2 MINOR: FR order fixed; error-footer scenario deferred to FR-013 resolution in clarify) |
| M2 A | 1 | a7dd47c5:daea8759 | fmt only (fixed by cargo fmt); 5 others declined (see Declined) |
| M2 B | 1 | 6d8a1de1:58cac1ec | CHANGES 2 MAJOR: startup test + hidden_text unit + docs wording fixed; Show-button message and two-session reducer tests declined (below); TDD log absent MINOR declined |
| M1 A+B | 1 | 29927781b4b54b1a221637c30fcac1e03b3efb4d:64ed549a8651dba5cfa433fc833d4d3540904298 | A: CHANGES 1 MAJOR (footer details tooltip dropped; now an open-detail unit [4]), fixed; B: CLEAN; MINOR stale doc fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | A | Subagent thread view ignores the toggle | Not in spec (FR-001..013 cover the main transcript); follow-up |
| M2 | A | Other windows keep the old value until restart | Same model as filter_open/folded_origins (loaded at startup, written per user); spec says one per user |
| M2 | A | Hidden notice not a selection unit | Designed so (T024); a notice, not content |
| M2 | A | visibility_changed only called from main.rs arm | Single writer by design (feature_write_isolation); only path |
| M2 | B | Show button emits ShowRawEvents(true) untested | One-line wiring; painted "Show" asserted; iced Element message not inspectable without a harness |
| M2 | B | Two sessions change in one step test | Both views read the one `core.settings.show_raw_events`; no per-session state exists |
| M2 | B | No tdd cycle-log | Feature does not use tdd/ artifacts |
| M2 | A | into_settings emits false placeholder | Spec task T016/T017 design; set_changed ignores key, comment documents |

## Handover

None.

## Open escalation

None. FR-013 answered by the user (error footers/orphans stay visible); applied to spec.md.

## Follow-ups not done

- Visual pass (quickstart Part B) NOT RUN (M2 and close): no Xvfb/xdotool; sway/grim exist but the client cannot open a seeded transcript and the showcase has no toggle. Recorded in quickstart.md; steps covered by tests.

- Subagent thread view (`thread_body` in ui/transcript/entries.rs) still shows raw/footer entries regardless of the toggle (review A, M2). Decision: not unbuilt behaviour (spec FR-001..013 and the issue's "transcript" cover the main view); follow-up issue candidate.
- tdd-verify F4/F6 (LOW): restart through a fresh App load and footer-updates-hidden-count tests not added.

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
