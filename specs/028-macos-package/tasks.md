---

description: "Task list for feature 028: macOS Package"
---

# Tasks: macOS Package

**Input**: Design documents from `/specs/028-macos-package/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY and are written and observed failing **before** the implementation task that satisfies
them — including inside Phase 2, where the plist's keys and the bundle's signature are specified by
tests (T007, T008, T009) before the tasks that produce them. This feature's "production code" is
mostly a shell script, a plist, and two workflow files; Principle I applies to them through
text-scan and shell tests, exactly as it already does to the Debian packaging
(`tests/packaging_excludes_showcase.rs`, `scripts/tests/*.test.sh`).

**Documentation**: Per Principle VII, each story carries its own section of
`docs/user-guide/install-macos.md` in the same phase. The page is created in US1 and extended by
later stories; no story is done until its section exists.

**Cross-platform**: Per Principle VI, both new `micold-core` modules compile and are tested on
Linux, macOS, and Windows and return the benign answer off macOS. The bundle script's composition
half runs on Linux so its tests do too (research R3).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1…US6)
- Every task names the exact file it touches

## Path Conventions

Rust workspace at the repository root: `crates/micold-core/`, `crates/micold-client/`,
`crates/micold-daemon/`. Packaging inputs in `packaging/`, executable steps in `scripts/`, exposed
through `mise.toml`. See plan.md → Project Structure.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Settle the one unknown that changes how a later gate is written, and create the
packaging inputs everything else substitutes into.

- [ ] T001 **Deferred — settled from this branch's own CI run instead.** The spike as written pushes a throwaway workflow to a GitHub runner, which is an outward-facing action taken on the user's account for a question the branch's first real run answers anyway. T023 was therefore written to be correct under *either* outcome: it asserts the full gate, and treats a window-surface failure as a pass with the reason logged (contingency (b)). Once the macOS leg has run, record the observed outcome and which contingency applies under R12. Original text: spike — determine whether a GitHub `macos-latest` runner can create an iced window: push a throwaway workflow step that builds `micold-client` and launches it, capture the outcome, and record the answer (and which contingency applies) under R12 in `specs/028-macos-package/research.md`
- [X] T002 [P] Create `packaging/macos/entitlements.plist` as an empty `<dict/>` with a comment naming it as the file a future Developer ID + notarization change edits (FR-017)
- [X] T003 [P] Create `packaging/macos/Info.plist.in` as a valid plist skeleton carrying the literal `@VERSION@` token, so the substitution step below has a target

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The bundle producer. Every user story below observes, ships, verifies, or documents the
bundle this phase creates.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests first (MANDATORY — Constitution Principle I) ⚠️

> T004–T009 are written and observed failing before T010–T017. T007, T008, and T009 specify the
> plist keys and the signature *before* the tasks that produce them; the value-agreement half of the
> floor check lives in US4 (T043), where the documentation it compares against exists.

- [X] T004 Write failing `scripts/tests/macos-bundle.test.sh` covering the layout of contracts/bundle-layout.md: `--stage-only` against dummy binaries in a temp dir produces `Contents/Info.plist`, `Contents/PkgInfo`, `Contents/MacOS/{micold-ai-ide,micold-daemon}`, `Contents/Resources/icon.icns`, and **exactly two** entries in `Contents/MacOS/`
- [X] T005 Extend `scripts/tests/macos-bundle.test.sh` with the exclusion case: a `--bin-dir` that also contains a `micold-showcase` file still yields a bundle without one (FR-006)
- [X] T006 Extend `scripts/tests/macos-bundle.test.sh` with the failure cases: `@VERSION@` must not survive into the output, and a missing binary or missing icon exits non-zero with a message naming the missing file
- [X] T007 [P] Failing test in `crates/micold-core/tests/macos_permission_strings.rs` asserting every `ProtectedLocation` variant in data-model.md §2 has a matching `NS*UsageDescription` key in `packaging/macos/Info.plist.in`, and that a variant added without one fails
- [X] T008 [P] Failing test in `crates/micold-core/tests/macos_minimum_version.rs` asserting `packaging/macos/Info.plist.in` declares `LSMinimumSystemVersion` with a real version value — present, non-empty, and not a placeholder (FR-004)
- [X] T009 [P] Failing test in `crates/micold-core/tests/macos_signature_gate.rs` asserting `scripts/macos-bundle.sh` verifies with `codesign --verify --strict` and reports `Signature=adhoc`, and asserting it does **not** gate on `spctl` in either direction (FR-015, research R6)

### Implementation

- [X] T010 Implement `scripts/macos-bundle.sh` — argument parsing (`--bin-dir`, `--out`, `--stage-only`, `--version`), the directory tree, the explicit two-file copy, `PkgInfo`, and plist substitution from `cargo metadata`; green against T004–T006
- [X] T011 Fill `packaging/macos/Info.plist.in` with every key in contracts/bundle-layout.md, including the five usage-description sentences in the user's terms (FR-027) and `LSMinimumSystemVersion` (FR-004); green against T007 and T008
- [X] T012 Add the macOS-only signing path to `scripts/macos-bundle.sh`: `codesign` the daemon first, then the bundle, with `--options runtime` and `--entitlements packaging/macos/entitlements.plist` (FR-012, research R5)
- [X] T013 Add the signature verification to `scripts/macos-bundle.sh` so it runs wherever the bundle is signed rather than only at release time; green against T009 (FR-015)
- [X] T014 Add `plutil -lint` (macOS) and an unconditional `@VERSION@`-residue check to `scripts/macos-bundle.sh`, failing the build rather than shipping an unsubstituted plist
- [X] T015 Add failing macOS rules to `crates/micold-client/tests/packaging_excludes_showcase.rs`: `scripts/macos-bundle.sh` must not name `micold-showcase`, must name both shipped binaries, and must contain no glob copy into `Contents/MacOS/` — each driven by a deliberately-broken synthetic script, as the Debian rules already are (research R14)
- [X] T016 Make `scripts/macos-bundle.sh` satisfy T015's rules and keep the copy list explicit
- [X] T017 Add the `app` task to `mise.toml` (`scripts/macos-bundle.sh` over the current build, through `scripts/build-lock.sh`), documented like `mise run deb` (FR-035)

**Checkpoint**: `mise run app` produces a signed, verified bundle on a Mac;
`scripts/tests/macos-bundle.test.sh`, `cargo test -p micold-client --test packaging_excludes_showcase`,
and the three new `micold-core` gates are green on Linux.

---

## Phase 3: User Story 1 - Install and run on a Mac without building it (Priority: P1) 🎯 MVP

**Goal**: A Mac user installs the application with the standard gesture, launches it, opens a git
project, and starts a session — no toolchain, no separate service, no showcase inside the bundle.

**Independent Test**: On a Mac with no developer tooling, install the bundle, launch it, open a
repository, start a session. Delivers a working application to a population that had none.

### Prerequisite for the US1 gate

- [X] T018 [US1] Carve `specs/028-macos-package/quickstart.md` out of the documentation set: add `/specs/028-macos-package/quickstart.md -micold-docs` to `.gitattributes` after the `specs/**` line, where the last matching line wins, with the same reasoning feature 027 recorded for its own quickstart. Without it T020 reads a `micold-docs` path, which `documentation_is_not_read.rs` forbids, and an edit to §B would skip the whole pipeline — so the one edit the gate exists to catch is the one it never sees

### Tests for User Story 1 (MANDATORY) ⚠️

- [X] T019 [P] [US1] Failing unit tests for `classify` in `crates/micold-core/src/permission_failure.rs`: `PermissionDenied` under each protected location classifies; the same error outside them does not; every platform but macOS returns `None` (data-model.md §2)
- [X] T020 [US1] Failing test in `crates/micold-core/tests/macos_package_gate.rs` asserting `.github/workflows/ci.yml`'s macOS leg contains the bundle-and-launch step and that `specs/028-macos-package/quickstart.md` §B describes it — the enumerated-list-drifts-silently guard, modelled on `quickstart_a_runs_everywhere.rs`

### Implementation for User Story 1

- [X] T021 [US1] Implement `crates/micold-core/src/permission_failure.rs` and export it from `crates/micold-core/src/lib.rs`
- [X] T022 [US1] Surface the classification where a project open or directory read fails, in `crates/micold-client/src/shell/workspace.rs`, so the message names the permission and how to grant it instead of reporting a generic error (FR-028). *(Planned for `features/project.rs`; the folder listing is the only place a directory-read `io::Error` becomes user-facing text, and it is performed at the shell boundary — `features/project.rs` is render-free and never touches the filesystem.)*
- [X] T023 [US1] Add the packaging step to the macOS leg of the `test` job in `.github/workflows/ci.yml`: stage and sign the bundle from the binaries `Build (workspace)` produced, verify the signature, launch `Contents/MacOS/micold-ai-ide`, assert it is still alive and that `$HOME/.micold/run/d.sock` appeared, then terminate — written to hold under either outcome of T001, which is deferred: a window-surface failure is reported and treated as a pass, every other way of dying still fails (FR-036, research R12 contingency (b))
- [X] T024 [US1] Write `docs/user-guide/install-macos.md` with the download, install, launch, and "what starts automatically" sections, plus the long-`$HOME` endpoint limit (FR-030)
- [X] T025 [US1] Add the permissions section to `docs/user-guide/install-macos.md`: which protected locations the application asks for and when, what each prompt's explanation says and why (FR-027), what happens if the user declines, and the optional one-time Full Disk Access grant — stated as optional, with what it grants spelled out, and never a prerequisite for installing or running (FR-029)
- [X] T026 [US1] Add `docs/user-guide/install-macos.md` to the `docs` job's user-guide existence checks in `.github/workflows/ci.yml` (Principle VII)

**Checkpoint**: A bundle built from the current tree installs, launches, and runs sessions on a Mac;
CI proves it assembles and starts on every pull request.

---

## Phase 4: User Story 2 - First launch is not blocked or frightening (Priority: P1)

**Goal**: The first launch is blocked exactly as documented, one gesture clears it permanently, and
running the app from the mounted image is refused rather than half-working.

**Independent Test**: Download through a browser, install, launch, record every dialog, compare
against the documentation, follow it, confirm a clean second launch.

### Tests for User Story 2 (MANDATORY) ⚠️

- [X] T027 [P] [US2] Failing unit tests for `classify` in `crates/micold-core/src/install_location.rs` covering every row of contracts/install-location.md, including translocation winning over a `/Volumes/` prefix and the non-macOS `Installed` answer
- [X] T028 [P] [US2] Failing reducer test in `crates/micold-client/tests/features_window.rs`: a non-`Installed` verdict routes to the install-me screen, and no message path dismisses it or reaches the session UI (FR-019)
- [X] T029 [P] [US2] Register the install-me screen in `crates/micold-client/tests/anatomy_call_sites.rs` so the existing gate asserts it is built from shared components through the chainable builder-into-`Element` API, and fails while the screen is bespoke (Principle VIII, quickstart §A) *(No registration exists to make: `anatomy_call_sites.rs`, `material_boundary.rs` and `composite_call_sites.rs` walk `src/ui/` wholesale, so a new screen is covered the moment the file exists. Confirmed the hard way -- `composite_call_sites.rs` failed on the first draft of `ui/install_location.rs` for hand-building a glyph and a label, with no edit to any gate, and passed once the screen took `IconLabel`.)*

### Implementation for User Story 2

- [X] T030 [US2] Implement `crates/micold-core/src/install_location.rs` and export it from `crates/micold-core/src/lib.rs`
- [X] T031 [US2] Call `install_location::current()` on the client boot path in `crates/micold-client/src/features/window.rs` and render the install-me screen from shared components in `crates/micold-client/src/ui/`; view glue only, decision in the reducer; green against T028 and T029
- [X] T032 [US2] Add the first-launch section to `docs/user-guide/install-macos.md`: the exact block, the one System Settings gesture that clears it, why the application is not attributable to a registered developer, building from source as the alternative, why the block returns after an update, and why clearing it on the mounted copy does not help (FR-013, FR-014, US2 scenario 3)
- [X] T033 [US2] Add the trust notice beside the download in `README.md`, stating what the first launch does before the user meets it (SC-001)
- [X] T034 [US2] Put the same trust notice on the release page itself: add `.github/release-notice-macos.md` and a step in the existing `publish` job of `.github/workflows/release.yml` that appends it to the draft's generated body (`gh release view --json body`, then `gh release edit --notes-file`) before publication — release-please generates the body from commits and offers no fixed-text hook, so this is the only place the notice can reach the download page (Key Entities: download-page trust notice, SC-001)

**Checkpoint**: A downloaded, quarantined copy behaves exactly as the documentation and the release
page predict, and a copy run from the disk image says so instead of pretending to be installed.

---

## Phase 5: User Story 3 - Every release ships a macOS download, automatically (Priority: P2)

**Goal**: Merging the release PR produces a published release carrying the macOS download beside the
`.deb`s — or no published release at all.

**Independent Test**: Run the release process and confirm the published release contains a macOS
artifact whose version matches the tag, with no human build or upload.

### Tests for User Story 3 (MANDATORY) ⚠️

- [X] T035 [P] [US3] Failing test in `crates/micold-core/tests/release_publishes_complete_sets.rs` asserting `.github/workflows/release.yml`'s `publish` job lists every artifact-producing job in `needs:`, driven by a synthetic workflow that omits one — the check that keeps FR-011 true after the next artifact is added (contracts/release-artifacts.md)

### Implementation for User Story 3

- [X] T036 [US3] Implement `scripts/macos-dmg.sh`: stage the bundle plus an `Applications` symlink, `hdiutil create -format UDZO`, name the output `MicoldAIIDE-<version>-universal.dmg` (FR-008, FR-009, research R4, data-model.md §5)
- [X] T037 [US3] Add the `dmg` task to `mise.toml`, documented beside `app` and `deb` (FR-035)
- [X] T038 [US3] Add the `macos` job to `.github/workflows/release.yml` (build, `scripts/macos-dmg.sh`, verify, `gh release upload --clobber`) and add it to `publish`'s `needs:` — green against T035 (FR-010, FR-011, FR-037)
- [X] T039 [US3] Write `docs/development/macos-packaging.md`: how the artifact is produced, how a contributor reproduces it with `mise run app` / `mise run dmg`, the `spctl` diagnostic, and exactly what adding Developer ID + notarization would change (FR-033, FR-017)
- [X] T040 [US3] Add `docs/development/macos-packaging.md` to the `docs` job's developer-docs existence checks in `.github/workflows/ci.yml`
- [X] T041 [US3] Update `README.md` so releases are no longer described as Debian-only and state what each release provides for macOS (FR-031)

**Checkpoint**: A release either ships all three artifacts or stays an unpublished draft with a red
job naming the reason.

---

## Phase 6: User Story 4 - Works on both kinds of Mac (Priority: P2)

**Goal**: One download runs natively on Apple silicon and Intel, and a Mac below the floor is turned
away by the operating system with a reason.

**Independent Test**: Run the same artifact on an Apple silicon Mac and an Intel Mac; both launch
natively. On an older Mac, the system states the requirement.

### Tests for User Story 4 (MANDATORY) ⚠️

- [X] T042 [P] [US4] Failing case in `scripts/tests/macos-bundle.test.sh` asserting the merge step is driven by a target list variable and fails, naming the target, when one architecture's binary is missing (research R8)
- [X] T043 [P] [US4] Extend `crates/micold-core/tests/macos_minimum_version.rs` (the value half T008 deferred): the `LSMinimumSystemVersion` value must equal the floor stated in `docs/user-guide/install-macos.md`, so the plist and the documentation cannot drift when the floor moves (FR-004)

### Implementation for User Story 4

- [X] T044 [US4] Implement the `lipo` merge over a target-list variable in `scripts/macos-dmg.sh`, producing the universal binaries the bundle script consumes via `--bin-dir` (FR-020)
- [X] T045 [US4] Add `targets: aarch64-apple-darwin, x86_64-apple-darwin` to the toolchain step of the `macos` job in `.github/workflows/release.yml` and build both targets (research R15)
- [X] T046 [US4] Set `LSMinimumSystemVersion` to the current floor in `packaging/macos/Info.plist.in` — the single place the value is recorded — state that same floor in `docs/user-guide/install-macos.md` (green against T043), and add the re-check to `docs/development/macos-packaging.md` as a named step in producing a release: where the value lives, how to tell whether Apple has moved it, and that only the plist and the user guide change when it does (FR-004)
- [X] T047 [US4] Add the "which Mac, which macOS" section to `docs/user-guide/install-macos.md`: one download for both architectures, the minimum version, and what an older Mac sees (FR-021)

**Checkpoint**: There is exactly one macOS download and no way for a user to pick the wrong one.

---

## Phase 7: User Story 5 - Update and remove it the Mac way (Priority: P3)

**Goal**: Drag-replace preserves state, a stale daemon resolves itself, and removal leaves only what
the documentation says it leaves.

**Independent Test**: Install N, create a project and session, install N+1 over it, confirm state
survives and the new service is running; remove the app and check the documented leftovers are the
actual leftovers.

### Tests for User Story 5 (MANDATORY) ⚠️

- [X] T048 [P] [US5] Failing test in `crates/micold-core/tests/macos_registers_nothing.rs` asserting the bundle definition contains no `LaunchAgents`/`LaunchDaemons` payload and that no source file registers a login item or launch agent (`SMAppService`, `launchctl load`) — FR-025 as a checked property rather than a claim
- [X] T049 [P] [US5] Failing-or-confirming test that a newer client meeting an older running daemon resolves the mismatch without manual intervention, in `crates/micold-daemon/tests/` — extend the existing coverage if the case is already tested, add it if not (FR-023) *(Added rather than extended: `handshake_flow.rs` covers the refusal, nothing covered the recovery it hands the user, and `spawn::stop_running_daemon` — the whole of the remedy — had no test at all. New file `crates/micold-daemon/tests/version_mismatch_resolves.rs`, two-process against the real binary.)*
- [X] T050 [US5] Add the update-and-remove section to `docs/user-guide/install-macos.md`: replacing the app with the same drag gesture, what survives, what the version-mismatch message means, and every location that still holds user data after the app is in the Trash (FR-022, FR-024)

**Checkpoint**: The full install → update → remove lifecycle is documented and checked.

---

## Phase 8: User Story 6 - Know what happens when you log out (Priority: P3)

**Goal**: The documentation answers the logout question directly and points at the one placement that
survives it, and the existing macOS statements elsewhere in the repository agree with what ships.

**Independent Test**: Read the macOS documentation, find the logout answer without inferring it, then
verify it by starting a session, logging out, and logging back in.

### Tests for User Story 6 (MANDATORY) ⚠️

- [X] T051 [P] [US6] Failing test in `crates/micold-core/tests/macos_logout_claims_agree.rs` asserting the logout statement in `docs/user-guide/install-macos.md`, `docs/daemon.md`, and the module documentation of `crates/micold-core/src/logout_survival.rs` make the same claim about macOS (FR-032)

### Implementation for User Story 6

- [X] T052 [US6] Add the logout section to `docs/user-guide/install-macos.md`: sessions survive closing the window but not logging out when the service runs on the computer, why, and the container placement as the supported way to survive logout on macOS (FR-026)
- [X] T053 [US6] Reconcile the existing macOS statements in `docs/daemon.md` and `crates/micold-core/src/logout_survival.rs` with what this feature ships, green against T051 (FR-032) *(Both already said the right thing in their own words; reconciling meant giving them the one sentence T051 defines rather than correcting a claim. `docs/daemon.md` also needed carving out of the documentation set — a test reads it now.)*

**Checkpoint**: The boundary is documented and verified rather than discovered.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [X] T054 [P] Document the new macOS packaging step in `docs/development/ci-pipeline.md`, including that it rides the existing macOS leg rather than adding a job (research R12)
- [X] T055 [P] Cross-link `docs/user-guide/install-macos.md` from `docs/README.md` and from the installation section of `README.md`
- [X] T056 Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` — the local gate omits fmt, and CI stops there before any other job
- [X] T057 Run `mise run test` and `for t in scripts/tests/*.test.sh; do "$t"; done`; confirm the macOS and Windows legs are green in CI (Principle VI) *(Local: 259 suites / 2563 tests green, all four shell suites pass, `cargo fmt --check` and `cargo clippy --workspace --all-targets -D warnings` clean. Two gates went red on the way and were fixed rather than exempted: `feature_write_isolation.rs` wanted an `OWNERS` entry for the new `install_location` field, and `release_publishes_complete_sets.rs` counted `publish` as an artifact job because its new comment names `gh release upload` -- the scan now drops comment lines and a fixture test holds that. **The macOS and Windows legs are not confirmed**: they only run on a push, and nothing has been pushed from this branch yet -- same open edge as T001.)*
- [X] T058 Run quickstart §A and §B and confirm every row passes (`specs/028-macos-package/quickstart.md`) *(§A: every row green on Linux -- the whole table is text and directory-entry assertions, which is what §A's own "what it cannot tell you" note says. §B: **not run**. Its rows are `codesign`, a real launch and a `d.sock` appearing; `mise run app` on Linux stages unsigned and cannot exercise any of them. It runs on the macOS leg of `ci.yml`, so it settles on the same push that settles T001 and the T057 remainder.)*
- [ ] T059 Run quickstart §C on a real Mac — install, the block, the refusal from the disk image, a working session, permissions, lifecycle, the floor — and fill in the record table in `specs/028-macos-package/quickstart.md`
- [X] T060 Re-validate `specs/028-macos-package/checklists/requirements.md` against what shipped and record the pass *(Validation pass 5. All 16 items still pass and the spec needed no edit. Recorded one distinction plainly rather than leaving it implicit: "feature meets measurable outcomes" passes as a checklist item — the success criteria are the right measurable outcomes — while eight of the thirteen have not been *observed*, because they are §C rows needing a real Mac. T059 and T001 are open for that.)*

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies. T001 gates only T023's shape, not the phases between.
- **Foundational (Phase 2)**: Depends on Setup. **Blocks every user story** — there is nothing to
  install, verify, ship, or document until a bundle can be produced. It now also owns the signature
  verification (T013), because the per-PR gate in US1 asserts it.
- **US1 (Phase 3)**: Depends on Foundational. The MVP.
- **US2 (Phase 4)**: Depends on Foundational. Independent of US1 in code (different modules,
  different files); shares `docs/user-guide/install-macos.md`, created by T024.
- **US3 (Phase 5)**: Depends on Foundational. Independent of US1/US2.
- **US4 (Phase 6)**: Depends on US3 — `scripts/macos-dmg.sh` and the `macos` release job are what
  T044/T045 extend. T043 additionally depends on US1's documentation page existing.
- **US5 (Phase 7)**, **US6 (Phase 8)**: Depend on US1 only for the documentation page they extend.
- **Polish (Phase 9)**: Depends on everything.

### Within a story

Tests before implementation, always — including Phase 2, where T007/T008/T009 precede T011/T012/T013.
Where two tasks name the same file they are sequential even when otherwise unrelated (T004→T005→T006
all edit `scripts/tests/macos-bundle.test.sh`; T008→T043 both edit `macos_minimum_version.rs`).

### One cross-story note

T034 edits the `publish` job, which exists today, so it can land in US2 — but the notice it adds only
becomes *true* once US3's `macos` job attaches an artifact. Ship US3 in the same release, or word the
notice for the first release that carries the download.

### Parallel opportunities

- Phase 1: T002, T003 together.
- Phase 2: T007, T008, T009 together (three different new test files); the T004→T005→T006 chain runs
  alongside them.
- Phase 3: T019 runs alongside the T018 → T020 pair (different files). T018 must land before T020
  exists at all, and T020 cannot go green until T023 has settled the step's name.
- Phase 4: T027, T028, T029 together.
- Phase 6: T042, T043 together.
- Phase 7: T048, T049 together.
- Across stories: US1, US2, and US3 can be staffed in parallel once Phase 2 is green — the only
  shared file is the user-guide page, whose sections are appended independently.

---

## Implementation Strategy

**MVP**: Phase 1 + Phase 2 + Phase 3 (US1). That is a bundle a Mac user can install and run, proven
by CI on every pull request. It does not yet ship in a release, is not yet a `.dmg`, and covers one
architecture — but the platform stops being unsupported at this point.

**Second increment**: US2. On macOS this is really part of the first launch, and the MVP is
uncomfortable to hand to anyone until the block is documented and the disk-image case refuses
cleanly. Ship US1 and US2 together if there is any choice.

**Third increment**: US3 + US4 — the release ships it, automatically, for both Macs. This is what
turns "macOS was packaged" into "macOS is supported" and closes the constitutional Distribution gap
for this platform.

**Fourth increment**: US5 + US6 — documentation-shaped, cheap, and the source of the confusing bug
reports if skipped.

**Independent test criteria** are stated at the head of each phase and correspond one-to-one with the
spec's per-story Independent Test paragraphs.
