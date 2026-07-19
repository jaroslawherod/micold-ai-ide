//! Persistence boundary for the known-projects catalog.
//!
//! Fronted as a trait so workspace logic is testable without touching the real user data
//! directory (Constitution Principle I). The production `JsonFileStore` (added with User
//! Story 2) uses `serde_json` + `directories`; a missing or corrupt file degrades to an
//! empty catalog rather than crashing (Principle IV; research R8). On-disk format is the
//! durable contract in `specs/002-project-workspace-management/contracts/storage-schema.md`.

use crate::project::{Availability, Project};
use crate::session::{Session, SessionId, SessionLabel, SessionLocation};
use crate::workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

/// The current on-disk schema version (see the storage-schema contract).
const SCHEMA_VERSION: u32 = 1;

/// How a load resolved. Always yields a usable [`Workspace`]; `status` distinguishes a
/// clean first run from a recovery so the app can optionally note it — neither aborts
/// startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOutcome {
    /// The loaded catalog (empty on a missing or recovered store).
    pub workspace: Workspace,
    /// What happened during the load.
    pub status: LoadStatus,
}

/// The disposition of a [`ProjectStore::load`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    /// A store file was present and parsed successfully.
    Loaded,
    /// No store file existed yet — treated as an empty catalog (first run).
    Missing,
    /// A store file existed but was unparseable — recovered to an empty catalog.
    Recovered,
}

/// Load and save the known-projects catalog on the local filesystem (local-first).
pub trait ProjectStore {
    /// Load the catalog. Never fails: a missing or corrupt store yields an empty catalog
    /// with the corresponding [`LoadStatus`] (research R8).
    fn load(&self) -> LoadOutcome;

    /// Persist the catalog. Writes atomically (temp file + rename) so a crash mid-save
    /// cannot truncate the list (research R8).
    fn save(&self, workspace: &Workspace) -> io::Result<()>;
}

/// The on-disk shape of the catalog. Unknown fields are ignored on read (serde default),
/// and missing optional fields take their defaults — both give forward compatibility
/// (storage-schema contract). `availability` is intentionally **not** persisted; it is
/// recomputed from the filesystem on load (FR-022).
#[derive(Debug, Serialize, Deserialize)]
struct StoredCatalog {
    schema_version: u32,
    #[serde(default)]
    last_active: Option<PathBuf>,
    #[serde(default)]
    projects: Vec<StoredProject>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredProject {
    path: PathBuf,
    display_name: String,
    #[serde(default)]
    is_git_repo: bool,
    // Feature 005, option A (contracts/storage-schema.md): an optional per-project sessions
    // array. `default` keeps older files (without the field) loading unchanged — forward
    // compatible, so no `schema_version` bump is required.
    #[serde(default)]
    sessions: Vec<StoredSession>,
    // Feature 008 (contracts/persistence.md): per-worktree display-name overrides, keyed by
    // worktree `dir_name`. `default` keeps older files (without the field) loading unchanged —
    // forward compatible, so no `schema_version` bump is required.
    #[serde(default)]
    worktree_display_names: BTreeMap<String, String>,
}

/// The persisted shape of a session (FR-020): identity, location, last-known title. Lifecycle
/// and terminal buffers are never persisted (FR-021). `worktree_dir` widened to
/// `Option<String>` in feature 010 (contracts/010-root-dir-session/storage-schema.md):
/// `Some(dir)` -> [`SessionLocation::Worktree`], `None`/absent -> [`SessionLocation::Default`].
/// Every session persisted before feature 010 has `worktree_dir` as a plain JSON string, which
/// `serde_json` deserializes into `Some(String)` unchanged — no migration, no schema bump.
#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    id: uuid::Uuid,
    #[serde(default)]
    worktree_dir: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

impl StoredSession {
    fn location_to_stored(location: &SessionLocation) -> Option<String> {
        match location {
            SessionLocation::Worktree(dir) => Some(dir.clone()),
            SessionLocation::Default => None,
        }
    }

    fn stored_to_location(worktree_dir: Option<String>) -> SessionLocation {
        match worktree_dir {
            Some(dir) => SessionLocation::Worktree(dir),
            None => SessionLocation::Default,
        }
    }
}

impl StoredCatalog {
    fn from_workspace(ws: &Workspace) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            last_active: ws.active.clone(),
            projects: ws
                .projects
                .iter()
                .map(|p| StoredProject {
                    path: p.path.clone(),
                    display_name: p.display_name.clone(),
                    is_git_repo: p.is_git_repo,
                    sessions: ws
                        .sessions
                        .get(&p.path)
                        .map(|list| {
                            list.iter()
                                .map(|s| StoredSession {
                                    id: s.id.0,
                                    worktree_dir: StoredSession::location_to_stored(&s.location),
                                    title: match &s.label {
                                        SessionLabel::Named(t) => Some(t.clone()),
                                        SessionLabel::Pending => None,
                                    },
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    worktree_display_names: ws
                        .worktree_names
                        .get(&p.path)
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect(),
        }
    }

    fn into_workspace(self) -> Workspace {
        let mut sessions: BTreeMap<PathBuf, Vec<Session>> = BTreeMap::new();
        let mut worktree_names: BTreeMap<PathBuf, BTreeMap<String, String>> = BTreeMap::new();
        let projects: Vec<Project> = self
            .projects
            .into_iter()
            .map(|p| {
                let restored: Vec<Session> = p
                    .sessions
                    .into_iter()
                    .map(|s| {
                        let label = match s.title {
                            Some(t) => SessionLabel::Named(t),
                            None => SessionLabel::Pending,
                        };
                        Session::restored(
                            SessionId::from_uuid(s.id),
                            StoredSession::stored_to_location(s.worktree_dir),
                            label,
                        )
                    })
                    .collect();
                if !restored.is_empty() {
                    sessions.insert(p.path.clone(), restored);
                }
                if !p.worktree_display_names.is_empty() {
                    worktree_names.insert(p.path.clone(), p.worktree_display_names);
                }
                Project {
                    path: p.path,
                    display_name: p.display_name,
                    is_git_repo: p.is_git_repo,
                    // Availability is not persisted; assume available until the caller
                    // refreshes it against the filesystem (FR-022).
                    availability: Availability::Available,
                }
            })
            .collect();

        // A `last_active` that does not match a known project is treated as no active
        // project (storage-schema contract).
        let active = self
            .last_active
            .filter(|path| projects.iter().any(|p| &p.path == path));

        Workspace {
            projects,
            active,
            sessions,
            worktree_names,
        }
    }
}

/// The production [`ProjectStore`]: a JSON file in the per-user data directory.
#[derive(Debug, Clone)]
pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    /// A store backed by an explicit file path (used by tests with a temp directory).
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// A store at the conventional per-user location: `<data_dir>/projects.json`, where
    /// `data_dir` is resolved by `directories` (XDG / Application Support / AppData). The
    /// application tuple MUST stay stable across releases (storage-schema contract).
    /// Returns `None` if no home/data directory can be determined.
    pub fn default_location() -> Option<Self> {
        directories::ProjectDirs::from("", "", "micold-ai-ide")
            .map(|dirs| Self::at(dirs.data_dir().join("projects.json")))
    }

    /// Path of the temporary file used for atomic writes.
    fn temp_path(&self) -> PathBuf {
        self.path.with_extension("json.tmp")
    }

    /// Path the corrupt file is moved to before recovery.
    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.bak")
    }
}

impl ProjectStore for JsonFileStore {
    fn load(&self) -> LoadOutcome {
        let contents = match std::fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return LoadOutcome {
                    workspace: Workspace::empty(),
                    status: LoadStatus::Missing,
                };
            }
            // Unreadable for any other reason: recover rather than crash (Principle IV).
            Err(_) => {
                return LoadOutcome {
                    workspace: Workspace::empty(),
                    status: LoadStatus::Recovered,
                };
            }
        };

        match serde_json::from_str::<StoredCatalog>(&contents) {
            Ok(stored) => LoadOutcome {
                workspace: stored.into_workspace(),
                status: LoadStatus::Loaded,
            },
            Err(_) => {
                // Corrupt: preserve the bad file (best-effort) and recover to empty.
                let _ = std::fs::rename(&self.path, self.backup_path());
                LoadOutcome {
                    workspace: Workspace::empty(),
                    status: LoadStatus::Recovered,
                }
            }
        }
    }

    fn save(&self, workspace: &Workspace) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let stored = StoredCatalog::from_workspace(workspace);
        let json = serde_json::to_string_pretty(&stored)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        // Atomic write: temp file in the same directory, then rename over the target.
        let temp = self.temp_path();
        std::fs::write(&temp, json)?;
        std::fs::rename(&temp, &self.path)?;
        Ok(())
    }
}
