//! T016 — worktree discovery: porcelain parse + classify + reconcile (FR-018/018a).

use micold_core::worktree::{classify, parse_worktrees, reconcile, WorktreeRecord, WorktreeStatus};
use std::path::{Path, PathBuf};

const PORCELAIN: &str = "\
worktree /repo
HEAD abc123
branch refs/heads/main

worktree /repo/.claude/worktrees/feat-login
HEAD def456
branch refs/heads/feat/login

worktree /repo/.claude/worktrees/fix-gone
HEAD 000000
branch refs/heads/fix/gone
prunable gitdir file points to non-existent location
";

#[test]
fn parses_all_records_with_branch_shortnames() {
    let records = parse_worktrees(PORCELAIN);
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].path, PathBuf::from("/repo"));
    assert_eq!(records[0].branch.as_deref(), Some("main"));
    assert_eq!(records[1].branch.as_deref(), Some("feat/login"));
    assert!(!records[1].prunable);
    assert!(records[2].prunable);
}

#[test]
fn classify_valid_missing() {
    let present = WorktreeRecord {
        path: PathBuf::from("/x"),
        branch: Some("feat/x".into()),
        prunable: false,
    };
    assert_eq!(classify(&present, true), WorktreeStatus::Valid);
    assert_eq!(classify(&present, false), WorktreeStatus::Missing);

    let prunable = WorktreeRecord {
        prunable: true,
        ..present
    };
    assert_eq!(classify(&prunable, true), WorktreeStatus::Missing);
}

// Feature 014 (T012, US2 acceptance #4): the location half of FR-005 is enforced here, not in
// the classifier. A worktree outside the project's `.claude/worktrees/` root is excluded by
// `reconcile()` on its path alone, so its name never reaches `Worktree::owner()` — the hiding
// rule introduces no new behavior for out-of-scope worktrees.
#[test]
fn reconcile_still_excludes_out_of_root_worktrees_whatever_their_name() {
    const OUT_OF_ROOT: &str = "\
worktree /elsewhere/agent-a885b42dc521fbda1
HEAD abc123
branch refs/heads/worktree-agent-a885b42dc521fbda1

worktree /repo/.claude/worktrees/feat-login
HEAD def456
branch refs/heads/feat/login
";
    let records = parse_worktrees(OUT_OF_ROOT);
    assert_eq!(records.len(), 2, "both records parse");

    let root = Path::new("/repo/.claude/worktrees");
    let on_disk = vec!["feat-login".to_string()];
    let worktrees = reconcile(&records, root, &[], &on_disk, &|_| true);

    assert_eq!(
        worktrees.len(),
        1,
        "only the worktree under the project's root is ours"
    );
    assert_eq!(worktrees[0].dir_name, "feat-login");
}

#[test]
fn reconcile_classifies_registered_and_orphan_dirs() {
    let records = parse_worktrees(PORCELAIN);
    let root = Path::new("/repo/.claude/worktrees");
    // feat-login exists; fix-gone is prunable; feat-orphan is on disk but unregistered.
    let on_disk = vec!["feat-login".to_string(), "feat-orphan".to_string()];
    let exists = |p: &Path| p == Path::new("/repo/.claude/worktrees/feat-login");

    let worktrees = reconcile(&records, root, &[], &on_disk, &exists);

    // The top-level /repo worktree is filtered out (not under .claude/worktrees).
    let by_name = |name: &str| {
        worktrees
            .iter()
            .find(|w| w.dir_name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .status
    };
    assert_eq!(worktrees.len(), 3);
    assert_eq!(by_name("feat-login"), WorktreeStatus::Valid);
    assert_eq!(by_name("fix-gone"), WorktreeStatus::Missing);
    assert_eq!(by_name("feat-orphan"), WorktreeStatus::Invalid);
}

// ---------------------------------------------------------------------------------------------
// 016 BUG-002 (T075): a worktree the app does not manage, shown because the user asked for it.
//
// `reconcile` decides what the list holds, and until now it decided it entirely from where a
// worktree lives. Inclusion adds the one thing that cannot be derived — the user's wish — and
// nothing else: what the entry then says about itself is still read from git and the filesystem
// (FR-028/FR-029/FR-031, research R13).
// ---------------------------------------------------------------------------------------------

const WITH_OUTSIDER: &str = "\
worktree /repo
HEAD abc123
branch refs/heads/main

worktree /repo/.claude/worktrees/feat-login
HEAD def456
branch refs/heads/feat/login

worktree /elsewhere/worktrees/fix-olx
HEAD 111111
branch refs/heads/fix/olx
";

#[test]
fn an_included_worktree_is_listed_where_it_is() {
    let records = parse_worktrees(WITH_OUTSIDER);
    let root = Path::new("/repo/.claude/worktrees");
    let outsider = PathBuf::from("/elsewhere/worktrees/fix-olx");

    let listed = reconcile(
        &records,
        root,
        std::slice::from_ref(&outsider),
        &[],
        &|_| true,
    );

    let found = listed
        .iter()
        .find(|w| w.path == outsider)
        .expect("an included worktree must appear in the list — that is the whole of FR-027");
    assert_eq!(found.dir_name, "fix-olx", "named by its own folder");
    assert_eq!(
        found.branch.as_deref(),
        Some("fix/olx"),
        "its branch is read from git like any other worktree's, never recorded (FR-028)"
    );
    assert!(
        found.included,
        "the entry must say it is included, because the list shows these by location too (FR-029)"
    );
    assert!(
        listed
            .iter()
            .any(|w| w.dir_name == "feat-login" && !w.included),
        "the app's own worktrees are unaffected and are not marked included"
    );
}

#[test]
fn a_worktree_nobody_included_is_still_dropped() {
    let records = parse_worktrees(WITH_OUTSIDER);
    let root = Path::new("/repo/.claude/worktrees");

    let listed = reconcile(&records, root, &[], &[], &|_| true);

    assert!(
        !listed
            .iter()
            .any(|w| w.path == Path::new("/elsewhere/worktrees/fix-olx")),
        "inclusion is per worktree and per project. A worktree outside the app's own directory is \
         listed because someone asked for it, never because it exists"
    );
    assert!(
        !listed.iter().any(|w| w.path == Path::new("/repo")),
        "and the project's own checkout is still not one of its worktrees"
    );
}

#[test]
fn an_included_path_git_no_longer_registers_is_not_invented() {
    let records = parse_worktrees(WITH_OUTSIDER);
    let root = Path::new("/repo/.claude/worktrees");
    let gone = PathBuf::from("/elsewhere/worktrees/deleted-by-hand");

    let listed = reconcile(&records, root, std::slice::from_ref(&gone), &[], &|_| true);

    assert!(
        !listed.iter().any(|w| w.path == gone),
        "the recorded path is a wish, not a fact. Git no longer reports this worktree, so there is \
         nothing to show — a row conjured from the recorded path alone is precisely the stale entry \
         FR-031 exists to prevent"
    );
}

#[test]
fn an_included_worktree_reports_its_health_like_any_other() {
    let records = parse_worktrees(WITH_OUTSIDER);
    let root = Path::new("/repo/.claude/worktrees");
    let outsider = PathBuf::from("/elsewhere/worktrees/fix-olx");

    // Registered with git, but its directory is no longer on disk.
    let listed = reconcile(&records, root, std::slice::from_ref(&outsider), &[], &|p| {
        p != outsider
    });

    assert_eq!(
        listed.iter().find(|w| w.path == outsider).map(|w| w.status),
        Some(WorktreeStatus::Missing),
        "status is derived, so an included worktree that has been deleted says so rather than \
         going quiet (FR-031)"
    );
}

#[test]
fn an_included_worktree_never_takes_a_name_the_app_is_already_using() {
    const COLLIDING: &str = "\
worktree /repo/.claude/worktrees/fix-olx
HEAD def456
branch refs/heads/fix/olx-ours

worktree /elsewhere/worktrees/fix-olx
HEAD 111111
branch refs/heads/fix/olx
";
    let records = parse_worktrees(COLLIDING);
    let root = Path::new("/repo/.claude/worktrees");
    let outsider = PathBuf::from("/elsewhere/worktrees/fix-olx");

    let listed = reconcile(
        &records,
        root,
        std::slice::from_ref(&outsider),
        &[],
        &|_| true,
    );

    assert_eq!(
        listed.len(),
        2,
        "both are shown; neither displaces the other"
    );
    let names: Vec<&str> = listed.iter().map(|w| w.dir_name.as_str()).collect();
    assert_eq!(
        names.iter().collect::<std::collections::HashSet<_>>().len(),
        2,
        "`dir_name` is the key sessions are stored against, so two worktrees sharing one would \
         hand each other's sessions out. Got {names:?}"
    );
    assert_eq!(
        listed
            .iter()
            .find(|w| !w.included)
            .map(|w| w.dir_name.as_str()),
        Some("fix-olx"),
        "the app's own keeps its name — sessions already address it by that key, and inclusion \
         must not move it out from under them"
    );
    assert_eq!(
        listed.iter().find(|w| w.included).map(|w| w.path.clone()),
        Some(outsider),
        "and nothing on disk was renamed to make room (FR-028)"
    );
}
