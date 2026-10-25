# Autopilot ledger — #1289 sandbox-project-setup

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on all tickets that are not yet in progress use speckit autopilot (issue #1289: Sandbox: per-project setup command and PATH for project toolchains); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1289
- **Worktree branch**: claude/project-thread-d7q05f
- **Started**: 2026-10-24
- **Phase**: M3
- **Next step**: M3 PR #1393 rebased onto main (protocol 70) and force-pushed — wait on CI, merge (rebase)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1347 | design (spec, plan, tasks) | merged | 86fcb6b28cfe72ea680999550251409cbfa0921f |
| #1353 | M1 sandbox PATH | merged (rebase) | 31a863fd3191b73aaea8000c244864413a5a1ad2 |
| #1385 | M2 setup core | merged (rebase) | 47997d190a6ee8533f9bf0c71d34185fc66276b2 |
| #1393 | M3 daemon runs setup | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T007 | full | sandbox home's ~/.local/bin and mise shims on PATH | #1353 | merged |
| M2 | T008–T017 | full | core setup model, output buffer, shell, StreamingChild, settings field | #1385 | merged |
| M3 | T018–T030 | full | protocol 70; daemon runs setup on create, streams output, MCP field, run/stop | #1393 | PR open |
| M4 | T031–T039 | full | records persist, restart → Interrupted, attach catch-up, delete stops first | — | pending |
| M5 | T040–T049 | full | Settings › Project, ProjectSetupSet sync, row tag, failure notification | — | pending |
| M6 | T050–T055 | full | Setup output dialog, delete confirmation line | — | pending |
| M7 | T056–T059 | light | Run setup / Stop setup menu entries | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-009: where is the setup command defined? | A: per-project app setting in settings.json; no repo file read | provisional — recommended option, awaiting user answer | docs/user-guide/settings.md#jira (per-project base URL in settings.json) |
| 2 | clarify | Story 4: per-project image in scope? | A: deferred to follow-up; second container daemon + remembered Daemon select documented (FR-025), Out of scope | provisional — recommended option, awaiting user answer | docs/user-guide/worktrees-and-sessions.md ~466 (Daemon select remembered per project) |
| 3 | plan | How does a daemon learn the setup command (MCP create_worktree has no client in the loop)? | Client stores it in settings.json and pushes ProjectSetupSet (in memory) to each daemon holding the project, on connect and on change | unit (research R4) | persist.rs sends service settings to active daemon only |
| 4 | plan | Which daemons get the home PATH folders? | Those with offered.json (app-owned container/pod) | unit (research R2) | provision::Ownership::AppOwned; MICOLD_IMAGE_REFERENCE absent on pods |
| 5 | tasks | Milestone cut vs plan's five suggestions | Seven: plan M3 split into M3 (run on create) / M4 (persistence, delete) by size; client cut along scenarios into M5 (command, sync, tag, notification), M6 (output view), M7 (US3 run/stop). M3 kept at 13 tasks (~1000 lines): protocol messages, their handlers and the run task cannot land apart without a second version bump or a server answering Unsupported. plan.md §Milestones updated to point at tasks.md | unit (milestones.md rules 2–3) | tasks.md §Milestones |
| 6 | tasks | How does the daemon look up `bash` (resolve_on_path is pub(crate) in core)? | New core `setup::shell::bash_on_path(&OsStr)` wraps it; data-model §shell_line updated | unit | provider.rs:575 private fn |
| 7 | M1 | FR-006 "a script that sets PATH MAY put entries ahead": does it? | No: the order is computed after environment-include, so the script's PATH is `rest`; the guide says its folders keep their order after the home folders. FR-006 is permissive (MAY), and the MUST half (folders survive env-include) holds | unit | state.rs `with_sandbox_path`; test `a_sandbox_spawn_path_puts_the_home_folders_after_the_offered_bins` |
| 8 | M1 | Empty `PATH` entries in `rest` | Dropped (an empty entry means the cwd); also `HOME=""` counts as unset | unit | data-model "An empty value counts as unset"; test `an_empty_rest_adds_no_empty_entry` |
| 9 | M1 (red CI) | `sandbox_real_history` host phase found no `claude`: the host daemon read the container's `offered.json` through the shared state directory, and `only_offered` now needs the CLI in the (container-path) offered `bin` | `offered_clis` ignores an offer naming a `bin` outside the daemon's own `ai-clis` dir (another daemon's); research R2 amended. An empty offer still counts as own | unit (systematic-debugging) | CI job 121003684367; test `another_daemons_offer_in_a_shared_state_directory_changes_nothing` (red without the guard) |
| 10 | M3 | `SetupChanged` to clients "attached to the project": the daemon holds one attachment per project, so other windows would miss it | Broadcast to every connected client, as `CatalogChanged` is; a client filters by project | unit | `state.rs` `attachments` (one per project) |
| 11 | M3 | Batching "through `ProgressThrottle`": it admits one line per gap and drops the rest | A 250 ms flush tick in the run task sends each watcher every line it has not had (`setup::BATCH_EVERY`); nothing is dropped | unit | `progress.rs` `admit` returns `None` inside the gap |
| 12 | M3 | `create_worktree_with_session` "keeps its own return", but MCP needs its `SetupStart` | Its `Ok` gains the `SetupStart` as a third element; the app's handler ignores it | unit | C4 needs `setup` on a create with a session too |
| 13 | M3 | Watchers inside `Entry` cannot hold a watch before the first run or across runs | `SetupRuns.watchers: (project, location) → {ClientId → next line}`; a watch with no record registers and sends nothing (there is no `RunId` to send). The reason line is stored as the output's last line, so a backlog after the end ends with it | unit | contract C1 "a new run on the same location keeps the watch" |
| 14 | M3 | `Entry.done` / `stop_and_wait`; where setup runs for a worktree | `done` left to M4 with `stop_and_wait`, its only reader. The folder is the worktree's `path` from the catalog's last refresh (so an included worktree runs in its own folder), `Default` the project repo; unknown → `NotFound` *There is no <location> here.* | unit | data-model §Daemon; T036 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b0a77d005ab504233e2ea45937bad3888c721813:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 PATH order + scenario; F2 stop vs disconnect; F3 FR-023a one run per worktree, notification per app; F4 bash/sh; F5 Default entry state/output; F6 citation) |
| Spec | 2 (sonnet, scoped) | daabf27317f59eec2cdebd72955b471611832070:8dead641fd56a494c0cee555f52cb82540b3d57d | CLEAN, 2 MINOR — both fixed in clarify (Story 1 scenarios renumbered in order; FR-011 names bash/sh/PowerShell and points to Platforms) |
| Clarify | 1 | 54752543 | 2 questions → provisional A/A applied (FR-009, FR-025, Out of scope, Story 4 removed) |
| Clarify | 2 | after round-1 answers | no critical ambiguities (UI placement and app→daemon command hand-off left to plan) → CLEAN |
| Plan | 1 | f5e4485bc71dd9eaf5ff5dfa2d7a3c0444768d51:cfc387029e91b351cae9a77b2329e2e5b12d471a | CHANGES: 2 MAJOR, 3 MINOR — all fixed (F1 daemon-wide persisted RunId counter + A12/A16 recreate test; F2 delete awaits run end via stop_and_wait bounded by RUN_OVERRUN, stale finish dropped, A12 asserts no record/log; F3 C4 three values + Started(RunId); F4 M4 split into M4a/M4b; F5 ProjectSetupSet carries daemon-catalog path, A16 mapped case) |
| Plan | 2 (sonnet, scoped) | c6904f37533cfa3183cad458c3e458eb852b92f7:71a9120e6a1fca096755b00c0c38b356273d79f9 | CLEAN, 1 MINOR — fixed, prose only (data-model §RunId: id allocated in memory under the lock, counter file persisted just after release, before the record write) |
| Tasks | 1 | afa51c1f5757e7d989ae7915d46182fa1e6dc0f7:0b28553c5169f0e4d6cc8ce59e6cff80cbff1039 | CHANGES: 3 MAJOR, 2 MINOR — all fixed (F1 A4 tests split into setup_model/setup_output/setup_shell.rs, Parallel line and M2 Verify updated; F2 G2 test now T052 before impl T053, deps updated; F3 "T020–T022 pass" moved to T027, deps T020–T022 → T025–T027, T021 red until T025–T027; F4 T022 gains a WorktreeCreate positive control so it is red until T025–T027; F5 `delete_warns_setup` predicate case in T050/T051, T054 uses it) |
| Tasks | 2 (sonnet, scoped) | e8a60871840097dcc1874c2b1084ebd6e30eb98e:62160ffc5876e446a80f7dcd6f1b623f4b490ab3 | CLEAN, no findings |
| M1 A | 1 (high) | bfedc8feff31a942849a46161efd9046cd8ac84b:65b4a48c150e71ebb1754ffda28df43e74b95f10 | CLEAN, 3 MINOR — all fixed (F1 a folder holding the separator drops only itself, 2 core tests; F2 A2 (a)/A3 scripts export `MISE_DATA_DIR=`, `resolve_with` doc corrected; F3 `only_offered` keeps a CLI only when its command is in its offered `bin`, new test) |
| M1 B | 1 (sonnet) | bfedc8feff31a942849a46161efd9046cd8ac84b:65b4a48c150e71ebb1754ffda28df43e74b95f10 | CHANGES: 1 MAJOR, 1 MINOR — fixed (F1 `tdd/cycle-log.md` added, red/green per behaviour; F2 guide says "container or Kubernetes daemon", prose only). Verify: toolchain_path 14/14, daemon `offered` 8/8 |
| M1 A | 2 (sonnet, scoped) | da8d7b2975423f682e8ed7b67404de7fa34812af:079a1970a4f38c03df7d287289a5d26dac1513f5 | CLEAN, no findings |
| M1 B | 2 (sonnet) | da8d7b2975423f682e8ed7b67404de7fa34812af:079a1970a4f38c03df7d287289a5d26dac1513f5 | CLEAN, 1 MINOR — fixed, prose only (guide paragraph re-wrapped to ~100 columns). Verify: toolchain_path 16/16, daemon `offered` 9/9 |
| M1 A | 3 (sonnet, scoped, red-CI fix c9320751) | 896f820bf2e94b14db372bee07de2cf983fe110b:c9320751 | CLEAN, 1 MINOR — logged as a follow-up, not fixed (offered.json with `clis: []` ambiguous across daemons) |
| M2 A | 1 (high) | 9d700a4cfc7e4e26deb5f78cbfbc8e4d190d44be:d5272e0d0a2cbf564ed5fa3810cbef45e2680a90 | CHANGES: 1 MAJOR, 3 MINOR — all fixed (F1 `MAX_LINE_BYTES` 64 KiB cuts a line with no end; F2 group kill before the reaping `try_wait` via `waitid` WNOWAIT, under the `reaped` lock; F3 `impl Drop for StreamingChild` kills and reaps; F4 one reader loop `read_chunks`, retries EINTR); cycle log M2 cycle 6 |
| M2 A | 2 (scoped, sonnet) | 14733e4bf5354abea71bdb1a930a50aa83ec28c2:8d6c87f9f2f5373395bf28093df3f363883d51b8 | CLEAN — all four round-1 fixes hold; 2 MINOR fixed (F1 `MAX_LINE_BYTES` 4 KiB + decoded cut: worst case about 40 MiB, 10,000-line floor kept; F2 the cut waits for a UTF-8 character, an open escape may still split); cycle log M2 cycle 7 |
| M2 B | 1 (sonnet) | 9d700a4cfc7e4e26deb5f78cbfbc8e4d190d44be:d5272e0d0a2cbf564ed5fa3810cbef45e2680a90 | CLEAN — Verify re-run all green; 2 MINOR, no change (client `into_settings`/`set_changed`/DEFERRED edits are in scope; ledger/tasks ticks are the own spec dir) |
| M3 A | 1 (high) | 76878fd79e7b3cda96fef785c06dae1e781bb505:92410eac003e260ba2f0996c20787408eced4676 | CHANGES: 1 MAJOR, 3 MINOR — F1 fixed (`SetupOutput` batches ≤ 1 MiB of line bytes); F2 fixed (`SetupRuns` own mutex, state lock only to send); F3 deferred to M4 T038 (delete stops and forgets the run); F4 fixed (`start_in` the created worktree's path); cycle log M3 cycle 3 |
| M3 B | 1 (sonnet) | 76878fd79e7b3cda96fef785c06dae1e781bb505:92410eac003e260ba2f0996c20787408eced4676 | CLEAN — Verify re-run green (setup_run 7/7, mcp_create_worktree_setup 2/2, mcp_create_worktree 43/43, core protocol 32/32); 2 MINOR fixed (B-F1 `send_ack` doc back on it, prose only; B-F2 = A-F4) |
| M3 A | 2 (scoped, sonnet) | 8dfc0a347798adf3e04dbda979314c7259e06dbe:9ae68f109d053306af8e9b8bb09a3b26ce94a192 | CLEAN, no findings — lock order setup→inner everywhere, no I/O under either mutex, chunking keeps watcher `next` consistent, 1 MiB bound holds |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. M3 rebase 1 (2026-10-25): onto af0d60077; `SessionLocation` derive conflict in session.rs (main `Hash`, M3 `Serialize, Deserialize`) — kept both.
M3 rebase 2 (2026-10-25): onto 36459b958 (38 commits), then f92be5b68 (3 test/spec commits of 1116). Main's feature 1271 took protocol 69, so 1289 renumbered to 69 → 70: `PROTOCOL_VERSION`, `FEATURE_026_PROTOCOL_VERSION` (schema_hash.rs), `the_protocol_version_is_70` (protocol_roundtrip.rs), `the_setup_messages_bump_the_protocol_to_70` (messages.rs; the first gate run caught it), version.rs/schema_hash.rs history comments keep 1271's line and add 1289's; tasks.md, plan.md, research.md, contracts/wire-and-settings.md say 70. Conflicts only in those history comments; no shared logic. `cargo test -p micold-daemon -p micold-core -p micold-client --no-run` clean; full gate (scripts/gate.sh) green at ef699805f and force-pushed. Main then gained 6 commits (1270 M3, #1288 sandbox basic image): they touch none of M3's files, make no protocol bump, and `git merge-tree` merges them clean, so the PR is behind but not in conflict.

## Open escalation

None. (Clarify Q1/Q2 asked of the user; provisional answers in *Decisions*.)

## Follow-ups not done

- M3 review A-F3: a setup run is keyed by (project, dir_name) and nothing stops or drops it on worktree delete, so a recreated same-name worktree gets Busy. M4 T038 (`stop_and_wait` + `forget` before `git worktree remove`) covers it.
- Per-project sandbox image (spec Story 4, deferred by clarify Q2): needs its own issue.
- M1 review A round 3 MINOR: an `offered.json` with `clis: []` cannot be told apart from the daemon's own, so a host daemon sharing the state dir would prepend home folders to its PATH. Fix: an ownership marker (`#[serde(default)] state: Option<PathBuf>` in `Offered`, filled by the writer; `offered_clis` returns None when Some and differs from `dir`).
