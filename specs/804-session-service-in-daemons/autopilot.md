# Autopilot ledger — #804 session-service-in-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Move the Session service settings (container runtime, limits) into the Daemons manager (#804); labels: enhancement, flow:feature, effort:low
- **Kind**: feature
- **Effort**: low
- **Issue**: #804
- **Worktree branch**: feat/804_move-the-session-service-settings-container
- **Started**: 2026-10-15
- **Phase**: milestones (M4 spec amendment PR open; M4 pending)
- **Next step**: merge the M4 amendment PR, then milestone M4 (T025–T032)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #869 | Design | merged | 4b96bf2b526ca9b42e7dd9993d4b437016f49f18 |
| #882 | M1 | merged | ebf69d4be76163577384bf9fc921ff66cb8eb957 |
| #889 | M2 | merged | bf0910213b7555c49ad0524ab880ddee40069806 |
| #900 | M3 | merged | 030ddb2ab98c254a7c760f637e82a30f669634cb |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Container settings in the Daemons form | #882 | merged |
| M2 | T014–T021 | full | Session service section removed | #889 | merged |
| M3 | T022–T023 | light | Existing settings carry over | #900 | merged |
| M4 | T025–T032 | full | Collapsible container sections (accordion, #890) | | pending |

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
| Review A M2 | 1 | 6b63e4a8c23f | CLEAN |
| Review B M2 | 1 | ed0dedca24e9:6b63e4a8c23f | CLEAN (1 MINOR: fmt uncommitted, committed) |
| Review B M1 | 1 | ef3356982b2d:e016b011a15f | CHANGES: 1 MAJOR declined (no red evidence), 1 MINOR fixed |
| Review B M3 | 1 | 280cc5ee2819 | CLEAN (1 MINOR: guide said "image", not tested; dropped the word, prose only) |
| Spec amendment M4 | 1 | 330332e3287d:030ddb2ab98c | CLEAN (3 MINOR fixed, prose/task names) |

## Declined review findings

- Review A M1 (code-review high), fixed: form save now refreshes `sandbox_boot` (the in-memory legacy block is page state, left to M2); removal no longer re-targets the legacy block; `ViewModel.legacy_default` follows `sandbox_daemon`. Declined: capabilities not re-probed when the draft's runtime changes (same as the settings page, a probe needs a restart); retained archive path refused after switching image source (spec'd UnusedArchivePath); restart prompt on any profile change (FR-004 restart-on-save); Draft/ContainerView duplication and `groups()` page bool (taste, MINOR); stale open settings_draft baseline (page removed in M2).

- Review B M1 F1 (MAJOR, no red-before-green evidence for T005-T007): cannot be produced after the fact; git history is one WIP commit. Recorded honestly in tdd/cycle-log.md. F2 fixed (T010 wording). F3 covered by the cycle log.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

- 804 M2: `SettingsDraft.daemon` (DaemonDraft mirror) and its `validate()`/`budget()` path plus `FieldId::Settings{Cpu,Memory,Pid,Storage}Limit` remain as dead legacy page state (no page edits them); remove with the legacy block in a later cleanup. The toolbar item "Session service diagnostics" is kept (it names the service, not the settings section).

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

- Tasks review round 1: 2 MAJOR (T018/T009 paths) fixed, 3 MINOR fixed; not re-reviewed (path/order fixes only).
