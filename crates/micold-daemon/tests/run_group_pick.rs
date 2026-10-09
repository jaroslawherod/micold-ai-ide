//! Pick this one on the service (feature 483, contracts/run-group-wire.md W3, integration.md): the
//! refusals in contract order, each changing nothing; a fast-forward and a merge commit; a base
//! branch that is checked out; and two windows picking at once.

// unix-only: the stand-in CLI is a `#!/bin/sh` script, as in `run_group_create.rs`.
#![cfg(unix)]

#[path = "support/runs.rs"]
mod runs_support;

use std::fs;
use std::path::Path;

use micold_core::protocol::messages::{ClientMsg, ErrorKind, OperationResult};
use micold_core::runs::{GroupId, Integration, RunGroup, RunStatus};
use micold_core::session::AiCli;
use runs_support::{
    attach, connect, create_msg, created, git, request, settled, window, Sandbox, ENV,
};

const CLAUDE: AiCli = AiCli::ClaudeCode;

fn pick(req: u64, project: &Path, group: GroupId, run: u8) -> ClientMsg {
    ClientMsg::RunGroupPick {
        req,
        project: project.to_path_buf(),
        group,
        run,
    }
}

/// A settled group of `n` prompted runs, and a window on the project.
async fn group_of(s: &Sandbox, n: usize) -> (runs_support::Client, RunGroup) {
    let project = s.project();
    let mut client = window(&s.state, &project).await;
    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; n])).await;
    let group = settled(&mut client, created(&answer)).await;
    (client, group)
}

/// Commit `file` = `content` in run `run`'s worktree.
fn run_commits(s: &Sandbox, group: &RunGroup, run: u8, file: &str, content: &str) {
    let wt = s.worktree(&group.runs[usize::from(run) - 1].names.dir_name);
    fs::write(wt.join(file), content).unwrap();
    git(&wt, &["add", "-A"]);
    git(&wt, &["commit", "-q", "-m", &format!("{file}: {content}")]);
}

/// Commit `file` = `content` on `base` without leaving `main` checked out.
fn base_commits(s: &Sandbox, file: &str, content: &str) {
    let p = s.project();
    git(&p, &["checkout", "-q", "base"]);
    fs::write(p.join(file), content).unwrap();
    git(&p, &["add", "-A"]);
    git(&p, &["commit", "-q", "-m", &format!("{file}: {content}")]);
    git(&p, &["checkout", "-q", "main"]);
}

/// Every ref, and the runs file as written.
fn snapshot(s: &Sandbox) -> (String, String) {
    let refs = git(
        &s.project(),
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    );
    let file = fs::read_to_string(s.runs_path()).unwrap_or_default();
    (refs, file)
}

fn tip(s: &Sandbox, branch: &str) -> String {
    git(
        &s.project(),
        &["rev-parse", &format!("refs/heads/{branch}")],
    )
}

/// Send a pick that must be refused with `kind`; the message, with refs and the runs file unchanged.
async fn refused(
    s: &Sandbox,
    client: &mut runs_support::Client,
    group: GroupId,
    run: u8,
    kind: ErrorKind,
) -> String {
    let before = snapshot(s);
    let (answer, pushes) = request(client, 9, pick(9, &s.project(), group, run)).await;
    let (got, message) = answer.expect_err("the pick is refused");
    assert_eq!(got, kind, "{message}");
    assert_eq!(snapshot(s), before, "a refusal changes nothing");
    assert!(pushes.is_empty(), "a refusal pushes nothing");
    message
}

/// W3 steps 1, 3, 4: unknown group, a run still being made, a failed run.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w3_unknown_busy_and_failed_runs_are_refused() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let project = s.project();
    git(&project, &["branch", "feat/login-page-2", "main"]);
    s.slow(2.0);
    let mut client = window(&s.state, &project).await;
    refused(&s, &mut client, GroupId::new(), 1, ErrorKind::NotFound).await;

    let (answer, _) = request(&mut client, 1, create_msg(1, &project, vec![CLAUDE; 3])).await;
    let id = created(&answer);
    // The runs are still being made, so the file and the run branches are moving: only the base
    // branch and the winner are held still.
    let base_before = tip(&s, "base");
    let (answer, _) = request(&mut client, 8, pick(8, &project, id, 1)).await;
    let (kind, busy) = answer.expect_err("a group still being made is busy");
    assert_eq!(kind, ErrorKind::Busy, "{busy}");
    assert!(
        busy.contains("run "),
        "names a run still being made: {busy}"
    );
    assert_eq!(tip(&s, "base"), base_before);

    let group = settled(&mut client, id).await;
    let failed = refused(&s, &mut client, id, 2, ErrorKind::Refused).await;
    let RunStatus::Failed { reason, .. } = &group.runs[1].status else {
        panic!("run 2 failed");
    };
    assert!(failed.contains(reason.as_str()), "{failed}");
    let missing = refused(&s, &mut client, id, 7, ErrorKind::NotFound).await;
    assert!(!missing.is_empty());
}

/// W3 step 5, US4 s9: uncommitted changes in the run name their files; once committed the pick goes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w3_uncommitted_changes_refuse_naming_the_files_until_committed() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 2).await;
    run_commits(&s, &group, 1, "done.txt", "x\n");
    let wt = s.worktree(&group.runs[0].names.dir_name);
    fs::write(wt.join("wip file.txt"), "w\n").unwrap();
    let message = refused(&s, &mut client, group.id, 1, ErrorKind::Refused).await;
    assert!(message.contains("wip file.txt"), "{message}");
    assert!(
        !message.contains("done.txt"),
        "committed files are not listed"
    );
    git(&wt, &["add", "-A"]);
    git(&wt, &["commit", "-q", "-m", "wip"]);
    let (answer, _) = request(&mut client, 10, pick(10, &s.project(), group.id, 1)).await;
    assert!(
        matches!(answer, Ok(OperationResult::RunPicked { .. })),
        "{answer:?}"
    );
}

/// W3 step 6, FR-013, SC-006: conflicts name the files and change nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn w3_conflicts_refuse_naming_the_files_and_change_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 2).await;
    run_commits(&s, &group, 1, "shared.txt", "run\n");
    base_commits(&s, "shared.txt", "base\n");
    let message = refused(&s, &mut client, group.id, 1, ErrorKind::Refused).await;
    assert!(message.contains("shared.txt"), "{message}");
}

/// US4 s1, I3: the base branch has not moved: it fast-forwards to the run's tip and the winner is
/// recorded only after; a second pick is refused with the winner reason (FR-018).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us4_s1_a_pick_fast_forwards_the_base_and_records_the_winner() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 3).await;
    run_commits(&s, &group, 2, "login.rs", "fn main() {}\n");
    let run_tip = tip(&s, &group.runs[1].names.branch);
    let (answer, pushes) = request(&mut client, 5, pick(5, &s.project(), group.id, 2)).await;
    match answer {
        Ok(OperationResult::RunPicked {
            group: g,
            run,
            integration,
        }) => {
            assert_eq!((g, run), (group.id, 2));
            assert_eq!(
                integration,
                Integration::FastForward {
                    base_tip: run_tip.clone()
                }
            );
        }
        other => panic!("accepted: {other:?}"),
    }
    assert_eq!(tip(&s, "base"), run_tip, "the base moved to the run's tip");
    assert_eq!(tip(&s, &group.runs[1].names.branch), run_tip);
    let pushed = match pushes.last() {
        Some(groups) => groups.clone(),
        None => runs_support::next_runs_push(&mut client, runs_support::BOUND)
            .await
            .expect("pushed"),
    };
    let held = pushed.iter().find(|g| g.id == group.id).unwrap();
    assert_eq!(held.winner, Some(2));
    assert_eq!(held.runs[1].status, RunStatus::Picked);
    assert_eq!(s.on_disk().groups[0].winner, Some(2));
    let message = refused(&s, &mut client, group.id, 1, ErrorKind::Refused).await;
    assert!(message.contains("already has a winner"), "{message}");
}

/// US4 s1, I4: the base moved on: a merge commit with the base tip and the run tip as parents, the
/// run branch unmoved.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_diverged_base_gets_a_merge_commit_naming_the_run_and_group() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 2).await;
    run_commits(&s, &group, 1, "run.txt", "r\n");
    base_commits(&s, "base.txt", "b\n");
    let (base_before, run_tip) = (tip(&s, "base"), tip(&s, &group.runs[0].names.branch));
    let (answer, _) = request(&mut client, 5, pick(5, &s.project(), group.id, 1)).await;
    let Ok(OperationResult::RunPicked {
        integration: Integration::MergeCommit { commit },
        ..
    }) = answer
    else {
        panic!("a merge commit: {answer:?}");
    };
    assert_eq!(tip(&s, "base"), commit);
    assert_eq!(
        git(&s.project(), &["rev-list", "--parents", "-n1", "base"]),
        format!("{commit} {base_before} {run_tip}")
    );
    assert_eq!(
        git(&s.project(), &["log", "-1", "--format=%s", "base"]),
        "Merge run 1 of login page into base"
    );
    assert_eq!(tip(&s, &group.runs[0].names.branch), run_tip);
}

/// I5: a base branch checked out in the project root is merged there, so its files follow.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_base_checked_out_in_the_project_is_merged_in_that_checkout() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 2).await;
    run_commits(&s, &group, 1, "run.txt", "r\n");
    git(&s.project(), &["checkout", "-q", "base"]);
    let run_tip = tip(&s, &group.runs[0].names.branch);
    let (answer, _) = request(&mut client, 5, pick(5, &s.project(), group.id, 1)).await;
    assert!(
        matches!(
            answer,
            Ok(OperationResult::RunPicked {
                integration: Integration::FastForward { .. },
                ..
            })
        ),
        "{answer:?}"
    );
    assert_eq!(tip(&s, "base"), run_tip);
    assert_eq!(
        fs::read_to_string(s.project().join("run.txt")).unwrap(),
        "r\n",
        "the checkout has the run's files"
    );
}

/// W3 step 8: a checked-out base with an edit the merge would overwrite refuses with git's message
/// and leaves the edit alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_busy_base_checkout_refuses_with_git_s_message_and_changes_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 2).await;
    run_commits(&s, &group, 1, "run.txt", "r\n");
    git(&s.project(), &["checkout", "-q", "base"]);
    fs::write(s.project().join("run.txt"), "mine\n").unwrap();
    let message = refused(&s, &mut client, group.id, 1, ErrorKind::Refused).await;
    assert!(message.contains("run.txt"), "{message}");
    assert_eq!(
        fs::read_to_string(s.project().join("run.txt")).unwrap(),
        "mine\n"
    );
    assert!(!s.project().join(".git/MERGE_HEAD").exists());
}

/// FR-018, US4 s6: two windows pick at once; one run is integrated and the other window is told the
/// group has a winner.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_concurrent_picks_integrate_exactly_one() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut first, group) = group_of(&s, 2).await;
    run_commits(&s, &group, 1, "one.txt", "1\n");
    run_commits(&s, &group, 2, "two.txt", "2\n");
    let project = s.project();
    let mut second = connect(&s.state).await;
    attach(&mut second, &project).await;
    let (a, b) = tokio::join!(
        request(&mut first, 20, pick(20, &project, group.id, 1)),
        request(&mut second, 21, pick(21, &project, group.id, 2)),
    );
    let answers = [a.0, b.0];
    let ok = answers.iter().filter(|a| a.is_ok()).count();
    assert_eq!(ok, 1, "exactly one pick is accepted: {answers:?}");
    let refused = answers.iter().find_map(|a| a.as_ref().err()).unwrap();
    assert_eq!(refused.0, ErrorKind::Refused);
    assert!(refused.1.contains("already has a winner"), "{}", refused.1);
    let winner = s.on_disk().groups[0].winner.expect("one winner");
    let loser_branch = &group.runs[usize::from(3 - winner) - 1].names.branch;
    assert_ne!(tip(&s, "base"), tip(&s, "main"));
    assert!(
        git(&project, &["branch", "--contains", &tip(&s, "base")]).contains("base"),
        "the base holds the winner"
    );
    assert!(
        !git(&project, &["rev-list", "base"]).contains(&tip(&s, loser_branch)),
        "the loser's commit is not on the base"
    );
}

/// US4 s1, integration.md: a run whose tip the base branch already holds has nothing to integrate:
/// the answer is a fast-forward at the unchanged base tip, and no commit is made.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_run_already_contained_in_the_base_integrates_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 2).await;
    // The run has no commit of its own, and the base moves on past it.
    base_commits(&s, "base.txt", "b\n");
    let project = s.project();
    let base_before = tip(&s, "base");
    let count_before = git(&project, &["rev-list", "--count", "base"]);
    let (answer, _) = request(&mut client, 5, pick(5, &project, group.id, 1)).await;
    let Ok(OperationResult::RunPicked { integration, .. }) = answer else {
        panic!("accepted: {answer:?}");
    };
    assert_eq!(
        integration,
        Integration::FastForward {
            base_tip: base_before.clone()
        }
    );
    assert_eq!(tip(&s, "base"), base_before, "the base did not move");
    assert_eq!(
        git(&project, &["rev-list", "--count", "base"]),
        count_before,
        "no commit was made"
    );
}

/// integration.md I5: the base branch moving between the pre-check and the merge in its checkout
/// refuses with nothing merged. A `git` on `PATH` moves `base` right after `git merge-tree`, which
/// is the pre-check, so the window is hit deterministically.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_base_that_moves_before_the_checkout_merge_is_refused_and_not_merged() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = ENV.lock().await;
    let s = Sandbox::new();
    let (mut client, group) = group_of(&s, 2).await;
    run_commits(&s, &group, 1, "run.txt", "r\n");
    let project = s.project();
    let moved = git(
        &project,
        &["commit-tree", "base^{tree}", "-p", "base", "-m", "moved"],
    );
    git(&project, &["branch", "moved", &moved]);
    git(&project, &["checkout", "-q", "base"]);
    let shim = s.bin.path().join("git");
    std::fs::write(
        &shim,
        "#!/bin/sh\n/usr/bin/git \"$@\"\nstatus=$?\n\
         case \"$*\" in *merge-tree*) /usr/bin/git -C \"$2\" update-ref refs/heads/base refs/heads/moved;; esac\n\
         exit $status\n",
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    let run_tip = tip(&s, &group.runs[0].names.branch);

    let (answer, pushes) = request(&mut client, 5, pick(5, &project, group.id, 1)).await;
    let (kind, message) = answer.expect_err("the pick is refused");
    assert_eq!(kind, ErrorKind::Refused, "{message}");
    assert!(message.contains("moved while"), "{message}");
    assert!(pushes.is_empty(), "a refusal pushes nothing");
    assert_eq!(tip(&s, "base"), moved, "the base was not merged into");
    assert_eq!(tip(&s, &group.runs[0].names.branch), run_tip);
    assert!(!project.join(".git/MERGE_HEAD").exists());
    assert!(!project.join("run.txt").exists(), "nothing was merged");
    assert_eq!(s.on_disk().groups[0].winner, None);
}
