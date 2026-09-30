//! The tool catalog and argument validation (feature 034, contracts/mcp-tools.md).
//!
//! `tools/list` answers [`list_result`]; `tools/call` arguments are turned into an [`Operation`] by
//! [`parse_call`] before the daemon touches anything, so malformed input fails `invalid_input` with
//! nothing changed (FR-013). Only the tools whose handlers ship are listed.
//!
//! `create_worktree` offers `new_branch`, `existing_local` and `track_remote` and never `overwrite`:
//! overwriting discards a branch, and no create is among the operations FR-014 confirms
//! (research R9).

use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::errors::OpError;
use crate::session::AiCli;
use crate::worktree::CreateMode;

/// The worktree a tool targets: the project root (`default`) or a worktree directory name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorktreeRef {
    /// The project root.
    Default,
    /// A worktree, by its directory name.
    Named(String),
}

impl WorktreeRef {
    /// The ref the tools use for the project root.
    pub const DEFAULT: &'static str = "default";

    /// `default` is the project root; any other string names a worktree directory.
    pub fn parse(text: &str) -> Self {
        if text == Self::DEFAULT {
            WorktreeRef::Default
        } else {
            WorktreeRef::Named(text.to_string())
        }
    }

    /// The ref as the tools report it.
    pub fn as_str(&self) -> &str {
        match self {
            WorktreeRef::Default => Self::DEFAULT,
            WorktreeRef::Named(name) => name,
        }
    }
}

impl std::fmt::Display for WorktreeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A session a tool targets, by its UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionRef(pub Uuid);

/// A validated tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Whoami,
    ListWorktrees { include_hidden: bool },
    ListBranches,
    ListSessions { worktree: Option<WorktreeRef> },
    GetSession { session: SessionRef },
    /// A new worktree on `branch`, in directory `name` (derived from the branch when absent).
    CreateWorktree {
        branch: String,
        name: Option<String>,
        mode: CreateMode,
    },
    /// A new session in `worktree`, started, optionally typed a first prompt (FR-017). The prompt
    /// is input text: it is never logged (FR-018).
    CreateSession {
        worktree: WorktreeRef,
        ai_cli: Option<AiCli>,
        prompt: Option<String>,
    },
}

impl Operation {
    /// The tool this operation came from.
    pub fn tool_name(&self) -> &'static str {
        match self {
            Operation::Whoami => "whoami",
            Operation::ListWorktrees { .. } => "list_worktrees",
            Operation::ListBranches => "list_branches",
            Operation::ListSessions { .. } => "list_sessions",
            Operation::GetSession { .. } => "get_session",
            Operation::CreateWorktree { .. } => "create_worktree",
            Operation::CreateSession { .. } => "create_session",
        }
    }

    /// Whether the operation changes anything, and so is audited (FR-018).
    pub fn is_mutating(&self) -> bool {
        !is_read_only(self.tool_name())
    }

    /// The target an audit line names: the worktree or session acted on, or the branch a new
    /// worktree is created for. Never input text.
    pub fn audit_target(&self) -> String {
        match self {
            Operation::Whoami | Operation::ListBranches => String::new(),
            Operation::ListWorktrees { .. } => String::new(),
            Operation::ListSessions { worktree } => worktree
                .as_ref()
                .map(|w| w.as_str().to_string())
                .unwrap_or_default(),
            Operation::GetSession { session } => session.0.to_string(),
            Operation::CreateWorktree { branch, name, .. } => {
                name.clone().unwrap_or_else(|| branch.clone())
            }
            Operation::CreateSession { worktree, .. } => worktree.as_str().to_string(),
        }
    }
}

/// One catalog entry.
struct Tool {
    name: &'static str,
    description: &'static str,
    /// `properties` of the input schema.
    properties: fn() -> Value,
    /// Required property names.
    required: &'static [&'static str],
    /// Whether the tool only reads (the `readOnlyHint` annotation; FR-018 audits the others).
    read_only: bool,
}

/// Whether `tool` is one of the read-only tools.
fn is_read_only(tool: &str) -> bool {
    TOOLS.iter().any(|t| t.name == tool && t.read_only)
}

fn no_properties() -> Value {
    json!({})
}

fn session_property() -> Value {
    json!({"session": {"type": "string", "format": "uuid", "description": "A session id from list_sessions."}})
}

/// The shipped tools, in the order `tools/list` names them.
const TOOLS: &[Tool] = &[
    Tool {
        name: "whoami",
        description: "Identify the calling session: its id, its project (name and path), the \
            worktree it runs in (\"default\" is the project root) and its AI CLI.",
        properties: no_properties,
        required: &[],
        read_only: true,
    },
    Tool {
        name: "list_worktrees",
        description: "List the project's worktrees as the app's sidebar shows them: \"default\" \
            (the project root) first, then each worktree with its branch, status, whether the app \
            created it, and how many sessions it hosts. Worktrees owned by an assistant are hidden \
            unless include_hidden is true.",
        properties: || {
            json!({"include_hidden": {"type": "boolean", "default": false,
                "description": "Also list worktrees owned by an assistant."}})
        },
        required: &[],
        read_only: true,
    },
    Tool {
        name: "list_branches",
        description: "List the project's local and remote-tracking branches, the worktree each \
            is checked out in, and why a branch cannot back a new worktree (null when it can).",
        properties: no_properties,
        required: &[],
        read_only: true,
    },
    Tool {
        name: "list_sessions",
        description: "List the project's sessions with their label, AI CLI, lifecycle, activity \
            and worktree; is_caller marks the calling session. Filter by worktree ref if given.",
        properties: || {
            json!({"worktree": {"type": "string", "minLength": 1,
                "description": "A worktree ref from list_worktrees, or \"default\"."}})
        },
        required: &[],
        read_only: true,
    },
    Tool {
        name: "get_session",
        description: "Describe one session of the project; a failed session includes its \
            failure_reason.",
        properties: session_property,
        required: &["session"],
        read_only: true,
    },
    Tool {
        name: "create_worktree",
        description: "Create a worktree exactly as the app's create-worktree dialog does: under \
            the project's .claude/worktrees/, recorded as created by the app, shown in every \
            window. mode new_branch (default) starts a new branch at HEAD; existing_local checks \
            out a local branch no worktree holds; track_remote starts a local branch from \
            <remote>/<branch>. name is the directory name (default: derived from the branch). A \
            session running in the project root (Default) is refused.",
        properties: || {
            json!({
                "branch": {"type": "string", "minLength": 1,
                    "description": "The branch the worktree checks out."},
                "name": {"type": "string", "minLength": 1,
                    "description": "The worktree's directory name."},
                "mode": {"type": "string",
                    "enum": ["new_branch", "existing_local", "track_remote"],
                    "default": "new_branch"},
                "remote": {"type": "string", "minLength": 1,
                    "description": "The remote to track; required with track_remote."},
            })
        },
        required: &["branch"],
        read_only: false,
    },
    Tool {
        name: "create_session",
        description: "Create and start a session in a worktree (or \"default\", the project \
            root) running an AI CLI (default: the user's default from Settings). With prompt, \
            the prompt is typed as the session's first input once its CLI is ready, and the call \
            returns once it is (prompt_delivered: true) or 60 s after the request \
            (prompt_delivered: false).",
        properties: || {
            json!({
                "worktree": {"type": "string", "minLength": 1,
                    "description": "A worktree ref from list_worktrees, or \"default\"."},
                "ai_cli": {"type": "string", "enum": ["claude_code", "copilot", "pi"]},
                "prompt": {"type": "string", "description": "The session's first input."},
            })
        },
        required: &["worktree"],
        read_only: false,
    },
];

/// The `tools/list` result.
pub fn list_result() -> Value {
    let tools: Vec<Value> = TOOLS
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": {
                    "type": "object",
                    "properties": (tool.properties)(),
                    "required": tool.required,
                    "additionalProperties": false,
                },
                "annotations": {"readOnlyHint": tool.read_only, "destructiveHint": false},
            })
        })
        .collect();
    json!({ "tools": tools })
}

/// Validate a `tools/call` into an [`Operation`]. An unknown tool, an unknown argument, or an
/// argument of the wrong type fails `invalid_input`.
pub fn parse_call(name: &str, arguments: &Value) -> Result<Operation, OpError> {
    let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
        return Err(OpError::invalid_input(format!("unknown tool \"{name}\"")));
    };
    let empty = Map::new();
    let args = match arguments {
        Value::Object(map) => map,
        Value::Null => &empty,
        _ => return Err(OpError::invalid_input("arguments must be an object")),
    };
    let allowed = (tool.properties)();
    if let Some(unknown) = args.keys().find(|k| allowed.get(k.as_str()).is_none()) {
        return Err(OpError::invalid_input(format!(
            "{name} has no argument \"{unknown}\""
        )));
    }
    Ok(match name {
        "whoami" => Operation::Whoami,
        "list_worktrees" => Operation::ListWorktrees {
            include_hidden: optional_bool(args, "include_hidden")?.unwrap_or(false),
        },
        "list_branches" => Operation::ListBranches,
        "list_sessions" => Operation::ListSessions {
            worktree: optional_worktree(args, "worktree")?,
        },
        "get_session" => Operation::GetSession {
            session: required_session(args, "session")?,
        },
        "create_worktree" => Operation::CreateWorktree {
            branch: required_string(args, "branch")?,
            name: optional_string(args, "name")?,
            mode: create_mode(args)?,
        },
        "create_session" => Operation::CreateSession {
            worktree: optional_worktree(args, "worktree")?.ok_or_else(|| {
                OpError::invalid_input(
                    "worktree must be a worktree ref from list_worktrees, or \"default\"",
                )
            })?,
            ai_cli: optional_ai_cli(args, "ai_cli")?,
            prompt: optional_text(args, "prompt")?,
        },
        _ => unreachable!("every catalog entry is parsed above"),
    })
}

fn optional_bool(args: &Map<String, Value>, key: &str) -> Result<Option<bool>, OpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(OpError::invalid_input(format!("{key} must be a boolean"))),
    }
}

fn optional_worktree(args: &Map<String, Value>, key: &str) -> Result<Option<WorktreeRef>, OpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(WorktreeRef::parse(s))),
        Some(_) => Err(OpError::invalid_input(format!(
            "{key} must be a worktree ref from list_worktrees, or \"default\""
        ))),
    }
}

fn required_session(args: &Map<String, Value>, key: &str) -> Result<SessionRef, OpError> {
    args.get(key)
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(SessionRef)
        .ok_or_else(|| OpError::invalid_input(format!("{key} must be a session id (a UUID)")))
}

fn required_string(args: &Map<String, Value>, key: &str) -> Result<String, OpError> {
    optional_string(args, key)?
        .ok_or_else(|| OpError::invalid_input(format!("{key} must be a non-empty string")))
}

fn optional_string(args: &Map<String, Value>, key: &str) -> Result<Option<String>, OpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
        Some(_) => Err(OpError::invalid_input(format!(
            "{key} must be a non-empty string"
        ))),
    }
}

/// A string that may be empty (input text).
fn optional_text(args: &Map<String, Value>, key: &str) -> Result<Option<String>, OpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(OpError::invalid_input(format!("{key} must be a string"))),
    }
}

fn create_mode(args: &Map<String, Value>) -> Result<CreateMode, OpError> {
    let remote = optional_string(args, "remote")?;
    let mode = optional_string(args, "mode")?;
    match mode.as_deref() {
        None | Some("new_branch") => Ok(CreateMode::NewBranch),
        Some("existing_local") => Ok(CreateMode::ReuseLocal),
        Some("track_remote") => remote
            .map(|remote| CreateMode::TrackRemote { remote })
            .ok_or_else(|| OpError::invalid_input("mode track_remote needs the remote to track")),
        Some(other) => Err(OpError::invalid_input(format!(
            "mode must be new_branch, existing_local or track_remote, not \"{other}\""
        ))),
    }
}

fn optional_ai_cli(args: &Map<String, Value>, key: &str) -> Result<Option<AiCli>, OpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => AiCli::ALL
            .into_iter()
            .find(|cli| cli.tool_name() == s)
            .map(Some)
            .ok_or_else(|| {
                OpError::invalid_input(format!(
                    "{key} must be claude_code, copilot or pi, not \"{s}\""
                ))
            }),
        Some(_) => Err(OpError::invalid_input(format!(
            "{key} must be claude_code, copilot or pi"
        ))),
    }
}
