//! The Overview feature in isolation (feature 813, T010; SC-004): only its own `State`, `Msg` and
//! `update`.

use micold_client::features::overview::{update, Msg, State};

fn toggled(settings_open: bool) -> Msg {
    Msg::Toggled { settings_open }
}

#[test]
fn toggled_opens_then_closes_and_closed_closes() {
    let mut state = State::default();
    assert!(!state.open);
    update(&mut state, toggled(false));
    assert!(state.open);
    update(&mut state, toggled(false));
    assert!(!state.open);
    update(&mut state, toggled(false));
    update(&mut state, Msg::Closed);
    assert!(!state.open);
}

#[test]
fn toggled_while_settings_is_open_changes_nothing() {
    let mut state = State::default();
    update(&mut state, toggled(true));
    assert!(!state.open);
    state.open = true;
    update(&mut state, toggled(true));
    assert!(state.open);
}

// Feature 970 (FR-001, FR-010, FR-011, SC-005; data-model: `full_screen()` is `open && full_screen`,
// the only reader).

use micold_core::overview::Activation;
use micold_core::session::SessionLocation;

fn full_screen_toggled(settings_open: bool) -> Msg {
    Msg::FullScreenToggled { settings_open }
}

/// An open Overview in full-screen mode.
fn in_full_screen() -> State {
    let mut state = State::default();
    update(&mut state, toggled(false));
    update(&mut state, full_screen_toggled(false));
    assert!(state.full_screen());
    state
}

#[test]
fn full_screen_toggles_only_while_open() {
    let mut state = State::default();
    update(&mut state, full_screen_toggled(false));
    assert!(!state.full_screen(), "closed: the toggle does nothing");

    update(&mut state, toggled(false));
    update(&mut state, full_screen_toggled(false));
    assert!(state.full_screen(), "open: the toggle enters full screen");
    update(&mut state, full_screen_toggled(false));
    assert!(!state.full_screen(), "again: it leaves");

    update(&mut state, full_screen_toggled(true));
    assert!(
        !state.full_screen(),
        "Settings open: the toggle does nothing"
    );

    assert!(state.request(Activation::RevealOnly(SessionLocation::Default)));
    update(&mut state, full_screen_toggled(false));
    assert!(
        !state.full_screen(),
        "an activation pending: the first one stands"
    );
}

/// A card menu hangs at fixed window pixels: the grid must not move beneath it.
#[test]
fn full_screen_does_not_toggle_under_a_card_menu() {
    let mut state = State::default();
    update(&mut state, toggled(false));
    update(
        &mut state,
        Msg::CardMenuOpened {
            location: SessionLocation::Default,
            anchor: (10, 10),
            origin: micold_client::features::overview::MenuOrigin::Pointer,
        },
    );
    assert!(state.card_menu.is_some(), "the menu is open");
    update(&mut state, full_screen_toggled(false));
    assert!(
        !state.full_screen(),
        "the menu open: the toggle does nothing"
    );
}

#[test]
fn opening_the_overview_clears_full_screen() {
    let mut state = in_full_screen();
    update(&mut state, Msg::Closed);
    update(&mut state, toggled(false));
    assert!(state.open);
    assert!(
        !state.full_screen(),
        "the Overview always opens in the normal layout"
    );
}

#[test]
fn every_exit_leaves_full_screen() {
    let exits: [fn(&mut State); 4] = [
        |s| {
            update(s, Msg::Closed);
        },
        |s| {
            update(s, toggled(false));
        },
        |s| {
            update(
                s,
                Msg::CardActivated(Activation::OfferStart(SessionLocation::Default)),
            );
        },
        |s| {
            update(
                s,
                Msg::CardActivated(Activation::RevealOnly(SessionLocation::Default)),
            );
        },
        // The `app.rs` Changes exit (`Outcome::ChangesRequested`) is driven through the app's
        // `update` in `overview.rs::opening_changes_leaves_full_screen`; the Compare and
        // `MainAreaTaken` exits are not covered at app level.
    ];
    for (index, exit) in exits.into_iter().enumerate() {
        let mut state = in_full_screen();
        exit(&mut state);
        assert!(!state.full_screen(), "exit {index} left full screen on");
    }
}

#[test]
fn a_project_switch_keeps_full_screen() {
    let mut state = in_full_screen();
    update(&mut state, Msg::ProjectSwitched);
    assert!(state.full_screen());
}

/// Feature 970 (FR-013, US2 #5): a zoom's scroll request is the root's task, not the Overview's
/// state: it changes nothing and asks for nothing.
#[test]
fn reflowed_changes_no_state() {
    let mut state = in_full_screen();
    let before = format!("{state:?}");
    let outcomes = update(&mut state, Msg::Reflowed(96.0));
    assert!(outcomes.is_empty(), "no outcome: {outcomes:?}");
    assert_eq!(format!("{state:?}"), before, "no state change");
}

// ---- Feature 1504 (T026, US2): pull request and session titles ----

mod location_titles {
    use micold_client::app::State as App;
    use micold_client::features::overview::location_title;
    use micold_client::features::pr_status::{Msg as PrMsg, Outcome, Phase};
    use micold_core::location_title::{LocationTitle, TitleSource};
    use micold_core::project::{Availability, Project};
    use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
    use micold_core::session::{AiCli, Session, SessionLabel, SessionLocation};
    use micold_core::worktree::{Worktree, WorktreeStatus};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    const DIR: &str = "project-thread-7041o6";
    const BRANCH: &str = "claude/project-thread-7041o6";
    const PR_TITLE: &str = "Add readable titles";
    const SESSION_TITLE: &str = "Fix the flaky resize test";

    fn project() -> PathBuf {
        PathBuf::from("/repo")
    }

    /// A project with one worktree whose folder and branch carry no ticket.
    fn app(branch: Option<&str>) -> App {
        let mut app = App::default();
        app.workspace.projects.push(Project {
            path: project(),
            display_name: "repo".into(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        app.workspace.active = Some(project());
        app.worktree.worktrees = vec![Worktree {
            dir_name: DIR.into(),
            path: PathBuf::from(format!("/repo/.claude/worktrees/{DIR}")),
            branch: branch.map(str::to_string),
            status: WorktreeStatus::Valid,
            included: false,
        }];
        app
    }

    fn title(app: &App) -> LocationTitle {
        location_title(app, &app.worktree.worktrees[0], DIR)
    }

    fn text_and_source(app: &App) -> (String, TitleSource) {
        let title = title(app);
        (title.text, title.source)
    }

    fn pr(number: u64, state: PrState) -> PullRequestStatus {
        PullRequestStatus {
            number,
            title: PR_TITLE.into(),
            url: format!("https://github.com/o/r/pull/{number}"),
            state,
            review: ReviewState::None,
            head: "abc".into(),
            conflict: false,
        }
    }

    fn open() -> PrState {
        PrState::Open {
            checks: CheckStatus::Passing,
        }
    }

    /// The pull request reading as it stands after a read of `statuses`.
    fn read(app: &mut App, statuses: Vec<(&str, PullRequestStatus)>) {
        app.pr_status.enabled = true;
        app.pr_status.read_at = Some(100);
        app.pr_status.statuses = statuses
            .into_iter()
            .map(|(branch, pr)| (branch.to_string(), pr))
            .collect();
    }

    fn session(app: &mut App, main: bool, label: SessionLabel) -> Session {
        let mut session =
            Session::start_new(SessionLocation::Worktree(DIR.into()), AiCli::ClaudeCode);
        session.main = main;
        session.label = label;
        app.workspace
            .sessions
            .entry(project())
            .or_default()
            .push(session.clone());
        session
    }

    /// R9, US2.1: an open or draft pull request of the branch titles the card; a merged or closed
    /// one, another branch's, or one read while the reading is off does not.
    #[test]
    fn an_open_or_draft_pull_request_of_the_branch_titles_the_card() {
        for state in [
            open(),
            PrState::Draft {
                checks: CheckStatus::Pending,
            },
        ] {
            let mut app = app(Some(BRANCH));
            read(&mut app, vec![(BRANCH, pr(1520, state))]);
            assert_eq!(
                text_and_source(&app),
                (PR_TITLE.into(), TitleSource::PullRequest(1520))
            );
        }
        for state in [PrState::Merged, PrState::Closed] {
            let mut app = app(Some(BRANCH));
            read(&mut app, vec![(BRANCH, pr(1520, state))]);
            assert_eq!(text_and_source(&app), (BRANCH.into(), TitleSource::Branch));
        }
        let mut other = app(Some(BRANCH));
        read(&mut other, vec![("feat/other", pr(1520, open()))]);
        assert_eq!(
            title(&other).source,
            TitleSource::Branch,
            "another branch's"
        );
        let mut off = app(Some(BRANCH));
        read(&mut off, vec![(BRANCH, pr(1520, open()))]);
        off.pr_status.enabled = false;
        assert_eq!(
            title(&off).source,
            TitleSource::Branch,
            "status reading off"
        );
        let mut unread = app(Some(BRANCH));
        read(&mut unread, vec![(BRANCH, pr(1520, open()))]);
        unread.pr_status.read_at = None;
        assert_eq!(title(&unread).source, TitleSource::Branch, "never read");
    }

    /// R8, US2.3: the main session's recorded or derived title titles the card; a pending or blank
    /// label, a non-main session's title and an archived main session's do not.
    #[test]
    fn the_main_sessions_title_titles_the_card() {
        for label in [
            SessionLabel::Named(SESSION_TITLE.into()),
            SessionLabel::Derived(SESSION_TITLE.into()),
        ] {
            let mut app = app(Some(BRANCH));
            session(&mut app, true, label);
            assert_eq!(
                text_and_source(&app),
                (SESSION_TITLE.into(), TitleSource::Session)
            );
        }
        for label in [SessionLabel::Pending, SessionLabel::Named("  ".into())] {
            let mut app = app(Some(BRANCH));
            session(&mut app, true, label);
            assert_eq!(title(&app).source, TitleSource::Branch);
        }
        let mut helper = app(Some(BRANCH));
        session(&mut helper, true, SessionLabel::Pending);
        session(
            &mut helper,
            false,
            SessionLabel::Named(SESSION_TITLE.into()),
        );
        assert_eq!(
            title(&helper).source,
            TitleSource::Branch,
            "a non-main session's title"
        );
        let mut elsewhere = app(Some(BRANCH));
        let mut other = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        other.main = true;
        other.label = SessionLabel::Named(SESSION_TITLE.into());
        elsewhere
            .workspace
            .sessions
            .entry(project())
            .or_default()
            .push(other);
        assert_eq!(
            title(&elsewhere).source,
            TitleSource::Branch,
            "another location's"
        );
    }

    /// FR-001: an open pull request wins over the session title.
    #[test]
    fn the_pull_request_wins_over_the_session() {
        let mut app = app(Some(BRANCH));
        session(&mut app, true, SessionLabel::Named(SESSION_TITLE.into()));
        read(&mut app, vec![(BRANCH, pr(1520, open()))]);
        assert_eq!(title(&app).source, TitleSource::PullRequest(1520));
    }

    /// US2.5: with none of the sources, the branch, else the folder, as today.
    #[test]
    fn nothing_gives_the_branch_then_the_folder() {
        assert_eq!(
            text_and_source(&app(Some(BRANCH))),
            (BRANCH.into(), TitleSource::Branch)
        );
        assert_eq!(
            text_and_source(&app(None)),
            (DIR.into(), TitleSource::Folder)
        );
    }

    /// FR-010, US2.7: a pull request reading that finds the branch's new pull request changes the
    /// title, and one that finds it closed, or archiving the session, drops to the next source,
    /// with no message of the Overview's own.
    #[test]
    fn a_reading_or_an_archive_changes_the_title_with_no_other_message() {
        let mut app = app(Some(BRANCH));
        app.pr_status.enabled = true;
        let main = session(&mut app, true, SessionLabel::Named(SESSION_TITLE.into()));
        assert_eq!(title(&app).source, TitleSource::Session);
        let finish = |app: &mut App, seq: u64, state: PrState| {
            app.pr_status.phase = Phase::Reading {
                seq,
                again: false,
                started: 200,
            };
            let _ = app.update_pr_status(PrMsg::Finished {
                seq,
                outcome: Outcome::Ok {
                    statuses: BTreeMap::from([(BRANCH.to_string(), pr(1520, state))]),
                    lists: BTreeMap::new(),
                    removable: BTreeSet::new(),
                    started_at: 200,
                },
                now: 300,
            });
        };
        finish(&mut app, 1, open());
        assert_eq!(
            text_and_source(&app),
            (PR_TITLE.into(), TitleSource::PullRequest(1520)),
            "the reading that found the pull request"
        );
        finish(&mut app, 2, PrState::Closed);
        assert_eq!(title(&app).source, TitleSource::Session, "it closed");
        for s in app.workspace.sessions.get_mut(&project()).unwrap() {
            if s.id == main.id {
                s.archived = true;
            }
        }
        assert_eq!(
            title(&app).source,
            TitleSource::Branch,
            "the session archived"
        );
    }
}
