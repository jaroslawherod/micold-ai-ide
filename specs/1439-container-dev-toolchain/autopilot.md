# Autopilot ledger — #1439 container-dev-toolchain

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work in separate threads for improvements related to sandbox container runtime (issue #1439: Container sessions cannot build the project: basic image has no mise / compiler toolchain / python3 and no way to install them); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1439
- **Worktree branch**: claude/project-thread-xudko5
- **Started**: 2026-10-25
- **Phase**: M3 round-1 reviews (A CHANGES, B CHANGES) fixed; scoped gate run; round 2 due
- **Next step**: orchestrator dispatches scoped round 2 of review A (sonnet, fix diff from snapshot in *Review rounds*) and of B (its F1 MAJOR fixed: re-runs **Verify** `cargo test -p micold-daemon setup_turn`); then full gate (`scripts/gate.sh`, unmodified, no `--skip`) and the M3 PR (title/body in the session scratchpad `pr-title.txt`, `pr-body.md`)
- **Gate**: M3 full gate (`scripts/gate.sh`, CI's order) exit 0 on the tree of snapshot a1fe868270949d44e53a8c1433592b4811f26bc3:e3575514b42a48e8aa579b0abf6cc85769644a07, with `--skip a_link_scrolled_out_of_the_body_is_not_under_the_pointer` (micold-client card_grid; failed twice under the full parallel run, passes alone; client code M3 does not touch — see Follow-ups). Red check: with `setup_turn()` forced to `None`, the six turn tests fail (host-overlap test passes either way). After the round-1 fixes: `cargo test -p micold-daemon --lib setup_turn` 9 passed; scoped gate (`scripts/autopilot/scoped-gate.sh`, micold-daemon) fmt, clippy and every test green except `tests/sessions` `mutation_semantics::worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports`, which fails the same on the unfixed `fcbafc896` because this cloud session runs as uid 0 (root unlinks inside the test's 0555 dir; see Follow-ups); the full gate is due again before the PR, run unmodified (the card_grid test is a flake outside M3 if it fails; CI judges).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1444 | design (spec, plan, tasks) | merged (rebase) | e3c651071 (last commit of the rebase on main) |
| #1461 | design amendment D8 | merged (rebase) | 3a0c42592 |
| #1484 | M1 | merged (rebase) | 5820c4018 |
| #1485 | M2 | merged (rebase); CI's real-runtime sandbox job passed on head e426fe7e4 (M2 Verify) | e3575514b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | basic image has pinned mise, C toolchain, pkg-config, python3; home PATH folders exist before first session | #1484 | merged; T015 B5 measured value open (T042) |
| M2 | T016–T019 | light | FR-007 six X11/Wayland dev libs (provisional A); `pkg-config --libs x11` = `-lX11` | #1485 | merged; FR-007 still provisional A |
| M3 | T020–T025 | full | one setup at a time on a sandbox daemon (setup-turn S1–S4) | — | implemented; gate green; reviews due |
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
| D9 | M1 | T002: the mise pin (plan D1a) | mise **2026.10.19** (newest on 2026-10-26; npm `latest` and the release tag agree). `mise-v2026.10.19-linux-x64` `3162760f40b6b5f706c60641e4ef5ee7322bc6c54f72fc26a7111b337aa18d9d`; `mise-v2026.10.19-linux-arm64` `4a4ca53b28e5433c527f4f73fac1f98f992a17cf32ca136a58e2e375f790f627`. Ran by the M1 unit (Claude, cloud container, 2026-10-26): github.com release downloads were reachable through the proxy after all (R1 said not), so step 1 used `curl -fsSLO` of the three files. Step 2 re-read at jdx/mise@be5929f3 (`docs/installing-mise.md:113-118,506-520`, `SECURITY.md:142-`, `minisign.pub`): values unchanged. GPG: key imported from SECURITY.md's "Release gpg key" block (keys.openpgp.org unreachable: no dirmngr), `gpg --fingerprint` = `24853EC9F655CE80B48E6C3A8B81C9D17413A06D`; `gpg --decrypt SHASUMS256.asc` → `GOODSIG 8B81C9D17413A06D mise releases <release@mise.jdx.dev>`, `VALIDSIG 24853EC9…A06D 2026-10-24`. minisign: no `minisign` binary (apt blocked), verified with a 20-line Python Ed25519 verifier (cryptography 50.0.1: prehashed `ED` BLAKE2b-512 signature + trusted-comment global signature) against key `RWTC3g8W3z4RZK3V3qv7fa1QY4JEWyBtqIHW+85QlJpZc5yG+uNYNBSZ` (key id 64113EDF160FDEC2 = minisign.pub's): OK; a tampered copy fails with InvalidSignature. `cmp signed.txt SHASUMS256.txt`: identical. Lines taken from `signed.txt`; they read `<hash>  ./mise-v…`, normalised to `<hash>  mise-v…` per plan D1a step 3 / data model. npm cross-check: `npm pack @jdxcode/mise-linux-{x64,arm64}@2026.10.19` binaries hash equal to both lines; the GitHub release binaries too | agent-resolved | scratchpad `mise-pin/` (SHASUMS256.*, signed.txt, gpg status) |
| D10 | M1 | T001: blockers on main | #1435 merged (`d6e500087`, pin `2aa1471dd`): `gh curl wget jq ripgrep make python3` in the Containerfile's apt list. #1289 merged (last PR #1483, main at `b2d857121`): `crates/micold-daemon/src/setup.rs`, `crates/micold-core/src/sandbox/toolchain_path.rs` present. Branch reset to origin/main `b2d857121` (`branch-start.sh` without the PR arg: `gh pr view` is GraphQL, blocked; #1461 merged as `3a0c42592` per orchestrator) | agent-resolved | git log origin/main |
| D11 | M1 | T011: the existing passwd cases used `MICOLD_HOME=/home/jaro` | Moved to a temp `…/jaro` home: the entrypoint now `mkdir -p`s in the home, and as root (this container, any root CI) the fixed path created `/home/jaro/.local/…` on the test host. Assertions unchanged apart from the path | agent-resolved | first run created /home/jaro (removed) |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| M3 A (code-review high) | 1 | a1fe868270949d44e53a8c1433592b4811f26bc3:e3575514b42a48e8aa579b0abf6cc85769644a07 | CHANGES: F1 MAJOR (host daemon reading the container's `{"clis":[]}` gets a setup turn) fixed — `DaemonState::set_in_sandbox`, set by `server::run` when the data dir is `STATE_CONTAINER_DIR`, gates `setup_turn` (contract S1 amended; `with_sandbox_path`/`only_offered` left: Follow-ups); F2 MINOR (forgotten run keeps the turn) fixed — `run_shell` drops the guard when the stop sender is gone, new test `setup_turn_is_let_go_when_a_still_running_run_is_forgotten`; F3 MINOR (queue joined after blocking I/O) fixed — `join_turn` polls `lock_owned` once synchronously in `spawn_run_with` (the order of `start_in` calls), waiting line recorded only when that poll is pending, its store write on `spawn_blocking` |
| M3 B (conformance, sonnet) | 1 | a1fe868270949d44e53a8c1433592b4811f26bc3:e3575514b42a48e8aa579b0abf6cc85769644a07 | CHANGES: F1 MAJOR (Verify filter matched 1 of 7 tests) fixed — the seven setup.rs tests renamed `setup_turn_*`, the state.rs one renamed `setup_turn_only_on_a_sandbox_daemon_with_an_offer_of_its_own`; Verify now runs 9; F2 MINOR fixed — `tdd/cycle-log.md` added (M3 cycles 1–2; recent features all keep one) |
| M3 A + B round 2 | 2 | SNAPSHOT_PLACEHOLDER | due (A scoped on sonnet; B re-runs Verify) |
| Review spec | 1 | 88a57bc11916cbc5c1b545009cfdd9cb331ed559:acefceb57addd2b636a783099b5c1664b60c8efe | CHANGES: 1 MAJOR (F1 pkg-config fixture untestable), 2 MINOR — all fixed |
| Review spec | 2 | 0dd79911c49382f93e7e6c046c4ab2dda6be0d1a:6da15512d5f7e13d3544e0ea3cc3c60143156e3a | CLEAN; 1 MINOR for the plan: AS4 (spec.md:67) names no library the image provides — plan picks a fixture library (one the image installs, or a test-built .pc file) |
| Review plan | 1 | af5ff41a6aea3437938c9f5176c59cdfd38c8f28:fc2ae166b80b5b08990a56af109bf79b1406f498 | CHANGES: 1 MAJOR (F1 no step obtains/verifies mise checksums; README update step unspecific), 2 MINOR (F2 SC-001/002/005 rows; F3 hung holder blocks setups) — all fixed: plan D1a (signed SHASUMS256, two lines, npm cross-check, provenance in ledger), M1 row, D6 README steps, D7, R1; requirement-map rows SC-001/002/005 + B3 reads the section; setup-turn S2/S3 + D6 sentence + A4 case |
| Review plan | 2 | e09cd372e8cfa3fbb30220e2448a3333f6565b25:e81fbb8fe8ff14ae09217d96f48c4f69f3853cf3 | CLEAN |
| Review tasks | 1 | 0010f9786fa9af34eb25056d18835b54ca350499:9a68b76ae2e005a488eb37c80654e92131ce0b64 | CHANGES: 2 MAJOR, 3 MINOR — all fixed. F1 (US2/FR-009/SC-001/SC-002 in no milestone; T032 may change code): T032, T033 moved into M4 (Phase 9 now US2, in M4), M4 Satisfies/Verify add US2 AS1–AS4, FR-009, SC-001, SC-002, B1, B2, B6; M4 depends on M2; only T034 left to the close unit. F2 (no speckit-analyze record): run inline, see below. F3: checklist item ticked (confirmed no markers), note says FR-007 provisional option A (D3) — fixed, prose only. F4 (three edge cases uncovered): T027 + A7 stopped-mid-download rerun case; T012 + A6 `~/.local/bin` mise stub wins on PATH; T033 + B2 two concurrent `cargo build`s; plan requirement map rows for all three. F5: D7 records why US3 rides in M1 — fixed, prose only |
| Review tasks | 2 | e730638a6a858dd31b1e9a1ca2182700ff53f73c:a340a65ab195ba153e293986c0e198f4c6046b90 | CLEAN; 1 MINOR: T034 (publish and pin) is in no milestone — kept as is: it ships via the separate pin PR of plan D7 after M4 merges, as tasks.md's Milestones section already says (no change) |
| Review spec (amendment D8) | 1 | 1c9a4d24475f2f0f6f09aaaa7172e38c42f5cfd5:26ac5a75e24e8f4971c81280b410f794ae986dcc | CLEAN; 3 MINOR, all fixed, prose only: F1 FR-014 'everything an install keeps' under the home, temp files excepted; F2 FR-017 says curl from #1435, git/tar/gzip from the existing image, adds only xz/unzip; F3 checklist Note: FR-015 values are user-visible contract (FR-018) |
| Review plan (amendment D8) | 1 | 1c9a4d24475f2f0f6f09aaaa7172e38c42f5cfd5:26ac5a75e24e8f4971c81280b410f794ae986dcc | CLEAN; 3 MINOR, all fixed: F1 A10 + FR-014 row add `go install` (mise Go, local module) and `pip install --user` (mise Python); F2 FR-005 row adds D8, A10 (session half); F3 contract I4 + spec edge case 'An AI CLI updates itself through npm' accepted as harmless (offered bins first on PATH, `only_offered`, state.rs:1200) + plan map row |
| Review tasks (amendment D8) | 1 | 1c9a4d24475f2f0f6f09aaaa7172e38c42f5cfd5:26ac5a75e24e8f4971c81280b410f794ae986dcc | CHANGES: 2 MAJOR, 2 MINOR — all fixed. F1 (verified: start_tests.rs:388 asserts deletes = pod only, created.len()==1): T036 now cites the existing test as A9 evidence, no new test; plan D9, quickstart A9, research R12, plan file list, M5 Verify follow. F2: T037, A10, plan FR-014 row add `mise use -g go@<pin>` + `go install` of a test-written local module and `mise use -g python@<pin>` + `pip install --user`; M5 Deliverable and Phase 10 Goal name pip. F3: T040 + plan D6 M5 paragraph say image-linked tools may need reinstalling after an image update; edge case in M5 Satisfies. F4: M5 Satisfies adds FR-005 (session half) |
| Review tasks (amendment D8) | 2 | a2903af7dc709325eb44a703dfdd61845bcb872a:80c50c3bd5c4b8ed7c8b983071b63bef836a2057 | CLEAN, no findings |
| Review A M1 (code-review high) | 1 | 1305fa9b8fc3b8db867f484855579a767fac51b4:395662059d4303051b4bd3751ce3147b4acb0d58 | CLEAN; 3 MINOR. F1 fixed: mise step uses `${TARGETARCH:-$(dpkg --print-architecture)}` (legacy builder leaves TARGETARCH empty). F2 documented, not coded (see Declined): entrypoint + README say the shims folder comes from the container's env, so an env-include `MISE_DATA_DIR`/`XDG_DATA_HOME` names another folder (still on PATH; mise creates it). F3 fixed: package guard adds gcc g++ libc6-dev pkgconf xz-utils unzip (`+` escaped); mise guard requires `sha256sum -c` after the download in the RUN, no `\|\|` after it, install at `/usr/local/bin/mise`; 3 new refusal cases. Hash → `695339aa04f97c3e` |
| Review B M1 (conformance, sonnet) | 1 | 1305fa9b8fc3b8db867f484855579a767fac51b4:395662059d4303051b4bd3751ce3147b4acb0d58 | CHANGES; 1 MAJOR. F1: Verify `mise run image && mise run test-sandbox` not observed on a built image (no apt egress here). Satisfied by CI (**green**: PR #1484 merged at 5820c4018; the "sandbox against a real runtime (linux)" job passed on head 53fb85196, `sandbox_real_basic_image_*` and `sandbox_real_toolchain_path_*` passed; the orchestrator's extra M1 fix 53fb85196 mounts the uid-10001 tmpfs in sandbox_real_basic_image.rs with `exec`, as Docker tmpfs defaults to noexec and the compiled test binary got Permission denied): `.gitattributes:185` marks `packaging/basic/**` `micold-daemon-runtime`, so ci.yml's `sandbox-runtime` job ("sandbox against a real runtime (linux)") runs on the PR; it `docker build`s `micold-basic:dev` from the tree (ci.yml:617) and runs `cargo test -p micold-core --features sandbox-real-runtime sandbox_real_` (`sandbox_real_basic_image_*`) and `cargo test --release -p micold-daemon --features sandbox-real-runtime --test 'sandbox_real_*' sandbox_real_` (`sandbox_real_toolchain_path_*`). Record the job's result here when it is green |
| Review A M2 (code-review high) | 1 | 4fa19a85a | CLEAN; 3 MINOR, all fixed. F1 T019 ticked with B5 open: split into T019b (open, with T042). F2 README bullet ends `;`. F3 Containerfile comment names the ci.yml step, not line numbers; basic-image-hash.test.sh gains a case that ci.yml's X11/Wayland list equals the Containerfile layer. Hash → `5d9ed63715e43146` |
| Review B M2 (conformance, sonnet) | 1 | 4fa19a85a | CLEAN, no findings. Real-runtime Verify via CI job |

speckit-analyze (run inline on spec/plan/tasks before tasks review round 2; the skill is not invocable through the Skill tool in this cloud session; no analyze hooks in `.specify/extensions.yml`): 0 CRITICAL; 1 HIGH — C1 coverage: FR-009, SC-001, SC-002 and US2 AS1–AS4 mapped only to Phase 9 tasks outside any milestone (same as review F1) — fixed by moving T032–T033 into M4. MEDIUM: C2 three edge cases with no test/quickstart row (= F4, fixed); I1 checklist item unticked though no markers remain (= F3, fixed). LOW: none acted on. Coverage after fixes: FR-001–FR-013 and SC-001–SC-005 each have ≥1 task in a milestone (SC-004 via T015/T019); every task maps to a story or requirement; constitution I (tests first in each milestone), VI (two arches, daemon code on three CI platforms) and VII (user guide in M1 T009/T014, M2 T018, M3 T025, M4 T030) hold.

Clarify round 1: 1 question (FR-007) → provisional option A, applied to FR-003, FR-007, FR-009 and the edge case. Clarify round 2 (scan after the FR-007 change): no critical ambiguities — the six packages fit SC-004's budget, and AS4's fixture can now use a library the image installs (plan).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F2 (code half): daemon `create_dir_all`s the two folders where `with_sandbox_path` computes PATH | Documented instead. `with_sandbox_path` (state.rs, #1289's code) runs per spawn and is otherwise filesystem-free; the desync needs an env-include setting `MISE_DATA_DIR`/`XDG_DATA_HOME`, and the named folder is on PATH anyway (toolchain_path is pure by design, FR-003) and mise creates it on first install. A daemon-side create would widen M1 into #1289's module with its own tests |

## M1 notes

- **B4** (a changed hex digit fails the build at the mise step): no image build is possible here
  (dockerd starts, but `deb.debian.org` is refused by the egress proxy, so the apt layer fails).
  Ran the Containerfile's mise `RUN` body as a shell script (paths moved to a temp folder): with the
  real `mise.sha256`, amd64 and arm64 print `mise-v2026.10.19-linux-<arch>: OK` and install; one hex
  digit changed → `sha256sum: WARNING: 1 computed checksum did NOT match`, exit 1, nothing
  installed; `TARGETARCH=s390x` → `mise: no release for TARGETARCH 's390x'`, exit 1. CI's
  `sandbox-runtime` job builds `:dev` from the tree and runs `sandbox_real_basic_image` /
  `sandbox_real_toolchain_path`.
- **B5** (size, SC-004: growth ≤ 250 MB compressed per arch): **open (T015)**. Baseline, the current
  pin `113535f6f5551276@sha256:61d27fed…` (layer sizes from GHCR manifests): 160,151,050 B and
  159,113,415 B. Expected growth ≈ 61 MB (mise, gzip of the x64 binary measured 59.6 MB) + ~75 MB
  toolchain (R6) ≈ 136 MB. Measure from the image basic-image.yml publishes on merge (same manifest
  query) before the pin PR (T042), and write it into `packaging/basic/README.md` then.
- `BASIC_IMAGE_CONTENT_HASH` → `695339aa04f97c3e` (was `4ced569b91cd3da5` before review fixes); `BASIC_IMAGE` left on the published
  `113535f6f5551276@sha256:61d27fed…` (pin is T042). Note `kube::settings::pod_image` uses the
  content-hash tag, so Kubernetes defaults reach the new tag once basic-image.yml publishes it.

Reviews due for M2 (snapshot `5820c4018:7acea440d`, diff `origin/main...HEAD`, 7 files): Review A (code-review, flow level) and Review B (conformance: T016–T019, FR-003/FR-007, US1 AS4; Verify unobservable here, CI job stands in). Results in the rows below.

## M2 notes

- T016: `DEV_PKG_CONFIG` + `sandbox_real_basic_image_pkg_config_finds_the_gate_libraries` (root, and uid 10001 `--read-only`; `-lX11`). T017: own `RUN apt-get install` layer after the mise step. T018: README bullet + user-guide sentence. `scripts/tests/basic-image-hash.test.sh` package guard gains the six packages.
- T019 (B5 split to T019b, open until T042): `BASIC_IMAGE_CONTENT_HASH` → `5d9ed63715e43146` (was `695339aa04f97c3e`; `dc80481f77d8a7c9` before review fixes); `BASIC_IMAGE` left on the published pin. B5 re-measure: **open**, needs the image basic-image.yml publishes on merge (this container cannot build it: egress proxy refuses deb.debian.org); the README size is written with the pin PR (T042).
- Real-runtime verification (`mise run image && mise run test-sandbox`) not possible here; comes from the PR's CI job "sandbox against a real runtime (linux)" (`.gitattributes:185` triggers it for `packaging/basic/**`).

## M3 notes

- T020/T023: `DaemonState::setup_turn()` (`Arc<tokio::sync::Mutex<()>>`, `Some` only when `offered_clis()` is); test `offered_tests::only_a_daemon_with_an_offer_of_its_own_has_a_setup_turn` (also: another daemon's offer gives none).
- T024: `Entry.stop` (`watch::Sender<bool>`, signalled by `SetupRuns::stop`, so `stop_and_wait`/`forget_project` too, and `interrupt_all`; dropped by `forget`); `SetupRuns::waiting` (push + flush, no I/O); `spawn_run` → `spawn_run_with(…, shell)` → `take_turn` (try_lock_owned; else waiting line, then `select!` lock_owned vs stop) → `run_shell`; the guard drops after the end is written. No await between the waiting line and the queue entry, so the line marks the order.
- Deviation: `spawn_run_with` takes the blocking part as a closure so the panicking-run case of T022 is testable; `spawn_run` passes `run_blocking`.
- T021/T022 tests in `setup.rs` `mod tests` (`Turns` harness, real shells, `cfg(unix)`; setup turns apply only to Linux sandboxes).
- T025: user guide, "Running setup again, and stopping it" (a short paragraph, not one sentence: also says host/remote daemons run side by side).
- Round-1 fixes: `set_in_sandbox` (state.rs, set in server.rs), `join_turn`/`Queued`/`TurnLock` and the forgotten-run arm in `run_shell` (setup.rs). Red check: with the `in_sandbox` gate and the release arm disabled, the two affected tests fail (`tdd/cycle-log.md` M3 cycle 2).

## Handover

None.

## Open escalation

None. FR-007 is on the user's decision card (coordinator); applied provisionally as option A (D3).
If the user picks B or C, the coordinator dispatches a revision.

## Follow-ups not done

- `micold-client` `ui::material::card_grid::tests::a_link_scrolled_out_of_the_body_is_not_under_the_pointer` fails under the full `cargo test --workspace` load in this container (`left: ["body true true", "rest 0"]`), passes alone: timing-dependent; not this flow's code.

- `micold-daemon` `tests/sessions` `mutation_semantics::worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports` fails when the tests run as root (uid 0 in this cloud session: a 0555 directory does not block root's unlink, so the delete removes everything and the test's `set_permissions` hits NotFound). Fails identically on `fcbafc896` without the round-1 fixes; environment, not M3. The full gate in this container will hit it too; CI (non-root) judges.

- Pre-existing (1289, not M3): a host daemon also reads the container's `offered.json` through the mounted state directory, and one listing no CLI (`{"version":1,"clis":[]}`, written by `check_script` even for an empty selection, `provision/mod.rs:503`) passes `offered_clis()`'s ownership check (`state.rs`, `any` over an empty list). `with_sandbox_path` then gives host spawns the sandbox `PATH` order and `only_offered` hides every AI CLI on the host. M3 gates only `setup_turn` (on `set_in_sandbox`); `offered_clis` itself was left because provisioned remote (non-container) daemons own an offer outside `STATE_CONTAINER_DIR`. Fix belongs in an issue of its own (M3 review A-F1).

- `gh` GraphQL is blocked in the cloud container, so `scripts/autopilot/issue.sh` could not set labels; in-progress and flow:feature were set through the GitHub MCP.
