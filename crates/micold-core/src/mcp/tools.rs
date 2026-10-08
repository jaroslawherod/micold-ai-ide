//! The tool catalog and argument validation (feature 034, contracts/mcp-tools.md).
//!
//! `tools/list` answers [`list_result`]; `tools/call` arguments are turned into an [`Operation`] by
//! [`parse_call`] before the daemon touches anything, so malformed input fails `invalid_input` with
//! nothing changed (FR-013). The catalog holds every tool of the contract; only the tools whose
//! handlers ship are listed or callable.
//!
//! `create_worktree` offers `new_branch`, `existing_local` and `track_remote` and never `overwrite`:
//! overwriting discards a branch, and no create is among the operations FR-014 confirms
//! (research R9).

use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::errors::OpError;
use crate::naming::{ConventionalType, WorktreeNaming};
use crate::project::{validate_rename, RenameError};
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

/// How many of a session's most recent lines `read_session_output` returns (FR-012): 1 to
/// [`LineCount::MAX`], after clamping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCount(u16);

impl LineCount {
    /// What a call that names no count gets.
    pub const DEFAULT: LineCount = LineCount(200);
    /// The most one call returns; a larger request is clamped to it.
    pub const MAX: LineCount = LineCount(2000);

    /// `requested` clamped to the maximum, or `None` below 1.
    pub fn new(requested: u64) -> Option<Self> {
        if requested == 0 {
            return None;
        }
        Some(LineCount(requested.min(u64::from(Self::MAX.0)) as u16))
    }

    /// The count.
    pub fn get(self) -> u16 {
        self.0
    }
}

/// Text an agent types into a session: never empty (FR-012a), only text, and never printed. Its
/// `Debug` shows the length only, so no log line or panic message can carry input text (FR-018).
#[derive(Clone, PartialEq, Eq)]
pub struct NonEmptyText(String);

/// Why a string is not text to type into a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotText {
    /// Nothing would be typed before the closing Enter.
    Empty,
    /// It holds a control character, which a terminal reads as a keystroke.
    Keystroke,
}

impl NonEmptyText {
    /// `text` as it is, or `None` when it is not text to type.
    ///
    /// Whitespace is text (assumption A-4), but line breaks alone are not: a submission drops its
    /// trailing line breaks, so only the closing Enter would be typed. A control character other
    /// than a line break or a tab is a keystroke (Ctrl-C interrupts, Escape drives a menu), and
    /// `interrupt_session` is the confirmed way to send one (FR-014).
    pub fn new(text: impl Into<String>) -> Option<Self> {
        Self::checked(text.into()).ok()
    }

    fn checked(text: String) -> Result<Self, NotText> {
        if text.trim_end_matches(['\r', '\n']).is_empty() {
            return Err(NotText::Empty);
        }
        if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err(NotText::Keystroke);
        }
        Ok(NonEmptyText(text))
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for NonEmptyText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NonEmptyText(<{} bytes>)", self.0.len())
    }
}

/// A validated tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Whoami,
    ListWorktrees {
        include_hidden: bool,
    },
    ListBranches,
    ListSessions {
        worktree: Option<WorktreeRef>,
    },
    GetSession {
        session: SessionRef,
    },
    /// The most recent `lines` of another session's primary terminal, as plain text (FR-012).
    ReadSessionOutput {
        session: SessionRef,
        lines: LineCount,
    },
    /// Type `text` into another session's primary process and submit it once. The text is input:
    /// it is never logged (FR-018).
    SendSessionInput {
        session: SessionRef,
        text: NonEmptyText,
    },
    /// A new worktree: on a literal `branch`, or on the branch and directory the New worktree
    /// form derives from type, ticket and name (feature 550).
    CreateWorktree(CreateWorktreeRequest),
    /// A new session in `worktree`, started, optionally typed a first prompt (FR-017). The prompt
    /// is input text: it is never logged (FR-018).
    CreateSession {
        worktree: WorktreeRef,
        ai_cli: Option<AiCli>,
        prompt: Option<String>,
    },
    /// Start an idle, failed or resumable session, as the sidebar does.
    StartSession {
        session: SessionRef,
    },
    StopSession {
        session: SessionRef,
    },
    InterruptSession {
        session: SessionRef,
    },
    DeleteSession {
        session: SessionRef,
    },
    /// Attach a provider's worktree: a `dir_name`, an absolute path or a branch, resolved by the
    /// daemon (feature 582, FR-005).
    AttachWorktree {
        worktree: String,
    },
    /// Sessions in the provider stores that the catalog does not hold, newest first (FR-011).
    ListResumableSessions {
        limit: usize,
        offset: usize,
        worktree: Option<WorktreeRef>,
    },
    /// Give a worktree (never `default`) a new display name; `display_name` is trimmed and
    /// non-blank, as the rename dialog requires.
    RenameWorktree {
        worktree: WorktreeRef,
        display_name: String,
    },
    /// Delete a worktree (never `default`).
    DeleteWorktree {
        worktree: WorktreeRef,
        stop_sessions: bool,
        delete_branch: bool,
    },
}

/// What `create_worktree` was asked for. Built only by the parser (feature 550, data-model.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateWorktreeRequest {
    /// A literal branch, in directory `name` (derived from the branch when absent).
    Literal {
        branch: String,
        name: Option<String>,
        mode: CreateMode,
    },
    /// The form's inputs: at least one of the four fields, never a `branch`, `mode` or `remote`.
    /// `name` is the description the branch and directory are derived from.
    Derived {
        type_: Option<ConventionalType>,
        ticket: Option<String>,
        name: Option<String>,
        github_issue: Option<u32>,
    },
}

impl CreateWorktreeRequest {
    /// The form's inputs of a derived request, as `naming::derive` takes them; `None` for a
    /// literal one. A missing name derives as blank, which `derive` refuses as the form does.
    pub fn naming(&self) -> Option<WorktreeNaming> {
        match self {
            CreateWorktreeRequest::Literal { .. } => None,
            CreateWorktreeRequest::Derived {
                type_,
                ticket,
                name,
                ..
            } => Some(WorktreeNaming {
                type_: *type_,
                ticket: ticket.clone(),
                name: name.clone().unwrap_or_default(),
            }),
        }
    }

    /// The target an audit line names: the branch, the name, or `#<issue>`. Never a ticket's or
    /// description's free text beyond what the caller named as the target.
    pub fn audit_target(&self) -> String {
        match self {
            CreateWorktreeRequest::Literal { branch, name, .. } => {
                name.clone().unwrap_or_else(|| branch.clone())
            }
            CreateWorktreeRequest::Derived {
                type_,
                ticket,
                name,
                github_issue,
            } => match (name, github_issue) {
                (Some(name), _) => name.clone(),
                (None, Some(issue)) => format!("#{issue}"),
                (None, None) => ticket
                    .clone()
                    .filter(|t| !t.trim().is_empty())
                    .or_else(|| type_.map(|t| t.as_str().to_string()))
                    .unwrap_or_else(|| "(derived)".to_string()),
            },
        }
    }
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
            Operation::ReadSessionOutput { .. } => "read_session_output",
            Operation::SendSessionInput { .. } => "send_session_input",
            Operation::CreateWorktree { .. } => "create_worktree",
            Operation::CreateSession { .. } => "create_session",
            Operation::StartSession { .. } => "start_session",
            Operation::StopSession { .. } => "stop_session",
            Operation::InterruptSession { .. } => "interrupt_session",
            Operation::DeleteSession { .. } => "delete_session",
            Operation::AttachWorktree { .. } => "attach_worktree",
            Operation::ListResumableSessions { .. } => "list_resumable_sessions",
            Operation::RenameWorktree { .. } => "rename_worktree",
            Operation::DeleteWorktree { .. } => "delete_worktree",
        }
    }

    /// The target an audit line names: the worktree or session acted on, or the branch a new
    /// worktree is created for. Never input text.
    pub fn audit_target(&self) -> String {
        match self {
            Operation::Whoami | Operation::ListBranches => String::new(),
            Operation::ListWorktrees { .. } => String::new(),
            Operation::ListResumableSessions { worktree, .. } => worktree
                .as_ref()
                .map(|w| w.as_str().to_string())
                .unwrap_or_default(),
            Operation::AttachWorktree { worktree } => worktree.clone(),
            Operation::ListSessions { worktree } => worktree
                .as_ref()
                .map(|w| w.as_str().to_string())
                .unwrap_or_default(),
            Operation::GetSession { session }
            | Operation::ReadSessionOutput { session, .. }
            | Operation::SendSessionInput { session, .. }
            | Operation::StartSession { session }
            | Operation::StopSession { session }
            | Operation::InterruptSession { session }
            | Operation::DeleteSession { session } => session.0.to_string(),
            Operation::CreateWorktree(request) => request.audit_target(),
            Operation::CreateSession { worktree, .. }
            | Operation::RenameWorktree { worktree, .. }
            | Operation::DeleteWorktree { worktree, .. } => worktree.as_str().to_string(),
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
    /// Whether FR-014 classifies it as destructive (the `destructiveHint` annotation).
    destructive: bool,
    /// Whether its handler ships: only these are listed and callable.
    shipped: bool,
}

/// Whether `tool` is a shipped tool that changes anything, and so is audited (FR-018). Named by
/// the tool, not the parsed operation, so a call whose arguments fail to parse is audited too.
pub fn is_mutating_tool(tool: &str) -> bool {
    TOOLS
        .iter()
        .any(|t| t.name == tool && t.shipped && !t.read_only)
}

fn no_properties() -> Value {
    json!({})
}

fn session_property() -> Value {
    json!({"session": {"type": "string", "format": "uuid", "description": "A session id from list_sessions."}})
}

/// The `worktree` argument of a tool that changes a worktree: `default` is not one.
fn worktree_property() -> Value {
    json!({"type": "string", "minLength": 1,
        "description": "A worktree ref from list_worktrees (not \"default\")."})
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
        destructive: false,
        shipped: true,
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
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "list_branches",
        description: "List the project's local and remote-tracking branches, the worktree each \
            is checked out in, and why a branch cannot back a new worktree (null when it can).",
        properties: no_properties,
        required: &[],
        read_only: true,
        destructive: false,
        shipped: true,
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
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "get_session",
        description: "Describe one session of the project; a failed session includes its \
            failure_reason.",
        properties: session_property,
        required: &["session"],
        read_only: true,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "list_resumable_sessions",
        description: "List the sessions the AI CLIs recorded for this project that the app does \
            not hold (find them with list_sessions otherwise), newest first, with the worktree \
            each ran in and whether it can be resumed. Reads only; resuming one is the app's \
            action. Attach its worktree with attach_worktree to bring its adopted sessions back.",
        properties: || {
            json!({
                "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESUMABLE_LIMIT,
                    "default": DEFAULT_RESUMABLE_LIMIT,
                    "description": "The most sessions to return."},
                "offset": {"type": "integer", "minimum": 0, "default": 0,
                    "description": "How many of the newest sessions to skip."},
                "worktree": worktree_property(),
            })
        },
        required: &[],
        read_only: true,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "read_session_output",
        description: "Read the most recent lines another session of the project showed in its \
            terminal, as plain text: lines (default 200, at most 2000) counts from the end, and \
            truncated says whether older lines exist. Reads the session's AI CLI, not a shell \
            opened beside it. A session cannot read itself. Refused when the user has turned \
            \"Let agents read and type into other sessions\" off in Settings.",
        properties: || {
            json!({
                "session": {"type": "string", "format": "uuid",
                    "description": "A session id from list_sessions."},
                "lines": {"type": "integer", "minimum": 1, "default": 200,
                    "description": "How many of the most recent lines; above 2000 is read as 2000."},
            })
        },
        required: &["session"],
        read_only: true,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "create_worktree",
        description: "Create a worktree exactly as the app's create-worktree dialog does: under \
            the project's .claude/worktrees/, recorded as created by the app, shown in every \
            window. Two alternatives. (1) A literal branch: branch, with mode new_branch (default, \
            starts a new branch at HEAD), existing_local (checks out a local branch no worktree \
            holds) or track_remote (starts a local branch from <remote>/<branch>), and name as the \
            directory name (default: derived from the branch). (2) The form's inputs: type (feat, \
            fix, chore, docs, refactor, test, build, ci, perf, style), ticket and name (the \
            description) give the branch type/ticket_name and the directory type-ticket_name, as \
            the form does, with the sidebar's type and issue tags; github_issue (a GitHub issue \
            number) will fill them from the issue but is not yet available. Pass branch, mode or \
            remote, or the derived inputs, never both. A derived name that collides with an \
            existing branch is refused: use branch with mode instead. The result carries the \
            worktree row plus branch, directory, and for derived inputs type and ticket. Any \
            session may create a worktree, one running in the project root (Default) too; \
            renaming and deleting a worktree are refused from there.",
        properties: || {
            json!({
                "branch": {"type": "string", "minLength": 1,
                    "description": "The literal branch the worktree checks out. Required unless \
                        type, ticket or name derive one."},
                "name": {"type": "string", "minLength": 1,
                    "description": "With branch: the worktree's directory name. Without branch: \
                        the description the branch and directory are derived from."},
                "mode": {"type": "string",
                    "enum": ["new_branch", "existing_local", "track_remote"],
                    "default": "new_branch",
                    "description": "Literal only: requires branch."},
                "remote": {"type": "string", "minLength": 1,
                    "description": "The remote to track; required with track_remote. Literal only."},
                "type": {"type": "string",
                    "enum": ConventionalType::ALL.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
                    "description": "Derived only: the Conventional-Commits type."},
                "ticket": {"type": "string",
                    "description": "Derived only: a ticket reference, slugified as the form does; \
                        blank means none."},
                "github_issue": {"type": "integer", "minimum": 1,
                    "description": "Derived only: a GitHub issue number. Not yet available."},
            })
        },
        required: &[],
        read_only: false,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "attach_worktree",
        description: "Attach a worktree an AI CLI created, so the sidebar lists it and its \
            sessions, without deleting or recreating it. worktree is its directory name, its \
            absolute path or its branch; nothing is checked out or moved. Reports whether it was \
            attached or already attached. A session running in the project root (Default) is \
            refused.",
        properties: || {
            json!({"worktree": {"type": "string", "minLength": 1,
                "description": "A dir_name from list_worktrees (include_hidden), an absolute \
                    path, or a branch name."}})
        },
        required: &["worktree"],
        read_only: false,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "rename_worktree",
        description: "Give a worktree a new display name, as the sidebar's rename does; its \
            directory and branch are unchanged. \"default\" cannot be renamed. A session running \
            in the project root (Default) is refused.",
        properties: || {
            json!({
                "worktree": worktree_property(),
                "display_name": {"type": "string", "minLength": 1,
                    "description": "The new name; leading and trailing whitespace is dropped."},
            })
        },
        required: &["worktree", "display_name"],
        read_only: false,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "delete_worktree",
        description: "Delete a worktree and, unless delete_branch is false, its branch, after the \
            user confirms in an app window. A worktree with live sessions is refused unless \
            stop_sessions is true. \"default\" and the calling session's own worktree cannot be \
            deleted; a session running in the project root (Default) is refused.",
        properties: || {
            json!({
                "worktree": worktree_property(),
                "stop_sessions": {"type": "boolean", "default": false,
                    "description": "Stop the worktree's live sessions first."},
                "delete_branch": {"type": "boolean", "default": true,
                    "description": "Also delete the worktree's branch."},
            })
        },
        required: &["worktree"],
        read_only: false,
        destructive: true,
        shipped: true,
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
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "start_session",
        description: "Start an idle, failed or resumable session, as the sidebar's start does (a \
            resumable session resumes its conversation). A session that is already starting, \
            running or restarting is left as it is. Returns the session's lifecycle.",
        properties: session_property,
        required: &["session"],
        read_only: false,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "stop_session",
        description: "Stop another session of the project after the user confirms in an app \
            window: its processes end, it becomes idle and stays resumable. A session cannot stop \
            itself.",
        properties: session_property,
        required: &["session"],
        read_only: false,
        destructive: true,
        shipped: true,
    },
    Tool {
        name: "interrupt_session",
        description: "Send an interrupt keystroke to another running session after the user \
            confirms in an app window; it keeps running. A session cannot interrupt itself.",
        properties: session_property,
        required: &["session"],
        read_only: false,
        destructive: true,
        shipped: true,
    },
    Tool {
        name: "send_session_input",
        description: "Type text into another running session of the project and submit it once, \
            exactly as if the user had typed it and pressed Enter; a multi-line text is one \
            submission. Check get_session first: the session should be awaiting input. A session \
            cannot type into itself. Depending on the user's \"Let agents read and type into \
            other sessions\" setting, the send goes through, waits for the user's confirmation \
            in an app window, or is refused.",
        properties: || {
            json!({
                "session": {"type": "string", "format": "uuid",
                    "description": "A session id from list_sessions."},
                "text": {"type": "string", "minLength": 1,
                    "description": "The text to type; it is submitted with one Enter."},
            })
        },
        required: &["session", "text"],
        read_only: false,
        destructive: false,
        shipped: true,
    },
    Tool {
        name: "delete_session",
        description: "Delete another session of the project after the user confirms in an app \
            window. A session cannot delete itself.",
        properties: session_property,
        required: &["session"],
        read_only: false,
        destructive: true,
        shipped: true,
    },
];

/// The `tools/list` result: the shipped tools.
pub fn list_result() -> Value {
    listing(TOOLS.iter().filter(|t| t.shipped))
}

/// Every catalogued tool, listed as `tools/list` would list it, whether or not its handler ships.
pub fn catalog() -> Value {
    listing(TOOLS.iter())
}

fn listing<'a>(tools: impl Iterator<Item = &'a Tool>) -> Value {
    let tools: Vec<Value> = tools
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
                "annotations": {"readOnlyHint": tool.read_only, "destructiveHint": tool.destructive},
            })
        })
        .collect();
    json!({ "tools": tools })
}

/// Validate a `tools/call` into an [`Operation`]. An unknown tool, a tool whose handler has not
/// shipped, an unknown argument, or an argument of the wrong type fails `invalid_input`.
pub fn parse_call(name: &str, arguments: &Value) -> Result<Operation, OpError> {
    if !TOOLS.iter().any(|t| t.name == name && t.shipped) {
        return Err(unknown_tool(name));
    }
    parse_operation(name, arguments)
}

fn unknown_tool(name: &str) -> OpError {
    OpError::invalid_input(format!("unknown tool \"{name}\""))
}

/// Validate the arguments of any catalogued tool, shipped or not, into an [`Operation`].
pub fn parse_operation(name: &str, arguments: &Value) -> Result<Operation, OpError> {
    let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
        return Err(unknown_tool(name));
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
        "read_session_output" => Operation::ReadSessionOutput {
            session: required_session(args, "session")?,
            lines: line_count(args, "lines")?,
        },
        "send_session_input" => Operation::SendSessionInput {
            session: required_session(args, "session")?,
            text: non_empty_text(args, "text")?,
        },
        "create_worktree" => Operation::CreateWorktree(create_worktree_request(args)?),
        "create_session" => Operation::CreateSession {
            worktree: optional_worktree(args, "worktree")?.ok_or_else(|| {
                OpError::invalid_input(
                    "worktree must be a worktree ref from list_worktrees, or \"default\"",
                )
            })?,
            ai_cli: optional_ai_cli(args, "ai_cli")?,
            prompt: optional_text(args, "prompt")?,
        },
        "start_session" => Operation::StartSession {
            session: required_session(args, "session")?,
        },
        "stop_session" => Operation::StopSession {
            session: required_session(args, "session")?,
        },
        "interrupt_session" => Operation::InterruptSession {
            session: required_session(args, "session")?,
        },
        "delete_session" => Operation::DeleteSession {
            session: required_session(args, "session")?,
        },
        "attach_worktree" => Operation::AttachWorktree {
            worktree: required_string(args, "worktree")?,
        },
        "list_resumable_sessions" => Operation::ListResumableSessions {
            limit: bounded_count(
                args,
                "limit",
                DEFAULT_RESUMABLE_LIMIT,
                Some(MAX_RESUMABLE_LIMIT),
            )?,
            offset: bounded_count(args, "offset", 0, None)?,
            worktree: optional_worktree(args, "worktree")?,
        },
        "rename_worktree" => Operation::RenameWorktree {
            worktree: named_worktree(args, "worktree", "renamed")?,
            display_name: display_name(args, "display_name")?,
        },
        "delete_worktree" => Operation::DeleteWorktree {
            worktree: named_worktree(args, "worktree", "deleted")?,
            stop_sessions: optional_bool(args, "stop_sessions")?.unwrap_or(false),
            delete_branch: optional_bool(args, "delete_branch")?.unwrap_or(true),
        },
        _ => unreachable!("every catalog entry is parsed above"),
    })
}

/// The largest issue number the form accepts (a GitHub issue number is a 32-bit integer).
const MAX_GITHUB_ISSUE: u64 = 2_147_483_647;

/// `create_worktree`'s arguments (contracts/create-worktree-tool.md): a literal `branch` (with
/// `name`, `mode`, `remote`) or the derived inputs (`type`, `ticket`, `github_issue`, `name`).
fn create_worktree_request(args: &Map<String, Value>) -> Result<CreateWorktreeRequest, OpError> {
    let present = |key: &str| !matches!(args.get(key), None | Some(Value::Null));
    let literal = ["branch", "mode", "remote"]
        .into_iter()
        .filter(|k| present(k))
        .collect::<Vec<_>>();
    let derived = ["type", "ticket", "github_issue"]
        .into_iter()
        .filter(|k| present(k))
        .collect::<Vec<_>>();
    if !literal.is_empty() && !derived.is_empty() {
        return Err(OpError::invalid_input(format!(
            "{} and {} are alternatives: pass a literal branch (with mode and remote), or the \
             derived inputs type, ticket and github_issue with name as the description, not both",
            literal.join(", "),
            derived.join(", ")
        )));
    }
    if present("branch") {
        return Ok(CreateWorktreeRequest::Literal {
            branch: required_string(args, "branch")?,
            name: optional_string(args, "name")?,
            mode: create_mode(args)?,
        });
    }
    if present("mode") || present("remote") {
        return Err(OpError::invalid_input(
            "mode and remote need a branch: pass branch, or use type, ticket and name instead",
        ));
    }
    let type_ = match optional_string(args, "type")? {
        None => None,
        Some(token) => Some(ConventionalType::from_token(&token).ok_or_else(|| {
            let allowed: Vec<&str> = ConventionalType::ALL.iter().map(|t| t.as_str()).collect();
            OpError::invalid_input(format!(
                "type must be one of {}, not \"{token}\"",
                allowed.join(", ")
            ))
        })?),
    };
    let ticket = optional_text(args, "ticket")?;
    let name = optional_string(args, "name")?;
    let github_issue = github_issue(args)?;
    if type_.is_none() && ticket.is_none() && name.is_none() && github_issue.is_none() {
        return Err(OpError::invalid_input(
            "provide a branch, or type, ticket and name (or github_issue) to derive one",
        ));
    }
    if github_issue.is_some() {
        return Err(OpError::invalid_input("github_issue is not supported yet"));
    }
    Ok(CreateWorktreeRequest::Derived {
        type_,
        ticket,
        name,
        github_issue,
    })
}

fn github_issue(args: &Map<String, Value>) -> Result<Option<u32>, OpError> {
    match args.get("github_issue") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .filter(|n| (1..=MAX_GITHUB_ISSUE).contains(n))
            .map(|n| Some(n as u32))
            .ok_or_else(github_issue_invalid),
        Some(_) => Err(github_issue_invalid()),
    }
}

fn github_issue_invalid() -> OpError {
    OpError::invalid_input(format!(
        "github_issue must be a whole number from 1 to {MAX_GITHUB_ISSUE}"
    ))
}

/// The default and the largest `limit` of `list_resumable_sessions`.
pub const DEFAULT_RESUMABLE_LIMIT: usize = 50;
pub const MAX_RESUMABLE_LIMIT: usize = 200;

/// A whole number that is at least 0 (`limit` also at least 1), at most `max` when given.
fn bounded_count(
    args: &Map<String, Value>,
    key: &str,
    default: usize,
    max: Option<usize>,
) -> Result<usize, OpError> {
    let min = usize::from(key == "limit");
    let invalid = || {
        OpError::invalid_input(match max {
            Some(max) => format!("{key} must be a whole number from {min} to {max}"),
            None => format!("{key} must be a whole number of at least {min}"),
        })
    };
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n >= min && max.is_none_or(|max| *n <= max))
            .ok_or_else(invalid),
        Some(_) => Err(invalid()),
    }
}

fn optional_bool(args: &Map<String, Value>, key: &str) -> Result<Option<bool>, OpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(OpError::invalid_input(format!("{key} must be a boolean"))),
    }
}

/// How many lines to read (FR-012): absent means the default, a whole number above the maximum is
/// clamped to it, and anything below 1 or not a whole number is invalid.
fn line_count(args: &Map<String, Value>, key: &str) -> Result<LineCount, OpError> {
    let invalid = || {
        OpError::invalid_input(format!(
            "{key} must be a whole number of at least 1 (at most {} are returned)",
            LineCount::MAX.get()
        ))
    };
    match args.get(key) {
        None | Some(Value::Null) => Ok(LineCount::DEFAULT),
        // `as_u64` is `None` for a negative number and for one with a fraction.
        Some(Value::Number(n)) => n.as_u64().and_then(LineCount::new).ok_or_else(invalid),
        Some(_) => Err(invalid()),
    }
}

/// Input text that must be given, not empty (FR-012a), and text only.
fn non_empty_text(args: &Map<String, Value>, key: &str) -> Result<NonEmptyText, OpError> {
    let text = optional_text(args, key)?
        .ok_or_else(|| OpError::invalid_input(format!("{key} must be a string")))?;
    NonEmptyText::checked(text).map_err(|why| {
        OpError::invalid_input(match why {
            NotText::Empty => format!("{key} cannot be empty"),
            NotText::Keystroke => {
                format!("{key} cannot hold a control character other than a line break or a tab")
            }
        })
    })
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

/// A worktree that is not the project root, which cannot be `verb` (U95).
fn named_worktree(
    args: &Map<String, Value>,
    key: &str,
    verb: &str,
) -> Result<WorktreeRef, OpError> {
    match optional_worktree(args, key)? {
        Some(WorktreeRef::Named(name)) => Ok(WorktreeRef::Named(name)),
        Some(WorktreeRef::Default) => Err(OpError::invalid_input(format!(
            "\"default\" is the project root, not a worktree; it cannot be {verb}"
        ))),
        None => Err(OpError::invalid_input(format!(
            "{key} must be a worktree ref from list_worktrees"
        ))),
    }
}

/// A new display name, checked and trimmed as the rename dialog does.
fn display_name(args: &Map<String, Value>, key: &str) -> Result<String, OpError> {
    let text = optional_text(args, key)?
        .ok_or_else(|| OpError::invalid_input(format!("{key} must be a string")))?;
    validate_rename(&text).map_err(|e| {
        OpError::invalid_input(match e {
            RenameError::Empty => format!("{key} cannot be empty"),
            RenameError::Whitespace => format!("{key} cannot be only whitespace"),
        })
    })
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
