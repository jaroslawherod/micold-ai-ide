//! Agent runs recorded against an origin session (feature 552, data-model.md).
//!
//! A run is reported by the AI CLI's own lifecycle signal (for Claude Code, the `SubagentStart` and
//! `SubagentStop` hooks), never inferred from a name. The list lives on the origin session, is keyed
//! by the provider's agent id within it, and is bounded.

use serde::{Deserialize, Serialize};

use crate::session::AiCli;

/// The most runs kept per origin session; the oldest stopped run is dropped first (FR-005).
pub const MAX_RUNS_PER_ORIGIN: usize = 256;

/// Whether a run is still going.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRunState {
    /// Started, and no stop report has arrived.
    Running,
    /// Stopped, or recorded from a stop report alone, or found running when its origin ended.
    Stopped,
}

/// One agent run of an origin session. Also the wire form (`AgentRunSummary`), which is postcard
/// on the real wire: no field may be skipped when serialising.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRun {
    /// The provider's id for the run; the key within one origin.
    pub agent_id: String,
    /// The kind of agent (for example `Explore`); display only.
    #[serde(default)]
    pub agent_type: Option<String>,
    /// The run's transcript path, opaque, from its stop report.
    #[serde(default)]
    pub transcript: Option<String>,
    /// Whether the run is still going.
    pub state: AgentRunState,
    /// Which AI CLI reported it.
    pub provider: AiCli,
}

/// A provider-native report about one run, already parsed out of the provider's payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSignal {
    /// A run started.
    Started {
        /// Which AI CLI reported it.
        provider: AiCli,
        /// The provider's id for the run.
        agent_id: String,
        /// The kind of agent, when reported.
        agent_type: Option<String>,
    },
    /// A run stopped.
    Stopped {
        /// Which AI CLI reported it.
        provider: AiCli,
        /// The provider's id for the run.
        agent_id: String,
        /// The kind of agent, when reported.
        agent_type: Option<String>,
        /// The run's transcript path, when reported.
        transcript: Option<String>,
    },
}

/// Apply `signal` to an origin's runs; `true` when the list changed.
///
/// A repeated start changes nothing; a stop sets `Stopped` and fills the transcript; a stop with no
/// earlier start inserts a `Stopped` run, and a start arriving after it leaves it stopped.
pub fn apply_signal(runs: &mut Vec<AgentRun>, signal: AgentSignal) -> bool {
    let (provider, agent_id, agent_type, stopped, transcript) = match signal {
        AgentSignal::Started {
            provider,
            agent_id,
            agent_type,
        } => (provider, agent_id, agent_type, false, None),
        AgentSignal::Stopped {
            provider,
            agent_id,
            agent_type,
            transcript,
        } => (provider, agent_id, agent_type, true, transcript),
    };
    if let Some(run) = runs.iter_mut().find(|r| r.agent_id == agent_id) {
        let before = run.clone();
        if run.agent_type.is_none() {
            run.agent_type = agent_type;
        }
        if stopped {
            run.state = AgentRunState::Stopped;
            if transcript.is_some() {
                run.transcript = transcript;
            }
        }
        return *run != before;
    }
    runs.push(AgentRun {
        agent_id,
        agent_type,
        transcript,
        state: if stopped {
            AgentRunState::Stopped
        } else {
            AgentRunState::Running
        },
        provider,
    });
    if runs.len() > MAX_RUNS_PER_ORIGIN {
        let victim = runs
            .iter()
            .position(|r| r.state == AgentRunState::Stopped)
            .unwrap_or(0);
        runs.remove(victim);
    }
    true
}

/// Mark every `Running` run `Stopped`: its stop report can no longer arrive (the origin exited, or
/// the daemon restarted). `true` when any changed.
pub fn stop_running(runs: &mut [AgentRun]) -> bool {
    let mut changed = false;
    for run in runs
        .iter_mut()
        .filter(|r| r.state == AgentRunState::Running)
    {
        run.state = AgentRunState::Stopped;
        changed = true;
    }
    changed
}
