# Autopilot ledger — #1439 container-dev-toolchain

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work in separate threads for improvements related to sandbox container runtime (issue #1439: Container sessions cannot build the project: basic image has no mise / compiler toolchain / python3 and no way to install them); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1439
- **Worktree branch**: claude/project-thread-xudko5
- **Started**: 2026-10-25
- **Phase**: plan
- **Next step**: plan review round 2 (re-review of round 1's fixes; orchestrator dispatches the reviewer: this unit has no Agent tool); if CLEAN or MINOR only, `tasks unit`

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Build on #1289 and #1435 or respecify? | Assume both land first; out of scope here | agent-resolved | orchestrator prompt; specs/1289-sandbox-project-setup/spec.md |
| D2 | spec | Which of the issue's options? | Option 1 (mise + C toolchain + pkg-config + python3 in the basic image) plus docs (option 3); dev variant only if FR-007 picks it | agent-resolved | issue #1439 "roughly in order of preference" |
| D3 | clarify | FR-007: where do the gate's X11/Wayland dev libraries come from? | Basic image carries the six CI packages; other projects use an own image; no dev variant | _(provisional: recommended option A, awaiting user decision)_ | issue #1439 acceptance "on the default image"; ci.yml:270-272; 806 FR-002 |
| D4 | plan | Concurrent `mise install` into one home safe (spec assumption)? | No: rustup fails (same toolchain twice) and loses component updates (two `component add` at once). Daemon serialises setup runs on sandbox daemons (contract setup-turn) | agent-resolved | research R5 probe (rustup 1.28.2, 2026-10-25) |
| D5 | plan | AS4 fixture library (spec review MINOR) | `x11` (`-lX11`, from FR-007); own-image half via the guide's `libpq-dev` example; option B → test-built `fixture.pc` | agent-resolved | research R7 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 88a57bc11916cbc5c1b545009cfdd9cb331ed559:acefceb57addd2b636a783099b5c1664b60c8efe | CHANGES: 1 MAJOR (F1 pkg-config fixture untestable), 2 MINOR — all fixed |
| Review spec | 2 | 0dd79911c49382f93e7e6c046c4ab2dda6be0d1a:6da15512d5f7e13d3544e0ea3cc3c60143156e3a | CLEAN; 1 MINOR for the plan: AS4 (spec.md:67) names no library the image provides — plan picks a fixture library (one the image installs, or a test-built .pc file) |
| Review plan | 1 | af5ff41a6aea3437938c9f5176c59cdfd38c8f28:fc2ae166b80b5b08990a56af109bf79b1406f498 | CHANGES: 1 MAJOR (F1 no step obtains/verifies mise checksums; README update step unspecific), 2 MINOR (F2 SC-001/002/005 rows; F3 hung holder blocks setups) — all fixed: plan D1a (signed SHASUMS256, two lines, npm cross-check, provenance in ledger), M1 row, D6 README steps, D7, R1; requirement-map rows SC-001/002/005 + B3 reads the section; setup-turn S2/S3 + D6 sentence + A4 case |
| Review plan | 2 | e09cd372e8cfa3fbb30220e2448a3333f6565b25:e81fbb8fe8ff14ae09217d96f48c4f69f3853cf3 | pending (orchestrator dispatches, sonnet; fix diff = `review-snapshot.sh diff` of round 1's snapshot) |

Clarify round 1: 1 question (FR-007) → provisional option A, applied to FR-003, FR-007, FR-009 and the edge case. Clarify round 2 (scan after the FR-007 change): no critical ambiguities — the six packages fit SC-004's budget, and AS4's fixture can now use a library the image installs (plan).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. FR-007 is on the user's decision card (coordinator); applied provisionally as option A (D3).
If the user picks B or C, the coordinator dispatches a revision.

## Follow-ups not done

- `gh` GraphQL is blocked in the cloud container, so `scripts/autopilot/issue.sh` could not set labels; in-progress and flow:feature were set through the GitHub MCP.
