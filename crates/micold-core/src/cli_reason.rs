//! Why a session would not find an AI CLI, and what would change that (feature 037).
//!
//! An AI CLI is "missing" when the `PATH` a session gets in one directory does not hold it. That
//! `PATH` is the login `PATH` plus whatever the environment-include script adds, so "not found"
//! has six different causes with six different remedies, and only the last of them is "it is not
//! installed". This module names the state ([`SpawnEnv`], research R2) and writes every sentence
//! said about it ([`explain`], research R4), so the Settings note, a refused start and a row's
//! list cannot give different reasons for one answer (FR-012).
//!
//! Pure: no I/O, no clock, no settings read. The service classifies the attempt it already made
//! (FR-014) and the client turns the state it was sent into words.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::env_include::EnvIncludeOutcome;
use crate::session::AiCli;
use crate::terminal::LaunchMode;

/// The label of the environment-include checkbox, as the Settings page shows it.
///
/// A sentence that tells the user to change a setting quotes these constants, and the page renders
/// its labels from them, so a sentence cannot name a label the page does not show (FR-003).
pub const LABEL_ENABLED: &str = "Source a script before each session";
/// The label of the script path field.
pub const LABEL_SCRIPT_PATH: &str = "Script path";
/// The label of the timeout field.
pub const LABEL_TIMEOUT: &str = "Timeout";

/// The state of the environment a session in one directory gets. One variant per row of FR-001.
///
/// No payload: the script's output is never carried (FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnEnv {
    /// Environment-include is off: sessions get only the login `PATH`.
    IncludeOff,
    /// On, and the script path is blank: no script is sourced.
    NoScriptPath,
    /// On, and nothing exists at the script path.
    ScriptNotFound,
    /// On, and the last attempt exited with an error. This is also where a path that exists and
    /// cannot be sourced falls (a directory, a file the user may not read): it is attempted and
    /// fails (FR-001's note).
    ScriptFailed,
    /// On, and the last attempt ran past the timeout.
    ScriptTimedOut,
    /// On, and the last attempt succeeded: the session's `PATH` is the login `PATH` plus what the
    /// script adds.
    Applied,
}

impl SpawnEnv {
    /// The state for these settings and this attempt. `None` when the settings call for an attempt
    /// and none is known.
    ///
    /// The order is the service's own: `enabled` first, then the blank path, then the attempt.
    pub fn classify(
        enabled: bool,
        script_path: &str,
        attempt: Option<&EnvIncludeOutcome>,
    ) -> Option<SpawnEnv> {
        if !enabled {
            return Some(SpawnEnv::IncludeOff);
        }
        if script_path.trim().is_empty() {
            return Some(SpawnEnv::NoScriptPath);
        }
        match attempt? {
            EnvIncludeOutcome::Disabled => None,
            EnvIncludeOutcome::MissingScript => Some(SpawnEnv::ScriptNotFound),
            EnvIncludeOutcome::NonZeroExit { .. } => Some(SpawnEnv::ScriptFailed),
            EnvIncludeOutcome::TimedOut { .. } => Some(SpawnEnv::ScriptTimedOut),
            EnvIncludeOutcome::Success => Some(SpawnEnv::Applied),
        }
    }

    /// Whether the script's `PATH` additions reached the session. Only then is "the CLI is not
    /// there" a true reason (FR-002).
    pub fn script_applied(self) -> bool {
        self == SpawnEnv::Applied
    }
}

/// Where sessions run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place<'a> {
    /// The service runs on this computer.
    ThisComputer,
    /// The service runs in a container built from this image reference.
    Image(&'a str),
}

/// The directory an attempt was made for (FR-004a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptDir<'a> {
    /// The user's home directory.
    Home,
    /// A project's or a session's own directory.
    Dir(&'a Path),
}

/// One reason and the action that would change it (the spec's *Unavailability reason*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// Why a session would not find the CLIs.
    pub reason: String,
    /// What the user can change.
    pub action: String,
}

/// "Claude Code", "Claude Code and GitHub Copilot", "A, B and C". `None` for an empty list.
///
/// Written out because it goes in a sentence, where a comma-separated list reads as a field value.
pub fn name_list(clis: &[AiCli]) -> Option<String> {
    let (last, rest) = clis.split_last()?;
    if rest.is_empty() {
        return Some(last.to_string());
    }
    let leading: Vec<String> = rest.iter().map(ToString::to_string).collect();
    Some(format!("{} and {last}", leading.join(", ")))
}

/// The reason `missing` would not be found in a session and the action for it. `None` when
/// nothing is missing.
pub fn explain(
    missing: &[AiCli],
    env: SpawnEnv,
    place: Place<'_>,
    dir: AttemptDir<'_>,
) -> Option<Explanation> {
    let names = name_list(missing)?;
    let several = missing.len() > 1;
    let them = if several { "them" } else { "it" };
    let (lead, path, install) = match place {
        Place::ThisComputer => (
            format!("A session would not find {names}:"),
            "the login PATH",
            format!("install {them} on the login PATH"),
        ),
        Place::Image(image) => (
            format!("A session in {image} would not find {names}:"),
            "the image's PATH",
            format!("use an image that puts {them} on its PATH"),
        ),
    };
    let dir = match dir {
        AttemptDir::Home => "your home directory".to_string(),
        AttemptDir::Dir(dir) => dir.display().to_string(),
    };
    let not_applied = |what: &str| {
        format!(
            "{lead} the startup script {what} for {dir}, so its PATH additions are not applied."
        )
    };
    let (reason, action) = match (env, place) {
        (SpawnEnv::IncludeOff, _) => (
            format!("{lead} sessions get only {path}, because \"{LABEL_ENABLED}\" is off."),
            format!("Turn it on if your startup file puts {them} on the PATH, or {install}."),
        ),
        (SpawnEnv::NoScriptPath, _) => (
            format!("{lead} no script is sourced, because \"{LABEL_SCRIPT_PATH}\" is empty."),
            format!(
                "Set \"{LABEL_SCRIPT_PATH}\" if a startup file puts {them} on the PATH, or \
                 {install}."
            ),
        ),
        (SpawnEnv::ScriptNotFound, _) => (
            not_applied("was not found"),
            format!("Correct \"{LABEL_SCRIPT_PATH}\"."),
        ),
        (SpawnEnv::ScriptFailed, _) => (
            not_applied("exited with an error"),
            format!("Fix the script named in \"{LABEL_SCRIPT_PATH}\"."),
        ),
        (SpawnEnv::ScriptTimedOut, _) => (
            not_applied("timed out"),
            format!(
                "Fix the script named in \"{LABEL_SCRIPT_PATH}\", or raise \"{LABEL_TIMEOUT}\"."
            ),
        ),
        (SpawnEnv::Applied, Place::ThisComputer) => {
            let (was, its_directory) = if several {
                ("were", "their directories")
            } else {
                ("was", "its directory")
            };
            (
                format!(
                    "{names} {was} not found on the PATH sessions get for {dir}: the login PATH \
                     plus what the startup script adds."
                ),
                format!("Install {them}, or make the script add {its_directory}."),
            )
        }
        // 027 FR-023b's sentence, kept byte for byte (FR-005). It names the image and no directory.
        (SpawnEnv::Applied, Place::Image(image)) => {
            let verb = if several { "aren't" } else { "isn't" };
            (
                format!("{names} {verb} in {image}."),
                "Sessions run in that image, so it has to provide any AI CLI you want to use."
                    .to_string(),
            )
        }
    };
    Some(Explanation { reason, action })
}

/// What a start refused for a missing AI CLI tells the user: [`explain`]'s reason and action for
/// `dir`, then what this start can do about it (contract W3).
///
/// A fresh start may go to another CLI. A resume continues a conversation only its own CLI holds,
/// so it is told to restart and never offered another one (FR-009).
pub fn start_refusal(
    cli: AiCli,
    env: SpawnEnv,
    place: Place<'_>,
    dir: AttemptDir<'_>,
    launch: LaunchMode,
) -> String {
    // 027's sentences for an image that lacks the CLI, kept byte for byte (Story 2 scenario 4).
    if let (SpawnEnv::Applied, Place::Image(image)) = (env, place) {
        return match launch {
            LaunchMode::Fresh => format!(
                "{cli} isn't in {image}, where sessions run. Choose an image that provides it, or \
                 start this session on another AI CLI."
            ),
            LaunchMode::Resume => format!(
                "{cli} isn't in {image}, where sessions run, and this conversation can only \
                 continue in it. Choose an image that provides it, then restart this session."
            ),
        };
    }
    let Some(Explanation { reason, action }) = explain(&[cli], env, place, dir) else {
        unreachable!("one CLI is missing, so there is an explanation");
    };
    match launch {
        LaunchMode::Fresh => format!("{reason} {action} Or start this session on another AI CLI."),
        LaunchMode::Resume => format!(
            "{reason} {action} Then restart this session: its conversation can only continue in \
             {cli}."
        ),
    }
}

/// What a refused start says when the answer in use carries no state (contract W5, research R8).
///
/// The cause is not known, so none is claimed (FR-002): the sentence says only what the answer
/// does, that a session would not find the CLI.
pub fn start_refusal_unknown(cli: AiCli) -> String {
    format!("{cli} would not be found by a session here. Start this session on another AI CLI.")
}
