# Autopilot ledger — #488 codex-opencode-providers

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement issue #488: Add Codex CLI and OpenCode as session providers; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #488
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-09
- **Phase**: milestone M6
- **Next step**: M6 gate + reviews A and B, then the orchestrator pushes and opens the PR from scratchpad/pr-488-m6.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #679 | Design (spec, plan, tasks) | MERGED (rebase) | 96f85d69d1e0d335f23e44fc93097483b913a5d7 |
| #680 | M1 | MERGED (rebase) | cf3881fd2bad0e79bc61c9be3669b77f3c6ef2df |
| #683 | M2 | MERGED (rebase) | eef53e0c7d6e501e7bd3fb0a6184ebade47c8253 |
| #684 | M3 | MERGED (rebase) | e9abd92efe2c7d8ad097ff82d4a7073477349524 |
| #690 | M4 | MERGED (rebase) | 26dbd73670583e4e4826b0c80f972359072deaf2 |
| #691 | M5 | MERGED (rebase) | 314405a5bc0b249ea033e6dfc1a21d0aef2d2872 |
| (orchestrator opens) | M6 | pending | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Start Codex/OpenCode, remembered, wire 39 | #680 | merged |
| M2 | T012–T015 | light | Unavailable providers explained | #683 | merged |
| M3 | T016–T022 | full | Codex resume + naming (seam) | #684 | merged |
| M4 | T023–T026 | full | OpenCode resume + naming | #690 | merged |
| M5 | T027–T030 | light | Honest activity, tool-server, first prompt | #691 | merged |
| M6 | T031–T034 | full | Sandbox image + sign-in | pending (orchestrator opens) | in progress |

## Decisions

- M1 T001 probe (npm install worked): codex 0.162.0, opencode-ai 1.18.35; V8/V10 fields/V13/V14/V15 need a signed-in session so were not probed, fallbacks stay; OPENCODE_DISABLE_AUTOUPDATE=1 set in launch_env; Codex has no env switch. M1 launches Fresh and Resume identically (no args) until M3/M4.
- Tasks unit: US1/US2 split (rule 3) and US3 split Codex/OpenCode; US6 docs ride in each story's milestone; Polish (T035) left to the close unit. Analyze findings I1,I2,C1,C2,T1,A1–A3,D1–D3 fixed in spec/tasks; SC-001 reworded (no number to measure).
- M2 T012/T013: characterization tests passed on existing behaviour (daemon: unavailable listing, MCP and app-path refusal naming the command with no record/terminal, availability after install without restart, removed provider keeps its provider); no code change (T013). T014 client test added in missing_cli_is_reported_where_it_is_chosen.rs.
- M2 visual pass: chooser and Settings follow the 037 design (missing CLIs named with the reason in a note, not as dimmed entries; chooser note only when 2+ CLIs are available). Kept; not changed in M2.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Decisions (M3)

- M3: seam `launch_args_in` takes `Option<&Path>` config dir; `sole_candidate` (core) is the bind-only-if-one rule; daemon `bind_minted_conversation` polls from `ops::start_session`, refuses to bind while another running same-provider same-cwd session is unbound; Minted providers skip the start-time "conversation gone" refusal (unbound = fresh). Codex naming via `read_title` (`first_turn::codex_first_turn`, 64 KiB); V8 line shape still unconfirmed against a signed-in CLI. Tests written with code, not red-first (honest note). OpenCode is Minted with fresh-only launch until M4.

## Decisions (M4)

- M4: OpenCode is read through the `opencode` CLI on the daemon's `PATH` (`session list --format json -n 50`, `export <id>`; 2 s, 256 KiB, exit 0 only). V10 field names are assumed: `id`, `directory`, and `created` or `time.created` (ms); export `messages[].info.role` / `parts[].text` (non-synthetic). Name = first typed turn (OpenCode's own title may be a placeholder). `has_recorded_conversation` = the binding exists (no spawn per sweep). Daemon tests extend codex_resume.rs with a stand-in `opencode`.

## Review rounds

- M4 A round 1 dfef1b4f…:d963162a: CHANGES (F1 MAJOR current_dir, fixed); round 2 (sonnet, fix diff): CLEAN.
- M4 B round 1: CHANGES, sole BLOCKER was that the daemon Verify had not run in the gate (it stopped at a root-only failure); re-run with --no-fail-fast: codex_resume 7/7 green. F2 MINOR (daemon tests not red-first) disclosed in cycle-log.

- M2 review A (fresh, high): CLEAN, 3 MINOR (MCP/app same-reason assertion added; comment fixed; 'as the default' dropped from docs). Review B (sonnet): could not run cargo (denied in its sandbox); the unit ran both Verify commands green itself; only MINOR otherwise. No counted rounds beyond 1.

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Plan | 1 | 528308c54088c27ab49cdd224d332fc9a3bf590f:e5185f251b532f03ff50c09d75a2730c42efbaab | CHANGES: 3 MAJOR fixed (minted-id design via binding files; required trait methods; CredentialLayout Vec),  2 MINOR fixed |
| Plan | 2 | 48a1570d5b81aa926304890b0d296d225ada78ce:e5185f251b532f03ff50c09d75a2730c42efbaab | CLEAN (2 MINOR, fixed prose only) |
| Spec | 1 | 7c296cdcbfffe844d73667e0a7202ac201a4cd1c:cb0c493ce4098522100c5f5837df60a8b6d37777 | CLEAN (3 MINOR, fixed prose only) |
| Tasks | 1 | e22c6bd38c87bb5d49e9aab99ac717d28b8552ef:96244711417951fe05bf6105add19e0cbd91cbfb | CLEAN (3 MINOR, fixed prose only; T035 covers FR-014/SC-006 in close) |
| M1 A (reviewer, sonnet) | 1 | 43ed7a7abe06108b9f82c3a5b75739a1068c7c3f:96f85d69d1e0d335f23e44fc93097483b913a5d7 | CHANGES: 1 MAJOR fixed (ai_cli error text + schema enum now from AiCli::ALL), 2 MINOR fixed (comment-tolerant trust header, docs) |
| M1 A (reviewer, sonnet) | 2 | a959e0e9f42b1e0fa2053a46082bfc93cb27c4b6:67df06aff34a0c102150116786686f8522188556 | CLEAN (2 MINOR: TWO const renamed WITHOUT_PI; `]` inside a trust-header comment declined) |
| M1 B (conformance, sonnet) | 1 | same | CHANGES: only BLOCKER was that the reviewer could not run cargo; the orchestrating unit ran Verify itself (green); 1 MINOR fixed |

- M3 A round 1 (snapshot 78ef01f81aea703b662d10bf1504a13767ac97cb:da63a8225eeafb577113692d3e8f72f482b3b49e): CHANGES, F1 MAJOR (shell peer blocks bind) fixed, F2/F4 fixed, F3 declined (below). Round 2 (snapshot 6caadbfccef19c0db653d5d9a08f26be5a0fe58e:af01facf278a685d91b9d2c4763e39b8d37c0715, sonnet, fix diff): CLEAN.
- M3 B round 1 (same snapshot, sonnet): CHANGES, F1 MAJOR (no TDD cycle log) fixed with tdd/cycle-log.md, F2/F3 fixed. Round 2: CLEAN, 2 MINOR left (env mutation in the terminal_backend test: that file's other tests use claude/copilot only; test-first order not followed, stated in the cycle log).
- M3 visual pass: not run, no UI change.
- M5 A (reviewer, sonnet) round 1 (snapshot cdae90711bbfb1ce934bf3ee8ad715c6fb97e802:940a8f2ad1470dd16f27b9346b69e3806dafaa56): CLEAN, 3 MINOR (cycle-log cell fixed; wrap and 2 s sleep left). M5 B (conformance, sonnet) round 1 (same snapshot): CLEAN, 2 MINOR (cycle-log cell and AS2 mapping fixed). M5 visual pass: not run, no UI change. T029: no code change.

## Decisions (M6)

- The sign-in mount is writable (rule N-4, `CredentialShare::writable`), not read-only as research R5 assumed; kept for every CLI so a refresh persists as for Claude Code (FR-013 "writing nothing existing providers would not write"). Guide says so. A read-only mount would break claude.
- `unshared_sign_in` still reports Claude Code's file only (`ai_cli_auth.first()`); the settings page names one path.

- M6 A round 1 f1744874…:b4825904: CHANGES (F1 MAJOR onboarding_record keyed on any sign-in, fixed to Claude's token; F2/F3 MINOR fixed). B round 1 (sonnet): CLEAN (MINORs: doc comment order, read-only wording, T034 text, all fixed). Gate found 6 existing sandbox tests assuming one sign-in file; updated.

## Declined review findings

- M4 review A F2 (MINOR, output cap checked after buffering): declined, the 2 s timeout bounds it; documented on `run_json`.

- M1 A round 2, MINOR trust.rs `project_header` uses the last `]`, so a comment containing `]` after the header is not read as trusted: declined, it fails closed (the project is simply not treated as trusted) and the file is Codex's own.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

- M3 A F3 (two simultaneous same-folder Codex sessions both stay unbound): intended never-guess rule (FR-006); user guide says so.

## Handover

None.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

None yet.
- M2 gap vs FR-003 wording / plan.md:68: Settings dropdown lists only available providers, the others named in the note; with a single installed CLI the chooser has no chevron and no note, so a missing provider is not explained there. Existing 037 behaviour; decide whether a later milestone or the close unit should list unavailable entries.
