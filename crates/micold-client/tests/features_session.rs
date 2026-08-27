//! Session foreground and project switching, in isolation (feature 021, SC-004).
//!
//! Same caveat as `features_worktree.rs`: these are `impl State` methods in Tier 1, so the file
//! builds a `State`. What it holds to is the other half of SC-004 — it names no other feature's
//! types. No sidebar rows, no overlays, no drafts.
//!
//! The three `let _ =` bindings below are the same decision spelled for a function whose answer is
//! only outcomes: this file asserts on no sidebar field, so dropping them is the boundary rather
//! than a shortcut. A test that asserts a *moved* consequence needs a draining helper instead —
//! `tests/switch_active.rs` has one, and T067a-6 records the four files that needed it.
//!
//! `switch_active` now answers `Option<Vec<Outcome>>` rather than `bool` — `None` is still the
//! refusal, and the outcomes are the sidebar consequences of arriving somewhere (T067a-6). This
//! file drops them on the floor deliberately: it asserts on no sidebar field, which is the same
//! boundary the header claims above, and the split turned out to fall exactly along it.
//!
//! The switch sequence is what is worth pinning here. Its step order is load-bearing
//! (data-model.md I1) and every step is individually plausible in the wrong place, which is exactly
//! the kind of thing a refactor breaks quietly.

use micold_client::app::State;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::session::{ForegroundChoice, SelectKind};
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use std::path::{Path, PathBuf};

fn project(path: &str) -> Project {
    Project {
        path: PathBuf::from(path),
        display_name: path.trim_start_matches('/').to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    }
}

/// Two open projects, `/a` active, each holding the sessions given. Returns the state plus the
/// session ids, since `Session::start_new` allocates them.
fn two_projects(a: usize, b: usize) -> (State, Vec<SessionId>, Vec<SessionId>) {
    let mut st = State::default();
    st.workspace.projects.push(project("/a"));
    st.workspace.projects.push(project("/b"));

    let sessions_a: Vec<Session> = (0..a)
        .map(|_| Session::start_new(SessionLocation::Default, AiCli::ClaudeCode))
        .collect();
    let sessions_b: Vec<Session> = (0..b)
        .map(|_| Session::start_new(SessionLocation::Default, AiCli::ClaudeCode))
        .collect();
    let ids_a = sessions_a.iter().map(|s| s.id).collect();
    let ids_b = sessions_b.iter().map(|s| s.id).collect();

    st.workspace
        .sessions
        .insert(PathBuf::from("/a"), sessions_a);
    st.workspace
        .sessions
        .insert(PathBuf::from("/b"), sessions_b);
    st.workspace.active = Some(PathBuf::from("/a"));
    (st, ids_a, ids_b)
}

#[test]
fn switching_to_an_unknown_project_changes_nothing_and_says_so() {
    let (mut st, a, _) = two_projects(1, 1);
    st.session.active = Some(a[0]);

    let switched = st.switch_active(Path::new("/nowhere")).is_some();

    assert!(
        !switched,
        "a project that is not open cannot be switched to"
    );
    assert_eq!(
        st.workspace.active.as_deref(),
        Some(Path::new("/a")),
        "a rejected switch must leave the active project alone — half-switching is worse than \
         not switching"
    );
    assert_eq!(
        st.session.active,
        Some(a[0]),
        "and it must leave the foreground alone too"
    );
}

#[test]
fn switching_away_and_back_returns_to_the_session_that_was_in_front() {
    let (mut st, a, _) = two_projects(2, 1);
    st.session.active = Some(a[1]);

    assert!(st.switch_active(Path::new("/b")).is_some());
    assert!(st.switch_active(Path::new("/a")).is_some());

    assert_eq!(
        st.session.active,
        Some(a[1]),
        "the outgoing foreground is recorded BEFORE activation (data-model.md I1); record it \
         after and you store the incoming project's session under the outgoing project's key, \
         which looks right until you switch back"
    );
}

#[test]
fn entering_a_project_with_no_recorded_foreground_falls_back_to_a_running_session() {
    let (mut st, _, b) = two_projects(1, 1);

    assert!(st.switch_active(Path::new("/b")).is_some());

    assert_eq!(
        st.session.active,
        Some(b[0]),
        "a first visit has nothing stored, so the project shows its first running session \
         rather than an empty shell"
    );
}

#[test]
fn a_switch_lands_on_a_terminal_ready_to_type() {
    // Reversed by feature 023 (FR-011). This asserted the opposite until then, on the reasoning
    // that arriving in a project is not the same as asking to type in it — true of arriving
    // somewhere by accident, but a project switch is deliberate, and what fills the pane afterwards
    // is a restored session's terminal. An explicit release made before the switch goes with it
    // (FR-021a): it was about the moment, not about the session.
    let (mut st, _, _) = two_projects(1, 1);
    st.update(micold_client::app::Message::Session(
        SessionMsg::TerminalFocusReleased,
    ));

    assert!(st.switch_active(Path::new("/b")).is_some());

    assert!(
        st.terminal_focused(),
        "switching to a project with a restored session must leave its terminal holding the \
         keyboard, with no press (FR-011)"
    );
}

#[test]
fn a_restart_in_the_active_project_raises_no_return_notice() {
    let (mut st, a, _) = two_projects(1, 1);

    st.note_background_restart(a[0]);

    assert!(
        !st.session.restarted_while_inactive.contains(&a[0]),
        "the user watched it happen — telling them about it on return would be noise"
    );
}

#[test]
fn a_restart_in_an_inactive_project_is_remembered_until_the_user_returns() {
    let (mut st, _, b) = two_projects(1, 1);

    st.note_background_restart(b[0]);
    assert!(
        st.session.restarted_while_inactive.contains(&b[0]),
        "it happened out of sight, so it is owed a notice"
    );

    assert!(st.switch_active(Path::new("/b")).is_some());

    assert!(
        !st.session.restarted_while_inactive.contains(&b[0]),
        "the marker is consumed on arrival, or the same notice reappears on every later visit"
    );
}

#[test]
fn sessions_are_located_by_worktree_regardless_of_whether_they_are_visible() {
    let mut st = State::default();
    st.workspace.projects.push(project("/a"));
    st.workspace.active = Some(PathBuf::from("/a"));

    let mut archived = Session::start_new(
        SessionLocation::Worktree("feat-a".into()),
        AiCli::ClaudeCode,
    );
    archived.archived = true;
    let id = archived.id;
    st.workspace
        .sessions
        .insert(PathBuf::from("/a"), vec![archived]);

    assert_eq!(
        st.sessions_in_worktree("feat-a"),
        vec![id],
        "deleting a worktree must terminate every session it hosts, and an archived session is \
         still a process — filtering by visibility here would leak one"
    );
}

#[test]
fn the_three_selection_kinds_stay_distinct() {
    assert_ne!(SelectKind::Simple, SelectKind::Semantic);
    assert_ne!(SelectKind::Semantic, SelectKind::Lines);
}

// --- Why the app landed on the session it did -------------------------------------------------
//
// Entering a project picks a session, and when it picks *none* the user is dropped on the project
// overview with nothing to go on. Four different situations produce that, and they want different
// answers: the project genuinely has no sessions, it has sessions but none running, or the resolve
// is looking under a key nothing was filed under. `explain_foreground` names which, so the log line
// the binary writes is a diagnosis rather than a shrug.

#[test]
fn the_remembered_session_is_chosen_when_it_is_still_running() {
    let (mut st, a, _) = two_projects(2, 1);
    st.session.active = Some(a[1]);
    st.record_foreground();

    assert_eq!(
        st.explain_foreground(Path::new("/a")),
        ForegroundChoice::Remembered(a[1]),
        "returning to a project puts you back on the session you left it on, not on its first one"
    );
}

/// BUG-001 / FR-003a: **this expectation changed with the spec**, and deliberately.
///
/// It used to assert the fallback — a remembered session that had stopped was treated as no memory
/// at all. That was a fair call while this feature's premise held (sessions keep running in the
/// background), and it stopped being one the moment lifecycle turned out not to persist: after a
/// restart every session is idle, so the rule discarded the memory in the ordinary case and landed
/// the user on the project overview. Meanwhile clicking that same row selects it with no lifecycle
/// check, so the two paths disagreed about the same session.
#[test]
fn a_remembered_session_is_restored_even_after_it_has_stopped() {
    let (mut st, a, _) = two_projects(2, 1);
    st.session.active = Some(a[1]);
    st.record_foreground();
    let stopped = a[1];
    if let Some((_, session)) = st.workspace.find_session_mut(stopped) {
        session.record_clean_exit();
    }

    assert_eq!(
        st.explain_foreground(Path::new("/a")),
        ForegroundChoice::Remembered(stopped),
        "you are put back where you were. A stopped session shows its scrollback and its state — \
         which is exactly what clicking it in the sidebar does, and the reason restoring it too is \
         consistency rather than indulgence"
    );
}

#[test]
fn a_remembered_session_that_was_closed_is_not_restored() {
    let (mut st, a, _) = two_projects(2, 1);
    st.session.active = Some(a[1]);
    st.record_foreground();
    let closed = a[1];
    if let Some((_, session)) = st.workspace.find_session_mut(closed) {
        session.archive();
    }

    assert_eq!(
        st.explain_foreground(Path::new("/a")),
        ForegroundChoice::FirstActive {
            chosen: a[0],
            remembered: Some(closed),
        },
        "closing a session hides it from the sidebar entirely, so restoring one would display a \
         session the user cannot see listed — the one condition worth keeping from the old rule"
    );
}

#[test]
fn restoring_a_stopped_session_does_not_start_it() {
    let (mut st, a, _) = two_projects(1, 1);
    st.session.active = Some(a[0]);
    st.record_foreground();
    if let Some((_, session)) = st.workspace.find_session_mut(a[0]) {
        session.record_clean_exit();
    }

    assert!(st.switch_active(Path::new("/b")).is_some());
    assert!(st.switch_active(Path::new("/a")).is_some());

    assert_eq!(st.session.active, Some(a[0]), "restored");
    assert!(
        !st.workspace.find_session(a[0]).unwrap().1.is_active(),
        "restoring is a display decision; starting a process is not. FR-001/FR-002 keep a switch \
         from disturbing session lifecycle, and this keeps that"
    );
}

#[test]
fn a_project_whose_sessions_have_all_stopped_says_that() {
    let (mut st, a, _) = two_projects(2, 1);
    for id in &a {
        if let Some((_, session)) = st.workspace.find_session_mut(*id) {
            session.record_clean_exit();
        }
    }

    assert_eq!(
        st.explain_foreground(Path::new("/a")),
        ForegroundChoice::NoneActive { sessions: 2 },
        "two sessions, neither running: landing on the overview is correct here, and the count is \
         what distinguishes it from having no sessions at all"
    );
}

#[test]
fn a_key_nothing_was_filed_under_is_its_own_answer() {
    let (st, _, _) = two_projects(1, 1);

    assert_eq!(
        st.explain_foreground(Path::new("/somewhere-else")),
        ForegroundChoice::NoSessionsForKey,
        "distinct from `NoneActive` on purpose. Sessions listed in the sidebar while the resolve \
         finds none means the two are looking under different keys — a bug that reads exactly like \
         'the app forgot my session', and that no amount of staring at the foreground logic finds"
    );
}

#[test]
fn the_choice_is_recorded_where_the_binary_can_log_it() {
    let (mut st, a, _) = two_projects(1, 1);
    st.session.active = Some(a[0]);
    st.record_foreground();

    assert!(st.switch_active(Path::new("/b")).is_some());
    assert!(st.switch_active(Path::new("/a")).is_some());

    assert_eq!(
        st.session.last_foreground_choice,
        Some(ForegroundChoice::Remembered(a[0])),
        "the reducer decides; the binary logs. Keeping the reason on the state is what lets the \
         log line say why without the decision leaking into the I/O boundary"
    );
}

/// `010` BUG-013. The client boots, restores its project, and resolves which session to show —
/// all **before** the daemon's catalog arrives. Sessions live on the daemon, so at that instant
/// the project has none and the resolve honestly answers `NoSessionsForKey`. Then the catalog
/// lands and the sessions appear, and nothing ever asked the question again.
///
/// The visible cost was not a missing selection: it was a sidebar that looked *empty*. A location
/// row opens when it holds the current session (`effective_open`), so with nothing current the
/// Default row stayed shut and the sessions inside it — present in state, listed in the catalog —
/// were never drawn. The session survived the restart in every layer except the one the user
/// can see.
#[test]
fn a_foreground_resolved_before_the_catalog_arrived_is_resolved_again_when_it_does() {
    let (mut st, ids_a, _) = two_projects(1, 0);
    st.workspace.active = Some(PathBuf::from("/a"));

    // Boot order: the resolve runs against a project the client has, but whose sessions are still
    // on the wire.
    let staged = st
        .workspace
        .sessions
        .remove(Path::new("/a"))
        .expect("sessions");
    let _ = st.restore_after_activation(Path::new("/a"));
    assert_eq!(
        st.session.last_foreground_choice,
        Some(ForegroundChoice::NoSessionsForKey),
        "with no sessions filed under the key, this is the honest answer — the bug is that it is final"
    );
    assert_eq!(st.session.active, None);

    // The catalog arrives; `reconcile_catalog` files the sessions under the project.
    st.workspace.sessions.insert(PathBuf::from("/a"), staged);

    assert!(
        st.resolve_foreground_after_catalog().is_some(),
        "the resolve must be re-run now that the data it needed exists"
    );
    assert_eq!(
        st.session.active,
        Some(ids_a[0]),
        "the session the daemon was hosting all along is now the current one, which is also what \
         opens its row in the sidebar"
    );
}

/// The narrow guard, and the reason it is narrow: FR-007 forbids choosing a session for the user
/// when they are landing on the project overview. `NoneActive` is that landing — the project *has*
/// sessions and none is running — and re-resolving there would make a choice the user did not ask
/// for. Only `NoSessionsForKey` means "the resolve ran against data that had not arrived".
#[test]
fn a_deliberate_landing_on_the_project_overview_is_left_alone() {
    let (mut st, _ids, _) = two_projects(1, 0);
    st.workspace.active = Some(PathBuf::from("/a"));
    // A project whose only session has stopped: sessions exist, none is active.
    for s in st.workspace.sessions.get_mut(Path::new("/a")).unwrap() {
        s.record_clean_exit();
    }
    let _ = st.restore_after_activation(Path::new("/a"));
    assert!(
        matches!(
            st.session.last_foreground_choice,
            Some(ForegroundChoice::NoneActive { .. })
        ),
        "got {:?}",
        st.session.last_foreground_choice
    );

    assert!(
        st.resolve_foreground_after_catalog().is_none(),
        "nothing was missing, so nothing is re-resolved — the user is on the overview because that \
         is where the rule put them (FR-007)"
    );
    assert_eq!(st.session.active, None);
}

/// A session already chosen must never be replaced by a later catalog: a reconnect mid-session
/// arrives at `on_connected` exactly like a boot does.
#[test]
fn a_catalog_arriving_mid_session_does_not_move_the_user() {
    let (mut st, ids_a, _) = two_projects(2, 0);
    st.workspace.active = Some(PathBuf::from("/a"));
    let _ = st.set_current_session(Some(ids_a[1]));

    assert!(st.resolve_foreground_after_catalog().is_none());
    assert_eq!(
        st.session.active,
        Some(ids_a[1]),
        "a reconnect must not relocate the user to a different session"
    );
}

/// `010` BUG-013 — the wiring half, which the three tests above cannot reach.
///
/// `resolve_foreground_after_catalog` can be perfectly correct and never called, which is the state
/// this codebase has been in three times now (`010` BUG-011, `012` BUG-003, `012` BUG-004): both
/// halves tested, the join untested, the application broken. `shell/daemon_sync.rs` lives in the
/// binary crate and cannot be reached from here, so the call is read out of the source — the same
/// idiom as `terminal_bar_stability.rs`'s gates, and for the same reason.
///
/// Order matters as much as presence: `on_connected` reads `active_session` further down to decide
/// whether to view-and-start a session or send the empty overview view. Re-resolving after that
/// read would set the session and then not act on it.
#[test]
fn the_connect_path_re_resolves_the_foreground_after_folding_the_catalog() {
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("shell")
            .join("daemon_sync.rs"),
    )
    .expect("daemon_sync.rs");

    let connected = src
        .split_once("pub fn on_connected")
        .expect("on_connected exists")
        .1;
    let fold = connected
        .find("reconcile_catalog(")
        .expect("on_connected folds the catalog");
    let resolve = connected.find("resolve_foreground_after_catalog()").expect(
        "on_connected must re-resolve the foreground: the boot-time resolve ran before this \
             catalog existed and answered `NoSessionsForKey` against sessions still on the wire",
    );
    let reads_active = connected
        .find("app.core.session.active")
        .expect("on_connected decides what to view from active_session");

    assert!(
        fold < resolve && resolve < reads_active,
        "the re-resolve must sit between folding the catalog and reading `active_session` \
         (fold={fold}, resolve={resolve}, read={reads_active})"
    );
}
// ---------------------------------------------------------------------------------------
// Which AI CLI a new session runs (feature 026, T022 — FR-001, FR-004, FR-005, FR-006, SC-001)
// ---------------------------------------------------------------------------------------
//
// This file owns every render-free session decision in US1. The branching lives here rather than in
// the implementation tasks because Principle I's GUI exception covers *drawing* and does not cover
// *branching* — `ui/sidebar.rs` only dispatches what these functions decide.

use micold_client::features::session::{PressTarget, StartIntent};

/// A state with a chosen default and a chosen availability set.
fn state_with(default_ai_cli: AiCli, available: &[AiCli]) -> State {
    let mut state = State::default();
    state.session.default_ai_cli = default_ai_cli;
    state.session.available_providers = available.to_vec();
    state
}

#[test]
fn a_start_with_no_override_uses_the_stored_default() {
    let state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode, AiCli::Copilot]);
    assert_eq!(state.session.provider_for_start(None), AiCli::Copilot);
}

#[test]
fn an_override_wins_and_leaves_the_setting_untouched() {
    // FR-004's two halves in one assertion. The second is true by shape — `provider_for_start`
    // reads the default and writes nothing — and asserting it anyway is what would catch an
    // implementation that "remembered" the last override as the new default.
    let state = state_with(AiCli::ClaudeCode, &[AiCli::ClaudeCode, AiCli::Copilot]);

    assert_eq!(
        state.session.provider_for_start(Some(AiCli::Copilot)),
        AiCli::Copilot
    );
    assert_eq!(
        state.session.default_ai_cli,
        AiCli::ClaudeCode,
        "choosing an override for one session must not change what the next one defaults to"
    );
}

#[test]
fn changing_the_default_changes_no_existing_sessions_provider() {
    // FR-005, and it is true *by shape*: `Session::provider` is a constructor argument with no
    // setter, so there is no message that could change it. What this test can hold is that the
    // shape has not quietly acquired one — a `pub` field assigned somewhere would compile.
    let mut state = state_with(AiCli::ClaudeCode, &[AiCli::ClaudeCode, AiCli::Copilot]);
    let existing = Session::start_new(SessionLocation::Default, AiCli::Copilot);
    state
        .workspace
        .sessions
        .insert(PathBuf::from("/repo"), vec![existing.clone()]);

    state.session.default_ai_cli = AiCli::ClaudeCode;

    assert_eq!(
        state.workspace.sessions[Path::new("/repo")][0].provider,
        AiCli::Copilot,
        "the open session still runs the CLI it was started on"
    );
    assert_eq!(
        state.session.provider_for_start(None),
        AiCli::ClaudeCode,
        "and the next new one takes the new default"
    );
}

#[test]
fn the_primary_half_starts_the_default_in_one_press() {
    // SC-001: the one-interaction start survives the affordance gaining a second half.
    let state = state_with(AiCli::ClaudeCode, &[AiCli::ClaudeCode, AiCli::Copilot]);
    assert_eq!(
        state.session.start_intent(PressTarget::Primary),
        StartIntent::Start(AiCli::ClaudeCode)
    );
}

#[test]
fn the_secondary_half_offers_the_installed_clis_and_starts_nothing() {
    let state = state_with(AiCli::ClaudeCode, &[AiCli::ClaudeCode, AiCli::Copilot]);
    assert_eq!(
        state.session.start_intent(PressTarget::Secondary),
        StartIntent::OfferChoice {
            providers: vec![AiCli::ClaudeCode, AiCli::Copilot],
            unavailable_default: None,
        },
        "and nothing to announce — this press asked for the list (BUG-001)"
    );
}

#[test]
fn a_single_installed_cli_has_no_secondary_half_at_all() {
    // FR-006. A "choose which one" control that opens a list of one is a worse single-CLI
    // experience than the plain button it replaced, so the half is absent rather than disabled.
    let state = state_with(AiCli::ClaudeCode, &[AiCli::ClaudeCode]);

    assert!(!state.session.start_affordance_offers_a_choice());
    assert_eq!(
        state.session.start_intent(PressTarget::Primary),
        StartIntent::Start(AiCli::ClaudeCode),
        "and the primary half is unchanged — the single-CLI user is unaffected by this feature"
    );
}

#[test]
fn an_unavailable_default_offers_the_choice_rather_than_starting_or_substituting() {
    // FR-002 and FR-004 scenario 4, and the case with three wrong answers, all plausible:
    // silently start the other CLI (FR-002 forbids substituting), silently do nothing (the user
    // pressed a button), or start the missing one and let the spawn fail (that is FR-010's story,
    // not this one — the application knows *now* that it cannot).
    //
    // There is a fourth clause, and asserting only the three above is how it shipped missing:
    // FR-002 says to *say* the default is unavailable. The answer therefore names it, and
    // `tests/unavailable_default_says_so.rs` follows it from here to the sentence (BUG-001).
    let state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    assert!(!state.session.default_ai_cli_is_available());
    assert_eq!(
        state.session.start_intent(PressTarget::Primary),
        StartIntent::OfferChoice {
            providers: vec![AiCli::ClaudeCode],
            unavailable_default: Some(AiCli::Copilot),
        }
    );
    assert_eq!(
        state.session.default_ai_cli,
        AiCli::Copilot,
        "and the stored default is not rewritten on the way past (research R11)"
    );
}

#[test]
fn no_installed_cli_means_nothing_to_offer() {
    let state = state_with(AiCli::ClaudeCode, &[]);
    for target in [PressTarget::Primary, PressTarget::Secondary] {
        assert_eq!(
            state.session.start_intent(target),
            StartIntent::NothingAvailable
        );
    }
    assert!(!state.session.start_affordance_offers_a_choice());
}

#[test]
fn only_installed_clis_are_ever_offered() {
    // FR-006 for the menus — the Settings select and the override list read this one function, so
    // an unavailable CLI cannot appear in one and not the other.
    let state = state_with(AiCli::ClaudeCode, &[AiCli::ClaudeCode]);
    assert_eq!(state.session.offered_providers(), vec![AiCli::ClaudeCode]);
    assert!(!state.session.offered_providers().contains(&AiCli::Copilot));
}
