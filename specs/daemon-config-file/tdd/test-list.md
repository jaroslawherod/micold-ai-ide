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
Baseline: `cargo test -p micold-client --test daemon_config_render --test daemon_config_stable --test daemon_config_validate` green at M2 start (4 + 2 + 5).

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
