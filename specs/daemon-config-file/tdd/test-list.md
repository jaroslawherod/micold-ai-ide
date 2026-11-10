---
feature: daemon-config-file
loop: inside-out
profile: .specify/memory/tdd-profile.md
spec_criteria: 5
planned_at: 2b6335441
updated_at: 2b6335441
suite_baseline: unknown
---

# Test list: Issue 2 (#1523), milestone M1

Derived at the milestone unit (no tasks-unit list was written). Scope: `micold-core::daemon_config`
and `micold-daemon config check`. Issues 3-15 add their own behaviors under their own headings.

| ID | Behavior | Traces to | State |
|---|---|---|---|
| A1 | The four contract §3 examples parse; `config check` exits 0 on each | US1, FR-001 | DONE |
| A2 | An invalid file exits 2 with `file:line:col` per error | US1, FR-003, FR-017 | DONE |
| U1 | Defaults per kind (data_dir, idle_stop, isolate_host_helpers, ssh.dir, clone_root, startup) | FR-001 | DONE |
| U2 | V1 unknown key (nested, array table, top level, retired `git.base`) | FR-003 | DONE |
| U3 | V2 schema missing / newer (names both) | FR-003 | DONE |
| U4 | V3 listen shape and kind | FR-017 | DONE |
| U5 | V4 relative path / `~` | FR-017 | DONE |
| U6 | V5 duplicates; V6 equal-specificity tokens accepted | FR-017 | DONE |
| U7 | V7 CLI capabilities, version shape | FR-017 | DONE |
| U8 | V8 daemon-owned args | FR-017 | DONE |
| U9 | V9 one credential, token_file CLIs | FR-017 | DONE |
| U10 | V10 settings_file/place on host/ssh, place.as escape | FR-017 | DONE |
| U11 | V12 remote needs clone_root | FR-017 | DONE |
| U12 | V13 timeout range | FR-017 | DONE |
| U13 | V14 secret mode (warning host/ssh, error sandbox) | FR-032 | DONE |
| U14 | V15 missing required file | FR-017 | DONE |
| U15 | V16 reserved names | FR-042 | DONE |
| U16 | V17 placed MCP config naming `micold` | FR-042 | DONE |
| U17 | V18 session.files / ssh key rules | FR-042 | DONE |
| U18 | V19 machine sign-in kind and exclusivity | FR-042 | DONE |
| U19 | SC-005: Debug/Display of config, SecretPath, view never show content | SC-005 | DONE |
| U20 | `config check` flags: --kind, --files, usage, unreadable file | FR-017 | DONE |

# Test list: Issue 4 (#1525), milestones M1-M2

Scope: `micold-client` `daemon_config_render`, `features::daemons` config check, the Daemons row.
Baseline: `cargo test -p micold-client --test main daemon_config` green at M2 start (4 + 2 + 5 at M2 start; 19 after the close PR).

| ID | Behavior | Traces to | State |
|---|---|---|---|
| I4-A1 | Each kind renders byte for byte to its golden file | US1, FR-005 | DONE |
| I4-A2 | A rendered file parses back to every setting it was rendered from | US1, FR-005 | DONE |
| I4-U1 | A value with an apostrophe and a backslash, a control character, an empty value or a quote survives the file | FR-005 | DONE |
| I4-U2 | Keys are written in the contract's fixed order | FR-005 | DONE |
| I4-U3 | A change to the settings changes the bytes | FR-005 | DONE |
| I4-U4 | `from_entry` gives the kind, name, listen and token path of each runtime | FR-005 | DONE |
| I4-U5 | An invalid file yields the errors of `config check` and is not delivered | FR-013f | DONE |
| I4-U6 | The errors reach the row's view model; a valid file clears them; a save checks the file and shows none for a valid one | FR-013f | DONE |
| I4-U7 | Each config error adds a line to the Daemons row | FR-013f | DONE |
| I4-D1 | `from_entry` builds the file from the daemon's saved settings; an invalid one saved in the form shows its error in the row | FR-005, FR-013f | DONE (T117; checking at registry load waits for delivery, 5-7) |

## Test list: Issue 5 (#1526), milestone M1a

Baseline: scoped `micold-client` and `micold-core` suites green on `origin/main` before M1a. Traces for U21-U24 and T080a: FR-005, FR-014, FR-029.

Host delivery before spawn (write-if-changed, mode 0600, no added env) and feeding a runtime-added
project into the next write.

| ID | Behavior | Traces to | State |
|---|---|---|---|
| U21 | `deliver_host` writes `<data_dir>/daemon.toml` mode 0600 on first call; a second call with unchanged settings is a no-op (file untouched) | FR-005 | DONE |
| U22 | A settings change (e.g. a runtime-added project) makes the next `deliver_host` call write again | FR-005, FR-014 | DONE |
| U23 | `spawn_detached_daemon` adds no `MICOLD_*` (or any) env beyond the test process's own (regression guard) | FR-005 | DONE |

## Test list: Issue 5 (#1526), milestone M1b

| ID | Behavior | Traces to | State |
|---|---|---|---|
| U24 | `ensure_build` for a user host uploads `daemon.toml` and each named secret by content hash; unchanged content sends identical commands, a rotated secret changes only its own; the remote script writes `0600`/`0700` and skips a matching hash | FR-005, FR-029 | DONE |

## Test list: Issue 14 (#1535), milestone M1

Baseline: `micold-client` `ui::daemon_details_dialog` 6 unit tests, `daemon_dialog_buttons` 6, `daemons_section_geometry` 22 (green at `0ad33a4e5`).

| ID | Behavior | Traces to | State |
|---|---|---|---|
| I14-1 | The details dialog keeps the full no-overflow assertion; only text in the delivered file's scroll container is exempt, and that exemption is itself asserted | US6, SC-005 | DONE |
| I14-2 | The dialog offers Restart to apply only when the status says `restart_needed`; painted text holds the delivered file and the resolved values | US6, FR-029 | DONE |
| I14-3 | The start script's output tail appears in no rendered line or row | US6, FR-029 | DONE |
| I14-4 | The start-script line is exact for every outcome and reason (`Ok`, `Skipped`, `Warned`, `Failed`; `Exit`, `Signal`, `Timeout`, `Spawn`, `FileMissing`) | US6, FR-029 | DONE |
| I14-6 | The user guide and the code comment say what ships: outcome, reason and time, no variable names | US6, FR-029 | DONE (docs) |

## Test list: Issue 10 (#1531), milestone M1

TDD verification (`tdd/verification.md`, Finding 2) found this milestone missing from this file entirely; added after the fact, at close-unit audit — see the matching note in `cycle-log.md`, there is no genuine red-phase evidence for these rows.

| ID | Behavior | Traces to | State |
|---|---|---|---|
| T055 | `micold-daemon gh-token --cwd <dir>` prints the token chosen for the cwd's `origin` (token line, then host line); exits 1 silently with none | US2, FR-011, FR-035, SC-003 | TEST_AFTER |
| T056 | The `gh` shim at `<data_dir>/bin/gh` (unix `sh`, Windows `gh.cmd`) execs the real `gh` with `GH_TOKEN` set from the chosen token, leaving an existing `GH_TOKEN` alone | US2, FR-011, FR-035, SC-003 | TEST_AFTER |
| T056 (PATH) | `<data_dir>/bin` is first on the session `PATH`, for every configured daemon kind | US2, FR-011, FR-035, SC-003 | TEST_AFTER (T148 adds the "prefix present" unit test; T149 adds the kind-agnostic one — see `tdd/verification.md` Finding 1 and 4) |
| T095 | The `gh` shim also sets `GH_HOST` from the chosen token's host (D15 row 7), left alone with no token chosen, same rule as `GH_TOKEN` (T056) | US2, FR-011, FR-035, SC-003 | TEST_AFTER |

## Test list: Issue 9 (#1530), milestones M1 to M3 and the close remediation

Baseline: `micold-core --lib` `daemon_config::matching` and `scoped_credential::` 81 green at `86468d87c`, before the close unit's tests below. Traces: US2, US3, FR-008, FR-010, FR-016, SC-004. Rows B1-B10 were derived from the tasks by `tdd/verification.md` (Issue 9): they landed with their code (`TEST_AFTER`, no red recorded). The T141-T146 rows are the tests the close unit added; their red state is in `cycle-log.md`.

| ID | Behavior | Traces to | State |
|---|---|---|---|
| B1 | `TokenChoice` order: most specific wins, ties go to the first in the file, expiry and reach respected | US2, FR-008 | DONE (TEST_AFTER) |
| B2 | Reach discovery and cache by token fingerprint | FR-008 | DONE (TEST_AFTER) |
| B3 | The helper is stateless: reads `MICOLD_DAEMON_CONFIG`, a replaced token is used by the next call | US2, SC-004 | DONE (TEST_AFTER) |
| B4 | Fallback helpers: untouched git when no token matches | US2, FR-010 | DONE (TEST_AFTER) |
| B5 | A sandbox clears inherited helpers | US3, FR-010 | DONE (T141 runs it through real git) |
| B6 | Session git env per `daemon.kind` | US3, FR-010 | DONE (TEST_AFTER; T141 runs it through real git) |
| B7 | Per-host rewrite and token `host` | FR-010 | DONE (TEST_AFTER) |
| B8 | `container_git_edits` by `daemon.kind` on both start paths | US3, FR-010 | DONE (TEST_AFTER; T142 removes the order dependence) |
| B9 | `[git] config_file` delivered, `commit.gpgsign` and `tag.gpgsign` stripped | US3, FR-010 | DONE (T143 asserts the delivered copy) |
| B10 | Grant messages retired, protocol bump | FR-016 | DONE (TEST_AFTER, compile-time) |
| T141 | `session_git_env` applied to real `git ls-remote`: chosen token authenticates, an unreached repo falls back, a sandbox never runs the planted helper | US3, FR-010 | DONE |
| T143 | The delivered gitconfig copy loses `commit.gpgsign` and `tag.gpgsign` on the sandbox and pod paths | US3, FR-010 | DONE |
| T144 | `credential_config::answer`: stale entry refreshed after 15 min, failed request keeps the last answer, replaced unfiltered token gets its own entry and the old one is pruned | US2, FR-008 | DONE |
