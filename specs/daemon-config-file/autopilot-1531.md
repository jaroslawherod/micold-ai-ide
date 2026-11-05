# Autopilot ledger — #1531 daemon-config-file (Issue 10)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1531` (`GH_TOKEN` for `gh` in sessions of every kind); labels: `flow:feature`, `effort:low`. Spec, Clarify and Plan skipped by the same precedent as #1524, #1525, #1526, #1529, #1530, #1534 and #1535: the design of the whole feature is already on main (PR #1509, `specs/daemon-config-file/`: spec.md, research.md, plan.md, `contracts/daemon-config.md` section `[[github.tokens]]`; FR-011, FR-035, SC-003); the tasks unit only cuts milestones for the "Issue 10" group (T055–T058, T095).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1531
- **Worktree branch**: feat/1531_gh-token-for-gh-in-sessions-of-every-kind
- **Started**: 2026-11-05
- **Phase**: M1 in progress (implementation done, tests green; gate/review/PR not yet run — see Handover)
- **Next step**: M1 — scoped gate + review A, then review B, then full gate and PR (`tasks/gate.md`, `tasks/pr.md`). See *Handover*.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1696 | Design (milestones for Issue 10) | merged | 90e17aeeb53635ae3c7b6027bc4057f559d2d744 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T055–T057, T095, T058 | full | `gh` shim sets `GH_TOKEN`/`GH_HOST` from the token chosen for the cwd's repository, on every session kind | — | implemented, unreviewed |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1531? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-011/FR-035/SC-003. Skipped as for #1524/#1525/#1526/#1529/#1530/#1534/#1535. | agent-resolved | specs/daemon-config-file/autopilot-1530.md |
| D2 | tasks | Split Issue 10? | No: 5 tasks, one shim (rule 3). Single milestone. | agent-resolved | milestones.md rule 3 |
| D3 | tasks | Tier | full: new PATH-injected shim that reads a chosen credential and sets it in a session's env (security-sensitive surface); not a reuse of an existing pattern | agent-resolved | milestone-format.md |
| D4 | tasks | speckit-analyze findings | 1 MEDIUM + 3 LOW, no CRITICAL/HIGH: fixed U1 (`GH_HOST` left alone with no token, same as `GH_TOKEN`) and U3 (Windows `gh.cmd` forwards exit code, cannot `exec`) by wording edits to T057/T095/Deliverable; U2 and N1 needed no fix (U2 is already covered by the milestone narrative; N1 is expected sequencing) | agent-resolved | speckit-analyze run |

| D5 | M1 | M1 needs #1530's `TokenChoice` chooser, which is not on `main` (unpushed local work in another worktree): wait, stack on #1530's branch, or implement behind a local seam? | Wait: pause the run with the design merged and resume once #1530's M1 and M2 are on `main`. Rejected stacking (that branch's M1 is unpushed and still in review, so the API may change) and the seam (it would split the acceptance criteria across two runs). Superseded by D6: start M1 now against #1530's merged M1 alone, without waiting for the rest. | decided by user | `AskUserQuestion` 2026-11-05; `git ls-tree origin/main crates/micold-core/src/daemon_config/matching.rs` empty; #1530 M1 commit `324482a39` not on `origin/feat/1530_...` (tip `02cf510bc`) |
| D6 | M1 | Start M1 now against #1530's merged `TokenChoice` chooser (PR #1700), without waiting for all of #1530? | Yes: start now. #1530's M1 (matching.rs/credential_config.rs/reach.rs/git_credential.rs/git_env.rs — T046–T054, T092–T094) is merged to `main`. #1530's M2 (PR #1713, kind-gated `container_git_edits`, `[git] config_file`/`GIT_CONFIG_GLOBAL`, retiring `GitCredentialGrant`/`Revoke`/`Targets`) is still open and out of scope: not depended on, not duplicated. Supersedes D5. | decided by user | `git ls-tree origin/main` holds matching.rs/credential_config.rs/reach.rs/git_credential.rs/git_env.rs; PR #1700 merged; PR #1713 open |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks 1531 | 1 | 7a198beac8244eaa27e1da4bb57ab949a3097bce:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3 | CHANGES: F1 MAJOR (Depends-on claimed Issue 9 M2, rationale only cites M1 tasks) fixed by narrowing to M1 merged; F2 MINOR (T056 no file path, wrongly tagged `[P]` with T055 in the same file) fixed by adding the path and dropping `[P]`. Both fixes are task-text only, no new round. |

## Declined review findings

None.

## Handover

M1 (T055–T057, T095, T058) is implemented and its own tests are green; not yet gated, reviewed or
opened as a PR. Rebased onto `origin/main` (branch-start.sh REBASED 5, previous PR #1696 MERGED)
before this work started.

**Done:**
- `crates/micold-core/src/github.rs`: `split_remote` made `pub` (was module-private) — `gh-token`
  reuses it to resolve `<dir>`'s `origin` for any forge, not only github.com (a token's `host` may
  be a GitHub Enterprise host).
- `crates/micold-daemon/src/gh_shim.rs` (new): `origin_repo` (generic remote parse), `resolve`
  (builds a synthetic `CredentialRequest{protocol:"https", host, path}` from `origin_repo` and
  calls `crate::credential_config::answer` — the exact function `git-credential get` uses, so the
  chooser and per-host matching are shared code, not a reimplementation), `unix_shim`/`windows_shim`
  (script text), `write_shim` (write-if-changed, mode 0755 on unix), `main` (`gh-token --cwd <dir>`
  CLI entry: two lines, token then host, on stdout; exit 1 silently with none — T055).
- `crates/micold-daemon/src/main.rs`: `gh-token` dispatch, same pattern as `git-credential`.
- `crates/micold-daemon/src/lib.rs`: `pub mod gh_shim;`.
- `crates/micold-daemon/src/state.rs`: `DaemonState::with_gh_shim_path`, called from
  `spawn_env_for` after `with_sandbox_path` — writes the shim to `<data_dir>/bin` and puts it
  first on the session's `PATH`, for every session kind (T057). A missing data directory or
  current-exe, or a write failure, is a no-op (a session must not fail to spawn over this).
- `crates/micold-daemon/Cargo.toml`: added the missing `[[test]]` entry for `gh_shim` (the crate
  has `autotests = false`, so a new integration test file needs one — this was not in tasks.md and
  is mechanical, not scope creep).
- `crates/micold-daemon/tests/gh_shim.rs` (new, T055/T056/T095): 7 tests, all green
  (`scripts/build-lock.sh cargo test -p micold-daemon --test gh_shim`, exit 0) — `gh-token`
  resolution (github.com and a GitHub Enterprise host, no token, no configuration), and the shim
  script itself exec'd against a fake `gh` (asserts the real exit code 42 comes through, proving
  `exec` ran it rather than a copy of the shim): sets `GH_TOKEN`/`GH_HOST` from the chosen token,
  leaves either alone if already set, leaves both unset with no token chosen.
- `docs/user-guide/settings.md`: new "`gh` in sessions" subsection under GitHub tokens (T058).
- `mise run check` (cargo check --workspace) green before the test file existed; the targeted
  `gh_shim` test is green (see above). **The full `mise run gate` has not been run yet.**

**Not done — next unit's work, in order:**
1. `mise run check`/`mise run gate` have not been re-run since the Cargo.toml/test-file fix; run
   the scoped gate (`scripts/autopilot/scoped-gate.sh`) first.
2. Review A (`code-review` skill, level `high`, `origin/main...HEAD`) in the scoped gate's shadow,
   per `tasks/gate.md` step 1.
3. Review B (conformance rubric, `autopilot-reviewer`, `model: sonnet`), per `tasks/gate.md` step 2.
4. Full gate (`mise run gate`), per `tasks/gate.md` step 3.
5. Commit (nothing is committed yet — see below), push with the `-c` flags this worktree's
   container needs (#1744, workaround in the ledger's last entry below), open the PR
   (`feat(daemon-config-file): …`, title ending `(#1531)`, body ending `Refs #1531`), record it here.
6. Nothing in this unit touched or depended on #1713 (#1530 M2, still open) — boundary held.

**Not committed.** `git status --porcelain` lists: `M crates/micold-core/src/github.rs`,
`M crates/micold-daemon/Cargo.toml`, `M crates/micold-daemon/src/lib.rs`,
`M crates/micold-daemon/src/main.rs`, `M crates/micold-daemon/src/state.rs`,
`M docs/user-guide/settings.md`, `?? crates/micold-daemon/src/gh_shim.rs`,
`?? crates/micold-daemon/tests/gh_shim.rs`, plus this ledger edit. The next unit should commit these
together (one commit for the feature + its tests + Cargo.toml, a separate `docs(1531):` commit for
this ledger) before running the gate, per `tasks/pr.md`.

## Open escalation

None open. The M1 blocker was asked and answered: see D5, superseded by D6 (start now). Not waiting
on the rest of #1530.

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
