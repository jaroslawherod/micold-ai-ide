//! The covered states, declared in exactly one place (feature 019, T019/T020, FR-016).
//!
//! Adding a screen to the gate means adding one entry to [`covered_states`] and nothing else. A
//! second registration site would make FR-016's "one place" claim false, which is why
//! `tests/layout_coverage_registry.rs` scans for one.
//!
//! [`revealing_states`] is a second *list*, not a second *site*: states pinned partway through an
//! animation, which the geometry fixture deliberately does not record. Both live in this file, and
//! the requirement is that this file is the only one either kind is declared in.
//!
//! Every state is built from fixed, invented data (FR-007). Nothing here reads the developer's
//! worktrees, configuration or session store — a fixture that recorded the author's own workspace
//! would be unreproducible anywhere else, including on the same machine tomorrow.

use micold_client::app::SIDEBAR_MIN_WIDTH;
use micold_client::features::sidebar;
use micold_client::features::worktree;
use std::path::PathBuf;

use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::session::SessionMenu;
use micold_client::features::settings::{
    AppearanceDraft, DaemonDraft, EnvironmentDraft, FieldError, SettingsDraft, SettingsSection,
    TerminalDraft,
};
use micold_client::features::window::FieldId;
use micold_client::features::worktree::WorktreeMenu;
use micold_client::features::worktree_form::jira_query::{JiraQuery, QueryRun};
use micold_client::features::worktree_form::{
    BranchSource, GithubAvailability, IssueDetailsState, IssueList, JiraAvailability, JiraSite,
    SearchState, WorktreeForm,
};
use micold_core::github::{GithubRepo, Issue, IssueListing, IssueLoadError};
use micold_core::issue_details::{
    Blockers, IssueDetails, Kind, PlainText, PrListState, ProjectMembership, Reference,
    RelatedIssue, RelatedList, RelatedPr, SubIssues,
};
use micold_core::jira::{IssueProvider, JiraConnection, ProjectKey};
use micold_core::jira_query::{QueryFailure, QueryMode, QueryRefusal, ScopedQuery, UserQuery};
use micold_core::project::Availability;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::typeahead::{rank, Query};
use micold_core::worktree::{BranchCandidate, BranchOrigin, Worktree, WorktreeStatus};

use super::layout::{Anchor, CoveredState, RevealingState, StateUnderTest};

/// The repository the issue-source states read (feature 034).
fn github_repo() -> GithubAvailability {
    GithubAvailability::Available(
        GithubRepo::from_remote_url("git@github.com:octo/widgets.git").expect("a GitHub URL"),
    )
}

/// A capped issue list with `1100` typed, the loaded match shown, and the search in `search`.
fn issue_search_state(search: SearchState) -> StateUnderTest {
    let mut state = with_project();
    let issues = vec![Issue::new(
        17,
        "Follow-up to 1100".to_string(),
        vec!["bug".to_string()],
        "2026-09-29T00:00:00Z".to_string(),
    )];
    let query = "1100";
    let issue_matches = rank(&issues, |i| i.row_text(), &Query::new(query));
    let mut form = issue_form(IssueList::Loaded {
        listing: IssueListing {
            issues,
            total_open: 1_200,
            complete: false,
        },
        gh: std::path::PathBuf::from("/usr/bin/gh"),
        searched: Vec::new(),
        search,
        descriptions: Default::default(),
    });
    form.issue_query = query.to_string();
    form.issue_matches = issue_matches;
    form.type_ = None;
    state.worktree_form.form = Some(form);
    StateUnderTest::new(state)
}

/// A related issue, for the details states.
fn related(number: u64, title: &str, open: bool) -> RelatedIssue {
    RelatedIssue {
        reference: Reference::Here(number),
        title: PlainText::new(title),
        open,
    }
}

/// A pull request, for the details states.
fn related_pr(number: u64, title: &str, state: PrListState) -> RelatedPr {
    RelatedPr {
        reference: Reference::Here(number),
        title: PlainText::new(title),
        state,
    }
}

/// An issue with every kind of relationship, one of its blockers open (837 US2-1, US2-2).
fn every_kind_details() -> IssueDetails {
    let list = |items: Vec<RelatedIssue>| {
        let total = items.len() as u32;
        RelatedList::new(items, total, 0)
    };
    let prs = |items: Vec<RelatedPr>| {
        let total = items.len() as u32;
        RelatedList::new(items, total, 0)
    };
    let names = |items: &[&str]| {
        RelatedList::new(
            items.iter().map(|n| PlainText::new(n)).collect(),
            items.len() as u32,
            0,
        )
    };
    IssueDetails {
        number: 17,
        blocked_by: Kind::Read(Blockers {
            list: list(vec![
                related(10, "Parser rewrite", true),
                related(9, "Old spike", false),
            ]),
            open: 1,
        }),
        blocking: Kind::Read(list(vec![related(14, "Release notes", true)])),
        parent: Kind::Read(Some(related(3, "Q4 epic", true))),
        sub_issues: Kind::Read(SubIssues {
            list: list(vec![
                related(15, "Tokenizer", false),
                related(16, "Grammar", true),
            ]),
            closed: 1,
        }),
        labels: Kind::Read(names(&["bug"])),
        assignees: Kind::Read(names(&["octocat", "hubot"])),
        milestone: Kind::Read(Some(PlainText::new("v1.4"))),
        projects: Kind::Read(RelatedList::new(
            vec![
                ProjectMembership {
                    name: PlainText::new("Roadmap"),
                    fields: vec![(PlainText::new("Status"), PlainText::new("In progress"))],
                },
                ProjectMembership {
                    name: PlainText::new("Backlog"),
                    fields: Vec::new(),
                },
            ],
            2,
            0,
        )),
        closing_prs: Kind::Read(prs(vec![related_pr(
            20,
            "Fix the parser",
            PrListState::Draft,
        )])),
        linked_prs: Kind::Read(prs(vec![related_pr(
            21,
            "Refactor lexer",
            PrListState::Merged,
        )])),
    }
}

/// The issue form with issue #17 picked and its details in `details` (837).
fn issue_details_state(details: IssueDetailsState) -> StateUnderTest {
    let mut state = with_project();
    let issues = vec![Issue::new(
        17,
        "Follow-up to the parser".to_string(),
        vec!["bug".to_string()],
        "2026-09-29T00:00:00Z".to_string(),
    )];
    let issue_matches = rank(&issues, |i| i.row_text(), &Query::new(""));
    let mut form = issue_form(IssueList::Loaded {
        listing: IssueListing {
            issues,
            total_open: 1,
            complete: true,
        },
        gh: std::path::PathBuf::from("/usr/bin/gh"),
        searched: Vec::new(),
        search: SearchState::Idle,
        descriptions: Default::default(),
    });
    form.issue_matches = issue_matches;
    form.ticket = "17".to_string();
    form.name = "Follow-up to the parser".to_string();
    form.type_ = None;
    form.picked_issue = Some(17);
    form.issue_details = details;
    state.worktree_form.form = Some(form);
    StateUnderTest::new(state)
}

/// A form on the issue source, its list in `issues`.
fn issue_form(issues: IssueList) -> WorktreeForm {
    WorktreeForm {
        source: BranchSource::Issue,
        github: github_repo(),
        issues,
        ..WorktreeForm::default()
    }
}

/// A form on the Jira source of a project with no GitHub remote (feature 533), its list in
/// `issues`.
fn jira_form(issues: IssueList) -> WorktreeForm {
    let connection = JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com")
        .expect("a Jira connection");
    WorktreeForm {
        source: BranchSource::Issue,
        github: GithubAvailability::Unavailable("This repository has no GitHub remote.".into()),
        jira: JiraAvailability::Connected(JiraSite::of(&connection)),
        provider: IssueProvider::Jira,
        issues,
        ..WorktreeForm::default()
    }
}

/// The query every advanced state ran (feature 1186).
const ADVANCED_QUERY: &str = "assignee = currentUser() AND priority = High";

/// [`ADVANCED_QUERY`] scoped to the Jira form's project `ABC`.
fn advanced_scoped() -> ScopedQuery {
    ScopedQuery::new(
        &ProjectKey::parse("ABC").expect("a project key"),
        &UserQuery::parse(ADVANCED_QUERY).expect("the query passes the checks"),
    )
}

/// The Jira form in advanced mode (feature 1186, contracts/selector-ui.md §1), its run at `run`
/// and its result list open or not.
fn jira_advanced_state(draft: &str, run: QueryRun, list_open: bool) -> StateUnderTest {
    let mut state = with_project();
    let mut form = jira_form(IssueList::NotRequested);
    form.jira_query = JiraQuery {
        mode: QueryMode::Advanced,
        draft: draft.to_string(),
        last_run: if matches!(run, QueryRun::Idle) {
            String::new()
        } else {
            ADVANCED_QUERY.to_string()
        },
        run,
        highlight: None,
        list_open,
    };
    state.worktree_form.form = Some(form);
    StateUnderTest::new(state)
}

/// The advanced results the list shows, as `add-worktree-dialog-issue-list-labels` holds them for
/// the basic list: twenty labels, which wrap; one; and none. Small enough that every row lies wholly
/// inside the list's viewport, which the rows gate needs.
fn jira_advanced_results(complete: bool) -> QueryRun {
    let issue = |key: &str, number: u64, title: &str, labels: &[&str], reporter: &str| {
        Issue::jira(
            key,
            number,
            title.to_string(),
            labels.iter().map(|l| l.to_string()).collect(),
            "2026-09-29T00:00:00Z".to_string(),
        )
        .reported_by(reporter)
    };
    let issues = vec![
        issue(
            "ABC-310",
            310,
            "Tooltip opens under the pointer",
            &[
                "area-add-worktree",
                "area-daemon",
                "area-picker",
                "area-sandbox",
                "area-sidebar",
                "area-terminal",
                "bug",
                "confirmed",
                "design",
                "documentation",
                "good-first-issue",
                "help-wanted",
                "linux",
                "macos",
                "needs-reproduction",
                "performance",
                "priority-high",
                "regression",
                "ux",
                "windows",
            ],
            "octocat",
        ),
        issue(
            "ABC-42",
            42,
            "Crash when opening empty project",
            &["bug"],
            "hubot",
        ),
        issue("ABC-7", 7, "Sidebar flickers on resize", &[], "monalisa"),
    ];
    QueryRun::Loaded {
        scoped: advanced_scoped(),
        listing: IssueListing {
            total_open: issues.len() as u64,
            issues,
            complete,
        },
    }
}

/// The add-worktree form with the issue list **open** over `issues` (feature 038, T006).
///
/// Open, so the rows are recorded: the list is floated from `Widget::overlay`, and a closed one
/// leaves the fixture nothing but the field. `add-worktree-dialog-issue-loaded` keeps it closed and
/// is where the field is covered; these states are where the rows are.
///
/// The load is complete and small. Every row has to lie wholly inside the list's eight-row viewport
/// for `gates/issue_rows_show_all_text.rs` to read it, and a list that scrolled would hang its
/// content outside the viewport, which `gates/containment.rs` would then have to be told about.
fn issue_list_state(issues: Vec<Issue>) -> StateUnderTest {
    let mut state = with_project();
    // Derived the way the application derives it, as the branch picker's state does.
    let issue_matches = rank(&issues, |i| i.row_text(), &Query::new(""));
    let total_open = issues.len() as u64;
    let mut form = issue_form(IssueList::Loaded {
        listing: IssueListing {
            issues,
            total_open,
            complete: true,
        },
        gh: std::path::PathBuf::from("/usr/bin/gh"),
        searched: Vec::new(),
        search: SearchState::Idle,
        descriptions: Default::default(),
    });
    form.issue_matches = issue_matches;
    form.issue_list_open = true;
    form.type_ = None;
    state.worktree_form.form = Some(form);
    StateUnderTest::new(state)
}

/// An issue as GitHub returns one, reported by `reporter`.
fn reported_issue(number: u64, title: &str, labels: &[&str], reporter: &str) -> Issue {
    Issue::new(
        number,
        title.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "2026-09-29T00:00:00Z".to_string(),
    )
    .reported_by(reporter)
}

/// The longest title GitHub accepts, in characters.
const LONGEST_TITLE: usize = 256;

/// `pattern` repeated out to exactly the longest title GitHub accepts.
///
/// The length is asserted rather than trusted: a pattern whose cut lands on a space would lose it
/// to nothing visible, and the state would then cover a 255-character title while its name says
/// otherwise.
fn longest_title(pattern: &str) -> String {
    let title: String = pattern.chars().cycle().take(LONGEST_TITLE).collect();
    assert_eq!(
        title.trim().chars().count(),
        LONGEST_TITLE,
        "the pattern has to end on a visible character at {LONGEST_TITLE}",
    );
    title
}

/// The two action anchors every add-worktree state names.
const ADD_WORKTREE_ANCHORS: &[Anchor] = &[
    Anchor {
        name: "dialog.root",
        path: &[],
    },
    Anchor {
        name: "dialog.actions",
        path: &[8, 0, 0, 1],
    },
];

/// A fixed project path. Never canonicalised against a real directory.
const PROJECT: &str = "/fixture/project";

/// A second project, never active. Only the switcher's covered state needs one — it is what makes
/// an *unmarked* row exist to be aligned against a marked one.
const OTHER: &str = "/fixture/other-project";

/// The toolbar's title, at the same path in every state: shell column → toolbar column → bar
/// container → bar row → leading child.
const TOOLBAR_TITLE: &[usize] = &[0, 0, 0, 0, 0, 0];

/// The app bar itself — §7.1's 64dp box, one level above the row that holds the title and the
/// actions, and one below the column that adds its divider. It is the band BUG-003's panels were
/// drawn across, so `gates/panel_placement.rs` is about this node and the states that open a panel
/// name it (T100).
const APP_BAR: &[usize] = &[0, 0, 0, 0];
/// The overflow (⋮) trigger, trailing action of the bar. The panel that clipped it opens from here.
const APP_BAR_OVERFLOW_TRIGGER: &[usize] = &[0, 0, 0, 0, 0, 3];
/// The project-switcher trigger, immediately left of it (feature 008, FR-004) — the same
/// `IconButton` since BUG-007, which is why the two paths now differ only in their last index.
const APP_BAR_SWITCHER_TRIGGER: &[usize] = &[0, 0, 0, 0, 0, 2];

// A layer index is **two per open-able surface**, and the paths below that begin at one were
// re-pointed twice on the way here. First 018's BUG-007 made the switcher a `MenuOverlay`, which is
// pushed whether or not it is open — it owns its own fade, so it must outlive the flag that opened
// it — giving every state one more layer. Then 017's BUG-002 made a surface's *backdrop*
// unconditional too, because a backdrop that came and went renumbered the panels above it and they
// inherited each other's transitions.
//
// (Both of those were written here as "BUG-007" and "BUG-008", unqualified, in a file whose other
// bug references are 018's. The second named no bug that existed: the backdrop fix is 017's
// BUG-002, per the commit that made it. 018's BUG-008 is a different bug entirely — the sidebar's
// context menus opening at a corner — and it arrived to find its number already spoken for.)
//
// So the arithmetic is now stated rather than discovered: base, then a backdrop and a panel for
// each surface `ui::view` pushes, in `stack_order`. A dialog sits above the two popovers, at layer
// 6; the overflow menu's panel is 2 and the switcher's is 4.

/// The tab strip above the panes (feature 1126, S7): the session area's second child, between the
/// header and the pane area. Filled in from the recorded tree rather than derived by reading the
/// view; an anchor that does not resolve fails by name
/// (`an_anchor_whose_path_does_not_resolve_fails_naming_it`), so a stale path here cannot go quiet.
pub const SESSION_STRIP: &[usize] = &[0, 0, 1, 1, 1];
/// The strip's own row (inside its toolbar surface): the scrolling members, the status cluster,
/// **New session** and the split buttons, a child list that never varies.
pub const SESSION_STRIP_ROW: &[usize] = &[0, 0, 1, 1, 1, 0];
/// The keyed row of members inside the scrolling region: `EdgeFade` → stack → layer → viewport →
/// row. No padding container since feature 027's trailing slack went (R9, R14).
pub const SESSION_STRIP_TABS: &[usize] = &[0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0];
/// **New session**, the strip row's third child.
pub const SESSION_STRIP_NEW_SESSION: &[usize] = &[0, 0, 1, 1, 1, 0, 2];
/// The split buttons, the strip row's last child.
pub const SESSION_STRIP_SPLIT: &[usize] = &[0, 0, 1, 1, 1, 0, 3];

/// The width the strip hands its scrolling region at the fixture's 1280dp window, as recorded: the
/// running application learns it from `Scrollable::on_viewport_resize`, a hand-built state does not,
/// and the edge fade reads it.
const TAB_STRIP_VIEWPORT: u32 = 837;

/// The member a state marks: `n` steps into the keyed row.
const fn strip_member(n: usize) -> [usize; 12] {
    [0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, n]
}
const SESSION_STRIP_TAB_ACTIVE: [usize; 12] = strip_member(2);
const SESSION_STRIP_TAB_EXITED: [usize; 12] = strip_member(3);
const SESSION_STRIP_TAB_SESSION: [usize; 12] = strip_member(0);
/// The Review tab of [`review_shown`] and [`review_hidden`]: after the main session, its shell,
/// the "+" and the helper (feature 1269).
const SESSION_STRIP_TAB_REVIEW: [usize; 12] = strip_member(4);

/// The session area's context header (feature 1126, A1): the session area's first child, above
/// the pane area, which moved one level down with it.
const SESSION_HEADER: &[usize] = &[0, 0, 1, 1, 0];

/// The header's Changes button (feature 1126, H9): the title line's second child, after the title.
pub const SESSION_HEADER_CHANGES: &[usize] = &[0, 0, 1, 1, 0, 0, 0, 1];

/// A **nested** sidebar row — the session under an expanded `feat-short`, at depth 1 in the tree
/// (BUG-005, T116). The sidebar's tree column is `…/2/0/0`, whose children are its rows in order:
/// the Idle group's header (feature 1125), the `Default` project row, then `feat-short`, then its
/// session, then the two remaining worktrees. Index 3 is that session.
///
/// Named because this is the node whose height differed from its siblings' for two days with no
/// fixture able to say so — every other covered state stops at depth 0.
const SIDEBAR_SESSION_ROW: &[usize] = &[0, 0, 1, 0, 0, 0, 2, 0, 0, 3];

/// Deliberately long, so the label/close-button relationship this gate was built to watch is under
/// real pressure at the canonical window size (FR-008b, FR-018).
const LONG_NAME: &str = "feat-a-deliberately-long-worktree-name-that-crowds-its-controls";

fn worktree(dir_name: &str, branch: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from(PROJECT)
            .join(".claude/worktrees")
            .join(dir_name),
        branch: Some(branch.to_string()),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// A project open, with a stable set of worktrees — the base every non-empty state builds on.
fn with_project() -> State {
    let mut workspace = super::workspace_with(vec![(PROJECT, vec![])]);
    // `workspace_with` ends by clearing `active`, which every other caller wants. Here it is the
    // difference between rendering the project surface and rendering "Open a folder to set it as
    // your working space." — four covered states were byte-identical until this line existed.
    workspace.active = workspace.projects.first().map(|p| p.path.clone());

    let state = State {
        sidebar: sidebar::State {
            width: 260,
            ..Default::default()
        },

        workspace,
        worktree: worktree::State {
            worktrees: vec![
                worktree("feat-short", "feat/short"),
                worktree(LONG_NAME, "feat/long"),
                worktree("fix-a-bug", "fix/a-bug"),
            ],
            ..Default::default()
        },
        ..State::default()
    };
    assert!(
        state.workspace.active_project().is_some(),
        "the covered state must have a project open, or it is not covering what it claims"
    );
    state
}

/// The project with worktrees offering six tag filters — four types, *issue* and *untyped* — at
/// sidebar `width` (feature 1125, T050).
fn with_six_filters(width: u16) -> State {
    let mut state = with_project();
    state.sidebar.width = width;
    state.worktree.worktrees = vec![
        worktree("feat-42_login", "feat/42_login"),
        worktree("fix-1117_ci-red", "fix/1117_ci-red"),
        worktree("docs-guide", "docs/guide"),
        worktree("chore-deps", "chore/deps"),
        worktree("scratch", "scratch"),
    ];
    assert_eq!(
        state.available_tag_filters().len(),
        6,
        "the chip bar must have six filter chips on offer"
    );
    state
}

/// The project with pull request status on, an open pull request with failing checks on the first
/// worktree's branch and a merged one on the third's (feature 040).
fn with_pull_requests(width: u16) -> State {
    use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
    let status = |number: u64, state| PullRequestStatus {
        number,
        title: format!("Pull request {number}"),
        url: format!("https://github.com/o/r/pull/{number}"),
        state,
        review: ReviewState::None,
        head: "abc".to_string(),
        conflict: false,
    };
    let mut state = with_project();
    state.sidebar.width = width;
    state.pr_status.enabled = true;
    state.pr_status.held = true;
    // Read just now, so the rows draw the current form; `with_stale_pull_requests` ages it.
    state.pr_status.read_at = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    );
    state.pr_status.statuses.insert(
        "feat/short".to_string(),
        status(
            7,
            PrState::Open {
                checks: CheckStatus::Failing,
            },
        ),
    );
    state
        .pr_status
        .statuses
        .insert("fix/a-bug".to_string(), status(8, PrState::Merged));
    state
}

/// [`with_pull_requests`], read long ago: every indicator is drawn in the stale form (feature 040
/// US4, FR-019).
fn with_stale_pull_requests(width: u16) -> State {
    let mut state = with_pull_requests(width);
    state.pr_status.read_at = Some(1_000);
    state
}

/// [`with_pull_requests`], and the merged pull request's branch holds nothing newer: its row carries
/// the **can be removed** chip beside the indicator (feature 040 US3).
fn with_removable_pull_request(width: u16) -> State {
    let mut state = with_pull_requests(width);
    state.pr_status.removable.insert("fix/a-bug".to_string());
    state
}

/// *Default* and one worktree per card state (feature 1125, T018): `needs-action` waits on a
/// permission prompt, `fix-checks` has an open pull request with failing checks (Needs attention),
/// `feat-working` a working session, `idle-one` nothing, and `merged-one` a merged pull request
/// holding all of its work (Done, collapsed as it starts).
fn with_every_card_state() -> State {
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
    use micold_core::session::SessionLifecycle;
    let mut state = with_project();
    state.worktree.worktrees = vec![
        worktree("needs-action", "feat/needs-action"),
        worktree("fix-checks", "fix/checks"),
        worktree("feat-working", "feat/working"),
        worktree("idle-one", "feat/idle-one"),
        worktree("merged-one", "feat/merged-one"),
    ];
    let session = |dir: &str, activity: ActivitySignal, paused: bool| {
        let mut session = Session::start_new(
            SessionLocation::Worktree(dir.to_string()),
            AiCli::ClaudeCode,
        );
        session.lifecycle = SessionLifecycle::Running;
        session.activity = activity;
        session.paused_on_user = paused;
        session
    };
    state
        .workspace
        .sessions
        .entry(PathBuf::from(PROJECT))
        .or_default()
        .extend([
            session("needs-action", ActivitySignal::AwaitingInput, true),
            session("feat-working", ActivitySignal::Working, false),
        ]);
    let status = |number: u64, state| PullRequestStatus {
        number,
        title: format!("Pull request {number}"),
        url: format!("https://github.com/o/r/pull/{number}"),
        state,
        review: ReviewState::None,
        head: "abc".to_string(),
        conflict: false,
    };
    state.pr_status.enabled = true;
    state.pr_status.held = true;
    state.pr_status.read_at = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    );
    state.pr_status.statuses.insert(
        "fix/checks".to_string(),
        status(
            7,
            PrState::Open {
                checks: CheckStatus::Failing,
            },
        ),
    );
    state
        .pr_status
        .statuses
        .insert("feat/merged-one".to_string(), status(8, PrState::Merged));
    state
        .pr_status
        .removable
        .insert("feat/merged-one".to_string());
    state
}

/// A structured session's pane showing its transcript (feature 939, T038): the recorded `tool-turn`
/// history, its tool card unfolded, and the view scrolled away from the end, so "Jump to latest"
/// floats over it (FR-017).
pub fn transcript_pane_detached() -> State {
    use micold_client::features::structured_transcript::TranscriptMsg;
    use micold_core::structured::transcript::EntryKind;

    let (mut state, id) = structured_pane(&super::transcript::fixture_lines("tool-turn"));
    let card = state
        .structured_transcript
        .transcript(id)
        .and_then(|t| {
            t.entries()
                .find(|e| matches!(e.kind(), EntryKind::ToolCall(_)))
                .map(|e| e.key().clone())
        })
        .expect("the tool turn holds a tool card");
    state.structured_transcript.toggle(id, card);
    // Scrolled to the top of a transcript taller than its viewport: detached (R9).
    state.update_transcript(TranscriptMsg::Scrolled {
        session: id,
        offset: 0,
        viewport: 300,
        content: 3000,
    });
    state
}

/// A structured session's pane drawn rich (feature 940, T037): the recorded `markdown-turn` reply —
/// headings, lists, its table, three code blocks — then `edit-turn` with its multi-line Edit card
/// unfolded on its diff, in a pane narrowed by the widest sidebar, following the end.
pub fn transcript_pane_rich() -> State {
    use micold_core::structured::transcript::EntryKind;

    let lines: Vec<String> = ["markdown-turn", "edit-turn"]
        .iter()
        .flat_map(|name| super::transcript::fixture_lines(name))
        .collect();
    let (mut state, id) = structured_pane(&lines);
    state.sidebar.width = micold_client::app::SIDEBAR_MAX_WIDTH;
    let card = state
        .structured_transcript
        .transcript(id)
        .and_then(|t| {
            t.entries()
                .rfind(|e| matches!(e.kind(), EntryKind::ToolCall(c) if c.name == "Edit"))
                .map(|e| e.key().clone())
        })
        .expect("the edit turn holds an Edit card");
    state.structured_transcript.toggle(id, card);
    state
}

/// A structured session's pane with feature 944's session bar and one open subagent thread
/// (T037): the recorded `subagent-turn`, then a compaction threshold below the context used (the
/// meter's warning) and a background command still running, its CLI running, the thread of the
/// `Agent` call unfolded.
pub fn transcript_pane_insights() -> State {
    use micold_core::session::SessionLifecycle;

    let mut lines = super::transcript::fixture_lines("subagent-turn");
    lines.push(
        serde_json::json!({
            "type": "autocompact_state",
            "value": {"enabled": true, "threshold": 30_000},
            "session_id": "<SESSION_ID>",
        })
        .to_string(),
    );
    lines.push(
        serde_json::json!({
            "type": "system", "subtype": "task_started", "session_id": "<SESSION_ID>",
            "task_id": "bg1", "tool_use_id": "toolu_bg", "description": "sleep 20; echo done",
            "task_type": "local_bash", "is_backgrounded": true,
        })
        .to_string(),
    );
    let (mut state, id) = structured_pane(&lines);
    for session in state
        .workspace
        .sessions
        .values_mut()
        .flatten()
        .filter(|s| s.id == id)
    {
        session.lifecycle = SessionLifecycle::Running;
    }
    let main = state
        .structured_transcript
        .transcript(id)
        .expect("the pane holds a transcript")
        .clone();
    let insights = state.structured_insights.insights(id).expect("insights");
    let parent = insights
        .threads_owned_by(&micold_core::structured::insights::Owner::Main, &main)
        .first()
        .map(|t| t.parent.clone())
        .expect("the recording holds a subagent thread");
    state.structured_insights.toggle_thread(id, &parent, &main);
    state
}

/// The project with one structured session open in its pane, its transcript `lines` applied and its
/// render cache refreshed as the window's update does.
pub fn structured_pane(lines: &[String]) -> (State, SessionId) {
    session_pane(micold_core::session::SessionKind::Structured, lines)
}

/// As [`structured_pane`], with a session of `kind` (feature 941: a terminal session's pane for
/// the composer gate).
pub fn session_pane(
    kind: micold_core::session::SessionKind,
    lines: &[String],
) -> (State, SessionId) {
    let session = Session::restored_of_kind(
        SessionId::new(),
        SessionLocation::Worktree("feat-short".to_string()),
        SessionLabel::Named("feat/short".to_string()),
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
        kind,
    );
    let id = session.id;
    let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = with_project();
    state.workspace = workspace;
    state.session.active = Some(id);
    let _jobs = state.structured_events(
        id,
        0,
        &super::transcript::sequenced(lines, 0),
        false,
        &micold_client::ui::syntax::GRAMMAR,
    );
    (state, id)
}

/// The registry. **This is the one place a covered state is declared** (FR-016).
pub fn covered_states() -> &'static [CoveredState] {
    &[
        // --- Feature 017's reduced parity set (FR-008) ------------------------------------------
        CoveredState {
            name: "main-shell-sidebar-expanded",
            build: || StateUnderTest::new(with_project()),
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "shell.body",
                    path: &[0],
                },
                // The two elements feature 017's defect was *between* — a name drawn across the
                // control beside it. T023 asked for these and they were missing until quickstart
                // Part B4 was run against a reintroduced defect and reported a bare path.
                //
                // Both are identified from the running gate rather than by reading the tree:
                // the label is the node B4's overflow names (187.6px allowed), and the cluster
                // beside it is `row_actions_cluster`, whose destructive control is Delete, not a
                // close — named for what it is.
                Anchor {
                    name: "sidebar.row.label",
                    // Re-pointed under BUG-005 (T115): the row gained one level when §7.2's height
                    // moved onto it, since a floor on a two-line row has to be a sibling of the
                    // whole body rather than of one of its lines. `…/2/0` is now the height spacer
                    // and `…/2/1` the content, so every path below the row gains a `1`. The gate
                    // named this anchor rather than letting it drift onto a neighbouring node,
                    // which is the whole reason anchors are named (019 FR-004).
                    // Re-pointed under feature 905 (T008): the row gained its state-icon slot after
                    // the twisty, so the label moved from `…/0/1` to `…/0/2` and the action
                    // cluster from `…/0/2` to `…/0/3`.
                    // Re-pointed under feature 1125 (T018): the Idle group's header is the tree's
                    // first row, so the long-named worktree's row moved from index 2 to 3.
                    path: &[0, 0, 1, 0, 0, 0, 2, 0, 0, 3, 1, 0, 2],
                },
                Anchor {
                    name: "sidebar.row.delete_button",
                    path: &[0, 0, 1, 0, 0, 0, 2, 0, 0, 3, 1, 0, 3, 1],
                },
                // `Toolbar`'s leading `text(title)` (`material/toolbar.rs:46`). Named on both shell
                // states rather than on all eleven: the toolbar is behind every dialog too, and an
                // anchor repeated on a state whose failures are never about it is noise.
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
            ],
        },
        // **Feature 029's busy header is deliberately not registered here** (T032). A refresh in
        // flight withholds the control's `on_press` and swaps its tooltip string, and neither is
        // geometry: an iced `Button` lays out the same whether or not it can be pressed, and a
        // tooltip's own text goes through the overlay pass on hover, never at rest. So the state
        // exists, and its fixture block would be `main-shell-sidebar-expanded` again, byte for
        // byte — which is the one thing this file's own history warns about. `error-add-worktree-
        // failed` and `error-project-unavailable` were each byte-identical to the state above
        // until they were built differently, and both say so where they sit.
        //
        // A comment is not a check, so the claim is one: `gates/refresh_busy_holds_the_header.rs`
        // resolves both headers and fails if any element moves, telling whoever moved it to
        // register the state here. Answer FR-006 with a spinner or a second glyph later and that
        // gate is what says the escape clause has expired.
        CoveredState {
            name: "main-shell-sidebar-collapsed",
            build: || {
                let mut state = with_project();
                state.sidebar.hidden = true;
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
            ],
        },
        // Feature 1125 (T018): every group header, Done collapsed as it starts; with the Overview
        // open (T040, R15), so the selected Overview row is recorded too.
        CoveredState {
            name: "main-shell-sidebar-grouped",
            build: || {
                let mut state = with_every_card_state();
                state.overview.open = true;
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // Feature 1125 (T050, US4.7, FR-016): the chip bar with six filter chips on offer (four
        // types, *issue*, *untyped*) at the narrowest, default and widest sidebar: at 180px they
        // wrap onto further lines left of add, hide and more.
        CoveredState {
            name: "main-shell-sidebar-chip-bar-narrowest",
            build: || StateUnderTest::new(with_six_filters(micold_client::app::SIDEBAR_MIN_WIDTH)),
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        CoveredState {
            name: "main-shell-sidebar-chip-bar-default",
            build: || {
                StateUnderTest::new(with_six_filters(micold_client::app::SIDEBAR_DEFAULT_WIDTH))
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        CoveredState {
            name: "main-shell-sidebar-chip-bar-widest",
            build: || StateUnderTest::new(with_six_filters(micold_client::app::SIDEBAR_MAX_WIDTH)),
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        CoveredState {
            name: "add-worktree-dialog-new-branch",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                // The Create/Cancel row. **The index differs per state and that is not avoidable**:
                // it is the last child of a column whose length depends on which fields the form
                // shows, so there is no single path that means "the actions" across all of them.
                // `the_action_row_anchors_are_action_rows` holds each one to the signature rather
                // than to the number.
                Anchor {
                    name: "dialog.actions",
                    path: &[8, 0, 0, 1],
                },
            ],
        },
        CoveredState {
            name: "add-worktree-dialog-existing-branch",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::Existing,
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                // Two fewer fields than the new-branch form: one picker in place of the select,
                // the ticket and the name. It was three until §7.7 moved the select's
                // free-standing `Type` label inside the control's own container.
                Anchor {
                    name: "dialog.actions",
                    path: &[8, 0, 0, 1],
                },
            ],
        },
        // 016 BUG-003 item 1's own finding, closed. The state above renders the dialog *before* any
        // branch listing has arrived, so `candidates` is empty and the picker draws a caption
        // instead of a field — which meant the fixture had never contained the search field at all.
        // That is a second reason no gate saw the icon drawn over the label: not only is a layout
        // tree blind to overlap, this one was not looking at the control.
        CoveredState {
            name: "add-worktree-dialog-branch-picker",
            build: || {
                let mut state = with_project();
                let candidates = vec![
                    BranchCandidate {
                        name: "feat/login-page".to_string(),
                        origin: BranchOrigin::Local,
                        blocked_by: None,
                    },
                    BranchCandidate {
                        name: "fix/crash-on-open".to_string(),
                        origin: BranchOrigin::Local,
                        blocked_by: None,
                    },
                    BranchCandidate {
                        name: "chore/bump-deps".to_string(),
                        origin: BranchOrigin::Remote {
                            remote: "origin".to_string(),
                        },
                        blocked_by: None,
                    },
                ];
                // Derived the way the application derives it, rather than written out: an invented
                // `branch_matches` could disagree with `candidates` in a way no reducer can
                // produce, and the fixture would then pin a state that cannot happen.
                let branch_matches = rank(&candidates, |c| c.name.as_str(), &Query::new(""));
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::Existing,
                    candidates,
                    branch_matches,
                    // Closed: the list is an overlay, and this state exists for the **field** —
                    // the node that was missing. `worktree-menu-open` is where an open overlay is
                    // covered.
                    branch_list_open: false,
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                Anchor {
                    name: "dialog.actions",
                    path: &[8, 0, 0, 1],
                },
            ],
        },
        // Feature 034: the GitHub issue chip, disabled, with the reason under the switch.
        CoveredState {
            name: "add-worktree-dialog-github-unavailable",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(WorktreeForm {
                    github: GithubAvailability::Unavailable(
                        "This repository has no GitHub remote.".to_string(),
                    ),
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-issue-loading",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(issue_form(IssueList::Loading { seq: 1 }));
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-issue-failed",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(issue_form(IssueList::Failed {
                    error: IssueLoadError::NotSignedIn,
                }));
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Feature 533 (FR-016): a project with both sources shows the GitHub / Jira choice.
        CoveredState {
            name: "add-worktree-dialog-source-choice",
            build: || {
                let mut state = with_project();
                let mut form = jira_form(IssueList::NotRequested);
                form.source = BranchSource::New;
                form.github = github_repo();
                form.provider = IssueProvider::Github;
                state.worktree_form.form = Some(form);
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Feature 533: the Jira source loading, refused, loaded past the cap and empty.
        CoveredState {
            name: "add-worktree-dialog-jira-loading",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(jira_form(IssueList::Loading { seq: 1 }));
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-credentials-refused",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(jira_form(IssueList::Failed {
                    error: IssueLoadError::CredentialsRefused,
                }));
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-loaded",
            build: || {
                let mut state = with_project();
                let issue = |key: &str, number: u64, title: &str, labels: &[&str]| {
                    Issue::jira(
                        key,
                        number,
                        title.to_string(),
                        labels.iter().map(|l| l.to_string()).collect(),
                        "2026-09-29T00:00:00Z".to_string(),
                    )
                };
                let issues = vec![
                    issue("ABC-123", 123, "Add login rate limit", &["backend"]),
                    issue("ABC-42", 42, "Crash when opening empty project", &["bug"]),
                ];
                let issue_matches = rank(&issues, |i| i.row_text(), &Query::new(""));
                let mut form = jira_form(IssueList::Loaded {
                    listing: IssueListing {
                        issues,
                        total_open: 2,
                        complete: false,
                    },
                    gh: std::path::PathBuf::new(),
                    searched: Vec::new(),
                    search: SearchState::Idle,
                    descriptions: Default::default(),
                });
                form.issue_matches = issue_matches;
                form.ticket = "ABC-42".to_string();
                form.picked_issue = Some(42);
                state.worktree_form.form = Some(form);
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-no-open-issues",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(jira_form(IssueList::Loaded {
                    listing: IssueListing {
                        issues: Vec::new(),
                        total_open: 0,
                        complete: true,
                    },
                    gh: std::path::PathBuf::new(),
                    searched: Vec::new(),
                    search: SearchState::Idle,
                    descriptions: Default::default(),
                }));
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Feature 1186: the Jira source in advanced mode — the prompt before any query ran, the
        // results open (the rows gate reads them) and capped, the run in flight, a refused query,
        // no match, and a query Jira rejected with no message.
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-prompt",
            build: || jira_advanced_state("", QueryRun::Idle, false),
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-results",
            build: || jira_advanced_state(ADVANCED_QUERY, jira_advanced_results(true), true),
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-capped",
            build: || jira_advanced_state(ADVANCED_QUERY, jira_advanced_results(false), false),
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-running",
            build: || {
                jira_advanced_state(
                    ADVANCED_QUERY,
                    QueryRun::Running {
                        seq: 1,
                        scoped: advanced_scoped(),
                    },
                    false,
                )
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-refused",
            build: || {
                jira_advanced_state(
                    "status = \"In Review",
                    QueryRun::Refused(QueryRefusal::UnclosedQuote),
                    false,
                )
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-no-match",
            build: || {
                jira_advanced_state(
                    ADVANCED_QUERY,
                    QueryRun::Loaded {
                        scoped: advanced_scoped(),
                        listing: IssueListing {
                            issues: Vec::new(),
                            total_open: 0,
                            complete: true,
                        },
                    },
                    false,
                )
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-rejected",
            build: || {
                jira_advanced_state(
                    ADVANCED_QUERY,
                    QueryRun::Failed {
                        scoped: advanced_scoped(),
                        failure: QueryFailure::Rejected(Vec::new()),
                    },
                    false,
                )
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Feature 1186 M4: Jira's own messages, one line each, and a load failure with Retry.
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-rejected-messages",
            build: || {
                jira_advanced_state(
                    ADVANCED_QUERY,
                    QueryRun::Failed {
                        scoped: advanced_scoped(),
                        failure: QueryFailure::Rejected(vec![
                            "Field 'asignee' does not exist or you do not have permission to view it.".into(),
                            "The value 'Hgh' does not exist for the field 'priority'.".into(),
                        ]),
                    },
                    false,
                )
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-jira-advanced-load-failed",
            build: || {
                jira_advanced_state(
                    ADVANCED_QUERY,
                    QueryRun::Failed {
                        scoped: advanced_scoped(),
                        failure: QueryFailure::Load(IssueLoadError::Unreachable(
                            "connection refused".into(),
                        )),
                    },
                    false,
                )
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Feature 1185: a component chosen over a capped Jira list: the filter row, the active
        // line with Clear filters and the cap caption, the tallest Jira form.
        CoveredState {
            name: "add-worktree-dialog-jira-filtered",
            build: || StateUnderTest::new(super::jira_picker::jira_filtered(with_project())),
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Rows, a long title, and the cap caption: more open issues than the load holds.
        CoveredState {
            name: "add-worktree-dialog-issue-loaded",
            build: || {
                let mut state = with_project();
                let issue = |number: u64, title: &str, labels: &[&str]| {
                    Issue::new(
                        number,
                        title.to_string(),
                        labels.iter().map(|l| l.to_string()).collect(),
                        "2026-09-29T00:00:00Z".to_string(),
                    )
                };
                let issues = vec![
                    issue(1234, LONG_NAME, &["bug", "good first issue"]),
                    issue(42, "Crash when opening empty project", &["bug"]),
                    issue(7, "Sidebar flickers on resize", &[]),
                ];
                // Derived the way the application derives it, as the branch picker's state does.
                let issue_matches = rank(&issues, |i| i.row_text(), &Query::new(""));
                let mut form = issue_form(IssueList::Loaded {
                    listing: IssueListing {
                        issues,
                        total_open: 1_234,
                        complete: false,
                    },
                    gh: std::path::PathBuf::from("/usr/bin/gh"),
                    searched: Vec::new(),
                    search: SearchState::Idle,
                    descriptions: Default::default(),
                });
                form.issue_matches = issue_matches;
                form.type_ = None;
                form.ticket = "42".to_string();
                form.name = "Crash when opening empty project".to_string();
                form.picked_issue = Some(42);
                state.worktree_form.form = Some(form);
                StateUnderTest::new(state)
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Feature 038: the open list's rows, two wrapping lines each (contracts/picker-row.md §6).
        // `gates/issue_rows_show_all_text.rs` reads all three; nothing in them is truncated.
        //
        // The longest title there is, twice. Words break where a title is expected to break; the
        // second has no space in it anywhere, which is the hard case — there is no place to break
        // it but between two glyphs, and a wrap that only knew about words would run it out of the
        // row on one line.
        CoveredState {
            name: "add-worktree-dialog-issue-list-longest-titles",
            build: || {
                issue_list_state(vec![
                    reported_issue(
                        2101,
                        &longest_title("A long issue title is cut off at the edge of its row. "),
                        &["bug"],
                        "octocat",
                    ),
                    reported_issue(
                        2102,
                        &longest_title("crates/micold-client/src/ui/material/picker.rs::"),
                        &["bug"],
                        "a-reporter-with-a-long-login",
                    ),
                ])
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // The second line at its three sizes: twenty labels, which wrap; one; and none, where the
        // line is the reporter alone and the separator must not be left dangling after it.
        CoveredState {
            name: "add-worktree-dialog-issue-list-labels",
            build: || {
                issue_list_state(vec![
                    reported_issue(
                        310,
                        "Tooltip opens under the pointer",
                        &[
                            "area: add-worktree",
                            "area: daemon",
                            "area: picker",
                            "area: sandbox",
                            "area: sidebar",
                            "area: terminal",
                            "bug",
                            "confirmed",
                            "design",
                            "documentation",
                            "good first issue",
                            "help wanted",
                            "linux",
                            "macos",
                            "needs reproduction",
                            "performance",
                            "priority: high",
                            "regression",
                            "ux",
                            "windows",
                        ],
                        "octocat",
                    ),
                    reported_issue(42, "Crash when opening empty project", &["bug"], "hubot"),
                    reported_issue(7, "Sidebar flickers on resize", &[], "monalisa"),
                ])
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // Rows of differing height in one list, the first line taking from one line to several. A
        // list of equal rows cannot show a row placed by the height of the one above it.
        CoveredState {
            name: "add-worktree-dialog-issue-list-mixed-heights",
            build: || {
                issue_list_state(vec![
                    reported_issue(11, "Sidebar flickers on resize", &[], "monalisa"),
                    reported_issue(
                        12,
                        "The add-worktree dialog forgets the base branch after the source \
                         switch is toggled twice",
                        &["bug", "area: add-worktree"],
                        "octocat",
                    ),
                    reported_issue(
                        13,
                        "Restoring a session after the daemon restarts replays the whole \
                         scrollback into the terminal before the prompt, so a long-running \
                         session takes several seconds to become usable again",
                        &["performance"],
                        "hubot",
                    ),
                    reported_issue(
                        14,
                        "When a worktree is deleted from outside the application while one of \
                         its sessions is still attached, the sidebar keeps the row, the row's \
                         menu still offers to open a terminal in it, and choosing that reports \
                         a failure that names neither the worktree nor the directory that is \
                         gone",
                        &["bug", "area: sidebar", "needs reproduction"],
                        "a-reporter-with-a-long-login",
                    ),
                ])
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // The search beyond the cap, running (FR-005a): "Searching GitHub…" (or "Searching Jira…") under the picker.
        CoveredState {
            name: "add-worktree-dialog-issue-searching",
            build: || issue_search_state(SearchState::Searching { seq: 2 }),
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // 837: the picked issue's details awaited (nothing shown), read (every kind, the blocked
        // one a warning) and failed (one muted line). Create and the footer keep their place.
        CoveredState {
            name: "add-worktree-dialog-issue-details-loading",
            build: || issue_details_state(IssueDetailsState::Loading { seq: 1, number: 17 }),
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-issue-details-loaded",
            build: || {
                issue_details_state(IssueDetailsState::Loaded {
                    number: 17,
                    details: Box::new(every_kind_details()),
                })
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "add-worktree-dialog-issue-details-failed",
            build: || {
                issue_details_state(IssueDetailsState::Failed {
                    number: 17,
                    message: IssueLoadError::Offline.message(
                        &GithubRepo::from_remote_url("git@github.com:octo/widgets.git")
                            .expect("a GitHub URL"),
                    ),
                })
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        // The search beyond the cap, failed: the error and Retry under the loaded matches.
        CoveredState {
            name: "add-worktree-dialog-issue-search-failed",
            build: || {
                issue_search_state(SearchState::Failed {
                    error: IssueLoadError::Offline,
                })
            },
            anchors: ADD_WORKTREE_ANCHORS,
        },
        CoveredState {
            name: "worktree-menu-open",
            build: || {
                let mut state = with_project();
                // Opened from the row, not from a corner (018 BUG-008). The point is well down the
                // sidebar precisely because the defect was invisible at the top of the list: the
                // panel used to be recorded at 24, 96 whichever row it belonged to, and a fixture
                // that only ever opened it near there could not tell the two apart.
                state.window.window_size = (1280, 800);
                state.worktree.menu_open = Some(WorktreeMenu {
                    dir_name: LONG_NAME.to_string(),
                    anchor: (120, 420),
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // The other sidebar menu, and the clamp (018 BUG-008, FR-029d / 015 FR-006). Opened close
        // enough to the bottom edge that the panel does not fit below it, so the fixture records a
        // menu that has been slid back inside rather than one that merely happened to fit.
        CoveredState {
            name: "session-menu-open-at-the-bottom-edge",
            build: || {
                let session = Session::restored(
                    SessionId::new(),
                    SessionLocation::Worktree("feat-short".to_string()),
                    SessionLabel::Named("feat/short".to_string()),
                    TerminalMode::Regular,
                    AiCli::ClaudeCode,
                );
                let id = session.id;
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());

                let mut state = with_project();
                state.workspace = workspace;
                state.sidebar.expanded.insert("feat-short".to_string());
                state.window.window_size = (1280, 800);
                state.session.menu_open = Some(SessionMenu {
                    id,
                    anchor: (120, 760),
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // --- The empty and error layouts (FR-008c) ----------------------------------------------
        // The screens least often looked at by eye, which is exactly where a gate earns most over
        // the human inspection feature 017 had to close on.
        CoveredState {
            name: "empty-no-project-open",
            build: || StateUnderTest::new(State::default()),
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        CoveredState {
            name: "empty-project-without-worktrees",
            build: || {
                let mut state = with_project();
                state.worktree.worktrees.clear();
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        CoveredState {
            name: "error-daemon-disconnected",
            build: || {
                StateUnderTest::new(with_project()).connection(ConnectionStatus::Disconnected)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // `worktree_error` is rendered by the add-worktree modal and nowhere else
        // (`ui/mod.rs:357`), so setting it on the main shell covered nothing — this state was
        // byte-identical to `main-shell-sidebar-expanded` until the dialog was opened with it.
        CoveredState {
            name: "error-add-worktree-failed",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                state.worktree_form.worktree_error =
                    Some("could not create the worktree: branch already checked out".to_string());
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                // One past the new-branch form: the error sentence is a field-column child of its
                // own, and it pushes the actions down.
                Anchor {
                    name: "dialog.actions",
                    path: &[8, 0, 0, 1],
                },
            ],
        },
        // FR-008c's second required layout, and the last of the three to be covered. `shell.rs`
        // renders an unavailable folder as a plain `Button::filled("Unavailable")` where an
        // available one gets an icon-plus-label composite, so the two are different geometry.
        //
        // It has to be *set* rather than arranged, because `FakeScanner::default()` answers
        // `is_available: true` and every state built through `workspace_with` therefore takes the
        // available branch by construction. That is why this was missed: not an oversight about
        // which states to add, but a scaffold that could not produce the state at all.
        //
        // No project active, which is the honest form of it — the folder is gone, `Workspace::
        // activate` refuses to open an unavailable project, and this is what a restart shows.
        CoveredState {
            name: "error-project-unavailable",
            build: || {
                let mut state = with_project();
                state.workspace.active = None;
                for project in &mut state.workspace.projects {
                    project.availability = Availability::Unavailable;
                }
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // --- The one state that scrolls (FR-011) ------------------------------------------------
        // Until this existed, nothing in the fixture overflowed its viewport. FR-011 requires
        // scroll-dependent geometry to be recorded at a defined offset, and `a_fresh_tree_samples_
        // at_rest` proved a fresh tree reports every scrollable at the top — over `State::default()`,
        // where nothing scrolls. The guarantee held over a tree in which no element's geometry
        // depended on scroll position, which is the overlay pass's failure exactly.
        //
        // The sidebar's list is the only scrollable in the application whose content is driven by
        // state, so overflowing it is the way in. The count is deliberate rather than generous:
        // enough that the content exceeds a 764.8px viewport with margin, and no more.
        CoveredState {
            name: "main-shell-sidebar-scrolled-to-top",
            build: || {
                let mut state = with_project();
                state.worktree.worktrees = (0..30)
                    .map(|i| worktree(&format!("feat-{i:02}"), &format!("feat/{i:02}")))
                    .collect();
                // Replacing the list leaves `with_project`'s records pointing at worktrees that
                // are gone, so re-record: without this the list holds thirty rows and shows none,
                // and the one state that scrolls stops scrolling.
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // --- The one state that exercises the overlay pass (FR-009) -----------------------------
        // Until this existed the fixture contained **zero** `over` records. The overlay pass was
        // written for `material::Select`, which lays its list out through `Widget::overlay` where
        // the base walk cannot see it — and then no covered state ever opened one. A pass that runs
        // on every state and records nothing looks exactly like a pass that found nothing, which is
        // the failure this feature exists to correct.
        //
        // The select's open flag is its own widget state with no accessor, so it is *caused* rather
        // than set: `pressing` dispatches a left press at the control's centre, the way a person
        // opens it. Still true after feature 022 replaced the `pick_list` this was written against
        // — the control changed, its openness stayed its own. See `resolve_pressing` for why the
        // entrance transition has to be settled first, and the list's arrival afterwards.
        CoveredState {
            name: "add-worktree-dialog-type-menu-open",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                // Index 4: the heading, the source switch, then the caption under it (feature 034),
                // the `* required` legend (feature 937), then the Type row, whose first child is
                // the select (feature 776: the AI CLI select shares the row).
                StateUnderTest::new(state).pressing(&[8, 0, 0, 0, 4, 0])
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                Anchor {
                    name: "dialog.type-select",
                    path: &[8, 0, 0, 0, 4, 0],
                },
                // Same form as `add-worktree-dialog-new-branch`; opening the menu adds an overlay
                // layer, not a field.
                Anchor {
                    name: "dialog.actions",
                    path: &[8, 0, 0, 1],
                },
            ],
        },
        // --- Added to exercise FR-016 end-to-end (T032) ------------------------------------------
        // The Settings view: the tallest form the application shows, and the only covered state
        // with a navigation rail, a checkbox, a text field carrying a validation error, and a
        // control row that has to fit two actions below a scrolling page. Nothing about
        // registering it needed a change outside this file, which is the point of adding it.
        //
        // Feature 027 turned it from a 420dp modal into a full-surface view, so what this records
        // is a *content* layout rather than a dialog's — the rail beside the page, and the actions
        // under it, both of which are new geometry that nothing else here covers.
        //
        // Every value is invented and fixed (FR-007) rather than taken from `SettingsDraft::
        // default()`, whose fields track the shipped defaults and would silently re-record the
        // fixture the day one of them changes. The daemon half is the exception the same rule
        // asks for: the Terminal section is the one on screen, so none of it is laid out — but the
        // shared credential is, as the rail's badge, so that is stated.
        CoveredState {
            name: "settings-view-with-validation-error",
            build: || {
                let mut state = with_project();
                let mut draft = SettingsDraft {
                    jira: Default::default(),
                    required: Default::default(),
                    github: Default::default(),
                    section: SettingsSection::Terminal,
                    appearance: AppearanceDraft {
                        theme: micold_core::theme::ThemePreference::Dark,
                        reduce_motion: false,
                    },
                    terminal: TerminalDraft {
                        scrollback_lines: "12000".to_string(),
                        save_terminal_history: true,
                    },
                    environment: EnvironmentDraft {
                        enabled: true,
                        script_path: "~/.config/micold/session-env.sh".to_string(),
                        timeout_secs: "5".to_string(),
                        default_ai_cli: micold_core::session::AiCli::ClaudeCode,
                        default_prompt_template: String::new(),
                        default_prompt_editor: Default::default(),
                        pi_activity_component: true,
                        tool_server_enabled: true,
                        worktree_guard_enabled: true,
                        desktop_notifications: true,
                        notification_kinds: Default::default(),
                        long_task_threshold_secs: "60".to_string(),
                        plan_usage_enabled: true,
                        plan_usage_warn_percent: "80".to_string(),
                        cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                    },
                    daemon: DaemonDraft::default(),
                    error: None,
                    baseline: None,
                };
                draft
                    .daemon
                    .profile
                    .credentials
                    .insert(micold_core::sandbox::CredentialShare::GitConfig);
                // Through `report`, not by setting the field: the section a rejection shows is the
                // rejection's own business (FR-029), and a fixture that set both by hand could
                // record a pairing the application cannot produce.
                draft.report(FieldError {
                    field: FieldId::SettingsScrollback,
                    section: SettingsSection::Terminal,
                    message: "Enter a number between 100 and 1000000.".to_string(),
                });
                state.settings.settings_draft = Some(draft);
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "settings.rail",
                    path: &[0, 0, 1, 0, 0, 0],
                },
                Anchor {
                    name: "dialog.actions",
                    path: &[0, 0, 1, 0, 1, 1],
                },
            ],
        },
        // The same surface with the rail collapsed to its icons (feature 027, FR-026c/d).
        //
        // A second state rather than a variation of the one above, because the two differ in the
        // one thing a fixture is for: the rail's width, and therefore where the section beside it
        // starts. FR-026c says the width the labels give up goes to the content, and until this
        // was recorded nothing anywhere held that to a number.
        //
        // The `Appearance` section, not `Terminal`: it is the shortest page here, so what the
        // fixture records of it is the rail and the boundary rather than a form that would move
        // whenever a field's copy changed.
        CoveredState {
            name: "settings-view-rail-collapsed",
            build: || {
                let mut state = with_project();
                let mut draft = SettingsDraft {
                    jira: Default::default(),
                    required: Default::default(),
                    github: Default::default(),
                    section: SettingsSection::Appearance,
                    appearance: AppearanceDraft {
                        theme: micold_core::theme::ThemePreference::Dark,
                        reduce_motion: false,
                    },
                    terminal: TerminalDraft {
                        scrollback_lines: "12000".to_string(),
                        save_terminal_history: true,
                    },
                    environment: EnvironmentDraft {
                        enabled: true,
                        script_path: "~/.config/micold/session-env.sh".to_string(),
                        timeout_secs: "5".to_string(),
                        default_ai_cli: micold_core::session::AiCli::ClaudeCode,
                        default_prompt_template: String::new(),
                        default_prompt_editor: Default::default(),
                        pi_activity_component: true,
                        tool_server_enabled: true,
                        worktree_guard_enabled: true,
                        desktop_notifications: true,
                        notification_kinds: Default::default(),
                        long_task_threshold_secs: "60".to_string(),
                        plan_usage_enabled: true,
                        plan_usage_warn_percent: "80".to_string(),
                        cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                    },
                    daemon: DaemonDraft::default(),
                    error: None,
                    baseline: None,
                };
                // The badge again, for the reason the state above states it: collapsed, a badge
                // has nowhere to be a chip and becomes a tint on the glyph, so a rail without one
                // would record a row shape the application does not always produce.
                draft
                    .daemon
                    .profile
                    .credentials
                    .insert(micold_core::sandbox::CredentialShare::GitConfig);
                state.settings.settings_draft = Some(draft);
                state.settings.settings_rail_collapsed = true;
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        // --- Feature 1126: the strip above the panes, at one tab (S9, SC-004) ---------------------
        //
        // It was `session-terminal-bottom-bar` (BUG-002), the bar under the terminal, which this
        // feature removed. One session and no shell instance: the strip holds the session's own tab
        // and the "+", and the stopped session offers its **restart** in the status cluster, so the
        // strip carries every control it can. A `Named` label, so the tab's text is a fixed string.
        CoveredState {
            name: "session-area-one-tab",
            build: || {
                StateUnderTest::new(with_active_session())
                    .panes(micold_core::pane_layout::PaneLayout::single())
            },
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                Anchor {
                    name: "session.strip",
                    path: SESSION_STRIP,
                },
                Anchor {
                    name: "session.strip.tabs",
                    path: SESSION_STRIP_TABS,
                },
                Anchor {
                    name: "session.strip.tabs.session",
                    path: &SESSION_STRIP_TAB_SESSION,
                },
                Anchor {
                    name: "session.strip.new_session",
                    path: SESSION_STRIP_NEW_SESSION,
                },
                Anchor {
                    name: "session.strip.split",
                    path: SESSION_STRIP_SPLIT,
                },
            ],
        },
        // --- Feature 1269: the Review tab in the strip (T038, U13, U16) ---------------------------
        //
        // `feat-short`'s main session (one shell) and a helper, with the location's Review tab:
        // shown, the header and the strip above the Changes view, the Review tab marked and the
        // split buttons disabled in place; hidden, the panes under the strip and the tab carrying a
        // badge of 3 drafts and the changed mark; and a sessionless worktree's review, whose strip
        // is its Review tab alone beside New session. They replace feature 1126's
        // `session-area-changes-open`, which drew the header alone over the view.
        CoveredState {
            name: "session-area-review-shown",
            build: review_shown,
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                Anchor {
                    name: "session.header.changes",
                    path: SESSION_HEADER_CHANGES,
                },
                Anchor {
                    name: "session.strip",
                    path: SESSION_STRIP,
                },
                Anchor {
                    name: "session.strip.tabs",
                    path: SESSION_STRIP_TABS,
                },
                Anchor {
                    name: "session.strip.tabs.review",
                    path: &SESSION_STRIP_TAB_REVIEW,
                },
                Anchor {
                    name: "session.strip.new_session",
                    path: SESSION_STRIP_NEW_SESSION,
                },
                Anchor {
                    name: "session.strip.split",
                    path: SESSION_STRIP_SPLIT,
                },
            ],
        },
        CoveredState {
            name: "session-area-review-hidden",
            build: review_hidden,
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                Anchor {
                    name: "session.header.changes",
                    path: SESSION_HEADER_CHANGES,
                },
                Anchor {
                    name: "session.strip",
                    path: SESSION_STRIP,
                },
                Anchor {
                    name: "session.strip.tabs",
                    path: SESSION_STRIP_TABS,
                },
                Anchor {
                    name: "session.strip.tabs.review",
                    path: &SESSION_STRIP_TAB_REVIEW,
                },
                Anchor {
                    name: "session.strip.new_session",
                    path: SESSION_STRIP_NEW_SESSION,
                },
                Anchor {
                    name: "session.strip.split",
                    path: SESSION_STRIP_SPLIT,
                },
            ],
        },
        // No status words (no session), so `session.strip` is not anchored: the fixed-controls
        // gate reads every anchored strip's children as pressable, and empty words are not.
        CoveredState {
            name: "session-area-review-sessionless",
            build: review_sessionless,
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                Anchor {
                    name: "session.header.changes",
                    path: SESSION_HEADER_CHANGES,
                },
                Anchor {
                    name: "session.strip.tabs",
                    path: SESSION_STRIP_TABS,
                },
                Anchor {
                    name: "session.strip.tabs.review",
                    path: &SESSION_STRIP_TAB_SESSION,
                },
                Anchor {
                    name: "session.strip.new_session",
                    path: SESSION_STRIP_NEW_SESSION,
                },
                Anchor {
                    name: "session.strip.split",
                    path: SESSION_STRIP_SPLIT,
                },
            ],
        },
        // --- Feature 939: a structured session's pane is its transcript (T038) -------------------
        //
        // One card open and detached from the end, so the state holds every part a transcript
        // pane lays out: entries, an unfolded card's body and the floating "Jump to latest".
        // `gates/transcript_jump_control_fits.rs` holds the control inside the viewport.
        CoveredState {
            name: "session-transcript-pane",
            build: || StateUnderTest::new(transcript_pane_detached()),
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                // Feature 1126: the header above the panes.
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                // The transcript's `Scrollable`, whose content `gates/containment.rs` lets overhang
                // its bottom edge (`TRANSCRIPT_VIEWPORT`).
                Anchor {
                    name: "transcript.viewport",
                    path: &[0, 0, 1, 1, 2, 0, 0, 1, 0],
                },
            ],
        },
        // --- Feature 940: the transcript pane drawn rich (T037) --------------------------------
        //
        // A formatted reply with its table and code blocks, and an Edit card open on its diff, in
        // a pane the widest sidebar narrows. `gates/transcript_rich_table_wraps.rs` holds a long
        // cell inside its column at a narrower width still.
        CoveredState {
            name: "session-transcript-rich",
            build: || StateUnderTest::new(transcript_pane_rich()),
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                // Feature 1126: the header above the panes.
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                // The transcript's `Scrollable`, whose content `gates/containment.rs` lets overhang
                // its bottom edge (`TRANSCRIPT_VIEWPORT`).
                Anchor {
                    name: "transcript.viewport",
                    path: &[0, 0, 1, 1, 2, 0, 0, 1, 0],
                },
            ],
        },
        // --- Feature 944: the session bar and an open subagent thread (T037) --------------------
        //
        // The meter in its warning, the totals, a running background task and the `Agent` call's
        // thread unfolded under its card. `gates/session_bar_fits.rs` holds the bar's figures
        // whole inside the pane.
        CoveredState {
            name: "session-transcript-insights",
            build: || StateUnderTest::new(transcript_pane_insights()),
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                // Feature 1126: the header above the panes.
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                Anchor {
                    name: "transcript.viewport",
                    path: &[0, 0, 1, 1, 2, 0, 0, 1, 0],
                },
            ],
        },
        // --- The strip with six and twelve tabs (feature 1126, S9; was BUG-002's tab strip) -------
        //
        // Six tabs: the main session with three shells (one running, the marked one, one exited) and
        // two helper sessions after the "+". Twelve: the main session with nine shells and the same
        // two helpers, which overflows the strip's region at the fixture's window, so the strip
        // scrolls and New session and the splits keep their place and size (FR-010). The marked
        // shell is neither the first nor the last tab, so "the indicator spans its own tab" and "it
        // spans everything up to here" read differently (feature 012 T057).
        //
        // Counts are tabs, session and shell together, as SC-004 counts them; the "+" is a member
        // too, so the keyed rows hold 7 and 13 children (`STRIP_MEMBERS`).
        CoveredState {
            name: "session-area-six-tabs",
            build: || strip_with(3, 2),
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                Anchor {
                    name: "session.strip",
                    path: SESSION_STRIP,
                },
                Anchor {
                    name: "session.strip.tabs",
                    path: SESSION_STRIP_TABS,
                },
                Anchor {
                    name: "session.strip.tabs.session",
                    path: &SESSION_STRIP_TAB_SESSION,
                },
                Anchor {
                    name: "session.strip.tabs.active",
                    path: &SESSION_STRIP_TAB_ACTIVE,
                },
                Anchor {
                    name: "session.strip.tabs.exited",
                    path: &SESSION_STRIP_TAB_EXITED,
                },
                Anchor {
                    name: "session.strip.new_session",
                    path: SESSION_STRIP_NEW_SESSION,
                },
                Anchor {
                    name: "session.strip.split",
                    path: SESSION_STRIP_SPLIT,
                },
            ],
        },
        CoveredState {
            name: "session-area-twelve-tabs",
            build: || strip_with(9, 2),
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "session.header",
                    path: SESSION_HEADER,
                },
                Anchor {
                    name: "session.strip",
                    path: SESSION_STRIP,
                },
                Anchor {
                    name: "session.strip.tabs",
                    path: SESSION_STRIP_TABS,
                },
                Anchor {
                    name: "session.strip.tabs.active",
                    path: &SESSION_STRIP_TAB_ACTIVE,
                },
                Anchor {
                    name: "session.strip.new_session",
                    path: SESSION_STRIP_NEW_SESSION,
                },
                Anchor {
                    name: "session.strip.split",
                    path: SESSION_STRIP_SPLIT,
                },
            ],
        },
        // --- BUG-005's nested rows (T116) -------------------------------------------------------
        //
        // Every state above this line holds **only depth-0 tree rows**. That is not a small gap: a
        // sidebar's whole reason to be a tree is that it nests, and the height of a nested row had
        // come to differ from the height of a top-level one — §7.2's floor rode on each row's
        // indent spacer, which is `Fixed(0)` wide at depth 0 and therefore void, so iced dropped it
        // and the floor applied to nested rows alone. The fixture could not see any of it. When the
        // floor was then deleted outright, taking 34% off every session row in the running
        // application, `layout_snapshot.txt` came out byte-identical — and the byte-identity was
        // reported as evidence that nothing had moved (T076).
        //
        // So this state exists to make a depth-1 row exist. 019's FR-008d is the requirement it
        // now answers: a screen's collapsed form is not a sample of its expanded one.
        CoveredState {
            name: "main-shell-worktree-expanded",
            build: || {
                let session = Session::restored(
                    SessionId::new(),
                    SessionLocation::Worktree("feat-short".to_string()),
                    SessionLabel::Named("feat/short".to_string()),
                    TerminalMode::Regular,
                    AiCli::ClaudeCode,
                );
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());

                let mut state = with_project();
                state.workspace = workspace;
                // The one line this state is about. Without it the session exists in the workspace
                // and the sidebar still draws a flat list of depth-0 rows.
                state.sidebar.expanded.insert("feat-short".to_string());
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
                // The nested row itself. Named rather than left as a bare path because a failure
                // here should say "the session row" — it is the node this whole state exists for,
                // and the one whose height silently differed from its parent's for two days.
                Anchor {
                    name: "sidebar.session_row",
                    path: SIDEBAR_SESSION_ROW,
                },
            ],
        },
        // --- BUG-003's two panels (T100) --------------------------------------------------------
        //
        // Neither had a covered state, which is why nothing named the panel that opens over the app
        // bar. The *geometry* was not absent — a closed `MenuOverlay` still yields a surface, so the
        // overflow panel is laid out at `1032, 52, 240 × 264` in nearly every state already, and the
        // fixture has recorded it there from the day it landed. What was missing is a state in which
        // either panel is **open** (an open surface carries a dismissal backdrop, which is a layer of
        // its own and shifts the panel's path), and any assertion at all about where a panel sits.
        // The second half is `gates/panel_placement.rs`; this is the first.
        CoveredState {
            name: "toolbar-overflow-menu-open",
            build: || {
                let mut state = with_project();
                state.help.help_menu_open = true;
                StateUnderTest::new(state)
            },
            anchors: &[
                // The three elements BUG-003 is *between*: the bar, the trigger the panel hangs
                // from, and the panel. A failure that named only a path would say nothing about
                // which of them moved.
                Anchor {
                    name: "app_bar",
                    path: APP_BAR,
                },
                Anchor {
                    name: "app_bar.overflow_trigger",
                    path: APP_BAR_OVERFLOW_TRIGGER,
                },
                Anchor {
                    name: "menu.panel",
                    path: &[2, 0],
                },
            ],
        },
        CoveredState {
            name: "project-switcher-open",
            build: || {
                let mut state = with_project();
                // A second, *inactive* project, so the panel holds a marked row and an unmarked one
                // and the fixture records where each label starts (FR-006a of feature 008). With one
                // project the list is all-marked, and a marker that shifted its neighbours sideways
                // would have nothing to shift.
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![]), (OTHER, vec![])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());
                state.workspace = workspace;
                state.project.switcher_open = true;
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "app_bar",
                    path: APP_BAR,
                },
                Anchor {
                    name: "app_bar.switcher_trigger",
                    path: APP_BAR_SWITCHER_TRIGGER,
                },
                // Layer 3, not 2: the closed overflow menu is still a surface (layer 1), and an open
                // surface's dismissal backdrop takes a layer of its own before the panel.
                Anchor {
                    name: "switcher.panel",
                    path: &[4, 0],
                },
            ],
        },
        // The Environment page (feature 034, U218): it gained the "Let AI sessions manage worktrees
        // and sessions" row, and no state here had laid that page out at all. The toggle is shown
        // off — the non-default value — so the fixture records the row as a user who turned it off
        // sees it, and every other value is invented and fixed like the states above.
        CoveredState {
            name: "settings-view-environment",
            build: || {
                let mut state = with_project();
                state.settings.settings_draft = Some(SettingsDraft {
                    jira: Default::default(),
                    required: Default::default(),
                    github: Default::default(),
                    section: SettingsSection::Environment,
                    appearance: AppearanceDraft {
                        theme: micold_core::theme::ThemePreference::Dark,
                        reduce_motion: false,
                    },
                    terminal: TerminalDraft {
                        scrollback_lines: "12000".to_string(),
                        save_terminal_history: true,
                    },
                    environment: EnvironmentDraft {
                        enabled: true,
                        script_path: "~/.config/micold/session-env.sh".to_string(),
                        timeout_secs: "5".to_string(),
                        default_ai_cli: micold_core::session::AiCli::ClaudeCode,
                        default_prompt_template: String::new(),
                        default_prompt_editor: Default::default(),
                        pi_activity_component: true,
                        tool_server_enabled: false,
                        worktree_guard_enabled: true,
                        desktop_notifications: true,
                        notification_kinds: Default::default(),
                        long_task_threshold_secs: "60".to_string(),
                        plan_usage_enabled: true,
                        plan_usage_warn_percent: "80".to_string(),
                        cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                    },
                    daemon: DaemonDraft::default(),
                    error: None,
                    baseline: None,
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        // Settings → GitHub issues (feature 034, T060; contract §4a): three entries, the middle one
        // a duplicate of the first ignoring case, so the fixture records a row carrying the refused
        // save's message and the move buttons disabled at both ends.
        CoveredState {
            name: "settings-view-github-issues",
            build: || {
                use micold_core::issue_types::LabelTypeEntry;
                use micold_core::naming::ConventionalType;
                let entry = |label: &str, type_| LabelTypeEntry {
                    label: label.to_string(),
                    type_,
                };
                let mut state = with_project();
                state.settings.settings_draft = Some(SettingsDraft {
                    jira: Default::default(),
                    required: Default::default(),
                    github: micold_client::features::settings::GithubDraft {
                        entries: vec![
                            entry("bug", ConventionalType::Fix),
                            entry("Bug", ConventionalType::Chore),
                            entry("documentation", ConventionalType::Docs),
                        ],
                        // On, so the page records the checked switch with its note (feature 040,
                        // UI §5/§6).
                        pr_status_enabled: true,
                        issues_enabled: true,
                    },
                    section: SettingsSection::GithubIssues,
                    appearance: AppearanceDraft::default(),
                    terminal: TerminalDraft {
                        scrollback_lines: "12000".to_string(),
                        save_terminal_history: true,
                    },
                    environment: EnvironmentDraft::default(),
                    daemon: DaemonDraft::default(),
                    error: Some(FieldError {
                        field: FieldId::IssueMappingLabel(1),
                        section: SettingsSection::GithubIssues,
                        message: "“bug” is already mapped above.".to_string(),
                    }),
                    baseline: None,
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        // Settings -> Jira (feature 533, FR-001, FR-004): the switch off, which is the whole page
        // (no credential field, nothing but the switch and its description), and the switch on
        // with an open project that has no connection yet.
        CoveredState {
            name: "settings-view-jira-off",
            build: || {
                let mut state = with_project();
                state.settings.settings_draft = Some(SettingsDraft {
                    jira: Default::default(),
                    required: Default::default(),
                    section: SettingsSection::Jira,
                    ..SettingsDraft::default()
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        CoveredState {
            name: "settings-view-jira-on-no-connection",
            build: || {
                let mut state = with_project();
                let project = state
                    .workspace
                    .active_project()
                    .map(|project| project.path.clone());
                let mut draft = SettingsDraft {
                    jira: Default::default(),
                    required: Default::default(),
                    section: SettingsSection::Jira,
                    ..SettingsDraft::default()
                };
                draft.jira.enabled = true;
                draft.jira.project = project;
                state.settings.settings_draft = Some(draft);
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        // Settings -> Jira connection form (feature 533, US3): connected, a field refused, and the
        // Keep-or-Remove question.
        CoveredState {
            name: "settings-view-jira-on-with-connection",
            build: || {
                StateUnderTest::new(jira_form_state(|draft| {
                    draft.jira.site = "https://acme.atlassian.net".to_owned();
                    draft.jira.project_key = "ABC".to_owned();
                    draft.jira.email = "me@acme.com".to_owned();
                    draft.jira.token_stored = true;
                    draft.jira.connection = micold_core::jira::JiraConnection::parse(
                        "https://acme.atlassian.net",
                        "ABC",
                        "me@acme.com",
                    )
                    .ok();
                }))
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        CoveredState {
            name: "settings-view-jira-on-field-error",
            build: || {
                StateUnderTest::new(jira_form_state(|draft| {
                    draft.jira.site = "acme".to_owned();
                    draft.error = Some(micold_client::features::settings::FieldError {
                        field: micold_client::features::window::FieldId::SettingsJiraSite,
                        section: SettingsSection::Jira,
                        message: micold_core::jira::SITE_REFUSED.to_owned(),
                    });
                }))
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        CoveredState {
            name: "settings-view-jira-keep-or-remove-prompt",
            build: || {
                StateUnderTest::new(jira_form_state(|draft| {
                    draft.jira.enabled = false;
                    draft.jira.connection = micold_core::jira::JiraConnection::parse(
                        "https://acme.atlassian.net",
                        "ABC",
                        "me@acme.com",
                    )
                    .ok();
                    draft.jira.prompt = draft.jira.project.clone().map(|project| {
                        micold_client::features::settings::KeepOrRemovePrompt { project }
                    });
                    draft.jira.enabled = true;
                }))
            },
            anchors: &[Anchor {
                name: "settings.rail",
                path: &[0, 0, 1, 0, 0, 0],
            }],
        },
        // Feature 040 (UI §2, §6): the pull request indicator on a row, with the switch on. The
        // first row has an open pull request with failing checks (the widest form), the third a
        // merged one (the narrowest), and the second none — a row without one beside rows with, so
        // the fixture records that it keeps today's geometry (FR-001, FR-011).
        CoveredState {
            name: "main-shell-sidebar-pull-request-indicator",
            build: || StateUnderTest::new(with_pull_requests(260)),
            anchors: &[],
        },
        // The same at the narrowest sidebar the application allows: the indicator keeps its size at
        // every width (FR-009).
        CoveredState {
            name: "main-shell-sidebar-pull-request-indicator-narrowest",
            build: || StateUnderTest::new(with_pull_requests(SIDEBAR_MIN_WIDTH)),
            anchors: &[],
        },
        // Feature 040 US3 (UI §6, U128): the same, with the third row's merged pull request
        // removable, so its row carries the indicator and the **can be removed** chip together.
        CoveredState {
            name: "main-shell-sidebar-pull-request-removable",
            build: || StateUnderTest::new(with_removable_pull_request(260)),
            anchors: &[],
        },
        // The same at the narrowest sidebar: both stay inside the row and clear of the name.
        CoveredState {
            name: "main-shell-sidebar-pull-request-removable-narrowest",
            build: || StateUnderTest::new(with_removable_pull_request(SIDEBAR_MIN_WIDTH)),
            anchors: &[],
        },
        // Feature 040 US4 (UI §6, U148): the same rows read long ago. The stale form changes the
        // indicator's colour role only, so the geometry is the current form's.
        CoveredState {
            name: "main-shell-sidebar-pull-request-stale",
            build: || StateUnderTest::new(with_stale_pull_requests(260)),
            anchors: &[],
        },
        // An agent's destructive request (feature 034, FR-014, U213): the confirmation every
        // window shows, over the main shell. The worktree delete with its branch and its sessions
        // is the longest headline the dialog can carry, so it is the one recorded.
        CoveredState {
            name: "agent-confirm-dialog",
            build: || {
                let mut state = with_project();
                state.agent_confirm.pending =
                    vec![micold_client::features::agent_confirm::Prompt {
                        session: micold_core::session::SessionId(Default::default()),
                        id: 1,
                        project: PathBuf::from(PROJECT),
                        caller_label: "planner".to_string(),
                        operation:
                            micold_core::protocol::messages::ConfirmOperation::DeleteWorktree {
                                stop_sessions: true,
                                delete_branch: true,
                            },
                        target_label: "feat-login-page".to_string(),
                    }];
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                Anchor {
                    name: "dialog.actions",
                    path: &[8, 0, 0, 1],
                },
            ],
        },
        // Feature 1120 (FR-015, FR-019, research R14): the Overview open over the fixture project,
        // one card per worktree and the Default card, in the card's header-row layout. The
        // text-overflow and containment gates run on the real cards here, which no other state
        // reaches.
        CoveredState {
            name: "overview-open",
            build: || {
                let mut state = with_project();
                state.overview.open = true;
                StateUnderTest::new(state)
            },
            // The first card's header row and its title line, taken from the resolved geometry:
            // the card is the grid's first cell, its column holds the header then the body.
            //
            // Both chains share the prefix `[0, 0, 1, 1, 0, 1, 0, 0, 0, 0]`, the path from the
            // window root down to the first card's column. From that column, `0` is the header row
            // and `1` is the body, whose `0, 0` is the title line. A wrapper added anywhere above
            // shifts the prefix; `the_overview_card_anchors_are_the_header_strip_and_the_title` in
            // `layout_snapshot.rs` then fails, because an anchor no longer lands on a header strip
            // or a title line.
            anchors: &[
                Anchor {
                    name: "overview.card.header",
                    path: &[0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 0],
                },
                Anchor {
                    name: "overview.card.title",
                    path: &[0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0],
                },
            ],
        },
        // Feature 1121 (R10, FR-017, SC-002): the Overview with its first card expanded over the
        // cards below it, so the expanded card's full texts pass the text-overflow and containment
        // gates in both schemes.
        CoveredState {
            name: "overview-expanded",
            build: || {
                let mut state = with_project();
                state.overview.open = true;
                let first = micold_client::ui::overview::cards(&state)
                    .first()
                    .map(|card| card.location.clone())
                    .expect("the project lists a card");
                state
                    .overview
                    .expansion
                    .apply(micold_core::overview_expansion::Event::FocusGained(first));
                StateUnderTest::new(state)
            },
            anchors: &[],
        },
        // Feature 1122 (FR-017, SC-004, G6): cards with an issue link, so the link passes the
        // text-overflow and containment gates in both schemes: alone in the branch line's place
        // (the title is the branch), after a 120-character branch that gives way to it (the title
        // is the issue's), and a card with no ticket beside them.
        CoveredState {
            name: "overview-issue-link",
            build: || {
                use micold_client::features::overview::{RepoLookup, TitleLookup};
                let mut state = with_project();
                let long_branch = format!("release-train-{}", "x".repeat(106));
                state.worktree.worktrees = vec![
                    worktree(
                        "feat-1122_overview-issue-link",
                        "feat/1122_overview-issue-link",
                    ),
                    worktree("feat-1124_long-branch", &long_branch),
                    worktree("feat_no-ticket", "feat/no-ticket"),
                    // Feature 1122 M5 (T048): a Jira key, linked on the open project's base URL.
                    worktree("feat-mic-87_add-login", "feat/mic-87_add-login"),
                ];
                state.overview.titles.insert(
                    1124,
                    TitleLookup::Known("Cards keep their issue link whole".into()),
                );
                state.overview.repo =
                    RepoLookup::Known(GithubRepo::from_remote_url("git@github.com:o/r.git"));
                state.settings.github_issues_enabled = true;
                state.settings.jira_enabled = true;
                let project = state.workspace.active.clone().expect("a project is open");
                state.settings.jira_base_urls.insert(
                    micold_core::jira::connection_key(&project),
                    micold_core::jira::JiraBaseUrl::parse("https://example.atlassian.net")
                        .expect("a valid base URL"),
                );
                state.overview.open = true;
                StateUnderTest::new(state)
            },
            anchors: &[],
        },
        // Feature 1123 (FR-017, SC-007, R10, G6): two cards with three pull requests each (open
        // with *checks failing*, draft with *checks pending*, merged), the first collapsed with
        // "+1 more" and the second expanded over the row below, so the rows pass the text-overflow and
        // containment gates in both schemes.
        CoveredState {
            name: "overview-pr-rows",
            build: || {
                use micold_core::pr_list::{BranchPrList, ListedPr};
                use micold_core::pull_request::{BranchPrState, CheckStatus, ReviewState};
                let mut state = with_project();
                // First in the list, so their cards share the first row and the expanded one lies
                // over the cards of the row below, as the Overview draws it.
                for (at, (dir, branch)) in [
                    ("feat-1123_x", "feat/1123_x"),
                    ("feat-1123_y", "feat/1123_y"),
                ]
                .into_iter()
                .enumerate()
                {
                    state.worktree.worktrees.insert(at, worktree(dir, branch));
                }
                let listed = |number: u64, state: BranchPrState, checks: CheckStatus| ListedPr {
                    number,
                    url: format!("https://github.com/o/r/pull/{number}"),
                    state,
                    checks,
                    review: ReviewState::None,
                    conflict: false,
                    created_at: format!("2026-10-0{}T00:00:00Z", number % 9 + 1),
                };
                let three = || {
                    BranchPrList::new(
                        vec![
                            listed(18, BranchPrState::Open, CheckStatus::Failing),
                            listed(17, BranchPrState::Draft, CheckStatus::Pending),
                            listed(16, BranchPrState::Merged, CheckStatus::None),
                        ],
                        false,
                        false,
                    )
                };
                state.pr_status.enabled = true;
                state.pr_status.read_at = Some(100);
                for branch in ["feat/1123_x", "feat/1123_y"] {
                    state.pr_status.lists.insert(branch.to_string(), three());
                }
                state.overview.open = true;
                let second = SessionLocation::Worktree("feat-1123_y".to_string());
                state
                    .overview
                    .expansion
                    .apply(micold_core::overview_expansion::Event::FocusGained(second));
                StateUnderTest::new(state)
            },
            anchors: &[],
        },
        // --- Feature 1124: the bottom status bar (T008) -----------------------------------------
        // The bar is the window column's last child, the body the one before it. With no banner
        // drawn (connected, sandbox not failed) the column holds the app bar, the body and the bar.
        CoveredState {
            name: "status-bar-steady",
            build: || StateUnderTest::new(with_active_session()).sandbox(running_sandbox()),
            anchors: STATUS_BAR_ANCHORS,
        },
        CoveredState {
            name: "status-bar-overview-full-screen",
            build: || {
                let mut state = with_project();
                state.overview.open = true;
                state.overview.full_screen = true;
                StateUnderTest::new(state).sandbox(running_sandbox())
            },
            anchors: STATUS_BAR_ANCHORS,
        },
        CoveredState {
            name: "status-bar-settings",
            build: || {
                let mut state = with_project();
                state.settings.settings_draft = Some(SettingsDraft::default());
                StateUnderTest::new(state).sandbox(running_sandbox())
            },
            anchors: STATUS_BAR_ANCHORS,
        },
        CoveredState {
            name: "status-bar-changes",
            build: || {
                let mut state = with_active_session();
                micold_client::features::changes::update(
                    &mut state.changes,
                    micold_client::features::changes::Msg::Opened {
                        project: PathBuf::from(PROJECT),
                        entry: SessionLocation::Worktree("feat-short".to_string()),
                    },
                );
                StateUnderTest::new(state).sandbox(running_sandbox())
            },
            anchors: STATUS_BAR_ANCHORS,
        },
        CoveredState {
            name: "status-bar-compare",
            build: || {
                let mut state = with_active_session();
                let group = compare_group();
                let id = group.id;
                micold_client::features::runs::update(
                    &mut state.runs,
                    micold_client::features::runs::Msg::GroupsChanged(vec![group]),
                );
                micold_client::features::runs::update(
                    &mut state.runs,
                    micold_client::features::runs::Msg::CompareOpened {
                        group: id,
                        project: PathBuf::from(PROJECT),
                    },
                );
                StateUnderTest::new(state).sandbox(running_sandbox())
            },
            anchors: STATUS_BAR_ANCHORS,
        },
        CoveredState {
            name: "status-bar-split-panes",
            build: || {
                use micold_core::pane_layout::{Axis, PaneLayout};
                let mut layout = PaneLayout::single();
                let first = layout.focused();
                layout
                    .split(first, Axis::Vertical, (1000.0, 600.0), (80.0, 160.0), None)
                    .expect("a 1000 px pane splits in two");
                StateUnderTest::new(with_active_session())
                    .sandbox(running_sandbox())
                    .panes(layout)
            },
            anchors: STATUS_BAR_ANCHORS,
        },
        // A menu open over the bar. It stands in for every floating surface: all are pushed after
        // `base`, through the same overlay stack, so all draw above the bar (FR-013).
        CoveredState {
            name: "status-bar-under-menu",
            build: || {
                let mut state = with_active_session();
                state.help.help_menu_open = true;
                StateUnderTest::new(state).sandbox(running_sandbox())
            },
            anchors: STATUS_BAR_ANCHORS,
        },
        // The image download in flight: the bar's sandbox item shows the stage and percent, and the
        // column still holds only the app bar, the body and the bar (US2, SC-003).
        CoveredState {
            name: "status-bar-bring-up",
            build: || {
                with_sandbox_state(micold_core::sandbox::lifecycle::SandboxState::Acquiring(
                    micold_core::sandbox::runtime::Progress {
                        stage: "Downloading".into(),
                        detail: Some("sha256:3f4a1b2c layer of the image".into()),
                        percent: Some(42),
                    },
                ))
            },
            anchors: STATUS_BAR_ANCHORS,
        },
        // The plan usage after the CLI item (US5): a reading with no reset time, so it is current
        // at any clock and the label is the same text every run.
        CoveredState {
            name: "status-bar-usage",
            build: status_bar_usage,
            anchors: STATUS_BAR_ANCHORS,
        },
        // --- Feature 1124, US3: banners stay for conditions with an action ----------------------
        // Another window holds the project and the sandbox did not start: both banners are drawn
        // under the app bar with their buttons, and both bar items show a warning (FR-009).
        CoveredState {
            name: "status-bar-warnings",
            build: || warnings_state(with_active_session()),
            anchors: STATUS_BAR_ANCHORS_WITH_BANNERS,
        },
        // The same over the Overview: the banners sit in the same slot under the toolbar (FR-010).
        CoveredState {
            name: "status-bar-warnings-overview",
            build: || {
                let mut state = with_project();
                state.overview.open = true;
                state.overview.full_screen = true;
                warnings_state(state)
            },
            anchors: STATUS_BAR_ANCHORS_WITH_BANNERS,
        },
        // A lost connection draws no banner row: the bar's connection item carries the error dot.
        CoveredState {
            name: "status-bar-disconnected",
            build: || {
                StateUnderTest::new(with_active_session())
                    .sandbox(running_sandbox())
                    .connection(ConnectionStatus::Disconnected)
            },
            anchors: STATUS_BAR_ANCHORS,
        },
    ]
}

/// Both banners and both warning items: the project is held by another window and the sandbox
/// failed to start (feature 1124, US3).
fn warnings_state(state: State) -> StateUnderTest {
    use micold_core::sandbox::lifecycle::{Failure, Stage};
    use micold_core::sandbox::placement::PlacementKind;
    use micold_core::sandbox::runtime::{RuntimeError, RuntimeKind};
    let mut sandbox =
        micold_client::features::sandbox::Sandbox::for_placement(PlacementKind::LocalSandbox);
    sandbox.failed(Failure {
        stage: Stage::Probing,
        error: RuntimeError::NotInstalled {
            kind: RuntimeKind::Docker,
        },
    });
    StateUnderTest::new(state)
        .sandbox(sandbox)
        .connection(ConnectionStatus::ProjectBusy {
            holder: "window 2".into(),
        })
}

/// The status bar and the body's top edge, in a window column holding the app bar, the body and
/// the bar (feature 1124).
const STATUS_BAR_ANCHORS: &[Anchor] = &[
    Anchor {
        name: "shell.root",
        path: &[],
    },
    Anchor {
        name: "shell.body",
        path: &[0, 0, 1],
    },
    Anchor {
        name: "status_bar",
        path: &[0, 0, 2],
    },
];

/// A project with one session at worktree `feat-short`, current.
fn with_active_session() -> State {
    let session = Session::restored(
        SessionId::new(),
        SessionLocation::Worktree("feat-short".to_string()),
        SessionLabel::Named("feat/short".to_string()),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    let active = session.id;
    let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());

    let mut state = with_project();
    state.workspace = workspace;
    state.session.active = Some(active);
    state
}

/// `state` with `entry`'s Review tab made and shown, the header following it with its counts read.
fn with_review_of(mut state: State, entry: SessionLocation) -> State {
    use micold_client::features::changes::{self, Load, Msg};
    changes::update(
        &mut state.changes,
        Msg::Opened {
            project: PathBuf::from(PROJECT),
            entry: entry.clone(),
        },
    );
    state.changes.header.entry = Some((PathBuf::from(PROJECT), entry));
    state.changes.header.load = Load::Ready(micold_core::context_header::ChangeSummary {
        files: 3,
        added: 42,
        removed: 7,
    });
    state
}

/// The worktree `feat-short` with its main session (one shell) and a helper, its Review tab shown
/// (feature 1269, `session-area-review-shown`).
pub fn review_shown() -> StateUnderTest {
    let mut under = strip_with(1, 1);
    under.state = with_review_of(
        under.state,
        SessionLocation::Worktree("feat-short".to_string()),
    );
    under
}

/// `drafts` draft comments of `feat-short`, as the service pushes them.
pub fn feat_short_drafts(state: &mut State, drafts: usize) {
    use micold_core::review::comment::{CommentId, CommentState, ReviewComment};
    use micold_core::review::{LineRange, RelPath, Side};
    let comment = || ReviewComment {
        id: CommentId::new(),
        path: RelPath::from_native("src/lib.rs").expect("a relative path"),
        side: Side::New,
        range: LineRange::new(1, 1).expect("a range"),
        quote: vec!["line 1".into()],
        text: "about it".into(),
        state: CommentState::Draft,
        created: 1,
        pr: None,
    };
    state.changes.reviews.insert(
        (PathBuf::from(PROJECT), "feat-short".to_string()),
        micold_client::features::changes::ReviewView {
            comments: (0..drafts).map(|_| comment()).collect(),
            sending: false,
            marks: Default::default(),
        },
    );
}

/// [`review_shown`] with the main session shown again, so the Review tab is hidden, and since then
/// the worktree's files changed and three drafts wait (feature 1269, `session-area-review-hidden`).
pub fn review_hidden() -> StateUnderTest {
    use micold_client::features::changes::{self, Msg};
    let mut under = review_shown();
    changes::update(&mut under.state.changes, Msg::SessionSelected);
    let _ = under.state.changes.tabs.files_changed(&(
        PathBuf::from(PROJECT),
        SessionLocation::Worktree("feat-short".to_string()),
    ));
    feat_short_drafts(&mut under.state, 3);
    under
}

/// The sessionless worktree `fix-a-bug`'s review shown, under its own header (feature 1269,
/// `session-area-review-sessionless`).
pub fn review_sessionless() -> StateUnderTest {
    let mut under = strip_with(1, 1);
    under.state.session.active = None;
    under.state = with_review_of(
        under.state,
        SessionLocation::Worktree("fix-a-bug".to_string()),
    );
    under
}

/// How many strip members (tabs and the "+") each strip state's keyed row holds (feature 1126,
/// S9): read by `gates/containment.rs` and `gates/strip_controls_hold_their_size.rs`.
pub const STRIP_MEMBERS: &[(&str, usize)] = &[
    ("session-area-one-tab", 2),
    ("session-area-six-tabs", 7),
    ("session-area-twelve-tabs", 13),
    ("session-area-review-shown", 5),
    ("session-area-review-hidden", 5),
];

/// The main session of `feat-short` with `shells` shell instances (the second marked, the third
/// exited when there are three or more) and `helpers` helper sessions after it (feature 1126 S9).
pub fn strip_with(shells: usize, helpers: usize) -> StateUnderTest {
    let mut session = Session::restored(
        SessionId::new(),
        SessionLocation::Worktree("feat-short".to_string()),
        SessionLabel::Named("feat/short".to_string()),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    session.main = true;
    let opened: Vec<_> = (0..shells).map(|_| session.open_shell_instance()).collect();
    for (i, id) in opened.iter().enumerate() {
        if i == 2 {
            session.mark_shell_exited(*id);
        } else {
            session.mark_shell_running(*id);
        }
    }
    if let Some(marked) = opened.get(1) {
        session.select_shell(*marked);
    }
    let active = session.id;
    let mut sessions = vec![session];
    for n in 0..helpers {
        sessions.push(Session::restored(
            SessionId::new(),
            SessionLocation::Worktree("feat-short".to_string()),
            SessionLabel::Named(format!("helper-{}", n + 1)),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        ));
    }
    let mut workspace = super::workspace_with(vec![(PROJECT, sessions)]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = with_project();
    state.workspace = workspace;
    state.session.active = Some(active);
    state.session.tab_strip_viewport_width = TAB_STRIP_VIEWPORT;
    // A pane area, so the strip draws the focused pane's split buttons as the application does.
    StateUnderTest::new(state).panes(micold_core::pane_layout::PaneLayout::single())
}

/// The status bar's longest texts (feature 1124, T009): the session's worktree on a container
/// daemon with a long name, and an image download with a long runtime line.
pub fn status_bar_long_texts() -> StateUnderTest {
    use micold_core::daemons::{ContainerSettings, DaemonRegistry, DaemonRuntime};
    let mut state = with_active_session();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let remote = registry
        .add(
            "build-farm-eu-west-container-with-a-very-long-name",
            DaemonRuntime::Container(ContainerSettings {
                profile: micold_core::sandbox::SandboxProfile::default(),
                container_name: "farm".into(),
                port: 7001,
                migration: None,
            }),
            true,
        )
        .expect("the daemon is added");
    state.settings.daemons = registry;
    state.workspace.bindings.insert(
        PathBuf::from(PROJECT),
        [("feat-short".to_string(), remote)].into_iter().collect(),
    );
    let sandbox = micold_client::features::sandbox::Sandbox {
        state: micold_core::sandbox::lifecycle::SandboxState::Acquiring(
            micold_core::sandbox::runtime::Progress {
                stage: "Downloading".into(),
                detail: Some("sha256:0123456789abcdef0123456789abcdef layer of the image".into()),
                percent: Some(42),
            },
        ),
        ..Default::default()
    };
    StateUnderTest::new(state).sandbox(sandbox)
}

/// The status bar with a plan-usage reading beside the CLI item (feature 1124, US5): the window
/// has a session, the sandbox is up, and the only usage window never resets.
pub fn status_bar_usage() -> StateUnderTest {
    with_usage(StateUnderTest::new(with_active_session()).sandbox(running_sandbox()))
}

/// The status bar's longest texts with a plan-usage reading too (feature 1124, US5).
pub fn status_bar_long_texts_with_usage() -> StateUnderTest {
    with_usage(status_bar_long_texts())
}

fn with_usage(mut under: StateUnderTest) -> StateUnderTest {
    let state = &mut under.state;
    state
        .plan_usage
        .received(Some(micold_core::plan_usage::UsageReading {
            windows: vec![micold_core::plan_usage::LimitWindow {
                key: "five_hour".into(),
                used_tenths: 370,
                resets_at: None,
            }],
            obtained_at: 0,
        }));
    under
}

/// A current session with the sandbox in `state` (feature 1124, US2).
pub fn with_sandbox_state(state: micold_core::sandbox::lifecycle::SandboxState) -> StateUnderTest {
    StateUnderTest::new(with_active_session()).sandbox(micold_client::features::sandbox::Sandbox {
        state,
        ..Default::default()
    })
}

/// A sandbox that is up: the bar reads "Sandbox ready".
fn running_sandbox() -> micold_client::features::sandbox::Sandbox {
    micold_client::features::sandbox::Sandbox {
        state: micold_core::sandbox::lifecycle::SandboxState::Running(
            micold_core::sandbox::runtime::ContainerId("fixture".to_string()),
        ),
        ..Default::default()
    }
}

/// A parallel-run group of two prompted runs, for Compare.
fn compare_group() -> micold_core::runs::RunGroup {
    use micold_core::runs::{GroupId, NewGroup, Run, RunGroup, RunStatus};
    RunGroup::new(NewGroup {
        id: GroupId(uuid::Uuid::from_u128(1124)),
        name: "bar".into(),
        naming: micold_core::naming::WorktreeNaming {
            type_: Some(micold_core::naming::ConventionalType::Feat),
            ticket: None,
            name: "bar".into(),
        },
        prompt: "Add a status bar".into(),
        base_branch: "main".into(),
        base_commit: "abc".into(),
        created: std::time::SystemTime::UNIX_EPOCH,
        runs: (1..=2u8)
            .map(|number| Run {
                number,
                provider: AiCli::ClaudeCode,
                names: micold_core::naming::DerivedNames {
                    dir_name: format!("feat-bar-{number}"),
                    branch: format!("feat/bar-{number}"),
                },
                session: None,
                status: RunStatus::Prompted,
            })
            .collect(),
    })
    .expect("a valid group")
}

/// States pinned partway through a reveal. **A second list in this file, not a second site.**
///
/// These are not covered states and must not become them: the geometry fixture excludes
/// mid-animation geometry (T030), and recording a frame partway through a reveal would churn the
/// fixture on any change to a duration or an easing curve — motion's business, not layout's. They
/// exist to be asserted *about*, by the containment invariant, and are recorded nowhere.
///
/// FR-016's "one place" is about where a *covered state* is declared, and this file is still that
/// place. A registry scan (T029) should expect both lists here and neither elsewhere.
pub fn revealing_states() -> &'static [RevealingState] {
    &[
        // Empty since feature 1125 removed the sidebar's filter accordion, the one reveal this
        // list pinned (`sidebar-filter-panel-mid-reveal`). The list stays for the next one.
    ]
}

/// A state with Settings open on the Jira page, the switch on and the open project set; `shape`
/// edits the draft (feature 533, US3).
fn jira_form_state(shape: impl FnOnce(&mut SettingsDraft)) -> State {
    let mut state = with_project();
    let project = state
        .workspace
        .active_project()
        .map(|project| project.path.clone());
    let mut draft = SettingsDraft {
        jira: Default::default(),
        required: Default::default(),
        section: SettingsSection::Jira,
        ..SettingsDraft::default()
    };
    draft.jira.enabled = true;
    draft.jira.project = project;
    shape(&mut draft);
    state.settings.settings_draft = Some(draft);
    state
}

/// [`STATUS_BAR_ANCHORS`] with the connection and sandbox banners drawn: the column then holds the
/// app bar, the two banners, the body and the bar.
const STATUS_BAR_ANCHORS_WITH_BANNERS: &[Anchor] = &[
    Anchor {
        name: "shell.root",
        path: &[],
    },
    Anchor {
        name: "shell.body",
        path: &[0, 0, 3],
    },
    Anchor {
        name: "status_bar",
        path: &[0, 0, 4],
    },
];
