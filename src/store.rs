//! Persistence boundary for the known-projects catalog and per-project state.
//!
//! Fronted as a trait so workspace logic is testable without touching the real user data
//! directory (Constitution Principle I). The production `JsonFileStore` (added with User
//! Story 2) uses `serde_json` + `directories`; a missing or corrupt file degrades to an
//! empty catalog rather than crashing (Principle IV; research R8). On-disk format is the
//! durable contract in `specs/002-project-workspace-management/contracts/storage-schema.md`.
//!
//! **Bugfix BUG-001 (2026-07-21)**: the catalog (`projects.json`) and each project's own state
//! (sessions, worktree display-name overrides, terminal mode) now live in **separate files** —
//! a small catalog plus one state file per project, named by [`project_id`]. A fault reading or
//! writing one project's state file is isolated to that project; it can no longer take every
//! other project's sessions down with it (FR-012a). A pre-split `projects.json` — with
//! `sessions`/`worktree_display_names` still embedded on a `StoredProject` entry — is read as a
//! one-time migration seed (`StoredProject`'s own fields, kept `#[serde(default)]` for
//! deserialization but `skip_serializing` so new saves never re-embed them) whenever a project's
//! new-style state file does not exist yet; the next `save` writes that data out to the
//! project's own state file and stops carrying it in the catalog.

use crate::project::{Availability, Project};
use crate::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use crate::workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

/// The current on-disk schema version (see the storage-schema contract). Shared by the catalog
/// and the per-project state file — both are additive-only so far and have never needed to
/// diverge.
const SCHEMA_VERSION: u32 = 1;

/// How a load resolved. Always yields a usable [`Workspace`]; `status` distinguishes a
/// clean first run from a recovery so the app can optionally note it — neither aborts
/// startup.
///
/// Reflects the **catalog's** load outcome only (bugfix BUG-001): a fault loading one project's
/// separate state file is isolated to that project (FR-012a) and never changes this status — see
/// `contracts/storage-schema.md` "Bugfix: per-project storage split".
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
    // Bugfix BUG-001 (2026-07-21): sessions and worktree-name overrides no longer live here —
    // they live in this project's own state file (see `StoredProjectState` /
    // `JsonFileStore::project_state_path`). These two fields are kept ONLY as a one-time
    // migration seed: `#[serde(default)]` lets an old (pre-split) `projects.json` — which still
    // has them embedded — deserialize normally, while `skip_serializing` means a save under the
    // new scheme never writes them back out, so the catalog naturally slims down to
    // `path`/`display_name`/`is_git_repo` after one save cycle. No `schema_version` bump
    // (removing what a save never emits again is compatible with every existing reader that
    // already tolerates unknown/missing fields).
    #[serde(default, skip_serializing)]
    sessions: Vec<StoredSession>,
    #[serde(default, skip_serializing)]
    worktree_display_names: BTreeMap<String, String>,
}

/// The persisted shape of a session (FR-020): identity, location, last-known title. Lifecycle
/// and terminal buffers are never persisted (FR-021). `worktree_dir` widened to
/// `Option<String>` in feature 010 (contracts/010-root-dir-session/storage-schema.md):
/// `Some(dir)` -> [`SessionLocation::Worktree`], `None`/absent -> [`SessionLocation::Default`].
/// Every session persisted before feature 010 has `worktree_dir` as a plain JSON string, which
/// `serde_json` deserializes into `Some(String)` unchanged — no migration, no schema bump.
/// `mode` is a second, independent feature 010 addition (FR-011;
/// contracts/persistence-schema.md).
#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    id: uuid::Uuid,
    #[serde(default)]
    worktree_dir: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    mode: StoredTerminalMode,
    /// Closed via FR-015a (bugfix BUG-003). `#[serde(default)]` — absent/`false` for a live
    /// session written before this feature; no `schema_version` bump. **Not authoritative** — a
    /// fast in-memory convenience only. The durable source of truth reconciliation (FR-020c)
    /// consults is the AI CLI provider's own marker file (`src/provider.rs`
    /// `mark_archived`/`is_archived`), which survives even if this field's own file is lost.
    #[serde(default)]
    archived: bool,
}

/// Serde-mapped mirror of [`TerminalMode`] (feature 010, research R5): kept as a separate type
/// rather than deriving `Serialize`/`Deserialize` directly on the pure-core enum, so the
/// persisted *shape* can evolve independently (mirrors `title`/`SessionLabel`).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
enum StoredTerminalMode {
    #[default]
    AiCli,
    Regular,
}

impl From<TerminalMode> for StoredTerminalMode {
    fn from(mode: TerminalMode) -> Self {
        match mode {
            TerminalMode::AiCli => StoredTerminalMode::AiCli,
            TerminalMode::Regular => StoredTerminalMode::Regular,
        }
    }
}

impl From<StoredTerminalMode> for TerminalMode {
    fn from(mode: StoredTerminalMode) -> Self {
        match mode {
            StoredTerminalMode::AiCli => TerminalMode::AiCli,
            StoredTerminalMode::Regular => TerminalMode::Regular,
        }
    }
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

    /// Build the persisted form of a live [`Session`] (shared by the catalog's legacy migration
    /// path and the per-project state file).
    fn from_session(session: &Session) -> Self {
        Self {
            id: session.id.0,
            worktree_dir: Self::location_to_stored(&session.location),
            title: match &session.label {
                SessionLabel::Named(t) => Some(t.clone()),
                SessionLabel::Pending => None,
            },
            mode: session.mode.into(),
            archived: session.archived,
        }
    }

    /// Restore a live [`Session`] from its persisted form (shared by the catalog's legacy
    /// migration path and the per-project state file).
    fn into_session(self) -> Session {
        let label = match self.title {
            Some(t) => SessionLabel::Named(t),
            None => SessionLabel::Pending,
        };
        let mut session = Session::restored(
            SessionId::from_uuid(self.id),
            Self::stored_to_location(self.worktree_dir),
            label,
            self.mode.into(),
        );
        session.archived = self.archived;
        session
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
                    // Never re-emitted (`skip_serializing`) — populating them here is harmless
                    // and keeps this method a straightforward mirror of `into_workspace`, but the
                    // per-project state file (see `save`) is what actually persists this data now.
                    sessions: ws
                        .sessions
                        .get(&p.path)
                        .map(|list| list.iter().map(StoredSession::from_session).collect())
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

    /// Consume the catalog into a [`Workspace`]. `sessions`/`worktree_names` are populated from
    /// whatever each `StoredProject` still carries embedded — for a post-split save that's always
    /// empty (bugfix BUG-001: `skip_serializing`); for a pre-split file it's the real data, and
    /// serves as the one-time migration seed `JsonFileStore::load` falls back to when a project's
    /// own state file does not exist yet.
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
                    .map(StoredSession::into_session)
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

/// The on-disk shape of one project's own state (bugfix BUG-001): sessions and worktree
/// display-name overrides, addressed by [`project_id`] rather than nested under the catalog.
/// Same forward-compatibility rules as the catalog (unknown fields ignored, missing optional
/// fields default).
#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredProjectState {
    schema_version: u32,
    #[serde(default)]
    sessions: Vec<StoredSession>,
    #[serde(default)]
    worktree_display_names: BTreeMap<String, String>,
}

impl StoredProjectState {
    fn from_workspace(ws: &Workspace, project_path: &Path) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            sessions: ws
                .sessions
                .get(project_path)
                .map(|list| list.iter().map(StoredSession::from_session).collect())
                .unwrap_or_default(),
            worktree_display_names: ws
                .worktree_names
                .get(project_path)
                .cloned()
                .unwrap_or_default(),
        }
    }
}

/// How a single project's state file resolved (bugfix BUG-001; mirrors [`LoadStatus`] but scoped
/// to one project rather than the whole catalog, per FR-012a's fault-isolation guarantee).
enum ProjectStateLoad {
    /// The file existed and parsed — its data is authoritative for this project.
    Found(StoredProjectState),
    /// No state file exists yet for this project — the caller falls back to whatever the
    /// catalog's own legacy-migration fields carry (empty for a project that never had one).
    Missing,
    /// The file existed but did not parse (or could not be read for another reason) — degrades
    /// to empty for this project only (research R8's precedent, scoped down by FR-012a: a fault
    /// here MUST NOT touch the catalog or any other project's state).
    Corrupt,
}

/// A stable, filesystem-safe identifier for a project's own state file, derived from its
/// canonical path (bugfix BUG-001). Deterministic within one build of this app: `DefaultHasher`
/// uses fixed keys (unlike `RandomState`), so the same path always yields the same id for as
/// long as the app's Rust toolchain doesn't change its standard hasher algorithm — a change there
/// would only orphan existing per-project files (degrading them to empty on next load, same as
/// any other missing-file case), never crash or corrupt data.
fn project_id(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Path of the temporary file used for an atomic write to `path`.
fn temp_path_for(path: &Path) -> PathBuf {
    path.with_extension("json.tmp")
}

/// Path a corrupt file at `path` is moved to before recovery.
fn backup_path_for(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

/// Write `state` to `path` atomically (temp file in the same directory, then rename), creating
/// the parent directory if needed. Shared by every per-project state write.
fn write_project_state(path: &Path, state: &StoredProjectState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let temp = temp_path_for(path);
    std::fs::write(&temp, json)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

/// Load one project's state file at `path` (bugfix BUG-001). Never fails: see
/// [`ProjectStateLoad`].
fn load_project_state(path: &Path) -> ProjectStateLoad {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return ProjectStateLoad::Missing,
        Err(_) => return ProjectStateLoad::Corrupt,
    };
    match serde_json::from_str::<StoredProjectState>(&contents) {
        Ok(state) => ProjectStateLoad::Found(state),
        Err(_) => {
            // Corrupt: preserve the bad file (best-effort) and recover to empty, isolated to
            // this project only (FR-012a) — mirrors the catalog's own corrupt-file handling.
            let _ = std::fs::rename(path, backup_path_for(path));
            ProjectStateLoad::Corrupt
        }
    }
}

/// The production [`ProjectStore`]: a JSON file in the per-user data directory, plus one sibling
/// state file per known project (bugfix BUG-001).
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

    /// Path of the temporary file used for the catalog's atomic write.
    fn temp_path(&self) -> PathBuf {
        temp_path_for(&self.path)
    }

    /// Path the corrupt catalog file is moved to before recovery.
    fn backup_path(&self) -> PathBuf {
        backup_path_for(&self.path)
    }

    /// Directory holding every project's own state file (bugfix BUG-001): a `projects/`
    /// subdirectory next to the catalog file.
    fn project_state_dir(&self) -> PathBuf {
        match self.path.parent() {
            Some(parent) => parent.join("projects"),
            None => PathBuf::from("projects"),
        }
    }

    /// The state file for a specific project, addressed by [`project_id`] (bugfix BUG-001).
    /// `pub` so tests can locate it directly without duplicating the naming scheme.
    pub fn project_state_path(&self, project_path: &Path) -> PathBuf {
        self.project_state_dir()
            .join(format!("{}.json", project_id(project_path)))
    }

    /// Delete a project's per-project state file when the project is forgotten (feature 014,
    /// FR-005). An already-absent file is success — forgetting a project that never had a state
    /// file (no sessions/overrides) is not an error, and the call is idempotent. Removing this
    /// file is what makes the discarded session metadata durable: `save` only (re)writes state
    /// files for projects still in the catalog, so without this the forgotten project's file would
    /// linger and could be reloaded if the folder were re-opened (FR-012). This file lives in the
    /// application's own data directory, never inside the project folder (so FR-006 is untouched).
    pub fn remove_project_state(&self, project_path: &Path) -> io::Result<()> {
        match std::fs::remove_file(self.project_state_path(project_path)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}

impl ProjectStore for JsonFileStore {
    fn load(&self) -> LoadOutcome {
        let (status, stored) = match std::fs::read_to_string(&self.path) {
            Ok(contents) => match serde_json::from_str::<StoredCatalog>(&contents) {
                Ok(stored) => (LoadStatus::Loaded, stored),
                Err(_) => {
                    // Corrupt: preserve the bad file (best-effort) and recover to empty.
                    let _ = std::fs::rename(&self.path, self.backup_path());
                    (
                        LoadStatus::Recovered,
                        StoredCatalog {
                            schema_version: SCHEMA_VERSION,
                            last_active: None,
                            projects: Vec::new(),
                        },
                    )
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => (
                LoadStatus::Missing,
                StoredCatalog {
                    schema_version: SCHEMA_VERSION,
                    last_active: None,
                    projects: Vec::new(),
                },
            ),
            // Unreadable for any other reason: recover rather than crash (Principle IV).
            Err(_) => (
                LoadStatus::Recovered,
                StoredCatalog {
                    schema_version: SCHEMA_VERSION,
                    last_active: None,
                    projects: Vec::new(),
                },
            ),
        };

        // Legacy fallback: whatever `sessions`/`worktree_display_names` a pre-split catalog
        // still carries embedded (bugfix BUG-001) — empty for any post-split save.
        let mut workspace = stored.into_workspace();

        // Each project's own state file is authoritative once it exists; a fault loading one is
        // isolated to that project and never affects the catalog or any other project (FR-012a).
        for project in &workspace.projects {
            let state_path = self.project_state_path(&project.path);
            match load_project_state(&state_path) {
                ProjectStateLoad::Found(state) => {
                    if state.sessions.is_empty() {
                        workspace.sessions.remove(&project.path);
                    } else {
                        let restored = state
                            .sessions
                            .into_iter()
                            .map(StoredSession::into_session)
                            .collect();
                        workspace.sessions.insert(project.path.clone(), restored);
                    }
                    if state.worktree_display_names.is_empty() {
                        workspace.worktree_names.remove(&project.path);
                    } else {
                        workspace
                            .worktree_names
                            .insert(project.path.clone(), state.worktree_display_names);
                    }
                }
                ProjectStateLoad::Missing => {
                    // Keep whatever the legacy catalog fallback already populated (possibly
                    // nothing, for a project that never had sessions/overrides).
                }
                ProjectStateLoad::Corrupt => {
                    workspace.sessions.remove(&project.path);
                    workspace.worktree_names.remove(&project.path);
                }
            }
        }

        LoadOutcome { workspace, status }
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

        // Each project's own state, written independently (FR-012a): a failure writing one
        // project's file never prevents another project's file (or the catalog above) from being
        // written. The first error encountered is returned, after every project has been tried.
        let mut first_err = None;
        for project in &workspace.projects {
            let state = StoredProjectState::from_workspace(workspace, &project.path);
            let path = self.project_state_path(&project.path);
            if let Err(err) = write_project_state(&path, &state) {
                first_err.get_or_insert(err);
            }
        }
        match first_err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}
