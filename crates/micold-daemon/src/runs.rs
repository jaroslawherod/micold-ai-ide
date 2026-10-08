//! Run groups held by the service (feature 483, contracts/run-group-wire.md W1, W2, W5).
//!
//! One list of [`RunGroup`]s per project, read from its runs file the first time anything asks for
//! it. A new group is written through the catalog before memory changes (W5), so a failed write
//! creates nothing. Each run then progresses in a task of its own ([`spawn_runs`]): no run waits
//! on, or rolls back, another (research R3, R4, FR-005). The prompt is never logged.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use micold_core::git::{Git, GitCli};
use micold_core::naming::WorktreeNaming;
use micold_core::protocol::messages::{DaemonMsg, ErrorKind};
use micold_core::runs::naming::derive_group;
use micold_core::runs::store::RunsFile;
use micold_core::runs::{GroupId, NewGroup, Run, RunGroup, RunStatus, RunStep, MAX_RUNS, MIN_RUNS};
use micold_core::session::{AiCli, SessionId};
use micold_core::worktree::{explain_directory_taken, CreateError, CreateMode};

use crate::catalog::Catalog;
use crate::ops;
use crate::review::Refusal;
use crate::state::DaemonState;

/// Every project's run groups the service has read so far.
#[derive(Debug, Default)]
pub struct Runs {
    projects: HashMap<PathBuf, Vec<RunGroup>>,
}

impl Runs {
    /// `project`'s groups, read from its runs file on first use.
    fn project(&mut self, catalog: &Catalog, project: &Path) -> &mut Vec<RunGroup> {
        self.projects
            .entry(project.to_path_buf())
            .or_insert_with(|| catalog.load_runs(project).groups)
    }

    /// The `RunGroupsChanged` that carries `project`'s whole list now.
    pub fn changed(&mut self, catalog: &Catalog, project: &Path) -> DaemonMsg {
        DaemonMsg::RunGroupsChanged {
            project: project.to_path_buf(),
            groups: self.project(catalog, project).clone(),
        }
    }

    /// Add `group` to `project`: written first, then held (W5). On a refusal nothing changed.
    pub fn add(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        group: RunGroup,
    ) -> Result<(), Refusal> {
        let groups = self.project(catalog, project);
        let mut next = groups.clone();
        next.push(group);
        let file = RunsFile { groups: next };
        catalog.save_runs(project, &file).map_err(|err| {
            tracing::warn!(project = %project.display(), %err, "run group not written");
            Refusal::new(
                ErrorKind::IoFailed,
                format!("the run group could not be saved: {err}"),
            )
        })?;
        *groups = file.groups;
        Ok(())
    }

    /// Move run `number` of `group` to `status`, recording `session` when given, if
    /// [`RunStatus::can_become`] allows it. `None` when the run is gone or the move is not allowed.
    ///
    /// The run's worktree or session already changed when this is called, so memory follows it
    /// even when the file cannot be written; the next successful write catches the file up.
    pub fn set_run(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        group: GroupId,
        number: u8,
        status: RunStatus,
        session: Option<SessionId>,
    ) -> Option<DaemonMsg> {
        let groups = self.project(catalog, project);
        let run = groups
            .iter_mut()
            .find(|g| g.id == group)?
            .runs
            .iter_mut()
            .find(|run| run.number == number)?;
        if !run.status.can_become(&status) {
            tracing::warn!(%group, run = number, "run status change not allowed; ignored");
            return None;
        }
        run.status = status;
        if session.is_some() {
            run.session = session;
        }
        let file = RunsFile {
            groups: groups.clone(),
        };
        if let Err(err) = catalog.save_runs(project, &file) {
            tracing::warn!(project = %project.display(), %group, run = number, %err, "run status not written");
        }
        Some(self.changed(catalog, project))
    }

    /// Forget `project`'s groups; the catalog removes its runs file (W5).
    pub fn forget_project(&mut self, project: &Path) {
        self.projects.remove(project);
    }
}

/// What a `RunGroupCreate` asks for (W1).
#[derive(Debug, Clone)]
pub struct CreateRequest {
    /// The project.
    pub project: PathBuf,
    /// What the runs' names derive from.
    pub naming: WorktreeNaming,
    /// The prompt every run gets.
    pub prompt: String,
    /// The branch every run starts from.
    pub base_branch: String,
    /// One provider per run, in run order.
    pub providers: Vec<AiCli>,
}

/// Check `request` in the order of W1, record the new group with every run `Creating`, and return
/// it. Nothing is created, written or pushed on a refusal. The caller answers, pushes the list and
/// then [`spawn_runs`].
pub async fn create(state: &Arc<DaemonState>, request: CreateRequest) -> Result<RunGroup, Refusal> {
    let CreateRequest {
        project,
        naming,
        prompt,
        base_branch,
        providers,
    } = request;
    let repo = match state.project_repo(&project) {
        // The catalog's flag is from when the project was last looked at: look again.
        Some((repo, true)) if repo.join(".git").exists() => repo,
        _ => {
            return Err(Refusal::new(
                ErrorKind::InvalidInput,
                format!("{} is not a known git repository", project.display()),
            ))
        }
    };
    let count = providers.len();
    if !(MIN_RUNS..=MAX_RUNS).contains(&count) {
        return Err(Refusal::new(
            ErrorKind::InvalidInput,
            format!("a run group takes {MIN_RUNS} to {MAX_RUNS} runs, not {count}"),
        ));
    }
    // `count` is at most MAX_RUNS here, so it fits.
    let names = derive_group(&naming, count as u8)
        .map_err(|err| Refusal::new(ErrorKind::InvalidInput, err.to_string()))?;
    if prompt.trim().is_empty() {
        return Err(Refusal::new(
            ErrorKind::InvalidInput,
            "the prompt is empty: enter what every run should do",
        ));
    }
    let base_commit = {
        let (repo, branch) = (repo.clone(), base_branch.clone());
        tokio::task::spawn_blocking(move || GitCli::new().branch_tip(&repo, &branch))
            .await
            .ok()
            .flatten()
    };
    let Some(base_commit) = base_commit else {
        return Err(Refusal::new(
            ErrorKind::NotFound,
            format!("the base branch {base_branch} is not a local branch"),
        ));
    };
    let runs = providers
        .into_iter()
        .zip(names)
        .zip(1..)
        .map(|((provider, names), number)| Run {
            number,
            provider,
            names,
            session: None,
            status: RunStatus::Creating,
        })
        .collect();
    let group = RunGroup::new(NewGroup {
        id: GroupId::new(),
        name: naming.name.trim().to_owned(),
        naming,
        prompt,
        base_branch,
        base_commit,
        created: SystemTime::now(),
        runs,
    })
    .map_err(|err| Refusal::new(ErrorKind::InvalidInput, err.to_string()))?;
    state.add_run_group(&project, group.clone())?;
    tracing::info!(
        project = %project.display(),
        group = %group.id,
        runs = count,
        base = %group.base_branch,
        "run group created"
    );
    Ok(group)
}

/// Start one task per run of `group` (W2): its worktree at the base commit, then its session
/// with the prompt as first input. Each reports its own transitions.
pub fn spawn_runs(state: &Arc<DaemonState>, project: &Path, group: &RunGroup) {
    let prompt: Arc<str> = Arc::from(group.prompt.as_str());
    for run in &group.runs {
        tokio::spawn(run_one(
            Arc::clone(state),
            project.to_path_buf(),
            group.id,
            // The tip recorded at create time, not the branch name: every run starts at the same
            // commit even if the branch moves meanwhile or a tag shares its name.
            group.base_commit.clone(),
            Arc::clone(&prompt),
            run.clone(),
        ));
    }
}

/// One run, start to end. Never returns an error: every failure is the run's status.
async fn run_one(
    state: Arc<DaemonState>,
    project: PathBuf,
    group: GroupId,
    base_commit: String,
    prompt: Arc<str>,
    run: Run,
) {
    let number = run.number;
    let set = |status: RunStatus, session: Option<SessionId>| {
        state.set_run_status(&project, group, number, status, session)
    };
    let mode = CreateMode::NewBranchAt { start: base_commit };
    if let Err(failure) =
        ops::create_worktree(&state, project.clone(), run.names.clone(), mode, None).await
    {
        let reason = worktree_reason(failure, &run.names.branch);
        set(
            RunStatus::Failed {
                step: RunStep::Worktree,
                reason,
            },
            None,
        );
        return;
    }
    if !set(RunStatus::Starting, None) {
        // The group is no longer held (its project was forgotten): start no session for it.
        return;
    }

    let Some((repo, _)) = state.project_repo(&project) else {
        set(session_failed("the project is no longer known"), None);
        return;
    };
    let cwd = repo.join(".claude/worktrees").join(&run.names.dir_name);
    let cli = run.provider;
    if let Some(why) = ops::cli_unavailable(&state, &cwd, cli).await {
        set(session_failed(why), None);
        return;
    }
    let first = ops::FirstPrompt {
        text: &prompt,
        asked: tokio::time::Instant::now(),
        // A run's prompt goes in as one paste, or not at all (R4).
        require_bracketed: true,
    };
    let created = ops::create_session_with_prompt(
        &state,
        ops::NewSession {
            project: &project,
            worktree_dir: &run.names.dir_name,
            cwd: &cwd,
            cli,
        },
        Some(first),
    )
    .await;
    match created {
        Err(err) => {
            set(
                session_failed(format!("could not create the session: {err}")),
                None,
            );
        }
        Ok((session, Ok(()))) => {
            set(RunStatus::Prompted, Some(session));
        }
        Ok((session, Err(why))) => {
            if matches!(why, ops::FirstPromptUndelivered::NotStarted) {
                // No agent ran, so there is nothing to pick: the run failed at its session.
                set(session_failed(why.message(session)), Some(session));
                return;
            }
            let reason = match why {
                // A terminal that went away or refused the write: said as for a CLI never ready,
                // as the MCP create does.
                ops::FirstPromptUndelivered::Typing(_) => {
                    ops::FirstPromptUndelivered::NotReady(cli, state.first_prompt_bound())
                        .message(session)
                }
                why => why.message(session),
            };
            set(RunStatus::PromptNotDelivered { reason }, Some(session));
        }
    }
}

fn session_failed(reason: impl Into<String>) -> RunStatus {
    RunStatus::Failed {
        step: RunStep::Session,
        reason: reason.into(),
    }
}

/// Why run's worktree on `branch` was not created, in the words the single create uses.
fn worktree_reason(failure: ops::CreateFailure, branch: &str) -> String {
    match failure {
        ops::CreateFailure::NotARepository => "the project is not a git repository".into(),
        ops::CreateFailure::Task(why) => format!("the worktree could not be created: {why}"),
        ops::CreateFailure::Create(err) => match err {
            CreateError::BranchInUse { branch, reason } => reason.explain(&branch),
            // A run's branch is always new: one that exists already is left as it is.
            CreateError::SituationChanged => format!("the branch {branch} already exists"),
            CreateError::DuplicateDir { dir } => explain_directory_taken(&dir).to_string(),
            CreateError::RolledBack(stderr) => {
                format!("git failed to create the worktree: {}", stderr.trim())
            }
        },
    }
}
