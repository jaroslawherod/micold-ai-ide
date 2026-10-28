---
feature: daemon-config-file
loop: inside-out
profile: .specify/memory/tdd-profile.md
spec_criteria: 0
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
| A2 | An invalid file exits 2 with `file:line:col` per error | US1, FR-003, FR-018 | DONE |
| U1 | Defaults per kind (data_dir, idle_stop, isolate_host_helpers, ssh.dir, clone_root, startup) | FR-001 | DONE |
| U2 | V1 unknown key (nested, array table, top level, retired `git.base`) | FR-003 | DONE |
| U3 | V2 schema missing / newer (names both) | FR-004 | DONE |
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
| U20 | `config check` flags: --kind, --files, usage, unreadable file | FR-018 | DONE |
