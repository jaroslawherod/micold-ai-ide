//! The tool catalog and argument validation (feature 034, contracts/mcp-tools.md).
//!
//! `tools/list` answers [`list_result`]; `tools/call` arguments are turned into an [`Operation`] by
//! [`parse_call`] before the daemon touches anything, so malformed input fails `invalid_input` with
//! nothing changed (FR-013). Only the tools whose handlers ship are listed.

use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::errors::OpError;

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
}

/// One catalog entry.
struct Tool {
    name: &'static str,
    description: &'static str,
    /// `properties` of the input schema.
    properties: fn() -> Value,
    /// Required property names.
    required: &'static [&'static str],
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
    },
    Tool {
        name: "list_branches",
        description: "List the project's local and remote-tracking branches, the worktree each \
            is checked out in, and why a branch cannot back a new worktree (null when it can).",
        properties: no_properties,
        required: &[],
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
    },
    Tool {
        name: "get_session",
        description: "Describe one session of the project; a failed session includes its \
            failure_reason.",
        properties: session_property,
        required: &["session"],
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
                "annotations": {"readOnlyHint": true, "destructiveHint": false},
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

fn optional_worktree(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<WorktreeRef>, OpError> {
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
