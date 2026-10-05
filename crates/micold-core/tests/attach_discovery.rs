//! Feature 582, T007 [US1]: which worktrees a project offers to attach (FR-001, FR-004, FR-013).
//!
//! Real git worktrees through the shared fixture, discovered with the production `discover`, so
//! the rule is checked over what git and the filesystem actually report.

#[path = "support/attach_fixture.rs"]
mod attach_fixture;

use std::collections::BTreeSet;

use attach_fixture::AttachFixture;
use micold_core::attach::{attachable_worktrees, AttachableWorktree, Availability, Unavailable};
use micold_core::git::GitCli;
use micold_core::session::{AiCli, Session, SessionLocation};
use micold_core::worktree::{classify_owner, discover, ProvenanceView, WorktreeOwner};

fn offered(
    fx: &AttachFixture,
    records: &BTreeSet<String>,
    sessions: &[Session],
) -> Vec<AttachableWorktree> {
    let found = discover(&GitCli, &fx.repo, &[]);
    attachable_worktrees(&found, &ProvenanceView::new(records), sessions)
}

#[test]
fn agent_owned_unrecorded_worktrees_are_listed() {
    let fx = AttachFixture::new(&["alpha", "beta", "gamma"]);
    let list = offered(&fx, &BTreeSet::new(), &[]);
    let names: Vec<&str> = list.iter().map(|w| w.dir_name.as_str()).collect();
    assert_eq!(
        names,
        ["alpha", "beta", "gamma"],
        "FR-001: every unrecorded provider worktree is offered"
    );
    assert!(list
        .iter()
        .all(|w| w.availability == Availability::Attachable));
    assert_eq!(list[0].branch.as_deref(), Some("worktree-alpha"));
    assert_eq!(list[0].path, fx.worktree_path("alpha"));
}

#[test]
fn a_recorded_worktree_is_not_listed() {
    let fx = AttachFixture::new(&["alpha", "beta"]);
    let records: BTreeSet<String> = ["alpha".to_string()].into();
    let list = offered(&fx, &records, &[]);
    let names: Vec<&str> = list.iter().map(|w| w.dir_name.as_str()).collect();
    assert_eq!(
        names,
        ["beta"],
        "FR-004: an attached worktree is never offered again"
    );
}

#[test]
fn a_prunable_or_missing_worktree_is_unavailable_missing() {
    let fx = AttachFixture::new(&["alpha"]);
    fx.add_missing_worktree("gone");
    let list = offered(&fx, &BTreeSet::new(), &[]);
    let gone = list.iter().find(|w| w.dir_name == "gone").expect("listed");
    assert_eq!(
        gone.availability,
        Availability::Unavailable(Unavailable::Missing)
    );
}

#[test]
fn only_worktrees_directly_under_the_provider_directory_qualify() {
    let fx = AttachFixture::new(&["alpha"]);
    // Beside the repository, under the fixture's already-canonical base (no `\\?\` form on Windows).
    let outside = fx.repo.parent().unwrap().join("elsewhere");
    attach_fixture::git(
        &fx.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "outside",
            outside.to_str().unwrap(),
        ],
    );
    // Included by the user, so `discover` returns it with `included: true`.
    let found = discover(&GitCli, &fx.repo, std::slice::from_ref(&outside));
    assert!(
        found.iter().any(|w| w.included),
        "precondition: the outside worktree is discovered"
    );
    let records = BTreeSet::new();
    let list = attachable_worktrees(&found, &ProvenanceView::new(&records), &[]);
    let names: Vec<&str> = list.iter().map(|w| w.dir_name.as_str()).collect();
    assert_eq!(
        names,
        ["alpha"],
        "a worktree outside .claude/worktrees is never an attach candidate"
    );
}

#[test]
fn an_unattached_provider_worktree_stays_hidden() {
    let fx = AttachFixture::new(&["alpha"]);
    let found = discover(&GitCli, &fx.repo, &[]);
    let records = BTreeSet::new();
    assert_eq!(
        classify_owner(&found[0], &ProvenanceView::new(&records)),
        WorktreeOwner::Agent,
        "FR-013: listing it as attachable must not change that it is hidden until attached"
    );
}

#[test]
fn session_count_and_provider_come_from_the_catalog_sessions_at_that_worktree() {
    let fx = AttachFixture::new(&["alpha", "beta"]);
    let sessions = vec![
        Session::start_new(SessionLocation::Worktree("alpha".into()), AiCli::Pi),
        Session::start_new(SessionLocation::Worktree("alpha".into()), AiCli::Pi),
        Session::start_new(SessionLocation::Default, AiCli::ClaudeCode),
    ];
    let list = offered(&fx, &BTreeSet::new(), &sessions);
    let alpha = list.iter().find(|w| w.dir_name == "alpha").unwrap();
    let beta = list.iter().find(|w| w.dir_name == "beta").unwrap();
    assert_eq!((alpha.session_count, alpha.provider), (2, Some(AiCli::Pi)));
    assert_eq!((beta.session_count, beta.provider), (0, None));
}

// ---------------------------------------------------------------------------------------
// T017 [US2]: resumable provider sessions (FR-006 - FR-010, FR-014, SC-002, SC-005)
// ---------------------------------------------------------------------------------------

mod resumable {
    use super::attach_fixture::{encoded, AttachFixture};
    use micold_core::attach::{
        discover_resumable, AttachTarget, DiscoverInput, ResumableDiscovery, ResumableStatus,
        SkipReason, StoreView, UnresumableReason,
    };
    use micold_core::git::GitCli;
    use micold_core::provider::{ActivitySource, AiCliProvider, ClaudeProvider, StoreDirs};
    use micold_core::provider::{FolderTrust, InputReadiness, ToolServerSupport};
    use micold_core::session::AiCli;
    use micold_core::terminal::LaunchMode;
    use micold_core::worktree::{discover, ProvenanceView};
    use std::cell::Cell;
    use std::collections::BTreeSet;
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};
    use uuid::Uuid;

    fn run_with(
        fx: &AttachFixture,
        provider: &dyn AiCliProvider,
        known: &BTreeSet<Uuid>,
        page: usize,
    ) -> ResumableDiscovery {
        let found = discover(&GitCli, &fx.repo, &[]);
        let records = BTreeSet::new();
        let stores = [StoreView {
            provider,
            config_dir: Some(fx.home.join(".claude")),
        }];
        discover_resumable(
            &stores,
            &DiscoverInput {
                root: &fx.repo,
                worktrees: &found,
                provenance: &ProvenanceView::new(&records),
                known_ids: known,
                page,
                only: None,
            },
        )
    }

    fn run(fx: &AttachFixture, known: &BTreeSet<Uuid>) -> ResumableDiscovery {
        run_with(fx, &ClaudeProvider, known, 200)
    }

    fn ids(found: &ResumableDiscovery) -> BTreeSet<Uuid> {
        found.sessions.iter().map(|s| s.id).collect()
    }

    #[test]
    fn this_projects_five_are_listed_and_the_siblings_two_never() {
        let fx = AttachFixture::new(&["alpha"]);
        let mut mine = BTreeSet::new();
        for _ in 0..3 {
            mine.insert(fx.seed_session(&fx.repo));
        }
        for _ in 0..2 {
            mine.insert(fx.seed_session(&fx.worktree_path("alpha")));
        }
        fx.seed_session(&fx.sibling);
        fx.seed_session(&fx.sibling.join(".claude/worktrees/beta"));
        let found = run(&fx, &BTreeSet::new());
        assert_eq!(ids(&found), mine, "FR-006, FR-007, SC-002");
    }

    #[test]
    fn a_root_session_targets_default_and_is_resumable() {
        let fx = AttachFixture::new(&[]);
        let id = fx.seed_session(&fx.repo);
        let found = run(&fx, &BTreeSet::new());
        let s = found.sessions.iter().find(|s| s.id == id).unwrap();
        assert_eq!(s.target, AttachTarget::Default);
        assert_eq!(s.status, ResumableStatus::Resumable);
        assert_eq!(s.provider, AiCli::ClaudeCode);
    }

    #[test]
    fn a_session_at_an_unattached_worktree_needs_the_worktree_attached() {
        let fx = AttachFixture::new(&["alpha"]);
        let id = fx.seed_session(&fx.worktree_path("alpha"));
        let found = run(&fx, &BTreeSet::new());
        let s = found.sessions.iter().find(|s| s.id == id).unwrap();
        assert_eq!(
            s.target,
            AttachTarget::Worktree {
                dir_name: "alpha".into()
            }
        );
        assert_eq!(s.status, ResumableStatus::NeedsWorktreeAttach);
    }

    #[test]
    fn a_session_already_in_the_catalog_is_not_listed() {
        let fx = AttachFixture::new(&[]);
        let known = fx.seed_session(&fx.repo);
        let other = fx.seed_session(&fx.repo);
        let found = run(&fx, &[known].into());
        assert_eq!(ids(&found), [other].into(), "scenario 3: never twice");
    }

    #[test]
    fn a_deleted_worktrees_sessions_are_unresumable_and_never_mapped_elsewhere() {
        let fx = AttachFixture::new(&["alpha"]);
        fx.add_missing_worktree("gone");
        let prunable = fx.seed_session(&fx.worktree_path("gone"));
        // A worktree git no longer lists at all.
        let unlisted = fx.seed_session(&fx.worktree_path("vanished"));
        let found = run(&fx, &BTreeSet::new());
        let find = |id| found.sessions.iter().find(|s| s.id == id).unwrap();
        assert_eq!(
            find(prunable).status,
            ResumableStatus::Unresumable(UnresumableReason::WorktreeMissing)
        );
        assert_eq!(
            find(unlisted).status,
            ResumableStatus::Unresumable(UnresumableReason::NoLocation)
        );
        assert_eq!(
            find(unlisted).target,
            AttachTarget::Worktree {
                dir_name: "vanished".into()
            },
            "never mapped to the root or another worktree"
        );
    }

    #[test]
    fn a_corrupt_entry_is_skipped_and_noted_while_the_rest_list() {
        let fx = AttachFixture::new(&[]);
        let good = fx.seed_session(&fx.repo);
        let bad = fx.worktree_path("bad");
        let dir = fx.home.join(".claude/projects").join(encoded(&bad));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{}.jsonl", Uuid::new_v4())), "{{{ nope\n").unwrap();
        let found = run(&fx, &BTreeSet::new());
        assert_eq!(ids(&found), [good].into(), "SC-005");
        assert!(found
            .notes
            .iter()
            .any(|n| n.reason == SkipReason::EntryCorrupt));
    }

    #[test]
    fn a_missing_store_gives_no_sessions_and_a_note() {
        let fx = AttachFixture::new(&[]);
        let found = discover_resumable(
            &[StoreView {
                provider: &ClaudeProvider,
                config_dir: Some(fx.home.join("no-such-store")),
            }],
            &DiscoverInput {
                root: &fx.repo,
                worktrees: &[],
                provenance: &ProvenanceView::none(),
                known_ids: &BTreeSet::new(),
                page: 50,
                only: None,
            },
        );
        assert!(found.sessions.is_empty());
        assert!(found
            .notes
            .iter()
            .any(|n| n.reason == SkipReason::StoreMissing && n.provider == Some(AiCli::ClaudeCode)));
    }

    #[test]
    fn a_provider_with_no_config_dir_is_a_note_not_an_error() {
        let found = discover_resumable(
            &[StoreView {
                provider: &ClaudeProvider,
                config_dir: None,
            }],
            &DiscoverInput {
                root: Path::new("/a/proj"),
                worktrees: &[],
                provenance: &ProvenanceView::none(),
                known_ids: &BTreeSet::new(),
                page: 50,
                only: None,
            },
        );
        assert!(found.sessions.is_empty());
        assert_eq!(found.notes.len(), 1);
    }

    #[test]
    fn a_windows_path_in_a_different_case_is_the_same_project() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = tmp.path().join(".claude");
        let wt = r"C:\Users\x\Proj\.claude\worktrees\alpha";
        let dir = config.join("projects").join(encoded(Path::new(wt)));
        std::fs::create_dir_all(&dir).unwrap();
        let id = Uuid::new_v4();
        std::fs::write(
            dir.join(format!("{id}.jsonl")),
            format!(
                "{{\"type\":\"user\",\"cwd\":{}}}\n",
                serde_json::to_string(wt).unwrap()
            ),
        )
        .unwrap();
        let found = discover_resumable(
            &[StoreView {
                provider: &ClaudeProvider,
                config_dir: Some(config),
            }],
            &DiscoverInput {
                root: Path::new(r"c:\users\x\proj"),
                worktrees: &[],
                provenance: &ProvenanceView::none(),
                known_ids: &BTreeSet::new(),
                page: 50,
                only: None,
            },
        );
        let s = found.sessions.iter().find(|s| s.id == id).expect("listed");
        assert_eq!(
            s.target,
            AttachTarget::Worktree {
                dir_name: "alpha".into()
            }
        );
    }

    /// Counts the reads discovery makes per conversation; everything else is Claude's.
    struct Counting {
        titles: Cell<usize>,
        labels: Cell<usize>,
    }

    impl AiCliProvider for Counting {
        fn id(&self) -> AiCli {
            ClaudeProvider.id()
        }
        fn display_name(&self) -> &'static str {
            ClaudeProvider.display_name()
        }
        fn command(&self) -> &'static str {
            ClaudeProvider.command()
        }
        fn is_available(&self, path: &OsStr) -> bool {
            ClaudeProvider.is_available(path)
        }
        fn launch_args(&self, id: Uuid, mode: LaunchMode) -> Vec<String> {
            ClaudeProvider.launch_args(id, mode)
        }
        fn config_dir(&self) -> Option<PathBuf> {
            ClaudeProvider.config_dir()
        }
        fn launch_env(&self) -> Vec<(String, String)> {
            ClaudeProvider.launch_env()
        }
        fn recorded_session_ids(&self, c: &Path, cwd: &Path) -> Vec<Uuid> {
            ClaudeProvider.recorded_session_ids(c, cwd)
        }
        fn has_recorded_conversation(&self, c: &Path, cwd: &Path, id: Uuid) -> bool {
            ClaudeProvider.has_recorded_conversation(c, cwd, id)
        }
        fn read_title(&self, c: &Path, cwd: &Path, id: Uuid) -> Option<String> {
            self.titles.set(self.titles.get() + 1);
            ClaudeProvider.read_title(c, cwd, id)
        }
        fn read_label(&self, c: &Path, cwd: &Path, id: Uuid) -> Option<String> {
            self.labels.set(self.labels.get() + 1);
            ClaudeProvider.read_label(c, cwd, id)
        }
        fn name_in_terminal_title(&self, t: &str, cwd: &Path) -> Option<String> {
            ClaudeProvider.name_in_terminal_title(t, cwd)
        }
        fn mark_archived(&self, c: &Path, cwd: &Path, id: Uuid) -> std::io::Result<()> {
            ClaudeProvider.mark_archived(c, cwd, id)
        }
        fn is_archived(&self, c: &Path, cwd: &Path, id: Uuid) -> bool {
            ClaudeProvider.is_archived(c, cwd, id)
        }
        fn activity_source(&self, c: &Path, cwd: &Path, id: Uuid) -> ActivitySource {
            ClaudeProvider.activity_source(c, cwd, id)
        }
        fn tool_server_support(&self) -> ToolServerSupport {
            ClaudeProvider.tool_server_support()
        }
        fn input_readiness(&self) -> InputReadiness {
            ClaudeProvider.input_readiness()
        }
        fn folder_trust(&self) -> FolderTrust {
            ClaudeProvider.folder_trust()
        }
        fn store_dirs(&self, c: &Path, root: &Path, worktrees: &[PathBuf]) -> StoreDirs {
            ClaudeProvider.store_dirs(c, root, worktrees)
        }
        fn last_activity(&self, c: &Path, cwd: &Path, id: Uuid) -> Option<SystemTime> {
            ClaudeProvider.last_activity(c, cwd, id)
        }
    }

    #[test]
    fn thousands_list_newest_first_and_only_the_page_is_read_for_titles() {
        let fx = AttachFixture::new(&[]);
        let dir = fx.home.join(".claude/projects").join(encoded(&fx.repo));
        std::fs::create_dir_all(&dir).unwrap();
        let base = SystemTime::now() - Duration::from_secs(100_000);
        let mut by_age = Vec::new();
        for n in 0..1500u64 {
            let id = Uuid::new_v4();
            let path = dir.join(format!("{id}.jsonl"));
            std::fs::write(&path, "{\"type\":\"user\"}\n").unwrap();
            let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
            file.set_modified(base + Duration::from_secs(n * 10)).unwrap();
            by_age.push(id);
        }
        let counting = Counting {
            titles: Cell::new(0),
            labels: Cell::new(0),
        };
        let found = run_with(&fx, &counting, &BTreeSet::new(), 50);
        let listed: Vec<Uuid> = found.sessions.iter().map(|s| s.id).collect();
        let newest_first: Vec<Uuid> = by_age.iter().rev().take(50).copied().collect();
        assert_eq!(listed, newest_first, "R4: newest first, bounded to the page");
        assert!(
            counting.titles.get() <= 50 && counting.labels.get() <= 50,
            "titles and labels are read for the returned page only: {} / {}",
            counting.titles.get(),
            counting.labels.get()
        );
    }

    fn tree(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                tree(&path, out);
            } else {
                out.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
    }

    #[test]
    fn discovery_leaves_the_provider_store_untouched() {
        let fx = AttachFixture::new(&["alpha"]);
        fx.seed_session(&fx.repo);
        fx.seed_session(&fx.worktree_path("alpha"));
        fx.seed_session(&fx.worktree_path("gone"));
        let snapshot = |fx: &AttachFixture| {
            let mut all = Vec::new();
            tree(&fx.home, &mut all);
            all
        };
        let before = snapshot(&fx);
        let _ = run(&fx, &BTreeSet::new());
        assert_eq!(before, snapshot(&fx), "FR-010: only read methods are called");
    }
}
