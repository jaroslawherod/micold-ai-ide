//! AI CLI provider abstraction (FR-024, bugfix BUG-002; reshaped by feature 026 FR-020–FR-022).
//!
//! The AI coding CLI that backs a session is reached through a single seam, so adding another
//! provider means adding one implementation and touching nothing else. Two exist: [`ClaudeProvider`]
//! (`claude`) and [`CopilotProvider`] (`copilot`).
//!
//! # What feature 026 changed, and why
//!
//! The trait used to encode *one* CLI's storage layout as trait-level defaults — `transcript_dir`
//! plus a `discover_transcript_session_ids` that listed it, and an `archived_marker_path` that
//! built `{id}.archived` inside it. That is a per-cwd *directory* whose `*.jsonl` filenames are
//! session ids, which is exactly and only how `claude` works. Copilot's is a per-cwd *index file*
//! listing ids (`sidebar-sessions-state/<sha256(cwd)>.json`), with each conversation in its own
//! directory. A second provider would have had to override the defaults to stop them being wrong,
//! which is how a seam comes to be un-substitutable while looking fine.
//!
//! So **the trait has no defaults at all now**. Every method is required, the layout arithmetic
//! moved into each implementation as private helpers, and the seam gained identity
//! ([`AiCliProvider::id`], [`AiCliProvider::display_name`]), availability
//! ([`AiCliProvider::is_available`]) and an activity source ([`ActivitySource`]).
//!
//! # Choosing one
//!
//! [`AiCli::provider`] — an exhaustive match, so the lookup is total by construction and no call
//! site has an `Option` to mishandle. It lives *here*, in the one crate all three can see:
//! `micold-daemon` depends on `micold-client` only as a dev-dependency and `micold-core` cannot
//! depend on it at all, while the daemon's catalog, state and supervisor and this crate's own
//! `terminal.rs` all need to resolve a provider from a session's [`AiCli`]. The client's
//! `Capabilities` delegates here rather than owning a second map.
//!
//! This module is therefore the **one place a concrete provider type is named**, and
//! `micold-client/tests/no_concrete_implementations.rs` lists it as an explicit exemption. It is
//! the definition site: it names both types by necessity.
//!
//! Pure + unit-testable: this module never spawns a process (the real PTY launch lives behind
//! [`crate::terminal::TerminalBackend`]); its I/O is limited to best-effort reads of a provider's
//! own on-disk conversation store, and a `PATH` lookup for [`AiCliProvider::is_available`].
//! Contracts: `specs/005-worktree-session-terminal/contracts/claude-cli.md` and
//! `specs/026-multi-provider-sessions/contracts/{ai-cli-provider,copilot-cli}.md`.

use crate::session::AiCli;
use crate::terminal::LaunchMode;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Where one session's busy/idle events come from (feature 026, FR-018).
///
/// The daemon's `Activity` state machine consumes the same events either way and is **not**
/// changed by this feature; this only says which mechanism delivers them.
///
/// Answering this does not commit the daemon to observing anything. A source is opened only for a
/// session the application is *supervising*; a session merely discovered under FR-014 is never
/// watched, however many of them a project holds (SC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivitySource {
    /// The provider pushes to the loopback hook receiver, and its launch needs the per-session
    /// settings file the daemon already writes (`claude` — feature 010's mechanism, unchanged).
    ///
    /// **Deliberately payload-free.** An earlier draft carried `Hooks { settings: PathBuf }`,
    /// which no provider can honour: that path is *chosen and written* by the daemon, not derived.
    /// `State::hook_settings_file` calls `receiver.prepare_settings(id)`, which writes a file
    /// containing a port picked at daemon start and a per-session bearer token. A pure
    /// `(config_dir, cwd, id)` derivation in this crate cannot see any of that. The variant's job
    /// is to name the mechanism; the daemon supplies the file exactly as it does today.
    Hooks,
    /// The daemon tails an append-only event log the provider writes, at a path the provider
    /// *can* derive (`copilot`).
    EventLog {
        /// The log to tail.
        path: PathBuf,
    },
    /// This provider reports nothing; the signal stays `Unknown`.
    None,
}

/// Abstraction over an AI coding CLI. Consolidates every provider-specific detail: identity,
/// availability, the launch command + argument shape, how the app-owned session id is passed and
/// resumed, where the conversation record lives, how a recorded conversation is detected, how the
/// session title is extracted, the durable close marker, and where activity events come from.
///
/// Adding a provider means adding an impl of this trait and one arm to [`AiCli::provider`] —
/// nothing above the seam changes.
///
/// **No method has a default.** That is the point of the reshape (FR-021): a default that assumes
/// one CLI's storage shape is how the previous version came to be un-substitutable, and the third
/// provider must inherit nothing it has to override.
pub trait AiCliProvider {
    // --- identity & availability ---

    /// The persisted, looked-up name of this provider.
    fn id(&self) -> AiCli;

    /// The user-facing name ("Claude Code", "GitHub Copilot"), used where the application **offers
    /// a choice or names a failure** — the Settings default, the per-session override list, the
    /// missing-CLI message. Those are menus and sentences.
    fn display_name(&self) -> &'static str;

    /// The executable to spawn, resolved on `PATH` — and also the string a sidebar row and the
    /// terminal bar carry as their label (`claude`, `copilot`), per FR-016/FR-016a.
    ///
    /// Two registers, one provider: a label inside a width budget is not a menu entry, and letting
    /// either leak into the other is the likeliest drift in this seam.
    fn command(&self) -> &'static str;

    /// Whether [`Self::command`] resolves on `PATH` right now. Never cached here and never
    /// persisted (research R11) — the answer changes when the user installs something.
    fn is_available(&self) -> bool;

    // --- launching ---

    /// The argument vector for a launch, given the app-owned session id and fresh/resume mode.
    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String>;

    /// The provider's base config directory (environment-derived), or `None` when it cannot be
    /// determined — callers treat that as "uncertain", never as "absent".
    fn config_dir(&self) -> Option<PathBuf>;

    // --- conversation storage ---

    /// Every session id this provider has recorded for `cwd` (FR-014).
    ///
    /// Best-effort: a missing or unreadable source contributes nothing, never an error, so
    /// discovery never fails a project open. Replaces feature 021's `transcript_dir` + listing
    /// default, which encoded `claude`'s layout for everybody.
    fn recorded_session_ids(&self, config_dir: &Path, cwd: &Path) -> Vec<Uuid>;

    /// Whether this provider has recorded a conversation for this session.
    fn has_recorded_conversation(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool;

    /// The latest title this provider has recorded, read from disk (best-effort I/O). NEVER
    /// errors: any missing file / read / parse failure yields `None`, so a title read never fails
    /// the session (FR-017).
    fn read_title(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String>;

    // --- durable close/remove suppression ---

    /// Record that the user closed or removed this session (FR-015): write an empty marker in the
    /// provider's own storage, so it survives even if the application's own record is lost.
    /// Best-effort I/O — a failure never fails the caller.
    fn mark_archived(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> io::Result<()>;

    /// Whether the user has closed or removed this session — the durable check discovery consults,
    /// independent of the application's own store.
    fn is_archived(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool;

    // --- activity ---

    /// How this provider's busy/idle events reach the daemon for one session. Pure, like every
    /// other derivation here — which is why [`ActivitySource::Hooks`] carries nothing.
    fn activity_source(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> ActivitySource;
}

impl AiCli {
    /// The implementation behind this name.
    ///
    /// An **exhaustive match**, not a map: the map made the lookup partial by type while every
    /// caller wanted it infallible, and a `provider(which)` that can be absent is exactly the shape
    /// that invites an `unwrap()` on the set-wide paths where a wrong answer silently archives
    /// someone's sessions. Totality is structural here, and it costs a `match`.
    pub fn provider(self) -> &'static dyn AiCliProvider {
        static CLAUDE: ClaudeProvider = ClaudeProvider;
        static COPILOT: CopilotProvider = CopilotProvider;
        match self {
            AiCli::ClaudeCode => &CLAUDE,
            AiCli::Copilot => &COPILOT,
        }
    }
}

impl std::fmt::Display for AiCli {
    /// The **human-readable** name — "Claude Code", "GitHub Copilot".
    ///
    /// Of the two registers this seam carries, `Display` is the one a menu wants, and menus are
    /// what formats an `AiCli`: the Settings select and the per-session override list both render
    /// their options through `ToString`. So this is [`AiCliProvider::display_name`], and the other
    /// register — [`AiCliProvider::command`], the `claude`/`copilot` a sidebar row and the terminal
    /// bar carry — is only ever reached by asking for it in as many words.
    ///
    /// That asymmetry is deliberate rather than incidental. Both strings hang off the same
    /// provider and letting either leak into the other is the likeliest drift here, so the rule is
    /// that the *implicit* rendering is the sentence form and the label form must be spelled out.
    /// `tests/features_settings.rs` and `tests/features_sidebar.rs` hold both ends of it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.provider().display_name())
    }
}

/// Which AI CLIs resolve **in this process's own environment** (feature 027, FR-023c).
///
/// The name is the point. FR-023c says availability MUST be determined *where sessions run* —
/// inside the sandbox under sandboxed placement, on the host under host placement — and the only
/// honest way to answer that is to ask the process that runs them. That process is the session
/// service, so the service calls this and reports the answer over the protocol
/// ([`crate::protocol::messages::ClientMsg::AiCliAvailabilityRequest`]).
///
/// **The client must not call it.** Under sandboxed placement the client is on the host and the
/// sessions are not, so the client's own `PATH` answers a different question — one whose answer
/// happens to look plausible, which is why the mistake survived a whole feature. Before 027 the
/// client probed itself and the picker offered whatever the *developer's* machine had installed;
/// `micold-client/tests/cli_availability_comes_from_the_service.rs` is the gate that keeps it from
/// coming back.
///
/// In `AiCli::ALL`'s order, so the list a picker is built from is the declared one.
pub fn available_here() -> Vec<AiCli> {
    AiCli::ALL
        .into_iter()
        .filter(|which| which.provider().is_available())
        .collect()
}

/// Whether `command` resolves to a file on `PATH` (feature 026, FR-006/FR-010).
///
/// Platform-neutral **without a `cfg`**, which is Principle VI's ask rather than a flourish: the
/// separator comes from [`std::env::split_paths`], and the Windows `.exe`/`.cmd` question is
/// answered by `PATHEXT`, which Windows sets and Unix does not. So one code path decides it on all
/// three platforms, and the least-exercised one is not the one with its own branch.
///
/// It deliberately does not check the executable bit: that needs `PermissionsExt` behind a
/// `cfg(unix)`, and a file on `PATH` under the CLI's own name that is not executable is a broken
/// installation the spawn will report anyway (FR-010's failure path).
fn resolves_on_path(command: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    let is_file = |candidate: PathBuf| {
        std::fs::metadata(candidate)
            .map(|meta| meta.is_file())
            .unwrap_or(false)
    };
    // `PATHEXT` entries include their own leading dot (`.EXE;.CMD;…`).
    let extensions: Vec<String> = std::env::var("PATHEXT")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| value.split(';').map(str::to_string).collect())
        .unwrap_or_default();
    std::env::split_paths(&path).any(|dir| {
        is_file(dir.join(command))
            || extensions
                .iter()
                .any(|ext| is_file(dir.join(format!("{command}{ext}"))))
    })
}

// ---------------------------------------------------------------------------------------
// Claude Code
// ---------------------------------------------------------------------------------------

/// Anthropic's `claude` (Claude Code) CLI. Verified against v2.1.210 (research R6). See the
/// `claude` profile in `specs/005-worktree-session-terminal/contracts/claude-cli.md`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeProvider;

impl ClaudeProvider {
    /// Environment variable overriding the default `~/.claude` config directory.
    const CONFIG_DIR_ENV: &'static str = "CLAUDE_CONFIG_DIR";

    /// `<config>/projects/<encoded-cwd>/` — every conversation transcript for sessions run in
    /// `cwd` (research R6). Pure path derivation, no I/O.
    ///
    /// A private helper rather than trait surface: it is `claude`'s layout and nothing else's, and
    /// leaving it on the seam is what made the seam un-substitutable (FR-021).
    fn transcript_dir(&self, config_dir: &Path, cwd: &Path) -> PathBuf {
        // The worktree path with every non-alphanumeric char replaced by `-`.
        let encoded: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        config_dir.join("projects").join(encoded)
    }

    /// The conversation transcript for one session. Pure path derivation, no I/O.
    fn transcript_path(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> PathBuf {
        self.transcript_dir(config_dir, cwd)
            .join(format!("{session_id}.jsonl"))
    }

    /// The durable close/remove marker, beside the transcript. Pure path derivation, no I/O.
    fn archived_marker_path(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> PathBuf {
        self.transcript_dir(config_dir, cwd)
            .join(format!("{session_id}.archived"))
    }

    /// The latest `{"type":"ai-title","aiTitle":"…"}` record in a JSONL transcript, if any.
    ///
    /// Latest wins — the title grows and changes with the conversation. Best-effort: blank and
    /// unparseable lines are skipped, and an empty `aiTitle` is ignored.
    fn parse_title(&self, transcript: &str) -> Option<String> {
        let mut latest = None;
        for line in transcript.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value.get("type").and_then(|t| t.as_str()) != Some("ai-title") {
                continue;
            }
            if let Some(title) = value.get("aiTitle").and_then(|t| t.as_str()) {
                if !title.is_empty() {
                    latest = Some(title.to_string());
                }
            }
        }
        latest
    }
}

impl AiCliProvider for ClaudeProvider {
    fn id(&self) -> AiCli {
        AiCli::ClaudeCode
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn command(&self) -> &'static str {
        "claude"
    }

    fn is_available(&self) -> bool {
        resolves_on_path(self.command())
    }

    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String> {
        let id = session_id.to_string();
        match mode {
            // The app owns the id up front (research R6).
            LaunchMode::Fresh => vec!["--session-id".to_string(), id],
            // Resume a specific session — never `--continue` (claude-cli.md).
            LaunchMode::Resume => vec!["--resume".to_string(), id],
        }
    }

    fn config_dir(&self) -> Option<PathBuf> {
        if let Ok(dir) = std::env::var(Self::CONFIG_DIR_ENV) {
            if !dir.is_empty() {
                return Some(PathBuf::from(dir));
            }
        }
        directories::UserDirs::new().map(|d| d.home_dir().join(".claude"))
    }

    fn recorded_session_ids(&self, config_dir: &Path, cwd: &Path) -> Vec<Uuid> {
        // List the per-cwd transcript directory and parse each `*.jsonl` filename as an id.
        // Best-effort: a missing or unreadable directory, or an unparseable filename, simply
        // contributes nothing. The `.jsonl` filter is also what excludes this provider's own
        // `<uuid>.archived` markers, which live in the same directory.
        let Ok(entries) = std::fs::read_dir(self.transcript_dir(config_dir, cwd)) else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                    return None;
                }
                path.file_stem()?.to_str()?.parse::<Uuid>().ok()
            })
            .collect()
    }

    fn has_recorded_conversation(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool {
        self.transcript_path(config_dir, cwd, session_id).exists()
    }

    fn read_title(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String> {
        let contents =
            std::fs::read_to_string(self.transcript_path(config_dir, cwd, session_id)).ok()?;
        self.parse_title(&contents)
    }

    fn mark_archived(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> io::Result<()> {
        let path = self.archived_marker_path(config_dir, cwd, session_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, "")
    }

    fn is_archived(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool {
        self.archived_marker_path(config_dir, cwd, session_id)
            .exists()
    }

    fn activity_source(
        &self,
        _config_dir: &Path,
        _cwd: &Path,
        _session_id: Uuid,
    ) -> ActivitySource {
        // Feature 010's mechanism, preserved byte-for-byte precisely *because* the variant carries
        // nothing: the daemon keeps choosing, writing and passing the hook settings file exactly
        // as it does today, and this only names which mechanism applies.
        ActivitySource::Hooks
    }
}

// ---------------------------------------------------------------------------------------
// GitHub Copilot
// ---------------------------------------------------------------------------------------

/// GitHub's `copilot` CLI. Verified against 1.0.62 and re-verified against 1.0.80 (research
/// R1–R6, R12). See `specs/026-multi-provider-sessions/contracts/copilot-cli.md`.
#[derive(Debug, Clone, Copy, Default)]
pub struct CopilotProvider;

impl CopilotProvider {
    /// Environment variable relocating the whole `~/.copilot` store (verified: no leakage).
    const CONFIG_DIR_ENV: &'static str = "COPILOT_HOME";

    /// `<base>/sidebar-sessions-state/<sha256_hex(cwd)>.json` — the per-working-directory index of
    /// session ids Copilot maintains for its own picker. Pure path derivation, no I/O.
    fn index_path(&self, config_dir: &Path, cwd: &Path) -> PathBuf {
        let hashed = crate::protocol::hashing::sha256_hex(cwd.to_string_lossy().as_bytes());
        config_dir
            .join("sidebar-sessions-state")
            .join(format!("{hashed}.json"))
    }

    /// `<base>/session-state/<uuid>/` — one conversation's own directory. Pure, no I/O.
    fn session_dir(&self, config_dir: &Path, session_id: Uuid) -> PathBuf {
        config_dir
            .join("session-state")
            .join(session_id.to_string())
    }

    /// `<base>/session-state/<uuid>/events.jsonl` — the append-only turn log. Created lazily on
    /// the first user message, so its *existence* is what "a conversation was recorded" means.
    fn events_path(&self, config_dir: &Path, session_id: Uuid) -> PathBuf {
        self.session_dir(config_dir, session_id)
            .join("events.jsonl")
    }

    /// `<base>/session-state/<uuid>/workspace.yaml` — the session's own metadata, of which exactly
    /// one key is ever read. Pure, no I/O.
    fn workspace_path(&self, config_dir: &Path, session_id: Uuid) -> PathBuf {
        self.session_dir(config_dir, session_id)
            .join("workspace.yaml")
    }

    /// The value of a top-level `key:` scalar in a small YAML document, or `None`.
    ///
    /// Purpose-built rather than a YAML dependency (research R4), and it handles exactly the three
    /// forms Copilot writes: plain, single-quoted and double-quoted. The quoted forms are not
    /// decoration — a summarised title routinely contains a colon (`name: 'Reply with the single
    /// word: hello'` is a captured fixture), and a naive `split(':')` reader truncates it at the
    /// first one while looking correct on every title that has none.
    ///
    /// Deliberately narrow: no block scalars, no anchors, no escapes beyond stripping the quotes.
    /// Anything it cannot read yields `None`, which is a label that stays `Pending` — never an
    /// error, and never a wrong title (FR-017).
    fn read_yaml_scalar(contents: &str, key: &str) -> Option<String> {
        for line in contents.lines() {
            // Top-level keys only: an indented `name:` belongs to some nested mapping we are not
            // reading, and taking it would attribute another object's value to the session.
            if line.starts_with(char::is_whitespace) {
                continue;
            }
            let Some(rest) = line.strip_prefix(key) else {
                continue;
            };
            let Some(value) = rest.strip_prefix(':') else {
                continue;
            };
            let value = value.trim();
            let unquoted = match (value.chars().next(), value.chars().last()) {
                (Some('\''), Some('\'')) | (Some('"'), Some('"')) if value.len() >= 2 => {
                    &value[1..value.len() - 1]
                }
                _ => value,
            };
            if unquoted.is_empty() {
                return None;
            }
            return Some(unquoted.to_string());
        }
        None
    }

    /// `<base>/session-state/<uuid>/micold.archived` — our sentinel in Copilot's storage.
    ///
    /// Safe *inside* the session directory because discovery reads the index file, not a directory
    /// listing, so this can never be misread as a session — `claude`'s `.jsonl`-extension filter
    /// has no counterpart to need here.
    fn archived_marker_path(&self, config_dir: &Path, session_id: Uuid) -> PathBuf {
        self.session_dir(config_dir, session_id)
            .join("micold.archived")
    }
}

impl AiCliProvider for CopilotProvider {
    fn id(&self) -> AiCli {
        AiCli::Copilot
    }

    fn display_name(&self) -> &'static str {
        "GitHub Copilot"
    }

    fn command(&self) -> &'static str {
        "copilot"
    }

    fn is_available(&self) -> bool {
        resolves_on_path(self.command())
    }

    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String> {
        let id = session_id.to_string();
        // `--no-remote` is deliberate, not incidental: without it Copilot enables remote session
        // access, and a session this application spawned on the user's behalf must not be remotely
        // steerable absent an explicit opt-in (Principle IV, research R12). It is a per-launch
        // flag, so no user configuration is modified (FR-011).
        //
        // `--allow-all-tools` is **not** passed: the session is interactive and the user answers
        // Copilot's permission prompts in its own terminal, exactly as they answer `claude`'s.
        match mode {
            LaunchMode::Fresh => vec!["--session-id".to_string(), id, "--no-remote".to_string()],
            // `--resume=<uuid>`, one argument: bare `--resume` opens Copilot's interactive picker,
            // and the application always targets a specific id.
            LaunchMode::Resume => vec![format!("--resume={id}"), "--no-remote".to_string()],
        }
    }

    fn config_dir(&self) -> Option<PathBuf> {
        // Home-relative on every platform, including Windows, where it is `%USERPROFILE%\.copilot`
        // — verified against the shipped CLI rather than assumed (T081). Copilot's own resolver
        // takes the home directory as an argument and neither the platform nor `%LOCALAPPDATA%`,
        // while its *cache* home right beside it takes all three; every `.copilot` literal in the
        // CLI joins it to `homedir()` with no branch. Nothing here needs a `cfg!(windows)` arm.
        if let Ok(dir) = std::env::var(Self::CONFIG_DIR_ENV) {
            if !dir.is_empty() {
                return Some(PathBuf::from(dir));
            }
        }
        directories::UserDirs::new().map(|d| d.home_dir().join(".copilot"))
    }

    fn recorded_session_ids(&self, config_dir: &Path, cwd: &Path) -> Vec<Uuid> {
        // Copilot's per-working-directory index — the same file its own session picker reads, so it
        // is as authoritative as anything on disk. One file read per location, against a scan of
        // every session directory ever created (253 on the development machine, a number that only
        // grows); the scan is the documented recovery path, never the primary route (research R3).
        //
        // Best-effort at every step: a missing, unreadable, truncated or empty file contributes
        // nothing and never an error, so a project open cannot fail on another vendor's file.
        let Ok(bytes) = std::fs::read(self.index_path(config_dir, cwd)) else {
            return Vec::new();
        };
        let Ok(index) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return Vec::new();
        };
        // `schemaVersion` is Copilot's, not ours. Anything other than `1` is treated as unreadable
        // rather than parsed hopefully — a format we have not seen is not a format we can read.
        if index.get("schemaVersion").and_then(|v| v.as_u64()) != Some(1) {
            return Vec::new();
        }
        index
            .get("sessionIds")
            .and_then(|ids| ids.as_array())
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str()?.parse::<Uuid>().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn has_recorded_conversation(&self, config_dir: &Path, _cwd: &Path, session_id: Uuid) -> bool {
        // `events.jsonl` is created lazily on the **first user message**, not at session start, so
        // its existence is exactly "a conversation was recorded". A session directory without it
        // was opened and never used (contract, "Recorded-conversation detection").
        //
        // Note `cwd` is unused: Copilot keys conversations by session id alone, and the working
        // directory only ever enters through the per-cwd *index*. The parameter stays because the
        // seam is shared — `claude` needs it, and a signature that varied by provider would not be
        // a seam.
        self.events_path(config_dir, session_id).exists()
    }

    fn read_title(&self, config_dir: &Path, _cwd: &Path, session_id: Uuid) -> Option<String> {
        // `name:` in the session's own `workspace.yaml`, absent until Copilot has summarised the
        // conversation and updated as it grows — the same lifecycle as `claude`'s `ai-title`, so
        // the same `Pending → Named` transition applies.
        //
        // Read with a purpose-built single-scalar reader rather than a YAML crate (research R4).
        // Exactly one key is ever wanted from this file: `cwd` is already known from the index, and
        // `git_root`/`repository`/`branch` are not used. A dozen lines against a new dependency
        // tree is not a close call.
        let contents = std::fs::read_to_string(self.workspace_path(config_dir, session_id)).ok()?;
        Self::read_yaml_scalar(&contents, "name")
    }

    fn mark_archived(&self, config_dir: &Path, _cwd: &Path, session_id: Uuid) -> io::Result<()> {
        let path = self.archived_marker_path(config_dir, session_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, "")
    }

    fn is_archived(&self, config_dir: &Path, _cwd: &Path, session_id: Uuid) -> bool {
        self.archived_marker_path(config_dir, session_id).exists()
    }

    fn activity_source(&self, config_dir: &Path, _cwd: &Path, session_id: Uuid) -> ActivitySource {
        // A path, unlike `claude`'s payload-free `Hooks`, because this one *is* arithmetic:
        // `<base>/session-state/<uuid>/events.jsonl`. The asymmetry is the point of the enum —
        // see [`ActivitySource::Hooks`] for why the other arm cannot carry one.
        //
        // Naming a source commits the daemon to nothing. It opens one only for a session it is
        // supervising; a session merely discovered under FR-014 is never watched (SC-006).
        ActivitySource::EventLog {
            path: self.events_path(config_dir, session_id),
        }
    }
}

// ---------------------------------------------------------------------------------------
// In-memory fake for unit tests. Public (not `#[cfg(test)]`) so integration tests in
// `tests/` can share it, matching `FakeGit` (FR-019, feature 021 T048). Pure — no process,
// no conversation record on disk.
// ---------------------------------------------------------------------------------------

use std::cell::RefCell;
use std::collections::BTreeMap;

/// An in-memory [`AiCliProvider`] for tests: canned paths and titles, and a record of the
/// launches it was asked for.
///
/// The point of faking this one is not the path arithmetic — the real providers' own tests cover
/// that — but everything *above* the seam: that session wiring asks the provider rather than
/// hardcoding one CLI, and asks it with the right id and mode. So the fake answers with a
/// distinctive command name and logs every `launch_args` call.
///
/// Since feature 026 the trait has **no defaults**, so this type implements all twelve methods.
/// That is deliberate and not merely mechanical: the four it used to inherit
/// (`has_recorded_conversation`, `read_title`, `mark_archived`, `is_archived`) all reached the real
/// filesystem, which is the one thing a fake exists to avoid — a fake that inherited them would
/// answer plausibly while quietly making a syscall.
///
/// `RefCell`, unlike the store fakes: no consumer boxes a provider across threads.
#[derive(Debug, Default)]
pub struct FakeAiCliProvider {
    inner: RefCell<FakeProviderState>,
}

#[derive(Debug, Default)]
struct FakeProviderState {
    /// Which provider this fake claims to be.
    id: AiCli,
    /// The configured base directory, or `None` for "cannot be determined".
    config_dir: Option<PathBuf>,
    /// Whether `is_available` answers true.
    available: bool,
    /// Titles keyed by conversation contents, consulted by `read_title`.
    titles: BTreeMap<String, String>,
    /// Conversation contents by `(cwd, id)` — what the fake has "on disk".
    conversations: BTreeMap<(PathBuf, Uuid), String>,
    /// Sessions marked archived, by `(cwd, id)`.
    archived: RefCell<Vec<(PathBuf, Uuid)>>,
    /// Every `(session_id, mode)` passed to `launch_args`, in call order.
    launches: Vec<(Uuid, LaunchMode)>,
}

impl FakeAiCliProvider {
    /// A provider with no config directory and no recorded conversations, claiming to be
    /// [`AiCli::ClaudeCode`] and to be installed.
    pub fn new() -> Self {
        let fake = Self::default();
        fake.inner.borrow_mut().available = true;
        fake
    }

    /// Which provider this fake answers [`AiCliProvider::id`] with — the field the mixed-provider
    /// tests key on.
    pub fn with_id(self, id: AiCli) -> Self {
        self.inner.borrow_mut().id = id;
        self
    }

    /// The directory `config_dir` reports. `None` is the "uncertain" answer, which several
    /// consumers treat specially and only a mixed test can catch them getting wrong.
    pub fn with_config_dir(self, dir: impl Into<PathBuf>) -> Self {
        self.inner.borrow_mut().config_dir = Some(dir.into());
        self
    }

    /// Make `config_dir()` answer `None` — "cannot be determined".
    pub fn with_no_config_dir(self) -> Self {
        self.inner.borrow_mut().config_dir = None;
        self
    }

    /// What `is_available()` answers.
    pub fn with_availability(self, available: bool) -> Self {
        self.inner.borrow_mut().available = available;
        self
    }

    /// Make `read_title` answer `title` for exactly these conversation contents.
    pub fn with_title(self, conversation: &str, title: &str) -> Self {
        self.inner
            .borrow_mut()
            .titles
            .insert(conversation.to_string(), title.to_string());
        self
    }

    /// Record a conversation for `(cwd, id)` — what [`AiCliProvider::has_recorded_conversation`],
    /// `read_title` and `recorded_session_ids` would find.
    pub fn with_conversation(self, cwd: impl Into<PathBuf>, id: Uuid, contents: &str) -> Self {
        self.inner
            .borrow_mut()
            .conversations
            .insert((cwd.into(), id), contents.to_string());
        self
    }

    /// Every `(session_id, mode)` passed to `launch_args`, in call order.
    pub fn launches(&self) -> Vec<(Uuid, LaunchMode)> {
        self.inner.borrow().launches.clone()
    }
}

impl AiCliProvider for FakeAiCliProvider {
    fn id(&self) -> AiCli {
        self.inner.borrow().id
    }

    fn display_name(&self) -> &'static str {
        "Fake AI CLI"
    }

    fn command(&self) -> &'static str {
        // Deliberately not `claude`: a caller that hardcoded the real name would still pass a
        // test that asserted the real name, and this is the difference.
        "fake-ai-cli"
    }

    fn is_available(&self) -> bool {
        self.inner.borrow().available
    }

    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String> {
        self.inner.borrow_mut().launches.push((session_id, mode));
        let flag = match mode {
            LaunchMode::Fresh => "--fresh",
            LaunchMode::Resume => "--resume",
        };
        vec![flag.to_string(), session_id.to_string()]
    }

    fn config_dir(&self) -> Option<PathBuf> {
        self.inner.borrow().config_dir.clone()
    }

    fn recorded_session_ids(&self, _config_dir: &Path, cwd: &Path) -> Vec<Uuid> {
        self.inner
            .borrow()
            .conversations
            .keys()
            .filter(|(recorded, _)| recorded == cwd)
            .map(|(_, id)| *id)
            .collect()
    }

    fn has_recorded_conversation(&self, _config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool {
        self.inner
            .borrow()
            .conversations
            .contains_key(&(cwd.to_path_buf(), session_id))
    }

    fn read_title(&self, _config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String> {
        let inner = self.inner.borrow();
        let contents = inner.conversations.get(&(cwd.to_path_buf(), session_id))?;
        inner.titles.get(contents).cloned()
    }

    fn mark_archived(&self, _config_dir: &Path, cwd: &Path, session_id: Uuid) -> io::Result<()> {
        self.inner
            .borrow()
            .archived
            .borrow_mut()
            .push((cwd.to_path_buf(), session_id));
        Ok(())
    }

    fn is_archived(&self, _config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool {
        self.inner
            .borrow()
            .archived
            .borrow()
            .contains(&(cwd.to_path_buf(), session_id))
    }

    fn activity_source(
        &self,
        _config_dir: &Path,
        _cwd: &Path,
        _session_id: Uuid,
    ) -> ActivitySource {
        ActivitySource::None
    }
}
