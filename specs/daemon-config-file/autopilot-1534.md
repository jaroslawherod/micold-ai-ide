# Autopilot ledger — #1534 daemon-config-file (Issue 13)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1534` (Projects and clone root from the configuration); labels: `flow:feature`, `effort:low`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524/#1525; the tasks unit only cuts milestones for the "Issue 13" group (T078–T080).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1534
- **Worktree branch**: feat/1534_projects-and-clone-root-from-the-configuration
- **Started**: 2026-11-01
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1628 | design: Issue 13 milestone | merged | 6c9d0d533e6ce0b4cbf6d62dec7a7706b9ac9d9c |
| #1638 | M1 | merged | 1f0f2a3190c1e30f1c722ba05fb9081d0baae516 |
| close | close the spec | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T078–T080, T078a | full | `[[projects]]` served at start; absent `path` cloned under `clone_root`; runtime projects in next rendered file | #1638 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec for #1534? | No: reuse `specs/daemon-config-file/` (FR-014, contract #projects, tasks T078–T080 exist); per-issue ledger as for #1524/#1525 | agent-resolved | specs/daemon-config-file/autopilot-1525.md |
| D2 | milestone | T080a (feed runtime projects into the next rendered file)? | Deferred: moved in tasks.md to just after T027 (Issue 5, which writes the daemon file); criterion 2 met at rendering level via `add_runtime_project` + golden | agent-resolved | specs/daemon-config-file/tasks.md |
| D3 | gate | Wait on the build lock for the full gate? | No: push now, CI validates (no `autopilot-gate-ok`); fmt checked locally; red CI goes to red-ci | user | escalation answer |

## Review rounds

- Tasks review round 1 (snapshot 54ec58b70e187d69795d7c6298d68365828afb6c:02ee6e4c8fbf0dc3f7f5a2f12747c0ce7baf7911): CHANGES; F1 (client-crate render test missing) fixed by T078a, F2 (MVP marker) fixed, prose-level; no re-review (speckit-analyze not run: one milestone cut over existing tasks).

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| B (sonnet, static; no cargo run) | 2 | f64a8637aecfadfd5546cde08dd14188d0384177:e2efebf99161549e09f538b0e4b63795900e82d6 | CLEAN (1 MINOR left: canonicalize vs stored path in serve_configured_project, harmless re-activation) |

## Declined review findings

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M1 follow-up: `DaemonSettings::add_runtime_project` (client, tested with a golden) has no production caller. The client only validates a generated file (`check_configs`/`validate_config`); nothing writes the file yet, and `from_entry` carries no per-daemon project list, so there is no "next rendered file" to feed. Wiring needs the daemon catalog's projects per daemon in `State` and the file-writing path (a later milestone). The user guide says a runtime-added project is not yet written into the file.
- M1 follow-up: contract R7 lists `[[projects]]` as live-reloaded; only serve-at-start is implemented and `restart_keys` does not report it. The guide says so. Live reload of `[[projects]]` is left to a later task.
- M1 follow-up: `serve_configured_project` canonicalizes for its already-known check but `add_project` stores the path as given; a symlinked path re-activates harmlessly.
- M1 follow-up: configured projects are served one after another (a hung clone delays the rest); `ensure_project`'s `exists()` check is outside `ensure_clone`'s per-destination lock.
- Close: tdd-verify (static, no cargo; build lock) gave PASS_WITH_GAPS, see tdd/verification.md Issue 13 F1-F5. F1 (no test starts the daemon with `--config` `[[projects]]` and checks the real `serve_configured_projects` entry point; code exists) and F2-F4 are test-strength gaps left for a run that can build; no behaviour is missing. T080a is Issue 5's.
