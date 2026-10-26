# Autopilot ledger — #1439 container-dev-toolchain

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work in separate threads for improvements related to sandbox container runtime (issue #1439: Container sessions cannot build the project: basic image has no mise / compiler toolchain / python3 and no way to install them); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1439
- **Worktree branch**: claude/project-thread-xudko5
- **Started**: 2026-10-25
- **Phase**: amendment (spec amendment D8, 2026-10-26)
- **Next step**: tasks re-review round 2 on the amendment (spec and plan reviews done), dispatched by the orchestrator; then the specs-only gate, PR title/body to the scratchpad, push; then M1 once #1289 has merged (#1435 merged)
- **Gate**: specs-only `scripts/check-criteria-observables.sh specs/1439-container-dev-toolchain/spec.md` exit 0 (spec has no criteria table; not opted in); every changed file is under `specs/`

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1444 | design (spec, plan, tasks) | merged (rebase) | e3c651071 (last commit of the rebase on main) |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | basic image has pinned mise, C toolchain, pkg-config, python3; home PATH folders exist before first session | — | waits for #1435, #1289 |
| M2 | T016–T019 | light | FR-007 six X11/Wayland dev libs (provisional A); `pkg-config --libs x11` = `-lX11` | — | waits for M1 and the FR-007 decision |
| M3 | T020–T025 | full | one setup at a time on a sandbox daemon (setup-turn S1–S4) | — | waits for #1289 |
| M4 | T026–T033 | full | fixture mise project builds end to end; repo `mise.toml` components + CONTRIBUTING; this repository's gate exits 0 in a container session on `:dev` (B1), B2, B6; sandbox page building section + own-image example (B3) | — | waits for M1, M2, M3 |
| M5 | T034–T041 | full | session installs its own build tools (mise, npm -g, cargo install, uv tool, go install) without root; install defaults on sandbox daemons; installs survive restart, container replacement, pod recreation | — | waits for M1, M4 |

T042 (publish and pin, D7; was T034 before amendment D8) goes to the close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Build on #1289 and #1435 or respecify? | Assume both land first; out of scope here | agent-resolved | orchestrator prompt; specs/1289-sandbox-project-setup/spec.md |
| D2 | spec | Which of the issue's options? | Option 1 (mise + C toolchain + pkg-config + python3 in the basic image) plus docs (option 3); dev variant only if FR-007 picks it | agent-resolved | issue #1439 "roughly in order of preference" |
| D3 | clarify | FR-007: where do the gate's X11/Wayland dev libraries come from? | Basic image carries the six CI packages; other projects use an own image; no dev variant | _(provisional: recommended option A, awaiting user decision)_ | issue #1439 acceptance "on the default image"; ci.yml:270-272; 806 FR-002 |
| D4 | plan | Concurrent `mise install` into one home safe (spec assumption)? | No: rustup fails (same toolchain twice) and loses component updates (two `component add` at once). Daemon serialises setup runs on sandbox daemons (contract setup-turn) | agent-resolved | research R5 probe (rustup 1.28.2, 2026-10-25) |
| D5 | plan | AS4 fixture library (spec review MINOR) | `x11` (`-lX11`, from FR-007); own-image half via the guide's `libpq-dev` example; option B → test-built `fixture.pc` | agent-resolved | research R7 |
| D6 | tasks | D1a's GPG step against mise's docs | Checked via GitHub code search of jdx/mise@b9e15069 (repo itself not readable here): `SHASUMS256.asc` is clearsigned (`scripts/release.sh` `gpg --clearsign -u 8B81C9D17413A06D`), release key fingerprint `24853EC9F655CE80B48E6C3A8B81C9D17413A06D` (`docs/installing-mise.md`, `SECURITY.md`); docs' primary check is minisign (`SHASUMS256.txt.minisig`, key `RWTC3g8W3z4RZK3V3qv7fa1QY4JEWyBtqIHW+85QlJpZc5yG+uNYNBSZ`). Plan D1a fixed: hashes read from the GPG-verified clearsigned text, minisign check added, txt must equal signed text; D6 README steps and research R1 follow. T002 re-reads the docs at run time | agent-resolved | GitHub code search results, 2026-10-25 |
| D7 | tasks | Why does US3 (P2, T011–T015) ride in M1 ahead of US1's remaining P1 parts? (tasks review F5) | Kept in M1: T013 changes `packaging/basic/entrypoint.sh`, which `BASIC_IMAGE_CONTENT_HASH` covers like the Containerfile, so M1 and US3 share one hash recompute (T010), one `:dev` rebuild and one `basic-image.yml` run; split, each would need its own. US3's five tasks are small (two test files, one entrypoint block, one guide sentence, B4/B5 records). US1's other parts (M2 FR-007, M3 setup turn, M4) depend on decisions or code US3 does not touch, so the order costs them nothing | agent-resolved | tasks.md Phase 3, T010; plan D3, D7 |

| D8 | amendment | Owner's added requirement (verbatim): "as part of image push enough tools that agent can install them on it's own. It shoould have persistentn space so they will not disapear after container restart" / "I mean that it cann install tools needed to build project" | Interpretation confirmed with the owner by the orchestrator: with no setup command a session installs user-level build tools itself (mise, npm -g, cargo install, uv tool, go install, pip --user under a mise Python), no root; they land in the sandbox home, are on PATH for later sessions, survive restart and replacement; apt stays out of reach (own image). Spec: US5, FR-005 widened, FR-014–FR-018, SC-006, 5 edge cases, persistence assumption now cited. Plan: D8 (install defaults `NPM_CONFIG_PREFIX`/`CARGO_INSTALL_ROOT`/`GOBIN` in `with_sandbox_path`, contract sandbox-install-env.md), D9 (persistence: tests only), D1 step 1 adds `xz-utils unzip` (`make`/`python3` already from #1435). Research R12–R15. Tasks: Phase 10 / M5 T034–T041; publish renumbered T042. #1289 overlap: none duplicated (1289 has no session-install or install-location requirement; its spec.md:333-335 only assumes setup's installs persist); collision: `state.rs`/`toolchain_path.rs` textual only (R15) | user (jaro, repo owner) via orchestrator | home persistence: sandbox/mod.rs:496-500,:841; daemons.rs:984-1010; shell/sandbox.rs:321-331; argv.rs:289-297; lifecycle.rs:324-331; kube/manifest.rs:21-26,:238-240; kube/lifecycle.rs:93-117,:187-191; kube/start.rs:543-548 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 88a57bc11916cbc5c1b545009cfdd9cb331ed559:acefceb57addd2b636a783099b5c1664b60c8efe | CHANGES: 1 MAJOR (F1 pkg-config fixture untestable), 2 MINOR — all fixed |
| Review spec | 2 | 0dd79911c49382f93e7e6c046c4ab2dda6be0d1a:6da15512d5f7e13d3544e0ea3cc3c60143156e3a | CLEAN; 1 MINOR for the plan: AS4 (spec.md:67) names no library the image provides — plan picks a fixture library (one the image installs, or a test-built .pc file) |
| Review plan | 1 | af5ff41a6aea3437938c9f5176c59cdfd38c8f28:fc2ae166b80b5b08990a56af109bf79b1406f498 | CHANGES: 1 MAJOR (F1 no step obtains/verifies mise checksums; README update step unspecific), 2 MINOR (F2 SC-001/002/005 rows; F3 hung holder blocks setups) — all fixed: plan D1a (signed SHASUMS256, two lines, npm cross-check, provenance in ledger), M1 row, D6 README steps, D7, R1; requirement-map rows SC-001/002/005 + B3 reads the section; setup-turn S2/S3 + D6 sentence + A4 case |
| Review plan | 2 | e09cd372e8cfa3fbb30220e2448a3333f6565b25:e81fbb8fe8ff14ae09217d96f48c4f69f3853cf3 | CLEAN |
| Review tasks | 1 | 0010f9786fa9af34eb25056d18835b54ca350499:9a68b76ae2e005a488eb37c80654e92131ce0b64 | CHANGES: 2 MAJOR, 3 MINOR — all fixed. F1 (US2/FR-009/SC-001/SC-002 in no milestone; T032 may change code): T032, T033 moved into M4 (Phase 9 now US2, in M4), M4 Satisfies/Verify add US2 AS1–AS4, FR-009, SC-001, SC-002, B1, B2, B6; M4 depends on M2; only T034 left to the close unit. F2 (no speckit-analyze record): run inline, see below. F3: checklist item ticked (confirmed no markers), note says FR-007 provisional option A (D3) — fixed, prose only. F4 (three edge cases uncovered): T027 + A7 stopped-mid-download rerun case; T012 + A6 `~/.local/bin` mise stub wins on PATH; T033 + B2 two concurrent `cargo build`s; plan requirement map rows for all three. F5: D7 records why US3 rides in M1 — fixed, prose only |
| Review tasks | 2 | e730638a6a858dd31b1e9a1ca2182700ff53f73c:a340a65ab195ba153e293986c0e198f4c6046b90 | CLEAN; 1 MINOR: T034 (publish and pin) is in no milestone — kept as is: it ships via the separate pin PR of plan D7 after M4 merges, as tasks.md's Milestones section already says (no change) |
| Review spec (amendment D8) | 1 | 1c9a4d24475f2f0f6f09aaaa7172e38c42f5cfd5:26ac5a75e24e8f4971c81280b410f794ae986dcc | CLEAN; 3 MINOR, all fixed, prose only: F1 FR-014 'everything an install keeps' under the home, temp files excepted; F2 FR-017 says curl from #1435, git/tar/gzip from the existing image, adds only xz/unzip; F3 checklist Note: FR-015 values are user-visible contract (FR-018) |
| Review plan (amendment D8) | 1 | 1c9a4d24475f2f0f6f09aaaa7172e38c42f5cfd5:26ac5a75e24e8f4971c81280b410f794ae986dcc | CLEAN; 3 MINOR, all fixed: F1 A10 + FR-014 row add `go install` (mise Go, local module) and `pip install --user` (mise Python); F2 FR-005 row adds D8, A10 (session half); F3 contract I4 + spec edge case 'An AI CLI updates itself through npm' accepted as harmless (offered bins first on PATH, `only_offered`, state.rs:1200) + plan map row |
| Review tasks (amendment D8) | 1 | 1c9a4d24475f2f0f6f09aaaa7172e38c42f5cfd5:26ac5a75e24e8f4971c81280b410f794ae986dcc | CHANGES: 2 MAJOR, 2 MINOR — all fixed. F1 (verified: start_tests.rs:388 asserts deletes = pod only, created.len()==1): T036 now cites the existing test as A9 evidence, no new test; plan D9, quickstart A9, research R12, plan file list, M5 Verify follow. F2: T037, A10, plan FR-014 row add `mise use -g go@<pin>` + `go install` of a test-written local module and `mise use -g python@<pin>` + `pip install --user`; M5 Deliverable and Phase 10 Goal name pip. F3: T040 + plan D6 M5 paragraph say image-linked tools may need reinstalling after an image update; edge case in M5 Satisfies. F4: M5 Satisfies adds FR-005 (session half) |
| Review tasks (amendment D8) | 2 | a2903af7dc709325eb44a703dfdd61845bcb872a:80c50c3bd5c4b8ed7c8b983071b63bef836a2057 | pending (scoped re-review of the fix diff, sonnet; orchestrator dispatches) |

speckit-analyze (run inline on spec/plan/tasks before tasks review round 2; the skill is not invocable through the Skill tool in this cloud session; no analyze hooks in `.specify/extensions.yml`): 0 CRITICAL; 1 HIGH — C1 coverage: FR-009, SC-001, SC-002 and US2 AS1–AS4 mapped only to Phase 9 tasks outside any milestone (same as review F1) — fixed by moving T032–T033 into M4. MEDIUM: C2 three edge cases with no test/quickstart row (= F4, fixed); I1 checklist item unticked though no markers remain (= F3, fixed). LOW: none acted on. Coverage after fixes: FR-001–FR-013 and SC-001–SC-005 each have ≥1 task in a milestone (SC-004 via T015/T019); every task maps to a story or requirement; constitution I (tests first in each milestone), VI (two arches, daemon code on three CI platforms) and VII (user guide in M1 T009/T014, M2 T018, M3 T025, M4 T030) hold.

Clarify round 1: 1 question (FR-007) → provisional option A, applied to FR-003, FR-007, FR-009 and the edge case. Clarify round 2 (scan after the FR-007 change): no critical ambiguities — the six packages fit SC-004's budget, and AS4's fixture can now use a library the image installs (plan).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Amendment part 2 (2026-10-26): recorded the three round-1 verdicts on amendment D8 and applied every
finding (commit after 6294a8553); round-2 snapshot `a2903af7dc709325eb44a703dfdd61845bcb872a:80c50c3bd5c4b8ed7c8b983071b63bef836a2057` taken for the tasks review (its two MAJORs
were fixed). Spec and plan reviews are done (CLEAN, MINORs fixed).

- **Next**: the orchestrator dispatches a scoped tasks re-review (round 2, sonnet) of
  `scripts/autopilot/review-snapshot.sh diff 1c9a4d24475f2f0f6f09aaaa7172e38c42f5cfd5:26ac5a75e24e8f4971c81280b410f794ae986dcc`; if CLEAN, the specs-only gate
  (`scripts/check-criteria-observables.sh specs/1439-container-dev-toolchain/spec.md`, exit 0 at
  part 2), PR title/body to the scratchpad, push. Never `gh pr`/`gh issue`.
- **Open findings**: none.

## Open escalation

None. FR-007 is on the user's decision card (coordinator); applied provisionally as option A (D3).
If the user picks B or C, the coordinator dispatches a revision.

## Follow-ups not done

- `gh` GraphQL is blocked in the cloud container, so `scripts/autopilot/issue.sh` could not set labels; in-progress and flow:feature were set through the GitHub MCP.
