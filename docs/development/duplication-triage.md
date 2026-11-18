# Duplication triage (issue #645)

Triage of every duplication cluster found by `mise run duplication` and the gap measurements of
[#645](https://github.com/jaroslawherod/micold-ai-ide/issues/645), done 2026-10-09 on `main`
(`6c43ef7d`). Doc only: no code changed, no child issue filed. Each table row is written so it can
become a child issue as it stands: files, what is duplicated, type, fix, lines, risk.
Tools and budgets: [duplication.md](duplication.md).

> **Status (#1959):** the iced desktop app (`micold-client`, `micold-material`) was removed. The
> widget, client-dialog and showcase sections and the client-only test rows (T1, T6-T9) are
> deleted; rows that remain cover core and daemon code. Counts and ranks below are as measured on
> 2026-10-09 and were not re-run.

## How to read it

**Types** (from the issue): **1** same concept, same behaviour: one shared fn/component. **2** same
shape, different data: table- or descriptor-driven. **3** same boilerplate around different logic:
move it into a type, trait or helper. **4** looks alike, evolves separately: **leave**, with the
reason stated. **5** reinvented helper: use the existing one. **6** tests: shared fixtures.

**Lines** is the net removal after the new helper is paid for, estimated by reading the code. Raw
jscpd clone sizes overstate it: pairs are counted twice and many hits are 6-14 line fragments of a
longer shape. **Risk**: low (additive, private, covered by tests), low-med, med (persistence, ordering
or a cross-crate API), med-high (visual or user-data behaviour), high (design decision or security).
**Score** = lines / risk weight (low 1, low-med 1.5, med 2, med-high 2.5, high 3). **Rank** orders all
actionable rows by score (ties: lower risk first); rank 1 goes first.

Rules from the issue hold for every child: no behaviour change, `mise run gate` green, net lines
removed reported in the PR, and lower the budget in `mise.toml` when the number drops.

## What the measurements say

Re-run 2026-10-09 (`jscpd --min-lines 5 --min-tokens 40`, tests included): 287 clones, 3.16 % of
lines; `similarity-rs`: 753 pairs. Findings that change the plan:

- Production code holds about **1,230** removable lines; test code about **2,750**. The issue's
  "93 % of the 6,810 lines are tests" holds, so the biggest payoff is shared test support, not widgets.
- Several claims of the issue comment were **refuted** by reading the code (section Helpers, H2, H4,
  H10 partly, H11). A sample of `similarity-rs` pairs (`worktree_form`, `daemon_sync`, `provider`,
  showcase) are shape coincidences: left (type 4).
- Total actionable: about **3,950** lines across about 50 rows; the type 4 rows are left on purpose.

## Order of attack

Ranks, best first. Tests first (cheap, safe), then the one widget helper, then small production tails.

| Rank | ID | Lines | Risk | One-line fix |
|---|---|---|---|---|
| 2 | T2 | 500 | med | daemon `tests/support/conn.rs` (Hello handshake) |
| 3 | T4 | 450 | med | merge the two attention support modules |
| 8 | T5 | 120 | low | `settings_set(req)` test constructor |
| 10 | T10 | 100 | low | in-file test setup helpers |
| 11 | T3 | 200 | med | `review_send.rs` onto `support/runs.rs` |
| 17 | K6 | 40 | low | `change_settings` in daemon `state.rs` |
| 19 | K4 | 35 | low | MCP session-op confirm helper |
| 24 | K16 | 22 | low | `declare_table` in tokens CSS |
| 28 | H8 | 18 | low | inline store/settings path wrappers |
| 31 | K14 | 15 | low | `git_output` / `git_failed` |
| 33 | H9 | 13 | low | delete `platform::write_owner_only` |
| 35 | K2 | 12 | low | `env_or_home` in provider |
| 36 | H10 | 18 | low-med | `style::to_rgb` |
| 39 | K10 | 28 | med-high | terminal-history `remove_tracked` |
| 40 | H1 | 10 | low | `unix_secs()` in core |
| 45 | H5 | 8 | low | shared `pathext()` |
| 47 | H7 | 6 | low | shared `hex` |
| 48 | K11 | 12 | med | `read_or_quarantine` |
| 49 | K22 | 12 | med | logging `install` |
| 50 | H3 | 5 | low-med | `paths::data_subdir` and app-id const |

Overlaps: C1 and C2 share the dialog tail (count about 80, not 95); W8, W2 and W7 build on W1; T4
needs T2's `support/conn.rs`; T5 needs T2.

## Core and daemon

| ID | Files / functions | What is duplicated (verified) | Type | Proposed fix | Lines | Risk |
|---|---|---|---|---|---|---|
| K1 | `core/provider.rs` Claude 689-714, Copilot 1002-1027, Pi 1396-1421, OpenCode 1934-1960 | Byte-identical `launch_args_in`, `new_conversations`, `bind` stubs. | **4 leave** | The trait doc says no method has a default, on purpose (FR-021). Removing about 80 lines reverses a stated design decision; a human must reopen it. | 0 (80 if reopened) | high |
| K2 | `provider.rs` `config_dir` for Claude 716, Copilot 1029, Pi 1423 | Each repeats "env var if non-empty else home-relative"; `env_dir` (1576) exists and Codex/OpenCode use it. | 5 | `env_or_home(env, rel)` built on `env_dir`. | 12 | low |
| K3 | `provider.rs` `last_activity`, `activity_source` impls | Only a 3-line `metadata().modified()` repeats. | **4 leave** | The path logic changes with each CLI. | 0 | - |
| K4 | `daemon/mcp/tools.rs` `stop_session` 363, `interrupt_session` 404, `delete_session` 455, `send_session_input` 1273, `start_session` 1133-1203 | Same `from_uuid`/`resolve_session_target`/`policy_for` prelude and an 11-line `ask_user(..)` block four times; the session row built twice. | 3 | `confirm_session_op(..)` and `session_row(..)` in the same file. | 35 | low |
| K5 | `daemon/mcp/tools.rs` `rename_worktree`, `start_session` | Only the 3-line `resolve_caller` + `check_policy` prelude. | **4 leave** | Outcome mapping, errors and ordering (policy first, live refresh) differ per tool. The issue's "~86 % similar" is shape only. | 0 | - |
| K6 | `daemon/state.rs:1794-1990`: nine `set_*` setters | Same 8 lines: lock, `catalog.set_x(v)?`, `settings_wire()`, broadcast `SettingsChanged`. | 3 | Private `change_settings(&self, f)`; leave `set_desktop_notifications`, `set_save_terminal_history`, `set_env_include` (extra steps). Keep lock-then-broadcast order. | 40 | low |
| K7 | `daemon/state.rs` `review_send` 2072 / `review_send_to_new_session` 2145 | Log, close the send, map `Undelivered`. | **4 leave** | One delivers to an existing session, the other creates one; `started` and outcome strings differ; error enums differ. | 0 | - |
| K8 | `state.rs` `recover_session_names` / `recover_live_session_names`; `wait_ready_for_input` / `wait_output_settled`; `review.rs` `forget_worktree` / `abort_send` | Shared part already factored out, or polling-loop shape only. | **4 leave** | - | 0 | - |
| K10 | `core/terminal_history/store.rs` `retry`, `forget` 173, `sweep` 205, `delete_all` 224 | Same `remove_file` match; differences are deliberate (retry-set re-insert on failure; `sweep` skips on NotFound). | 3 | `remove_tracked(&self, state, name, session, Retain)`. | 28 | med-high: deletes user data (FR-023/024) |
| K11 | `core/store.rs` `load_reviews` 757 / `load_runs` 808 | Read, default on error, parse, quarantine to `.corrupt` on parse failure. | 3 | `read_or_quarantine<T: Default>(path, parse)`. | 12 | med: persistence recovery |
| K12 | `core/store.rs` mirror `Stored*` structs, `runs/store.rs` `From` pairs | Field-by-field conversion. | **4 leave** | Persisted-schema types are decoupled on purpose so a model change cannot alter the disk format. | 0 | - |
| K13 | `daemon/catalog.rs` `save_reviews`/`save_runs`, `mark_session_stopped`/`running` | 8-10 line wrappers. | **4 leave** | Lifecycle guards differ. | 0 | - |
| K14 | `core/git.rs` `run_git` 332, `remote_list` 531, `branch_exists` 379; `core/review/git.rs` `failed()` 50 | `"git {} failed: {}"` formatting and hand-built `no_window(Command::new("git")).arg("-C")..` three times. | 5 | `git_output(repo, args)` and `git_failed(args, &Output)` in `git.rs`; `branch_exists` keeps the exit code via `git_output`. | 15 | low |
| K15 | `core/git.rs` `worktree_add_*` 398-465 | Argument lists differ. | **4 leave** | Each is a distinct `git worktree add` form with a comment on why; a table hurts readability. | 0 | - |
| K16 | `core/tokens/css.rs` 133-270: `type_scale`, `corner_radii`, `motion`, `states`, `spacing_scale` | Loop over a `(name, value)` table calling `declare(..)`. | 2 | `declare_table(out, prefix, unit, items)`. | 22 | low: CSS snapshot tests |
| K17 | `core/mcp/tools.rs` extractors 840-1036 | Same `match args.get(key)` opening; error wording differs and is pinned by contract tests. | 3, **skip** | `optional_with` is possible but reads worse. | 0 | - |
| K18 | `core/mcp/tools.rs` `audit_target` 250 / 299; `review/diff.rs` `UnifiedIndex`/`SideIndex` | Same method names on different types. | **4 leave** | Different structures and match arms. | 0 | - |
| K19 | `core/sandbox/*` message tables, real vs recording `CommandRunner` | Per-variant message tables; trait signature only. | **4 leave** | Evolve with each error variant. | 0 | - |
| K22 | `daemon/logging.rs` `init` 255-336 | journald / terminal / no-dir / file branches each repeat `registry().with(..).try_init()` plus a `Logging{..}` literal. | 3 | Local macro or boxed layers (layer types differ). | 12 | med: generic bounds |

## Helpers (type 5), verified against the issue comment

| ID | Claim | Verdict | Fix | Lines | Risk |
|---|---|---|---|---|---|
| H1 | Unix seconds written 4 times | **Confirmed.** `daemon/state.rs:428`, `client/ui/sidebar.rs:864`, `client/shell/pr_status.rs:54`, inline in `daemon/logging.rs:94`. `core/clock.rs` is the monotonic `Uptime` and deliberately wall-clock free: do not put it there. | `pub fn unix_secs() -> u64` in a new small core module | 10 | low |
| H2 | Atomic file write 3 times | **Mostly refuted.** `owner_only::write_with` is 0600 + fsync + dir 0700; `store::write_then_rename` is plain `fs::write` with a unique temp name, deliberately (BUG-025/T153); `client/shell/sandbox.rs:174-205` targets `~/.claude.json`, where `ensure_dir` would chmod `$HOME`. `protocol/auth.rs:118` is a fourth, non-atomic variant. | Leave. At most an `owner_only` variant without `ensure_dir` for `sandbox.rs` (about 18 lines). | 0 | high: credential and permission semantics |
| H3 | `ProjectDirs` lookup repeated | **Confirmed**, wider: 13 production `ProjectDirs::from("", "", "micold-ai-ide")` sites, 26 literals. The daemon trio (hooks, `mcp/server`, `state.rs`) is identical. `logging.rs` uses `data_local_dir` on purpose (BUG-015). | `micold_core::paths::{APP_ID, data_dir(), data_subdir()}`; do not fold `data_local_dir`. Value is the single constant, lines break even. | 5 | low-med |
| H4 | `$HOME` read by hand in 3 places | **Refuted.** `permission_failure.rs:155`, `endpoint.rs:212` (macOS only, rejects empty, errors) and `legacy_units.rs:117` (`XDG_CONFIG_HOME` first) have different semantics. | Leave (type 4). | 0 | - |
| H5 | `PATHEXT` parsed twice | **Confirmed.** `provider.rs:521`, `client/shell/links.rs:146` (cfg windows, filters empties; the first does not). | `pathext()` in core, filtered | 8 | low |
| H6 | Hand-built git `Command` | **Confirmed**; see K14. | K14 | in K14 | low |
| H7 | Hex formatter twice | **Confirmed.** `protocol/auth.rs:139 hex`, `hashing.rs:149 sha256_hex`. `hashing.rs` is `include!`d by `build.rs`: keep the new fn dependency-free. | `hex(bytes)` in `hashing.rs` | 6 | low |
| H8 | Wrappers around `temp_path_for` / `backup_path_for` | **Confirmed.** `store.rs:691,696` and `settings.rs:623,627` are one-liners with one caller each; `settings.rs` re-implements `backup_path_for`. | Inline; make `backup_path_for` `pub(crate)`; `write_atomic(path, json)` | 18 | low |
| H9 | `daemon/platform::write_owner_only` pass-through | **Confirmed.** `platform/mod.rs:72`; one production caller (`mcp/server.rs:87`) and one test import. | Delete, call `owner_only::write` | 13 | low |
| H10 | Client colour conversions | **Partly confirmed.** `syntax.rs:63 rgb` equals `composition_contrast.rs:42 rgb`; `terminal.rs:153`, `terminal.rs:120`, `divider.rs:80` repeat `style::color`. **`syntax::mix` vs `style::blend` refuted**: `u8` rounding on `Rgb` vs `f32` with alpha forced to 1; unifying would shift values the contrast gate checks. | `style::to_rgb(Color)` beside `style::color`; leave mix/blend. **Not done (#732, PR #859 closed):** `material_boundary` forbids feature modules (`syntax.rs`) from referencing `material::style`, and the `terminal.rs` sites take alacritty's `vte::ansi::Rgb`, not `tokens::Rgb`; the one clean merge netted +1 line | 18 | low-med |
| H11 | `ellipsise` vs `elide_middle` | **Refuted.** `progress.rs:224` truncates at the end; `elide_middle` keeps head and tail. Different behaviour. (The `daemon/progress.rs` pairs are all tests.) | Leave (type 4). | 0 | - |

## Tests (type 6): where most of the duplicated lines are

| ID | Files / functions | What is duplicated | Type | Proposed fix | Lines | Risk |
|---|---|---|---|---|---|---|
| T2 | 39 files in `micold-daemon/tests/` hand-build `ClientMsg::Hello{..}` (32 without a support module): `diff_layout_setting`, `pr_status_setting`, `settings_long_task_threshold`, `worktree_refresh`, `busy_connection`, ... | Duplex pipe, `serve_connection`, `Framed`, `Hello` with 8 fields, expect `Welcome`; about 25 lines per copy; `connect_and_attach` in 3 files. | 1 + 3 | `tests/support/conn.rs`: `connect`, `connect_as`, `connect_and_attach`; fold in `connect_with_welcome` from `support/attention.rs`. Tests that vary the Hello (fingerprint, version mismatch) keep their own. | 500 | med |
| T3 | `daemon/tests/review_send.rs:50-445` vs `support/runs.rs` | Fake-claude install, env guard, sandbox of repo + service, request plumbing copied; `runs.rs` has 3 users. | 1 | `#[path]` reuse of `support/runs.rs`; shared `Env`/`install_claude` in `support/mod.rs`; keep `slow`/`typed` as methods. Diff the two `Sandbox`es first, they have diverged. | 200 | med |
| T4 | `settings_long_task_threshold.rs:36-135`, `settings_notification_kinds.rs:75-135`, `attention_support/mod.rs`, `support/attention.rs` | Four copies of "workspace JSON with N idle Claude sessions, `state_on`, register"; two parallel attention support modules with the same `session_id`, `idle_process`, `connect`, `next_frame`, `Service`. | 1 | Keep `support/attention.rs`; add `Service::with_threshold` and the `claims`/`kinds` helpers; delete `attention_support/`. Merge constructors, not behaviours. After T2. | 450 | med |
| T5 | 7 daemon test files build `ClientMsg::SettingsSet{..}` with about 10 `None` | Constructors differing in which field is `Some`. | 2 | `settings_set(req)` in `support/conn.rs`, override one field. No `Default` on the wire struct (production change). | 120 | low |
| T10 | `core/tokens/css.rs` tests 430-560, `core/path_insert/mod.rs` tests 375-535, `daemon/progress.rs` tests 90-156 | Same setup before different assertions; css tests restate the radius table. | 3 | `primed()` in progress tests; `plan_in_sandbox(..)`; one exposed corner list iterated by code and test. | 100 | low |
| T11 | `core/link/runnable.rs` 69-125 vs 209-263 | Test restates the runnable-extension lists the code holds. | **4 leave** | FR-013 "verbatim": the test is an independent oracle; sharing the const makes it check the code against itself. | 0 | - |
| T12 | `micold-core/tests/owner_only.rs` vs `mcp_binding_file_mode.rs`; `inventory/mod.rs` (561 duplicated lines); `stream_view.rs`, `hooks_receiver.rs` self-clones | Not examined in depth: both tests reimplement `mode()` and tempdir plumbing; the others are large self-clones. | 6 (to confirm) | Examine when T1 lands (inventory overlaps it). | unknown | low |

## Summary

| Area | Actionable lines | Left (type 4) rows |
|---|---|---|
| Tests (T, W13, C16-17) | about 2,750 | T11, W14 |
| Widgets (W1-W10) | about 550 | W11, W12 |
| Client app (C) | about 380 | C3, C11 (mostly), C15, C18 |
| Core and daemon (K, H) | about 255 | K1, K3, K5, K7, K8, K12, K13, K15, K17-K19, H2, H4, H11 |
| Showcase (S) | about 45 | S1, S2, S5 |

Order of work, once children are filed: T1, T2, T4, T6, T7, W13 (tests, about 2,000 lines, low
risk), then W1 with W2, W8, W7 on the same helper, then C7, C1+C2, W4, and the small production
helpers from rank 17 down. Lower the budgets in `mise.toml` after each merge.
