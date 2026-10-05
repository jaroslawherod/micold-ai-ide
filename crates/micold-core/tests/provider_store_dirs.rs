//! Feature 582, T018 [US2]: which store directories of a provider could hold a project's sessions
//! (FR-006, FR-007, FR-014; research R3).

#[path = "support/attach_fixture.rs"]
mod attach_fixture;

use std::path::{Path, PathBuf};

use attach_fixture::{encoded, AttachFixture};
use micold_core::attach::SkipReason;
use micold_core::protocol::hashing::sha256_hex;
use micold_core::provider::{AiCliProvider, ClaudeProvider, CopilotProvider, PiProvider};
use uuid::Uuid;

fn claude_cwds(fx: &AttachFixture, worktrees: &[PathBuf]) -> Vec<PathBuf> {
    let found = ClaudeProvider.store_dirs(&fx.home.join(".claude"), &fx.repo, worktrees);
    let mut cwds: Vec<PathBuf> = found.dirs.into_iter().map(|d| d.cwd).collect();
    cwds.sort();
    cwds
}

#[test]
fn claude_returns_the_root_and_the_git_listed_worktrees() {
    let fx = AttachFixture::new(&["alpha"]);
    let alpha = fx.worktree_path("alpha");
    let cwds = claude_cwds(&fx, std::slice::from_ref(&alpha));
    assert!(cwds.contains(&fx.repo), "the encoded root is a store dir");
    assert!(cwds.contains(&alpha), "a git-listed worktree is a store dir");
}

#[test]
fn claude_finds_the_store_dir_of_a_worktree_git_no_longer_lists() {
    let fx = AttachFixture::new(&["alpha"]);
    // `gone` was deleted: no worktree in the list, but its store directory remains and its
    // transcript's first line names a cwd under this project's `.claude/worktrees/`.
    let gone = fx.worktree_path("gone");
    fx.seed_session(&gone);
    let cwds = claude_cwds(&fx, &[]);
    assert!(
        cwds.contains(&gone),
        "R3: a directory under the encoded `.claude/worktrees/` whose transcript cwd is inside \
         this project's worktrees is accepted; got {cwds:?}"
    );
}

#[test]
fn claude_never_returns_a_sibling_projects_directory() {
    let fx = AttachFixture::new(&[]);
    fx.seed_session(&fx.sibling);
    fx.seed_session(&fx.sibling.join(".claude/worktrees/beta"));
    let cwds = claude_cwds(&fx, &[]);
    assert!(
        cwds.iter().all(|c| !c.starts_with(&fx.sibling)),
        "FR-007: `proj-x` is not `proj`; got {cwds:?}"
    );
}

#[test]
fn claude_rejects_a_matching_name_whose_transcript_cwd_is_elsewhere() {
    let fx = AttachFixture::new(&[]);
    let lookalike = fx.worktree_path("evil");
    // The directory name encodes this project's worktree path, but the transcript says it ran
    // somewhere else (the encoding is lossy: `a/b-c` and `a-b/c` collide).
    let dir = fx.home.join(".claude/projects").join(encoded(&lookalike));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{}.jsonl", Uuid::new_v4())),
        "{\"type\":\"user\",\"cwd\":\"/somewhere/else\"}\n",
    )
    .unwrap();
    let cwds = claude_cwds(&fx, &[]);
    assert!(
        !cwds.contains(&lookalike) && !cwds.contains(&PathBuf::from("/somewhere/else")),
        "R3: accepted only when the cwd is under this project's worktrees; got {cwds:?}"
    );
}

#[test]
fn claude_skips_a_directory_with_a_corrupt_first_line_and_says_so() {
    let fx = AttachFixture::new(&[]);
    let bad = fx.worktree_path("bad");
    let dir = fx.home.join(".claude/projects").join(encoded(&bad));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{}.jsonl", Uuid::new_v4())),
        "this is not json\n",
    )
    .unwrap();
    let found = ClaudeProvider.store_dirs(&fx.home.join(".claude"), &fx.repo, &[]);
    assert!(found.dirs.iter().all(|d| d.cwd != bad));
    assert!(
        found
            .notes
            .iter()
            .any(|n| n.reason == SkipReason::EntryCorrupt),
        "FR-009: the bad entry is skipped and reported; got {:?}",
        found.notes
    );
}

#[test]
fn claude_matches_a_windows_style_path_case_insensitively() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = tmp.path().join(".claude");
    let root = Path::new(r"c:\users\x\proj");
    let wt = r"C:\Users\x\Proj\.claude\worktrees\alpha";
    let dir = config.join("projects").join(encoded(Path::new(wt)));
    std::fs::create_dir_all(&dir).unwrap();
    let line = format!(
        "{{\"type\":\"user\",\"cwd\":{}}}\n",
        serde_json::to_string(wt).unwrap()
    );
    std::fs::write(dir.join(format!("{}.jsonl", Uuid::new_v4())), line).unwrap();
    let found = ClaudeProvider.store_dirs(&config, root, &[]);
    assert!(
        found.dirs.iter().any(|d| d.cwd == Path::new(wt)),
        "a path differing only in case and separators is the same project; got {:?}",
        found.dirs
    );
}

#[test]
fn copilot_and_pi_return_the_known_locations_only() {
    let fx = AttachFixture::new(&["alpha"]);
    let alpha = fx.worktree_path("alpha");
    for found in [
        CopilotProvider.store_dirs(&fx.home.join(".copilot"), &fx.repo, std::slice::from_ref(&alpha)),
        PiProvider.store_dirs(&fx.home.join(".pi"), &fx.repo, std::slice::from_ref(&alpha)),
    ] {
        let mut cwds: Vec<PathBuf> = found.dirs.into_iter().map(|d| d.cwd).collect();
        cwds.sort();
        let mut want = vec![fx.repo.clone(), alpha.clone()];
        want.sort();
        assert_eq!(cwds, want, "no unknown-directory scan for these stores");
    }
}

#[test]
fn copilot_lists_this_projects_sessions_from_its_index() {
    let fx = AttachFixture::new(&[]);
    let config = fx.home.join(".copilot");
    let id = Uuid::new_v4();
    let index = config
        .join("sidebar-sessions-state")
        .join(format!("{}.json", sha256_hex(fx.repo.to_string_lossy().as_bytes())));
    std::fs::create_dir_all(index.parent().unwrap()).unwrap();
    std::fs::write(
        index,
        format!("{{\"schemaVersion\":1,\"sessionIds\":[\"{id}\"]}}"),
    )
    .unwrap();
    let ids = CopilotProvider.recorded_session_ids(&config, &fx.repo);
    assert_eq!(ids, vec![id]);
    assert!(
        CopilotProvider
            .recorded_session_ids(&config, &fx.sibling)
            .is_empty(),
        "the sibling project has no index of its own"
    );
}

#[test]
fn pi_lists_this_projects_sessions_from_its_directory() {
    let fx = AttachFixture::new(&[]);
    let config = fx.home.join(".pi/agent");
    let id = Uuid::new_v4();
    let encoded_cwd = format!(
        "--{}--",
        fx.repo
            .to_string_lossy()
            .trim_start_matches('/')
            .replace(['/', '\\', ':'], "-")
    );
    let dir = config.join("sessions").join(encoded_cwd);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("2026-01-01T00-00-00_{id}.jsonl")), "{}\n").unwrap();
    assert_eq!(PiProvider.recorded_session_ids(&config, &fx.repo), vec![id]);
    assert!(PiProvider
        .recorded_session_ids(&config, &fx.sibling)
        .is_empty());
}

#[test]
fn a_provider_with_no_readable_store_lists_nothing() {
    let fx = AttachFixture::new(&[]);
    for ids in [
        CopilotProvider.recorded_session_ids(&fx.home.join("none"), &fx.repo),
        PiProvider.recorded_session_ids(&fx.home.join("none"), &fx.repo),
        ClaudeProvider.recorded_session_ids(&fx.home.join("none"), &fx.repo),
    ] {
        assert!(ids.is_empty(), "FR-014: no store means no sessions, no error");
    }
}
