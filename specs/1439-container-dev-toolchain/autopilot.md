# Autopilot ledger — #1439 container-dev-toolchain

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work in separate threads for improvements related to sandbox container runtime (issue #1439: Container sessions cannot build the project: basic image has no mise / compiler toolchain / python3 and no way to install them); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1439
- **Worktree branch**: claude/project-thread-xudko5
- **Started**: 2026-10-25
- **Phase**: clarify
- **Next step**: user answers the FR-007 escalation; clarify unit is continued with it, applies it to spec.md, runs the next clarify round; then plan

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 88a57bc11916cbc5c1b545009cfdd9cb331ed559:acefceb57addd2b636a783099b5c1664b60c8efe | CHANGES: 1 MAJOR (F1 pkg-config fixture untestable), 2 MINOR — all fixed |
| Review spec | 2 | 0dd79911c49382f93e7e6c046c4ab2dda6be0d1a:6da15512d5f7e13d3544e0ea3cc3c60143156e3a | CLEAN; 1 MINOR for the plan: AS4 (spec.md:67) names no library the image provides — plan picks a fixture library (one the image installs, or a test-built .pc file) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

- **Clarify round 1, Q1 (category 1, scope)** — FR-007: where do the X11/Wayland development libraries
  this repository's `cargo test --workspace` links (ci.yml:270-272: libxkbcommon-dev libwayland-dev
  libx11-dev libxcursor-dev libxrandr-dev libxi-dev) come from?
  - A (Recommended): the basic image carries them. Issue #1439's acceptance is "on the default image";
    the six -dev packages are a few MB each (libx11-dev 2.5 MB installed), well inside SC-004's 250 MB;
    feature 806 FR-002 excludes only the daemon, AI CLIs and credentials, not build libraries; any
    winit/iced project benefits.
  - B: own image `FROM` the basic image, documented for this repo; FR-009/SC-001 move to that image
    (departs from the issue's acceptance).
  - C: a published "dev" image variant, selectable per daemon: second publish/hash/pin pipeline and a
    form choice — the largest scope.
- Other scan categories: no critical ambiguity (setup-command wording, fixture library and
  concurrency are plan items already recorded).

## Follow-ups not done

- `gh` GraphQL is blocked in the cloud container, so `scripts/autopilot/issue.sh` could not set labels; in-progress and flow:feature were set through the GitHub MCP.
