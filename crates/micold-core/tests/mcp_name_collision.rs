//! A user's own server named `micold` wins over the binding (feature 034, contracts/binding.md §6,
//! research R14; U47–U57). The check is read-only, and a file it cannot read or parse counts as
//! "not taken".

use std::path::{Path, PathBuf};

use micold_core::mcp::binding::{name_taken, ConfigLocations};
use micold_core::provider::ToolServerSupport;
use serde_json::json;
use tempfile::TempDir;

const CLAUDE: ToolServerSupport = ToolServerSupport::McpConfigArg;
const COPILOT: ToolServerSupport = ToolServerSupport::AdditionalMcpConfig;

/// A fixture home, a Copilot config dir inside it, and a session working directory.
struct Fixture {
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("home/.copilot")).unwrap();
        std::fs::create_dir_all(root.path().join("project")).unwrap();
        std::fs::create_dir_all(root.path().join("claude-config")).unwrap();
        Self { root }
    }

    fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    fn cwd(&self) -> PathBuf {
        self.root.path().join("project")
    }

    fn claude_config_dir(&self) -> PathBuf {
        self.root.path().join("claude-config")
    }

    fn locations(&self) -> ConfigLocations {
        ConfigLocations {
            home: Some(self.home()),
            claude_config_dir: None,
            copilot_config_dir: Some(self.home().join(".copilot")),
        }
    }

    fn with_claude_config_dir(&self) -> ConfigLocations {
        ConfigLocations {
            claude_config_dir: Some(self.claude_config_dir()),
            ..self.locations()
        }
    }

    fn write(&self, path: &Path, value: serde_json::Value) -> PathBuf {
        std::fs::write(path, value.to_string()).unwrap();
        path.to_path_buf()
    }
}

fn servers(name: &str) -> serde_json::Value {
    json!({ name: {"type": "stdio", "command": "x"} })
}

#[test]
fn a_top_level_entry_in_claude_json_is_taken() {
    let f = Fixture::new();
    let path = f.write(
        &f.home().join(".claude.json"),
        json!({"mcpServers": servers("micold")}),
    );
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), Some(path));
}

#[test]
fn an_entry_under_the_sessions_project_path_is_taken() {
    let f = Fixture::new();
    let cwd = f.cwd().to_string_lossy().into_owned();
    let path = f.write(
        &f.home().join(".claude.json"),
        json!({"projects": {cwd: {"mcpServers": servers("micold")}}}),
    );
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), Some(path));
}

#[test]
fn an_entry_under_another_projects_path_is_not_taken() {
    let f = Fixture::new();
    f.write(
        &f.home().join(".claude.json"),
        json!({"projects": {"/somewhere/else": {"mcpServers": servers("micold")}}}),
    );
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), None);
}

#[test]
fn with_claude_config_dir_set_its_claude_json_is_read() {
    let f = Fixture::new();
    let path = f.write(
        &f.claude_config_dir().join(".claude.json"),
        json!({"mcpServers": servers("micold")}),
    );
    assert_eq!(
        name_taken(CLAUDE, &f.with_claude_config_dir(), &f.cwd()),
        Some(path)
    );
}

#[test]
fn with_claude_config_dir_set_the_home_claude_json_is_not_read() {
    let f = Fixture::new();
    f.write(
        &f.home().join(".claude.json"),
        json!({"mcpServers": servers("micold")}),
    );
    assert_eq!(
        name_taken(CLAUDE, &f.with_claude_config_dir(), &f.cwd()),
        None
    );
}

#[test]
fn an_entry_in_the_projects_mcp_json_is_taken_for_claude() {
    let f = Fixture::new();
    let path = f.write(
        &f.cwd().join(".mcp.json"),
        json!({"mcpServers": servers("micold")}),
    );
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), Some(path));
}

#[test]
fn an_entry_in_copilots_mcp_config_is_taken_for_copilot() {
    let f = Fixture::new();
    let path = f.write(
        &f.home().join(".copilot/mcp-config.json"),
        json!({"mcpServers": servers("micold")}),
    );
    assert_eq!(name_taken(COPILOT, &f.locations(), &f.cwd()), Some(path));
    assert_eq!(
        name_taken(CLAUDE, &f.locations(), &f.cwd()),
        None,
        "Copilot's file says nothing about Claude"
    );
}

#[test]
fn a_server_with_another_name_is_not_taken() {
    let f = Fixture::new();
    f.write(
        &f.home().join(".claude.json"),
        json!({"mcpServers": servers("micold2")}),
    );
    f.write(
        &f.cwd().join(".mcp.json"),
        json!({"mcpServers": servers("other")}),
    );
    f.write(
        &f.home().join(".copilot/mcp-config.json"),
        json!({"mcpServers": servers("Micold")}),
    );
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), None);
    assert_eq!(name_taken(COPILOT, &f.locations(), &f.cwd()), None);
}

#[test]
fn absent_files_are_not_taken() {
    let f = Fixture::new();
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), None);
    assert_eq!(name_taken(COPILOT, &f.locations(), &f.cwd()), None);
}

#[test]
fn a_malformed_file_is_not_taken() {
    let f = Fixture::new();
    std::fs::write(
        f.home().join(".claude.json"),
        "{\"mcpServers\": {\"micold\"",
    )
    .unwrap();
    std::fs::write(f.cwd().join(".mcp.json"), "not json").unwrap();
    std::fs::write(f.home().join(".copilot/mcp-config.json"), "[1,2").unwrap();
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), None);
    assert_eq!(name_taken(COPILOT, &f.locations(), &f.cwd()), None);
}

#[test]
fn an_unreadable_file_is_not_taken() {
    // A directory where the file should be cannot be read as a file on any platform.
    let f = Fixture::new();
    std::fs::create_dir_all(f.home().join(".claude.json")).unwrap();
    std::fs::create_dir_all(f.cwd().join(".mcp.json")).unwrap();
    std::fs::create_dir_all(f.home().join(".copilot/mcp-config.json")).unwrap();
    assert_eq!(name_taken(CLAUDE, &f.locations(), &f.cwd()), None);
    assert_eq!(name_taken(COPILOT, &f.locations(), &f.cwd()), None);
}

#[test]
fn an_unsupported_cli_has_nothing_to_collide_with() {
    let f = Fixture::new();
    f.write(
        &f.home().join(".claude.json"),
        json!({"mcpServers": servers("micold")}),
    );
    let pi = ToolServerSupport::Unsupported {
        reason: "Pi has no MCP support",
    };
    assert_eq!(name_taken(pi, &f.locations(), &f.cwd()), None);
}
