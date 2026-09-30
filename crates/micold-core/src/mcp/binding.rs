//! The per-session launch binding to the tool server (feature 034, contracts/binding.md §4–§6).
//!
//! A bound session is spawned with a few extra arguments pointing its CLI at a per-session config
//! file that names one HTTP server, `micold`, with the session's bearer credential. No user or
//! project configuration file is ever written (FR-003): the file lives under the service's own data
//! directory, and the arguments add to the user's servers rather than replacing them.
//!
//! This module is pure: it builds the arguments and the file's bytes, and checks (read-only) whether
//! the user already configured a server named `micold`. The daemon writes the file and logs.

use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::jsonrpc::SERVER_NAME;
use crate::provider::ToolServerSupport;

/// How long the CLI waits for one tool call, in milliseconds. A destructive call can wait up to 60 s
/// for the user's answer (FR-014), so the CLI's own timeout must be longer.
pub const TOOL_TIMEOUT_MS: u64 = 120_000;

/// What a bound spawn adds: the arguments, appended after every other argument, and the contents of
/// the file they name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingPlan {
    pub args: Vec<String>,
    pub file_contents: String,
}

/// Why a session of an AI CLI is started unbound (FR-005). Its `Display` is the log text of
/// contracts/binding.md §5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// The user turned the tool server off in Settings (FR-004).
    Disabled,
    /// The CLI cannot reach an MCP server; the provider's reason.
    Unsupported(&'static str),
    /// The user already configured a server named `micold`, in this file.
    NameTaken(PathBuf),
    /// The service's tool server is not running.
    ServerUnavailable,
    /// The binding file could not be written.
    WriteFailed(String),
    /// A crash respawn of a session that was not bound when it started: it keeps what it started
    /// with (FR-004).
    UnboundAtStart,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkipReason::Disabled => f.write_str("disabled in settings"),
            SkipReason::Unsupported(reason) => f.write_str(reason),
            SkipReason::NameTaken(path) => write!(
                f,
                "a server named \"{SERVER_NAME}\" is already configured in {}",
                path.display()
            ),
            SkipReason::ServerUnavailable => f.write_str("tool server unavailable"),
            SkipReason::WriteFailed(error) => write!(f, "could not write the binding: {error}"),
            SkipReason::UnboundAtStart => f.write_str("not bound when it started"),
        }
    }
}

/// The binding for a CLI with `support`, reaching the server at `url` with `credential`, through the
/// config file at `file`. An unsupported CLI is skipped with its reason.
pub fn plan(
    support: ToolServerSupport,
    url: &str,
    credential: &str,
    file: &Path,
) -> Result<BindingPlan, SkipReason> {
    let mut entry = json!({
        "type": "http",
        "url": url,
        "headers": {"Authorization": format!("Bearer {credential}")},
        "timeout": TOOL_TIMEOUT_MS,
    });
    let file_arg = file.to_string_lossy().into_owned();
    let args = match support {
        ToolServerSupport::McpConfigArg => vec![
            "--mcp-config".to_string(),
            file_arg,
            "--allowedTools".to_string(),
            format!("mcp__{SERVER_NAME}"),
        ],
        ToolServerSupport::AdditionalMcpConfig => {
            entry["tools"] = json!(["*"]);
            vec![
                "--additional-mcp-config".to_string(),
                format!("@{file_arg}"),
                "--allow-tool".to_string(),
                SERVER_NAME.to_string(),
            ]
        }
        ToolServerSupport::Unsupported { reason } => return Err(SkipReason::Unsupported(reason)),
    };
    let document = json!({"mcpServers": {SERVER_NAME: entry}});
    Ok(BindingPlan {
        args,
        file_contents: serde_json::to_string_pretty(&document).expect("JSON always serialises"),
    })
}

/// Where the user's own MCP configuration may name servers, as the session would see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigLocations {
    /// The user's home directory.
    pub home: Option<PathBuf>,
    /// `CLAUDE_CONFIG_DIR`, when it is set in the session's environment.
    pub claude_config_dir: Option<PathBuf>,
    /// The Copilot CLI's config directory (`COPILOT_HOME` or `~/.copilot`).
    pub copilot_config_dir: Option<PathBuf>,
}

impl ConfigLocations {
    /// The `.claude.json` Claude reads: `$CLAUDE_CONFIG_DIR/.claude.json` when that is set, else
    /// `~/.claude.json`.
    pub fn claude_json(&self) -> Option<PathBuf> {
        match &self.claude_config_dir {
            Some(dir) => Some(dir.join(".claude.json")),
            None => self.home.as_ref().map(|home| home.join(".claude.json")),
        }
    }
}

/// The first user configuration file that already names a server `micold` for a session of a CLI
/// with `support` working in `cwd`, or `None` (research R14). Read-only; a file that is absent,
/// unreadable or malformed names nothing.
pub fn name_taken(
    support: ToolServerSupport,
    locations: &ConfigLocations,
    cwd: &Path,
) -> Option<PathBuf> {
    match support {
        ToolServerSupport::McpConfigArg => {
            if let Some(path) = locations.claude_json() {
                if let Some(doc) = read_json(&path) {
                    let cwd_key = cwd.to_string_lossy();
                    let in_project = doc
                        .get("projects")
                        .and_then(|projects| projects.get(cwd_key.as_ref()));
                    if names_micold(&doc) || in_project.is_some_and(names_micold) {
                        return Some(path);
                    }
                }
            }
            let project_file = cwd.join(".mcp.json");
            read_json(&project_file)
                .filter(names_micold)
                .map(|_| project_file)
        }
        ToolServerSupport::AdditionalMcpConfig => {
            let path = locations
                .copilot_config_dir
                .as_ref()?
                .join("mcp-config.json");
            read_json(&path).filter(names_micold).map(|_| path)
        }
        ToolServerSupport::Unsupported { .. } => None,
    }
}

/// Whether `object.mcpServers` has a key `micold`.
fn names_micold(object: &Value) -> bool {
    object
        .get("mcpServers")
        .and_then(Value::as_object)
        .is_some_and(|servers| servers.contains_key(SERVER_NAME))
}

pub(crate) fn read_json(path: &Path) -> Option<Value> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}
