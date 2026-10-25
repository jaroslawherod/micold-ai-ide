//! A project's setup command and the record of one run of it (feature 1289, data-model
//! §`micold_core::setup`).
//!
//! The daemon runs a project's setup command in each new worktree (FR-010). This module holds the
//! pure parts: the command itself, a run's record and its transitions, the words the app shows for
//! each state (contract U2) and the reason line that ends a run's output (SC-004). The output
//! capture is [`output`], the choice of shell is [`shell`].

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod output;
pub mod shell;

/// A non-empty shell command line, trimmed of surrounding whitespace. Inner newlines are kept, so
/// a multi-line script stays one command. Not a secret: `Debug` shows the full text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetupCommand(String);

impl SetupCommand {
    /// The command in `text`, trimmed; `None` when nothing but whitespace is left (the setting is
    /// then unset).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_owned()))
    }

    /// The command line as the shell receives it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for SetupCommand {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// A plain string, parsed: a blank one is refused, so no record or setting holds an empty command.
impl<'de> Deserialize<'de> for SetupCommand {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| serde::de::Error::custom("a setup command is never blank"))
    }
}

/// One setup run's id, unique across the whole daemon (data-model §RunId).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(pub u64);

/// Why a run failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// A non-zero exit status.
    Exit(i32),
    /// Ended by a signal that **Stop setup** did not send (Unix).
    Signal(i32),
    /// The shell could not be spawned, with the OS reason, or the folder is gone.
    NotStarted(String),
}

/// Where a run is. Every state but `Running` is an end; an ended run never changes in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupState {
    Running,
    Succeeded,
    Failed(Failure),
    Stopped,
    Interrupted,
}

impl SetupState {
    /// The row tag's text (contract U2).
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Running => "Setting up…",
            Self::Succeeded => "Set up",
            Self::Failed(_) => "Setup failed",
            Self::Stopped => "Setup stopped",
            Self::Interrupted => "Setup interrupted",
        }
    }

    /// The line that ends an ended run's output (SC-004); `None` while it runs.
    #[must_use]
    pub fn reason_line(&self) -> Option<String> {
        Some(match self {
            Self::Running => return None,
            Self::Succeeded => "Exited with status 0.".to_owned(),
            Self::Failed(Failure::Exit(code)) => format!("Exited with status {code}."),
            Self::Failed(Failure::Signal(signal)) => format!("Ended by signal {signal}."),
            Self::Failed(Failure::NotStarted(reason)) => format!("Could not start: {reason}."),
            Self::Stopped => "Stopped.".to_owned(),
            Self::Interrupted => "Interrupted: the daemon stopped.".to_owned(),
        })
    }

    /// Whether the run has ended.
    #[must_use]
    pub fn is_ended(&self) -> bool {
        !matches!(self, Self::Running)
    }
}

/// [`SetupState`] as a JSON record holds it: `{"state":"failed","failure":{"exit":2}}` (contract
/// C2). An internally tagged enum needs a self-describing format, which the wire's postcard is
/// not, so the wire uses [`PlainState`] instead.
#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum TaggedState {
    Running,
    Succeeded,
    Failed { failure: Failure },
    Stopped,
    Interrupted,
}

/// [`SetupState`] in a compact format (postcard on the wire).
#[derive(Serialize, Deserialize)]
enum PlainState {
    Running,
    Succeeded,
    Failed(Failure),
    Stopped,
    Interrupted,
}

impl Serialize for SetupState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            match self {
                Self::Running => TaggedState::Running,
                Self::Succeeded => TaggedState::Succeeded,
                Self::Failed(failure) => TaggedState::Failed {
                    failure: failure.clone(),
                },
                Self::Stopped => TaggedState::Stopped,
                Self::Interrupted => TaggedState::Interrupted,
            }
            .serialize(serializer)
        } else {
            match self {
                Self::Running => PlainState::Running,
                Self::Succeeded => PlainState::Succeeded,
                Self::Failed(failure) => PlainState::Failed(failure.clone()),
                Self::Stopped => PlainState::Stopped,
                Self::Interrupted => PlainState::Interrupted,
            }
            .serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for SetupState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(if deserializer.is_human_readable() {
            match TaggedState::deserialize(deserializer)? {
                TaggedState::Running => Self::Running,
                TaggedState::Succeeded => Self::Succeeded,
                TaggedState::Failed { failure } => Self::Failed(failure),
                TaggedState::Stopped => Self::Stopped,
                TaggedState::Interrupted => Self::Interrupted,
            }
        } else {
            match PlainState::deserialize(deserializer)? {
                PlainState::Running => Self::Running,
                PlainState::Succeeded => Self::Succeeded,
                PlainState::Failed(failure) => Self::Failed(failure),
                PlainState::Stopped => Self::Stopped,
                PlainState::Interrupted => Self::Interrupted,
            }
        })
    }
}

/// One run's record, as persisted and as summarised on the wire. `ended_at` is `Some` exactly when
/// the state is not `Running`: [`SetupRun::start`] and [`SetupRun::finish`] are the only ways to
/// make one, and a record that breaks the rule does not deserialize.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawRun")]
pub struct SetupRun {
    id: RunId,
    /// The text that ran, kept so the view shows what ran after the setting changes.
    command: SetupCommand,
    state: SetupState,
    /// Seconds since the epoch (`unix_time::unix_secs`).
    started_at: u64,
    ended_at: Option<u64>,
}

/// [`SetupRun`] before its rule is checked.
#[derive(Deserialize)]
struct RawRun {
    id: RunId,
    command: SetupCommand,
    state: SetupState,
    started_at: u64,
    ended_at: Option<u64>,
}

impl TryFrom<RawRun> for SetupRun {
    type Error = &'static str;

    fn try_from(raw: RawRun) -> Result<Self, Self::Error> {
        if raw.state.is_ended() != raw.ended_at.is_some() {
            return Err("a setup run has an end time exactly when it has ended");
        }
        Ok(Self {
            id: raw.id,
            command: raw.command,
            state: raw.state,
            started_at: raw.started_at,
            ended_at: raw.ended_at,
        })
    }
}

/// A [`SetupRun::finish`] that the transitions do not allow; the run comes back unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub run: SetupRun,
    pub to: SetupState,
}

impl SetupRun {
    /// A new run of `command`, running since `now`.
    #[must_use]
    pub fn start(id: RunId, command: SetupCommand, now: u64) -> Self {
        Self {
            id,
            command,
            state: SetupState::Running,
            started_at: now,
            ended_at: None,
        }
    }

    /// End a running run in `to` at `now`. Refused unless the run is `Running` and `to` is an end:
    /// an ended run never changes in place, the next run replaces its record.
    pub fn finish(self, to: SetupState, now: u64) -> Result<Self, Refused> {
        if self.state.is_ended() || !to.is_ended() {
            return Err(Refused { run: self, to });
        }
        Ok(Self {
            state: to,
            ended_at: Some(now),
            ..self
        })
    }

    #[must_use]
    pub fn id(&self) -> RunId {
        self.id
    }

    #[must_use]
    pub fn command(&self) -> &SetupCommand {
        &self.command
    }

    #[must_use]
    pub fn state(&self) -> &SetupState {
        &self.state
    }

    #[must_use]
    pub fn started_at(&self) -> u64 {
        self.started_at
    }

    #[must_use]
    pub fn ended_at(&self) -> Option<u64> {
        self.ended_at
    }
}

/// A run as the wire summarises it: the record plus how much output it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupRunSummary {
    pub run: SetupRun,
    /// The index one past the last output line kept.
    pub output_lines: u64,
}

/// What asking for a run got (contract C4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupStart {
    /// A run started with this id.
    Started(RunId),
    /// The project has no setup command.
    NotConfigured,
    /// A run is already going in that worktree (FR-023a).
    Busy,
}
