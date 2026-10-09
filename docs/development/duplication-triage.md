# Duplication triage (issue #645)

Triage of every duplication cluster found by `mise run duplication` and the gap measurements of
[#645](https://github.com/jaroslawherod/micold-ai-ide/issues/645), done 2026-10-09 on `main`
(`6c43ef7d`). Doc only: no code changed, no child issue filed. Each table row is written so it can
become a child issue as it stands: files, what is duplicated, type, fix, lines, risk.
Tools and budgets: [duplication.md](duplication.md).

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
- The `ui/cdk` / `ui/material` cross-file clones (`select.rs`, `ripple.rs`, `tooltip.rs`,
  `keyboard_elsewhere.rs`, `context_area.rs`, `reflow.rs`, `navigation_drawer.rs`) are **one cause**:
  hand-forwarded single-child iced `Widget` methods (row W1). They are not shared focus or popup logic.
  One passthrough helper in `ui/cdk` removes most of them.
- Many `ui/material` files in the clone list (`*_anatomy.rs`, `ripple_*.rs`, `picker_*.rs`,
  `style_*.rs`, `field_focus.rs`, `test_support.rs`) are `#[cfg(test)]`-only modules: type 6, not widgets.
- Several claims of the issue comment were **refuted** by reading the code (section Helpers, H2, H4,
  H10 partly, H11). A sample of `similarity-rs` pairs (`worktree_form`, `daemon_sync`, `provider`,
  showcase) are shape coincidences: left (type 4).
- Total actionable: about **3,950** lines across about 50 rows; the type 4 rows are left on purpose.

## Order of attack

Ranks, best first. Tests first (cheap, safe), then the one widget helper, then small production tails.

| Rank | ID | Lines | Risk | One-line fix |
|---|---|---|---|---|
| 1 | T1 | 600 | low | `tests/support/source_scan.rs` for the 22 guard tests |
| 2 | T2 | 500 | med | daemon `tests/support/conn.rs` (Hello handshake) |
| 3 | T4 | 450 | med | merge the two attention support modules |
| 4 | T6 | 220 | low | shared modal table `tests/support/modals.rs` |
| 5 | T7 | 200 | low | `main_tests.rs` drainers and `connected_app()` |
| 6 | W13 | 200 | low | material test helpers into `test_support.rs` |
| 7 | W1 | 250 | med | `ui/cdk/passthrough.rs` widget forwarding |
| 8 | T5 | 120 | low | `settings_set(req)` test constructor |
| 9 | C7 | 150 | low-med | `floating_surface!` macro for 28 impls |
| 10 | T10 | 100 | low | in-file test setup helpers |
| 11 | T3 | 200 | med | `review_send.rs` onto `support/runs.rs` |
| 12 | C2 | 50 | low | `material::dialog::confirm` |
| 13 | W4 | 50 | low | field text builders, `form_field::beneath` |
| 14 | T8 | 45 | low | `ripple_clipping` uses `test_support` |
| 15 | C1 | 45 | low | shared rename dialog view |
| 16 | W2 | 60 | low-med | overlay forwarding helpers |
| 17 | K6 | 40 | low | `change_settings` in daemon `state.rs` |
| 18 | C17 | 40 | low | shell test fixtures |
| 19 | K4 | 35 | low | MCP session-op confirm helper |
| 20 | T9 | 30 | med | share core/client test fixtures (optional) |
| 21 | C10 | 40 | low-med | `parse_range` for settings number fields |
| 22 | W8 | 55 | med | section_list forwarding via W1 |
| 23 | S3 | 25 | low | showcase `text_button` |
| 24 | K16 | 22 | low | `declare_table` in tokens CSS |
| 25 | S4 | 20 | low | showcase `hover_area` |
| 26 | W3 | 20 | low | one focus operation in cdk |
| 27 | C14 | 20 | low | `is_primary_shift_chord` |
| 28 | H8 | 18 | low | inline store/settings path wrappers |
| 29 | W9 | 17 | low | local style/quad helpers |
| 30 | W5 | 35 | med | shared paragraph draw in `shaped_text.rs` |
| 31 | K14 | 15 | low | `git_output` / `git_failed` |
| 32 | C9 | 15 | low | `clamp_highlight` |
| 33 | H9 | 13 | low | delete `platform::write_owner_only` |
| 34 | C12 | 20 | low-med | sandbox `blocking_task` |
| 35 | K2 | 12 | low | `env_or_home` in provider |
| 36 | H10 | 18 | low-med | `style::to_rgb` |
| 37 | W6 | 30 | med-high | style state helpers |
| 38 | C16 | 12 | low | notification test fixture |
| 39 | K10 | 28 | med-high | terminal-history `remove_tracked` |
| 40 | H1 | 10 | low | `unix_secs()` in core |
| 41 | W7 | 25 | med-high | animation `draw_content` |
| 42 | C13 | 10 | low | issues `update_then_follow` |
| 43 | C5 | 10 | low | `scroll_panel_to` |
| 44 | C8 | 8 | low | `insert_sorted` (marginal) |
| 45 | H5 | 8 | low | shared `pathext()` |
| 46 | C11 | 15 | med | daemon_sync project guard only |
| 47 | H7 | 6 | low | shared `hex` |
| 48 | K11 | 12 | med | `read_or_quarantine` |
| 49 | K22 | 12 | med | logging `install` |
| 50 | H3 | 5 | low-med | `paths::data_subdir` and app-id const |

Overlaps: C1 and C2 share the dialog tail (count about 80, not 95); W8, W2 and W7 build on W1; T4
needs T2's `support/conn.rs`; T5 needs T2.

## Widgets: `micold-client/src/ui/cdk`, `ui/material`

| ID | Files / functions | What is duplicated (verified) | Type | Proposed fix | Lines | Risk |
|---|---|---|---|---|---|---|
| W1 | `cdk/context_area.rs:96-250`, `cdk/keyboard_elsewhere.rs:66-203`, `material/keyboard_focus.rs:257-400` (`TakesTheKeyboard`), `material/select.rs:712-836` (`ListWatch`), `terminal_pane.rs:172` (`GridSizeReporter`); partly `ripple.rs:176`, `cdk/tooltip.rs:197`, `text_area.rs:149` | Single-child `Widget` wrappers hand-forward `children/diff/size/layout/draw/mouse_interaction/operate/overlay` identically, about 70 lines each; only `update`, tag and state differ. This is the whole `cdk` vs `material` clone cluster in the issue. | 3 | New `ui/cdk/passthrough.rs`: a `forward_to_child!` macro or a `Wrapper` trait plus generic widget emitting the eight forwards. Convert the five fully transparent wrappers first; Ripple, Tooltip, Chrome adopt partially. | 250 | med: no pixels change, but Theme/Renderer generics differ between wrappers; widget tests catch regressions |
| W2 | `cdk/picker.rs:182-275, 437-467` (`Menu` overlay), `cdk/tooltip.rs:310-368, 477-488` (`Panel` overlay), `select.rs:780-818` | Overlay `draw/update/mouse_interaction/operate` all look up `layout.children().next()` and forward; only `layout()` (flip vs place) differs. | 3 | Overlay forwarding helpers beside W1; keep both `layout()`. | 60 | low-med: hit-testing, covered by picker/tooltip tests |
| W3 | `material/filled_field.rs:87-146` | Three `Operation` structs (`AsksControlForFocus`, `FocusesTheControl`, `UnfocusesTheControl`) with identical `traverse`; `keyboard_elsewhere.rs` already uses stock `focusable::unfocus`. | 1/5 | One `FocusOp` enum in cdk; reuse the stock op for unfocus. | 20 | low |
| W4 | `material/form_field.rs:135-146, 282-291`, `text_field.rs:95-106`, `select.rs:147-158`, `typeahead.rs:92-103`, `settings/mod.rs:127-135` | (a) `label()/supporting()/error()` builders pasted on four field types; (b) the "beneath" row (padding or `Space`) repeated in `form_field.rs` and `settings/mod.rs`. | 3 / 5 | (a) embedded `FieldText` with delegating methods; (b) `form_field::beneath(text)` reused by settings. | 50 | low |
| W5 | `material/ellipsized.rs:125-230` vs `line_clamp.rs:73-157` | Same skeleton: paragraph cache with reshape-on-width, tag/state, draw = anchor + clip + `fill_paragraph`. Cut algorithm and `size()` differ. | 3 | Shared `State<P>` and `draw_paragraph` in `material/shaped_text.rs`; do not merge the widgets. | 35 | med: text clipping; needs visual or golden check |
| W6 | `material/style.rs`: `filled`, `outlined`, `text_button`, `circular_icon_button`, `menu_row`, `scrollbar`, `checkbox`, `field_input` | Same shape: match status, state-layer fill at HOVER/PRESSED, disabled alpha 0.38, then a `Style`. Colour sources and borders genuinely differ. | 2 (helpers only) | `state_overlay` and `disabled_alpha` helpers; no table-driven rewrite. | 30 | med-high: visual core, pinned by `style_snapshot` tests |
| W7 | `material/animation.rs` draw at 508-548, 756-784, 916-944, 1084-1128 | Each wrapper returns when hidden, then draws `content` from `tree.children[0]`; the extra step differs per effect. | 3 | After W1: `draw_content` helper; keep each `draw` body (timing differs). | 25 | med-high: animation timing |
| W8 | `material/section_list.rs` `RowSlide` 799-896 vs `Rail` 1026-1105 | `operate`/`overlay`/`mouse_interaction` 96-97 % identical (index the active child, forward). | 3 | W1 variant with a child-index selector. | 55 | med |
| W9 | `toggle_chip.rs:197-234`, `split_view.rs:493-525` | Same `Style{border, radius}` built twice; two literal `fill_quad` blocks. | 3 | Local closure/const and a `quad()` fn. | 17 | low |
| W10 | `ui/panes.rs:56-146` | `split_button`/`close_button` share a Tooltip+IconButton construction. `terminals` vs `terminal_status` only share iteration. | 3 / 4 | `pane_button(icon, tip, msg, r)`; leave the terminals pair (different outputs). | 7 | low |
| W11 | `material/diff_view.rs`: `side_row`, `half`, `line_row`, `number`, `header_row` | Row assembly suffix coincides. | **4 leave** | Unified and side-by-side layouts evolve independently; the 85 % similarity is shape coincidence; a shared fn would save about 2 lines. | 0 | - |
| W12 | `terminal_pane.rs:1832` `mouse_interaction` vs `:172`; `layout` 106 vs 995 | 172 is a pure forward (covered by W1); 1832 computes link-hover interaction. | 4 | Leave the rest. | 0 | - |
| W13 | Test-only modules and test sections: `select_anatomy`, `menu_anatomy`, `picker_motion`, `picker_parity`, `picker_press`, `field_focus`, `ripple_*`, `anatomy_size`, `pull_request_indicator`/`unread_mark` `size_of()`, ellipsized/line_clamp tests, keyboard_focus vs select/text_area tests | Renderer setup, `roles()`, `size_of()` probe, press/release/hover drivers, measure loops. | 6 | Put them in `material/test_support.rs`; table-drive repeated anatomy asserts only where a failure still names the component. | 200 | low |
| W14 | `material/style_states.rs:70-107`, `style_shape.rs:42-50`, `menu_anatomy`/`dialog_anatomy`/`button_anatomy` contract values, `tab.rs:419`/`tab_strip.rs:63` `roles()` | Repeated snapshot assertions. | **4 leave** | The repetition is the spec: each line pins one state or component; table-driving hides which one broke. | 0 | - |

## Client dialogs, features and shell: `micold-client` outside `ui/cdk`, `ui/material`

| ID | Files / functions | What is duplicated (verified) | Type | Proposed fix | Lines | Risk |
|---|---|---|---|---|---|---|
| C1 | `ui/rename.rs:8-80` vs `ui/worktree_rename.rs:8-80` | Line-for-line equal: only title, caption, label, `FieldId`, three messages and the draft type differ. The `RenameError` message match is identical. | 2 | `ui::rename_dialog::view(RenameSpec{..})`; error text as `RenameError::message()` in `micold_core::project`. | 45 | low |
| C2 | `ui/confirm_session_remove.rs`, `confirm_placement.rs`, `confirm_forget.rs`, `confirm_delete.rs`, `confirm_link_open.rs`, `confirm_agent_request.rs`, `confirm_dismiss_group.rs`, `confirm_pick_run.rs`, tail of `attach_dialog.rs`/`parallel_dialog.rs` | Each ends with `Surface::new(dialog::body(..), Dialog, r).width(Fixed(N))` plus a filled and an outlined button. The issue's "26 identical lines" is this tail. | 3 | `material::dialog::confirm(r, headline, body, label, on_confirm, on_cancel, width)` in `material/dialog.rs`; C1 reuses it. | 50 | low |
| C3 | `dialog()` registration fns in about 12 files | Same signature and 9-line doc comment; the body is one `as_ref().map(..)`. | **4 leave** | Only comments repeat; the registry takes fns, not closures, and that is an overlay design change. | 0 | - |
| C5 | `ui/focus.rs:137-170` (`ShowFocused`) vs `ui/picker_scroll.rs:122-152` (`ShowHighlight`) | Same `Operation` shape and `delta_into_view` + `scroll_by` tail; guards and targets differ. | 3 | `scroll_panel_to(target, bounds, content_bounds, translation, state)` in `focus.rs`. | 10 | low |
| C7 | about 28 `impl FloatingSurface` blocks in `features/{project,worktree,help,attach,agent_confirm,runs,session,settings,sidebar}.rs` | Each is three methods: constant `id`, constant `layer`, `dismissal = for_layer(L).cancelled_by(msg)`. Too short for jscpd to flag, so the clone list understates it. | 2 | `floating_surface!` macro next to the trait; convert only the uniform impls. Must accept `Self::ID` and `SurfaceId::new(..)`. | 150 | low-med: macro hides impls from grep |
| C8 | `features/worktree.rs:272-312` (`included`, `created`), `features/project.rs:196-207` | Push-if-absent, sort by `dir_name`, `list_changed`. Existence test differs (`dir_name` vs `path`). | 3 | `insert_sorted(state, wt, same)`. Two callers: marginal, optional. | 8 | low |
| C9 | `features/worktree_form.rs` (55 pairs) | Pairs are small reducers sharing `with_form`; real overlap is the highlight clamp in `rematch_issues`/`rematch_branches` and the two `*_highlight_moved`. | 5 for clamp/move; 4 for the rest | `clamp_highlight(h, len)` beside `move_highlight`. Leave the other 50 pairs: distinct state transitions. | 15 | low |
| C10 | `features/settings.rs:667-836`: `mib`, `count`, `scrollback`, `timeout`, `long_task_threshold` | Number-field validators: trim, parse, range, three error branches. `timeout` and `long_task_threshold` differ only in constants and noun. | 2 | `parse_range<T>(text, field, section, min, max, noun, empty_ok)`; keep message wording (tests pin it). | 40 | low-med: messages are user-visible |
| C11 | `shell/daemon_sync.rs` (38 pairs): `on_rename_confirmed`/`on_worktree_rename_confirmed`, `on_worktree_include/exclude_requested`, `on_shell_instance_*` | Same outer skeleton, but the order of `update` vs `send_op` and the guards are the logic (rename sends only if the reducer cleared the draft; delete updates last). | 4 with a narrow 3 | Leave. Only the "needs an active project" guard is extractable (`project_op`). Do not build a generic `confirmed()`. | 15 | med: ordering is behaviour |
| C12 | `shell/sandbox.rs:515-540, 807-880` | `diagnostics`, `check_alive`, `stop`, `reread_container` each build `Task::future(spawn_blocking(CliRuntime::new(..).find(..)))`. The `run` wrappers (631-648, 1172-1183, 1424-1434) are distinct test decorators. | 3; 4 for `run` | `blocking_task(plan, work, then)` in `sandbox.rs`; leave `run`. | 20 | low-med: keep `reread_container`'s `cfg(test)` runner swap |
| C13 | `shell/issues.rs:183-226` | Both loaded-handlers snapshot `issue_description_request()`, update the form, then `newly_awaited_descriptions` + `start_issue_descriptions`. | 3 | `update_then_follow(app, msg)`. | 10 | low |
| C14 | `keymap.rs:188-221` | `is_release_chord` / `is_new_terminal_chord` identical but for the letter. | 2 | `is_primary_shift_chord(key, mods, letter)`. | 20 | low |
| C15 | `crates/micold-client/build.rs` vs `micold-daemon/build.rs` | Byte-identical 18-line build scripts. | **4 leave** | Sharing needs a build-dependency crate for 18 lines; the two crates' build steps may diverge. | 0 | - |
| C16 | `shell/desktop_notify/macos.rs:256-269`, `windows.rs:132-145` | The `notification(title, body)` test fixture copied into both OS test modules; the other hit is a `use` block. | 6 | Shared sample-notification fixture; leave imports. | 12 | low |
| C17 | `shell/workspace.rs` (7 pairs), `shell/startup.rs:499-529 vs 633-683` | Test code: `Workspace` setup and the `RepoRootQuery` pull-and-reply sequence. The two 2-line "not a repo" refuse branches in production stay. | 6 | `here_workspace()` helper in startup tests; `ask_repo_root(app, rx)` in workspace tests. | 40 | low |
| C18 | `features/session.rs` (7 pairs) | `arm_tab_reveal; focus_terminal` tail and `if let Some(session)` head. | **4 leave** | Each mutates different session fields with its own commentary; a helper saves about 3 lines per fn. | 0 | - |

## Showcase

| ID | Files / functions | What is duplicated | Type | Proposed fix | Lines | Risk |
|---|---|---|---|---|---|---|
| S1 | `showcase/sections/controls.rs`, `atoms.rs`, `surfaces.rs` (about 100 similarity pairs) | `arrange(vec![posed("x", widget...)], Layout::Inline)` with different builder chains per widget. | **4 leave** | Each fn is a copyable usage example of one component and changes with that component's API; a table would hide the calls a reader copies. | 0 | - |
| S2 | `showcase/sections/motion.rs:57-153` | Already factored through `demo()`; each demo shows how its own wrapper is built. | **4 leave** | Same reason as S1. | 0 | - |
| S3 | `motion.rs:57-91`, `controls.rs:32-64` | `Button::with_content(Text::new(label, role, roles), variant, roles)` 7 times in showcase, 21 times crate-wide. | 5 | Check for an existing label constructor on `material::Button`; else a `text_button` helper in `gallery.rs`. | 25 | low |
| S4 | `showcase/sections/floating.rs:370-380, 402-412` | Same Tooltip over a 420x160 caption Surface; two strings and the delay differ. | 3 | Private `hover_area(caption, tip, roles)`. | 20 | low |
| S5 | `floating.rs:304-330`, `atoms.rs:255-286`, header blocks | Tooltip-position poses and `use` lines. | **4 leave** | Position and glyph vary by design; imports are not code. | 0 | - |

## Core and daemon

| ID | Files / functions | What is duplicated (verified) | Type | Proposed fix | Lines | Risk |
|---|---|---|---|---|---|---|
| K1 | `core/provider.rs` Claude 689-714, Copilot 1002-1027, Pi 1396-1421, OpenCode 1934-1960 | Byte-identical `launch_args_in`, `new_conversations`, `bind` stubs. | **4 leave** | The trait doc says no method has a default, on purpose (FR-021). Removing about 80 lines reverses a stated design decision; a human must reopen it. | 0 (80 if reopened) | high |
| K2 | `provider.rs` `config_dir` for Claude 716, Copilot 1029, Pi 1423 | Each repeats "env var if non-empty else home-relative"; `env_dir` (1576) exists and Codex/OpenCode use it. | 5 | `env_or_home(env, rel)` built on `env_dir`. | 12 | low |
| K3 | `provider.rs` `last_activity`, `activity_source` impls | Only a 3-line `metadata().modified()` repeats. | **4 leave** | The path logic changes with each CLI. | 0 | - |
| K4 | `daemon/mcp/tools.rs` `stop_session` 363, `interrupt_session` 404, `delete_session` 455, `send_session_input` 1273, `start_session` 1133-1203 | Same `from_uuid`/`resolve_session_target`/`policy_for` prelude and an 11-line `ask_user(..)` block four times; the session row built twice. | 3 | `confirm_session_op(..)` and `session_row(..)` in the same file. | 35 | low |
| K5 | `daemon/mcp/tools.rs` `attach_worktree`, `rename_worktree`, `start_session` | Only the 3-line `resolve_caller` + `check_policy` prelude. | **4 leave** | Outcome mapping, errors and ordering (policy first, live refresh) differ per tool. The issue's "~86 % similar" is shape only. | 0 | - |
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
| H10 | Client colour conversions | **Partly confirmed.** `syntax.rs:63 rgb` equals `composition_contrast.rs:42 rgb`; `terminal.rs:153`, `terminal.rs:120`, `divider.rs:80` repeat `style::color`. **`syntax::mix` vs `style::blend` refuted**: `u8` rounding on `Rgb` vs `f32` with alpha forced to 1; unifying would shift values the contrast gate checks. | `style::to_rgb(Color)` beside `style::color`; leave mix/blend | 18 | low-med |
| H11 | `ellipsise` vs `elide_middle` | **Refuted.** `progress.rs:224` truncates at the end; `elide_middle` keeps head and tail. Different behaviour. (The `daemon/progress.rs` pairs are all tests.) | Leave (type 4). | 0 | - |

## Tests (type 6): where most of the duplicated lines are

| ID | Files / functions | What is duplicated | Type | Proposed fix | Lines | Risk |
|---|---|---|---|---|---|---|
| T1 | 22 files in `micold-client/tests/` define `code_only(src)` (about 30 lines each); 12 also define `walk(dir, out)` + a sources collector (`cdk_no_appearance`, `material_boundary`, `motion_tokens`, `anatomy_call_sites`, `component_api_opacity`, `composite_call_sites`, `one_overlay_implementation`, `idle_requests_no_frames`, `root_is_routing_only`, `root_vocabulary_is_cross_cutting`, `features_are_render_free`, ...) | Source-scanning guard-test plumbing pasted. `tests/support/state_scan.rs` already has `code_only` and `sources()` but only two files use it. | 5 | `tests/support/source_scan.rs` with `code_only`, `walk_rs`, `sources_under`; keep a `strip_comments` variant for the three comment-only copies. Diff each copy's string stripping before swapping. | 600 | low |
| T2 | 39 files in `micold-daemon/tests/` hand-build `ClientMsg::Hello{..}` (32 without a support module): `diff_layout_setting`, `pr_status_setting`, `settings_long_task_threshold`, `worktree_refresh`, `busy_connection`, ... | Duplex pipe, `serve_connection`, `Framed`, `Hello` with 8 fields, expect `Welcome`; about 25 lines per copy; `connect_and_attach` in 3 files. | 1 + 3 | `tests/support/conn.rs`: `connect`, `connect_as`, `connect_and_attach`; fold in `connect_with_welcome` from `support/attention.rs`. Tests that vary the Hello (fingerprint, version mismatch) keep their own. | 500 | med |
| T3 | `daemon/tests/review_send.rs:50-445` vs `support/runs.rs` | Fake-claude install, env guard, sandbox of repo + service, request plumbing copied; `runs.rs` has 3 users. | 1 | `#[path]` reuse of `support/runs.rs`; shared `Env`/`install_claude` in `support/mod.rs`; keep `slow`/`typed` as methods. Diff the two `Sandbox`es first, they have diverged. | 200 | med |
| T4 | `settings_long_task_threshold.rs:36-135`, `settings_notification_kinds.rs:75-135`, `attention_support/mod.rs`, `support/attention.rs` | Four copies of "workspace JSON with N idle Claude sessions, `state_on`, register"; two parallel attention support modules with the same `session_id`, `idle_process`, `connect`, `next_frame`, `Service`. | 1 | Keep `support/attention.rs`; add `Service::with_threshold` and the `claims`/`kinds` helpers; delete `attention_support/`. Merge constructors, not behaviours. After T2. | 450 | med |
| T5 | 7 daemon test files build `ClientMsg::SettingsSet{..}` with about 10 `None` | Constructors differing in which field is `Some`. | 2 | `settings_set(req)` in `support/conn.rs`, override one field. No `Default` on the wire struct (production change). | 120 | low |
| T6 | `client/tests/overlay_dismissal_delta.rs`, `overlay_dispatch_ordering.rs`, `overlay_registry.rs`, `overlay_transition_identity.rs` | The modal table (`about`, `project_selector`, `rename_project`, ...) with open fn and close message, four copies of about 60 lines. | 2 | `tests/support/modals.rs`: `modals()`, `open_dialog`. | 220 | low |
| T7 | `client/src/main_tests.rs` (11,209 lines), `shell/daemon_sync.rs` tests | 28 hand-written drainers over `UnboundedReceiver<ClientMsg>`, 33 tests with the same `unbounded(); base_app(); Outbox::new(tx)` start; `answer_remotes` byte-identical at 6208 and 8501. | 1 + 3 | `drain(rx)`, `sent_matching(rx, f)`, `connected_app()`; one `answer_remotes`. | 200 | low |
| T8 | `ui/material/ripple_clipping.rs:33-77` vs `test_support.rs:11-69` | `renderer()` and `block_on` copied (type 5: `test_support` exists for this). | 5 | Use `super::test_support::{renderer, block_on}`. | 45 | low |
| T9 | `client/tests/support/mod.rs` vs `core/tests/support/mod.rs` | 63-line clone: `fake_scanner`, `running_session`, `idle_session`, `failed_session`, `workspace_with`. | 6 | `#[path]` include or a `micold-test-support` dev crate. Optional: couples two crates for a small saving. | 30 | med |
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
