# Autopilot ledger — #1439 container-dev-toolchain

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work in separate threads for improvements related to sandbox container runtime (issue #1439: Container sessions cannot build the project: basic image has no mise / compiler toolchain / python3 and no way to install them); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1439
- **Worktree branch**: claude/project-thread-xudko5
- **Started**: 2026-10-25
- **Phase**: tasks
- **Next step**: tasks review round 1 (orchestrator dispatches the reviewer with rubrics/tasks.md: this unit has no Agent tool); then close the checklist item, specs-only gate, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | basic image has pinned mise, C toolchain, pkg-config, python3; home PATH folders exist before first session | — | waits for #1435, #1289 |
| M2 | T016–T019 | light | FR-007 six X11/Wayland dev libs (provisional A); `pkg-config --libs x11` = `-lX11` | — | waits for M1 and the FR-007 decision |
| M3 | T020–T025 | full | one setup at a time on a sandbox daemon (setup-turn S1–S4) | — | waits for #1289 |
| M4 | T026–T031 | full | fixture mise project builds end to end; repo `mise.toml` components + CONTRIBUTING; sandbox page building section + own-image example | — | waits for M1, M3 |

Phase 9 (T032–T034: quickstart B1, B2, B6, publish and pin) goes to the close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Build on #1289 and #1435 or respecify? | Assume both land first; out of scope here | agent-resolved | orchestrator prompt; specs/1289-sandbox-project-setup/spec.md |
| D2 | spec | Which of the issue's options? | Option 1 (mise + C toolchain + pkg-config + python3 in the basic image) plus docs (option 3); dev variant only if FR-007 picks it | agent-resolved | issue #1439 "roughly in order of preference" |
| D3 | clarify | FR-007: where do the gate's X11/Wayland dev libraries come from? | Basic image carries the six CI packages; other projects use an own image; no dev variant | _(provisional: recommended option A, awaiting user decision)_ | issue #1439 acceptance "on the default image"; ci.yml:270-272; 806 FR-002 |
| D4 | plan | Concurrent `mise install` into one home safe (spec assumption)? | No: rustup fails (same toolchain twice) and loses component updates (two `component add` at once). Daemon serialises setup runs on sandbox daemons (contract setup-turn) | agent-resolved | research R5 probe (rustup 1.28.2, 2026-10-25) |
| D5 | plan | AS4 fixture library (spec review MINOR) | `x11` (`-lX11`, from FR-007); own-image half via the guide's `libpq-dev` example; option B → test-built `fixture.pc` | agent-resolved | research R7 |
| D6 | tasks | D1a's GPG step against mise's docs | Checked via GitHub code search of jdx/mise@b9e15069 (repo itself not readable here): `SHASUMS256.asc` is clearsigned (`scripts/release.sh` `gpg --clearsign -u 8B81C9D17413A06D`), release key fingerprint `24853EC9F655CE80B48E6C3A8B81C9D17413A06D` (`docs/installing-mise.md`, `SECURITY.md`); docs' primary check is minisign (`SHASUMS256.txt.minisig`, key `RWTC3g8W3z4RZK3V3qv7fa1QY4JEWyBtqIHW+85QlJpZc5yG+uNYNBSZ`). Plan D1a fixed: hashes read from the GPG-verified clearsigned text, minisign check added, txt must equal signed text; D6 README steps and research R1 follow. T002 re-reads the docs at run time | agent-resolved | GitHub code search results, 2026-10-25 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 88a57bc11916cbc5c1b545009cfdd9cb331ed559:acefceb57addd2b636a783099b5c1664b60c8efe | CHANGES: 1 MAJOR (F1 pkg-config fixture untestable), 2 MINOR — all fixed |
| Review spec | 2 | 0dd79911c49382f93e7e6c046c4ab2dda6be0d1a:6da15512d5f7e13d3544e0ea3cc3c60143156e3a | CLEAN; 1 MINOR for the plan: AS4 (spec.md:67) names no library the image provides — plan picks a fixture library (one the image installs, or a test-built .pc file) |
| Review plan | 1 | af5ff41a6aea3437938c9f5176c59cdfd38c8f28:fc2ae166b80b5b08990a56af109bf79b1406f498 | CHANGES: 1 MAJOR (F1 no step obtains/verifies mise checksums; README update step unspecific), 2 MINOR (F2 SC-001/002/005 rows; F3 hung holder blocks setups) — all fixed: plan D1a (signed SHASUMS256, two lines, npm cross-check, provenance in ledger), M1 row, D6 README steps, D7, R1; requirement-map rows SC-001/002/005 + B3 reads the section; setup-turn S2/S3 + D6 sentence + A4 case |
| Review plan | 2 | e09cd372e8cfa3fbb30220e2448a3333f6565b25:e81fbb8fe8ff14ae09217d96f48c4f69f3853cf3 | CLEAN |
| Review tasks | 1 | 0010f9786fa9af34eb25056d18835b54ca350499:9a68b76ae2e005a488eb37c80654e92131ce0b64 | pending (orchestrator dispatches; also confirm checklist item "No [NEEDS CLARIFICATION] markers remain" — none in the artifacts; FR-007 provisional per D3) |

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
