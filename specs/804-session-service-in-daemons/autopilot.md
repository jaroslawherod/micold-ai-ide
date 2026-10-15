# Autopilot ledger — #804 session-service-in-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Move the Session service settings (container runtime, limits) into the Daemons manager (#804); labels: enhancement, flow:feature, effort:low
- **Kind**: feature
- **Effort**: low
- **Issue**: #804
- **Worktree branch**: feat/804_move-the-session-service-settings-container
- **Started**: 2026-10-15
- **Phase**: tasks
- **Next step**: M1 after design PR merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #869 | Design | open | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Container settings in the Daemons form | #… | pending |
| M2 | T014–T021 | full | Session service section removed | #… | pending |
| M3 | T022–T023 | light | Existing settings carry over | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b13a0044e288eb4d69133c3183da0ba44d895f63:1bf62326e8f9f29b3ef9df261c3ddba110f6ae54 | CHANGES: 4 MAJOR (fixed; F2 routed to clarify) |
| Spec | 2 | 23e1dba8160d1800dfdd6d8915d4550176991274:1bf62326e8f9f29b3ef9df261c3ddba110f6ae54 | CLEAN (1 MINOR: remote-daemon marker has no FR; for clarify) |
| Plan | 1 | e8b9da12b0d78a98730b2678305343a44c3e1daa:506df1d556f5f8a14ab742264002c269ec5851fa | CHANGES: 2 MAJOR (badge source, budget error type) fixed; 3 MINOR fixed |
| Plan | 2 | 2ae1b02bed2b8ad66a62b54a31fda380948a8911:d0659308fae210beb1fdf3989b8f0b8d68845619 | CLEAN (1 MINOR: name parse_budget in plan D1, in data-model) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 (unit hit the 150k cap; not yet pushed, no PR). Branch reset to origin/main (#869 merged).
Done, committed in one WIP commit, T001-T011 ticked in tasks.md:
- core: `Settings::sync_legacy_daemon` + tests (crates/micold-core/src/settings.rs).
- client: `parse_budget`/`LimitField`/`ProfileTexts` in features/settings.rs (page uses them);
  `Draft` text mirrors + Msgs + `FormError` ids in features/daemons.rs; new
  ui/settings/container_fields.rs (`ContainerView`, `Surface::{Page,Form}`, `groups`) used by both
  the page (ui/settings/daemon.rs, wrappers `limits`/`network_warning` kept) and the Daemons form
  (ui/settings/daemons.rs; host daemon shows only the survive note, SSH none); new FieldIds
  `Daemon{ImagePath,Credential,SurviveLogout,CpuLimit,MemoryLimit,PidLimit,StorageLimit}`;
  `MachineFacts` passed to `daemons::view`; `ViewModel.legacy_default`; shell/routing.rs calls
  `sync_legacy_daemon` after `set_registry`; tests appended to tests/daemons_feature.rs;
  tests/settings_sections.rs folds container_fields.rs into daemon.rs.
- Passing so far: micold-core sync tests, daemons_feature, sandbox_limits, settings_sections,
  features_settings, client lib tests.
Next: T012 run geometry gates (field-note column, form layout; `cargo test -p micold-client`
whole, fix overflow in ui/settings/daemons.rs), T013 user guide docs/user-guide/sandboxed-daemon.md
(then tick both), `mise run gate` (fmt/clippy: check unused imports in features/settings.rs and
daemon.rs), review A (code-review high) and review B (conformance, sonnet), visual pass for the
longer form, PR per pr.md (title `feat(804): container settings in the Daemons form (#804)`,
ends `Refs #804`). Ledger: set M1 PR and previous PR #869 merge SHA 4b96bf2b526ca9b42e7dd9993d4b437016f49f18.
Note: the tdd cycle log was not written (tests and impl were written together for T001/T003).

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

- Tasks review round 1: 2 MAJOR (T018/T009 paths) fixed, 3 MINOR fixed; not re-reviewed (path/order fixes only).
