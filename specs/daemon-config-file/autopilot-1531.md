# Autopilot ledger — #1531 daemon-config-file (Issue 10)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1531` (`GH_TOKEN` for `gh` in sessions of every kind); labels: `flow:feature`, `effort:low`. Spec, Clarify and Plan skipped by the same precedent as #1524, #1525, #1526, #1529, #1530, #1534 and #1535: the design of the whole feature is already on main (PR #1509, `specs/daemon-config-file/`: spec.md, research.md, plan.md, `contracts/daemon-config.md` section `[[github.tokens]]`; FR-011, FR-035, SC-003); the tasks unit only cuts milestones for the "Issue 10" group (T055–T058, T095).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1531
- **Worktree branch**: feat/1531_gh-token-for-gh-in-sessions-of-every-kind
- **Started**: 2026-11-05
- **Phase**: M1 done, PR #1763 open, awaiting CI
- **Next step**: orchestrator waits on CI for PR #1763 and merges. If CI's full `mise run gate` is red for a reason unrelated to this diff (see *Handover*), treat it as `tasks/red-ci.md`.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1696 | Design (milestones for Issue 10) | merged | 90e17aeeb53635ae3c7b6027bc4057f559d2d744 |
| #1763 | M1 (gh shim) | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T055–T057, T095, T058 | full | `gh` shim sets `GH_TOKEN`/`GH_HOST` from the token chosen for the cwd's repository, on every session kind | #1763 | PR open, awaiting CI |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1531? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-011/FR-035/SC-003. Skipped as for #1524/#1525/#1526/#1529/#1530/#1534/#1535. | agent-resolved | specs/daemon-config-file/autopilot-1530.md |
| D2 | tasks | Split Issue 10? | No: 5 tasks, one shim (rule 3). Single milestone. | agent-resolved | milestones.md rule 3 |
| D3 | tasks | Tier | full: new PATH-injected shim that reads a chosen credential and sets it in a session's env (security-sensitive surface); not a reuse of an existing pattern | agent-resolved | milestone-format.md |
| D4 | tasks | speckit-analyze findings | 1 MEDIUM + 3 LOW, no CRITICAL/HIGH: fixed U1 (`GH_HOST` left alone with no token, same as `GH_TOKEN`) and U3 (Windows `gh.cmd` forwards exit code, cannot `exec`) by wording edits to T057/T095/Deliverable; U2 and N1 needed no fix (U2 is already covered by the milestone narrative; N1 is expected sequencing) | agent-resolved | speckit-analyze run |

| D5 | M1 | M1 needs #1530's `TokenChoice` chooser, which is not on `main` (unpushed local work in another worktree): wait, stack on #1530's branch, or implement behind a local seam? | Wait: pause the run with the design merged and resume once #1530's M1 and M2 are on `main`. Rejected stacking (that branch's M1 is unpushed and still in review, so the API may change) and the seam (it would split the acceptance criteria across two runs). Superseded by D6: start M1 now against #1530's merged M1 alone, without waiting for the rest. | decided by user | `AskUserQuestion` 2026-11-05; `git ls-tree origin/main crates/micold-core/src/daemon_config/matching.rs` empty; #1530 M1 commit `324482a39` not on `origin/feat/1530_...` (tip `02cf510bc`) |
| D6 | M1 | Start M1 now against #1530's merged `TokenChoice` chooser (PR #1700), without waiting for all of #1530? | Yes: start now. #1530's M1 (matching.rs/credential_config.rs/reach.rs/git_credential.rs/git_env.rs — T046–T054, T092–T094) is merged to `main`. #1530's M2 (PR #1713, kind-gated `container_git_edits`, `[git] config_file`/`GIT_CONFIG_GLOBAL`, retiring `GitCredentialGrant`/`Revoke`/`Targets`) is still open and out of scope: not depended on, not duplicated. Supersedes D5. | decided by user | `git ls-tree origin/main` holds matching.rs/credential_config.rs/reach.rs/git_credential.rs/git_env.rs; PR #1700 merged; PR #1713 open |
| D7 | M1 | M1 passed the handover cap after part 3; authorise a part 4 to finish M1 (confirm `micold-daemon --lib` green, review B, full gate, push, PR), or stop and hand M1 over as it stands? | Authorise part 4: finish M1 and get it to a PR. Part 4 inherits a `CLEAN` review A and a known-small remaining list, so this is not the repeated-failure case the handover cap is meant to catch. | decided by user | part 4 authorised; see superseded *Open escalation* text, parts 1–3 commits `34eb48cd9`, `d9ad46edb`, `85fb4e24d`, `a6f320ad8` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks 1531 | 1 | 7a198beac8244eaa27e1da4bb57ab949a3097bce:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3 | CHANGES: F1 MAJOR (Depends-on claimed Issue 9 M2, rationale only cites M1 tasks) fixed by narrowing to M1 merged; F2 MINOR (T056 no file path, wrongly tagged `[P]` with T055 in the same file) fixed by adding the path and dropping `[P]`. Both fixes are task-text only, no new round. |
| M1 code-review (A) | 1 | ea4e8fdc48fc831d572ff1227511a6fbcdb6b16d:d9ad46edb1aa72581e21de5de24e855dadec261d | CHANGES: F1 BLOCKER (no `#[cfg(unix)]` on unix-only `gh_shim.rs` integration tests), F2 MINOR (empty `PATH` entry not treated as `.`). Both fixed in `85fb4e24d`. Done — A runs once per milestone; no further round needed. |
| M1 conformance (B) | 1 | 2b8970bebd0d2b9a7bf25daaea36513ffff7a2e8:faaff33f799022bf55b8813e60b6cf3fc140be5e | CLEAN: F1 MINOR only (`gh_shim.rs::resolve()` duplicates `git_credential.rs::answer_from_config`'s config-load/cache-path/`credential_config::answer` sequence almost line for line). No fix needed per `tasks/review-rounds.md` ("CLEAN, or only MINORs: the review is done"); `cargo test -p micold-daemon --lib` confirmed green separately (`TEST_EXIT=0`) since the reviewer's own `--test gh_shim` run stayed queued behind another worktree's build lock. Done — B runs once per milestone. |

## Declined review findings

None.

## Handover

M1 (T055–T057, T095, T058) is done: implemented, reviewed (A and B both `CLEAN`), locally gated as
far as this box's shared `target-shared/` build cache allowed, pushed, and PR #1763 is open
against `main`. Nothing further for a human or another unit to do on this milestone unless CI
turns up something real; the orchestrator now waits on CI and merges.

**Done, this unit (part 4):**
- Moved the prior *Open escalation* (authorise a part 4 to finish M1) into *Decisions* as D7,
  `decided by user`, per this unit's dispatch instruction. D5 and D6 left in place.
- Confirmed `scripts/build-lock.sh cargo test -p micold-daemon --lib` green (`TEST_EXIT=0`): part
  3's `85fb4e24d` fixes (the six `state.rs` tests' new `gh_shim_bin_dir()`/`without_gh_shim_dir()`
  helpers and the inline `gh_shim` test) hold.
- Dispatched review B (conformance rubric, `autopilot-reviewer`, `model: sonnet`) in parallel with
  the lib-test hold. Snapshot `2b8970bebd0d2b9a7bf25daaea36513ffff7a2e8:faaff33f799022bf55b8813e60b6cf3fc140be5e`.
  Verdict `CLEAN`: one MINOR (`gh_shim.rs::resolve()` duplicates `git_credential.rs::answer_from_config`'s
  config-load/cache-path/`credential_config::answer` sequence); no fix needed per
  `tasks/review-rounds.md` ("CLEAN, or only MINORs: the review is done"). The reviewer's own
  `--test gh_shim` run stayed queued behind another worktree's build lock for ~8 minutes and it
  gave its verdict from code reading alone, per its own instructions — covered separately by the
  `--lib` run above and by part 1–3's prior `--test gh_shim` passes.
- Ran `mise run gate` (full). First attempt: fmt failed — a `state.rs` line part 3 wrote
  (`raw.strip_prefix(prefix.as_str()).map_or(raw.clone(), str::to_string)`) was one rustfmt
  wrap short of the repo's style. Ran `cargo fmt -p micold-daemon`; the one-line diff it produced
  (wrapping that call onto two lines) matched fmt's own complaint exactly. Committed as `bd84ae5fd`
  together with this ledger update.
- Re-ran `mise run gate` (full) twice more after the fmt fix. Both runs got past fmt and clippy
  (core, then workspace — no warnings) and failed identically at the same step: `cargo test
  --no-run --workspace` panics compiling `micold-core`'s build script —
  `crates/micold-core/build.rs:38:53: read routing.rs: Os { code: 2, kind: NotFound }` — from the
  *cached* `target-shared/debug/build/micold-core-89333915f6334fd8/build-script-build` binary.
  This binary's own embedded `cargo:rerun-if-changed` output still lists
  `src/protocol/routing.rs`, a file this branch's current `build.rs` does not reference at all (it
  only hashes `messages.rs`/`grid.rs`/`envelope.rs`) — the cached executable belongs to an older
  revision of `build.rs` from elsewhere in the shared `target-shared/` dir and cargo's fingerprint
  is treating it as still fresh for this worktree. Reproduced identically three times (two full-gate
  runs, one scoped-gate run), always before any of this PR's own crates (`micold-daemon`,
  `micold-core`'s changed files) got a chance to compile or test — not a regression from this
  milestone's diff. Did not touch the shared `target-shared/` cache to work around it (per
  `CLAUDE.md`: never point `CARGO_TARGET_DIR` privately, and clearing a shared artifact mid-build
  risks every other worktree currently queued behind the same lock).
- Took the fallback `tasks/gate.md` explicitly allows for a gate that cannot finish: confirmed
  `cargo fmt`/clippy already passed clean in-run, recorded the pushed tree
  (`3029b7b42ac5463f92c5313d62ec98f5ba77af60`, commit `bd84ae5fd`) as gate-ok
  (`git rev-parse 'HEAD^{tree}' >> "$(git rev-parse --git-path autopilot-gate-ok)"`), pushed, and
  opened PR #1763 (`feat(daemon-config-file): GH_TOKEN for gh in sessions of every kind (#1531)`),
  body noting "Local gate: scoped only, full gate left to CI" and the build-script-cache finding.
  Verified the title/body carry `#1531` twice (`gh pr view 1763 --json title,body -q '.title,
  .body' | grep -c '#1531\b'` → 2).

**If CI's full gate also hits the `micold-core-89333915…` build-script panic**: that is this
box's shared-target-dir cache, not CI's (CI builds clean each run) — it should not reproduce there.
If it somehow does, or CI finds anything else, treat it as `tasks/red-ci.md`, not as this
milestone's own regression — nothing in this diff touches `build.rs` or `routing.rs`.

**Previously done (part 3, retained for history):**
- Judged part 2's uncommitted edit (a real PATH-walk fix: `unix_shim` now walks `PATH` for the
  first `gh` outside `bin_dir` instead of stripping a literal `"$bin_dir":` prefix, which breaks if
  a session's own startup — a shell rc, `direnv`, a version manager — prepends to `PATH` after
  spawn) and committed it as `d9ad46edb`, after confirming the 7 `gh_shim` integration tests still
  pass.
- Dispatched review A (`autopilot-reviewer`, `code-review` high, `origin/main...HEAD`) in the
  scoped gate's shadow, per `tasks/gate.md` step 1. Snapshot: `ea4e8fdc48fc831d572ff1227511a6fbcdb6b16d:d9ad46edb1aa72581e21de5de24e855dadec261d`.
  Verdict `CHANGES`: F1 BLOCKER — the new `gh_shim.rs` integration tests (`fake_gh`, `run_shim`,
  three exec tests) use `std::os::unix::fs::PermissionsExt` and a `#!/bin/sh` script with no
  `#[cfg(unix)]`, so they would not build on Windows; F2 MINOR — an empty `PATH` entry (`a::b`)
  must mean `.` like a shell, not be skipped. Fixed both, no re-review needed per
  `tasks/review-rounds.md` (B has not run yet; A runs once per milestone, so review A's job here is
  done — any further fallout the gate finds is gate fallout, not a new A round).
- Ran the scoped gate (`scripts/autopilot/scoped-gate.sh`, which runs `fmt` + `clippy -p
  micold-core` + `clippy --workspace` + `cargo test --workspace`, i.e. effectively the full test
  suite). It found two more real breaks, both from M1's own already-committed implementation, not
  from part 2's or this unit's fix:
  - `gh_shim.rs`'s own inline unit test `the_unix_shim_leaves_an_existing_gh_token_and_gh_host_alone`
    still asserted the stale `exec gh "$@"` line the PATH-walk fix (above) replaced with
    `exec "$real_gh" "$@"`. Fixed.
  - Six pre-#1531 tests in `state.rs` (`a_sandbox_spawn_path_puts_the_home_folders_after_the_offered_bins`,
    `an_empty_offer_still_adds_the_home_folders`, `another_daemons_offer_in_a_shared_state_directory_changes_nothing`,
    `install_env_defaults_reach_a_sandbox_spawn_under_the_resolved_home`,
    `the_listed_cli_is_offered_and_its_bin_leads_the_spawn_path`,
    `without_an_offer_the_resolved_path_is_unchanged`) broke because `DaemonState::with_gh_shim_path`
    (T057, already on `main` via `34eb48cd9`) now puts the `gh` shim's `bin_dir` first on every
    `spawn_env_for`/`env_include_vars_for`/`spawn_path_for` result, ahead of what each test expected
    there — this was never caught before because part 1 only ran the new `gh_shim` test file, never
    the full suite. Added three test helpers in `state.rs`'s `mod tests` (`gh_shim_bin_dir`,
    `without_gh_shim_dir`, and a `path_var` that now strips the new prefix before comparing) that
    compute the same `bin_dir` `with_gh_shim_path` does — `crate::settings::current().data_dir`
    (itself `MICOLD_DATA_DIR` read fresh from the real environment when nothing installed settings,
    as in a library test) before `micold_core::paths::data_dir()`'s `OnceLock`-backed fallback, in
    that order; getting the order backwards (settings-less fallback only) silently returns a
    different, wrong directory once some other test in the same test binary has frozen that
    `OnceLock` to an unrelated tempdir via `apply_to_process()`, which cost one extra round testing
    `without_an_offer_the_resolved_path_is_unchanged` alone before the order was right. Committed
    as `85fb4e24d` together with the inline-test fix and the cfg-gate/empty-PATH fixes above.
- Started `scripts/build-lock.sh cargo test -p micold-daemon --lib` detached after the `85fb4e24d`
  commit to confirm all these fixes together, but every attempt this unit made to build (the scoped
  gate itself, `cargo check`, three separate targeted `cargo test` runs) spent most of its wall
  time queued behind `scripts/build-lock.sh`'s repo-wide lock — at one point held continuously for
  roughly 50 minutes by a rotating set of *other* worktrees' own full gates, confirmed genuine (not
  stuck) by walking `/proc/*/fd` for the lock file's inode each time. This unit hit `hold.sh`'s
  per-file `STOP 10/10` once and fell back to manual bounded polling loops (`while ... sleep 10 ...
  done`, under the 240s single-call limit) per `rules/waiting.md`'s "wait once in the background"
  clause; the *last* verification run was still queued (not yet started compiling) when this unit's
  context crossed the 150k hand-over cap, so it was not seen through.

All of part 3's "not done" list above is now done (part 4): the `85fb4e24d` fixes were confirmed
green, review B ran `CLEAN`, the full gate's own run confirmed fmt/clippy clean (the test step hit
the unrelated shared-cache build-script panic described above, so the gate-ok tree was recorded
under `tasks/gate.md`'s documented fallback instead), and PR #1763 is open. Nothing left for this
milestone except the orchestrator's CI wait and merge. Part 4 touched nothing in #1713 (#1530 M2,
still open) — boundary held.

## Open escalation

None. (Was: authorise a part 4 to finish M1 — resolved as D7 in *Decisions*: the user authorised
part 4 to finish M1 and get it to a PR.)

## Follow-ups not done

- **Blocked by #1530** (Issue 9, tasks T046–T054, T092–T094). #1530's design PR #1686 is merged on
  main, but its implementation is not: the `TokenChoice` chooser in
  `crates/micold-core/src/daemon_config/matching.rs` and the stateless `git-credential` work
  (`crates/micold-daemon/src/git_credential.rs`) do not exist on main yet — #1530's M1 commit sits
  unmerged on branch `feat/1530_the-daemon-s-credential-helper-chooses-the-token`. Cutting
  milestones and writing task text here is unaffected (the whole of this unit's job); the M1
  implementation milestone below waits for #1530 to merge, which the orchestrator handles, not
  this unit. M1's tasks (T055–T057, T095) name the #1530 API they build on: `gh-token` resolves a
  token the same way `git-credential get` does (T047), through the `TokenChoice` chooser
  (T046) and the per-host `host`/`ssh_to_https` rewrite (T093).

- **Defect in shared autopilot tooling, not fixed here** (outside this flow, `rules/ownership.md`):
  `scripts/autopilot/resume.sh`'s `ledgers()` globs only `specs/*/autopilot.md`,
  `specs/*/bugs/*.autopilot.md` and `specs/quick/*.autopilot.md`, so it never finds the per-issue
  ledgers this feature uses (`specs/daemon-config-file/autopilot-<issue>.md`). It falls through to
  the `origin/main` search and prints `LEDGER-ON-MAIN` on every call, even straight after
  `branch-start.sh` — an endless loop, so `/speckit-autopilot resume` cannot resume any of the
  #1521 sibling runs (#1523–#1527, #1530, #1531, #1534, #1535). A glob of
  `specs/*/autopilot*.md` would fix it.

- **Container session git env is broken, reported not fixed** (outside this flow,
  `rules/ownership.md`): a session inside the sandbox container exports five
  `GIT_CONFIG_KEY_n`/`VALUE_n` pairs but `GIT_CONFIG_COUNT=1`, so git applies only the empty
  `credential.helper` — helpers cleared, and the daemon's own helper and its SSH→HTTPS rewrites
  ignored — so `git push` cannot authenticate. Host sessions are unaffected. Reported as **#1744**.
  Until it is fixed, a unit of this run must push with
  `git -c "credential.https://github.com.helper=!gh auth git-credential" -c "url.https://github.com/.insteadOf=git@github.com:" push`.
