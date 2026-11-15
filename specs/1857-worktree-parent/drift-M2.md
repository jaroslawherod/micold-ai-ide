# Drift, milestone M2

- T015 named the test module of `crates/micold-client/src/features/sidebar.rs`. The tests live in
  `crates/micold-client/tests/sidebar_parent.rs`, a module of the `main` test binary
  (`crates/micold-client/tests/main.rs`, one binary, #1175), beside the other sidebar tests, and
  use the shared `crates/micold-client/tests/support/sidebar_fixture.rs`.
- Unplanned files the feature needed (decision D9 in the ledger):
  - `crates/micold-core/src/naming.rs`: `Tag::Parent(ParentLabel)`, the tag the row chip and the
    filter read.
  - `crates/micold-core/src/protocol/messages.rs`, `crates/micold-core/src/protocol/version.rs`:
    `WorktreeSnapshot.parent` on the wire, PROTOCOL_VERSION 82; `docs/daemon.md` and
    `docs/user-guide/orchestrator-mode.md` name the new version. The schema pins move with it:
    `crates/micold-core/tests/protocol_roundtrip.rs`, `crates/micold-core/tests/schema_hash.rs`.
  - `crates/micold-core/src/worktree_parent.rs`: `ParentView::label`, the one place that turns a
    stored link into the label (current name, removed flag).
  - `crates/micold-daemon/src/state.rs`: fills the snapshot's parent from the catalog;
    `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs` covers it.
  - `crates/micold-client/src/ui/sidebar.rs`: `filter_label` and `chip_bar` name the parent chip
    and take `TagFilter` by reference.
- Mechanical edits, a consequence of the new `Worktree.parent` / `WorktreeSnapshot.parent` field
  (fixtures gain `parent: None`) and of `TagFilter` losing `Copy` (it carries the label, so uses
  `.clone()` or a reference); behaviour unchanged:
  `crates/micold-core/src/ssh/remote_path.rs`, `crates/micold-core/tests/worktree_model.rs`,
  `crates/micold-client/src/catalog_sync.rs`, `crates/micold-client/src/features/session_strip.rs`,
  `crates/micold-client/src/main_tests.rs`, `crates/micold-client/src/shell/daemon_sync.rs`,
  `crates/micold-client/src/ui/overview.rs`, `crates/micold-client/src/ui/session_area.rs`,
  `crates/micold-client/src/ui/status_bar.rs`, `crates/micold-client/src/ui/toolbar.rs`, and the
  client tests
  `crates/micold-client/tests/add_worktree_dismissal.rs`, `crates/micold-client/tests/app_state.rs`,
  `crates/micold-client/tests/attention_view_report.rs`, `crates/micold-client/tests/catalog_two_daemons.rs`,
  `crates/micold-client/tests/directory_availability.rs`, `crates/micold-client/tests/features_changes.rs`, `crates/micold-client/tests/features_jira_status.rs`,
  `crates/micold-client/tests/features_overview.rs`, `crates/micold-client/tests/features_sidebar.rs`, `crates/micold-client/tests/features_worktree.rs`,
  `crates/micold-client/tests/gates/containment.rs`, `crates/micold-client/tests/gates/context_menu_anchor.rs`,
  `crates/micold-client/tests/layout_text_overflow.rs`, `crates/micold-client/tests/logical_state_ownership.rs`, `crates/micold-client/tests/main.rs`,
  `crates/micold-client/tests/overlay_registry.rs`, `crates/micold-client/tests/overview.rs`, `crates/micold-client/tests/overview_card_menu.rs`,
  `crates/micold-client/tests/sidebar_attention.rs`, `crates/micold-client/tests/sidebar_state.rs`, `crates/micold-client/tests/sidebar_state_icon.rs`,
  `crates/micold-client/tests/sidebar_tree.rs`, `crates/micold-client/tests/support/covered_states.rs`, `crates/micold-client/tests/support/sidebar_fixture.rs`,
  `crates/micold-client/tests/support/spawn_tree.rs`, `crates/micold-client/tests/switch_active.rs`, `crates/micold-client/tests/terminal_focus.rs`,
  `crates/micold-client/tests/upgrade_single_daemon.rs`, `crates/micold-client/tests/worktree_daemon_label.rs`, `crates/micold-client/tests/worktree_delete.rs`,
  `crates/micold-client/tests/worktree_form_daemon_choice.rs`, `crates/micold-client/tests/worktree_visibility.rs`.
- Review B F3: the new tag grew `WorktreeNode`, so clippy flagged `RunRow` with
  `large_enum_variant`. The first cut silenced it with an `#[allow]`; it is now boxed
  (`RunRow::Worktree(Box<WorktreeNode>)`), which touches `crates/micold-client/src/features/navigation.rs`,
  `crates/micold-client/src/ui/overview.rs` and `crates/micold-client/src/ui/sidebar.rs`.
- `plan-drift.sh` reported `MISSING quickstart.md` (T018): the visual pass is recorded in
  `specs/1857-worktree-parent/quickstart.md`, and the script does not compare paths under `specs/`.
- `scripts/autopilot/brief.py milestone` found no M2 because tasks.md had no `## Milestones`
  section (the milestones were one bold line in its header); the section is added, so the brief and
  `plan-drift.sh` now resolve M1, M1b, M2 and M3.
