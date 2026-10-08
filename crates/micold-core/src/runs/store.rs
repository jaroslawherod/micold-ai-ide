//! The stored shape of a project's run groups (feature 483, data-model.md § Persistence):
//! `runs/<project id>.json` beside `reviews/`, written and read by
//! [`crate::store::JsonFileStore::save_runs`] and [`crate::store::JsonFileStore::load_runs`].

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::{GroupId, Run, RunGroup, RunStatus};
use crate::naming::{DerivedNames, WorktreeNaming};
use crate::session::{AiCli, SessionId};

/// The only version this build writes and reads.
pub const RUNS_FILE_VERSION: u32 = 1;

/// Every run group of one project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunsFile {
    /// The groups, oldest first.
    pub groups: Vec<RunGroup>,
}

/// A run as stored: its names flat on the run (`"dir_name"`, `"branch"`).
#[derive(Serialize, Deserialize)]
struct StoredRun {
    number: u8,
    provider: AiCli,
    dir_name: String,
    branch: String,
    session: Option<SessionId>,
    status: RunStatus,
}

#[derive(Serialize, Deserialize)]
struct StoredGroup {
    id: GroupId,
    name: String,
    naming: WorktreeNaming,
    prompt: String,
    base_branch: String,
    base_commit: String,
    created: SystemTime,
    runs: Vec<StoredRun>,
    winner: Option<u8>,
}

#[derive(Serialize, Deserialize)]
struct StoredRuns {
    version: u32,
    groups: Vec<StoredGroup>,
}

impl RunsFile {
    /// The file's JSON text.
    pub fn to_json(&self) -> String {
        String::new()
    }

    /// Read the file's JSON text; an error for anything unparseable, of another version, or
    /// holding a group that breaks [`RunGroup::validate`].
    pub fn from_json(text: &str) -> Result<Self, String> {
        let _ = text;
        Err(String::new())
    }
}

impl From<&Run> for StoredRun {
    fn from(run: &Run) -> Self {
        Self {
            number: run.number,
            provider: run.provider,
            dir_name: run.names.dir_name.clone(),
            branch: run.names.branch.clone(),
            session: run.session,
            status: run.status.clone(),
        }
    }
}

impl From<StoredRun> for Run {
    fn from(run: StoredRun) -> Self {
        Self {
            number: run.number,
            provider: run.provider,
            names: DerivedNames {
                dir_name: run.dir_name,
                branch: run.branch,
            },
            session: run.session,
            status: run.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runs::tests::{naming, run};
    use crate::runs::RunStep;
    use crate::store::JsonFileStore;
    use serde_json::json;
    use std::path::Path;
    use uuid::Uuid;

    fn sample() -> RunsFile {
        let statuses = [
            RunStatus::Creating,
            RunStatus::Starting,
            RunStatus::Prompted,
            RunStatus::PromptNotDelivered {
                reason: "not ready".into(),
            },
            RunStatus::Failed {
                step: RunStep::Worktree,
                reason: "exists".into(),
            },
            RunStatus::Failed {
                step: RunStep::Session,
                reason: "no cli".into(),
            },
            RunStatus::Picked,
        ];
        let runs = statuses
            .into_iter()
            .enumerate()
            .map(|(i, status)| {
                let mut r = run(i as u8 + 1, status);
                r.session = (i % 2 == 0).then(|| SessionId::from_uuid(Uuid::from_u128(i as u128)));
                r.provider = if i % 2 == 0 {
                    AiCli::ClaudeCode
                } else {
                    AiCli::Copilot
                };
                r
            })
            .collect();
        RunsFile {
            groups: vec![RunGroup {
                id: GroupId(Uuid::from_u128(483)),
                name: "login page".into(),
                naming: naming(),
                prompt: "Add a login page".into(),
                base_branch: "main".into(),
                base_commit: "0123abcd".into(),
                created: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_000),
                runs,
                winner: None,
            }],
        }
    }

    #[test]
    fn the_file_has_the_data_model_shape() {
        let value: serde_json::Value =
            serde_json::from_str(&sample().to_json()).expect("the file is JSON");
        assert_eq!(value["version"], json!(1));
        let group = &value["groups"][0];
        assert_eq!(group["name"], json!("login page"));
        assert_eq!(group["base_branch"], json!("main"));
        assert_eq!(group["winner"], json!(null), "no winner is written as null");
        let first = &group["runs"][0];
        assert_eq!(first["number"], json!(1));
        assert_eq!(first["dir_name"], json!("feat-login-page-1"));
        assert_eq!(first["branch"], json!("feat/login-page-1"));
        assert!(first.get("names").is_none(), "the names are flat on the run");
    }

    #[test]
    fn a_group_with_a_run_in_every_status_round_trips() {
        let file = sample();
        assert_eq!(
            RunsFile::from_json(&file.to_json()).expect("its own output parses"),
            file
        );
    }

    #[test]
    fn unknown_fields_are_ignored_on_read() {
        let mut value: serde_json::Value =
            serde_json::from_str(&sample().to_json()).expect("JSON");
        value["later"] = json!(true);
        value["groups"][0]["later"] = json!({ "x": 1 });
        value["groups"][0]["runs"][0]["later"] = json!("y");
        assert_eq!(
            RunsFile::from_json(&value.to_string()).expect("unknown fields are ignored"),
            sample()
        );
    }

    #[test]
    fn garbage_another_version_or_an_invalid_group_is_refused() {
        assert!(RunsFile::from_json("{ not json").is_err());
        assert!(RunsFile::from_json(r#"{ "version": 2, "groups": [] }"#).is_err());
        let mut value: serde_json::Value =
            serde_json::from_str(&sample().to_json()).expect("JSON");
        value["groups"][0]["runs"] = json!([]);
        assert!(
            RunsFile::from_json(&value.to_string()).is_err(),
            "a group with no runs breaks the invariants"
        );
    }

    #[test]
    fn the_runs_path_is_beside_the_reviews_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = JsonFileStore::at(dir.path().join("projects.json"));
        let project = Path::new("/work/app");
        let runs = store.runs_path(project);
        let reviews = store.reviews_path(project);
        assert_eq!(runs.parent().and_then(Path::file_name), Some("runs".as_ref()));
        assert_eq!(runs.parent().and_then(Path::parent), reviews.parent().and_then(Path::parent));
        assert_eq!(runs.file_name(), reviews.file_name(), "addressed by the project id");
    }

    #[test]
    fn a_missing_file_loads_empty_and_a_corrupt_one_is_kept_aside() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = JsonFileStore::at(dir.path().join("projects.json"));
        let project = Path::new("/work/app");
        assert_eq!(store.load_runs(project), RunsFile::default());
        let path = store.runs_path(project);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, "{ broken").expect("write");
        assert_eq!(store.load_runs(project), RunsFile::default());
        let mut aside = path.as_os_str().to_os_string();
        aside.push(".corrupt");
        assert!(Path::new(&aside).exists(), "the unreadable file is kept aside");
        assert!(!path.exists());
    }

    #[test]
    fn save_then_load_gives_the_groups_back_and_remove_deletes_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = JsonFileStore::at(dir.path().join("projects.json"));
        let project = Path::new("/work/app");
        store.save_runs(project, &sample()).expect("saved");
        assert_eq!(store.load_runs(project), sample());
        store.remove_runs(project).expect("removed");
        assert!(!store.runs_path(project).exists());
        store.remove_runs(project).expect("removing an absent file is success");
    }
}
