//! Stand-in AI CLIs for the tool-server tests that start real sessions (feature 034): scripts on a
//! private `PATH` that record their launch and what is typed into them, and the trust records the
//! real CLIs would read.
#![allow(dead_code)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde_json::json;

/// Every variable a test changes, restored on drop.
pub struct Env {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl Env {
    pub fn set(vars: &[(&'static str, Option<OsString>)]) -> Self {
        let saved = vars
            .iter()
            .map(|(name, value)| {
                let previous = std::env::var_os(name);
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// Each CLI's own record that it trusts `project` (and so every worktree below it), as the real
/// ones write it: `~/.claude.json` and `~/.copilot/config.json` with its leading comment.
pub fn trust_project(home: &Path, project: &Path) {
    let claude = json!({"projects": {project.to_str().unwrap(): {"hasTrustDialogAccepted": true}}});
    std::fs::write(home.join(".claude.json"), claude.to_string()).unwrap();
    std::fs::create_dir_all(home.join(".copilot")).unwrap();
    let copilot = json!({"trustedFolders": [project]});
    std::fs::write(
        home.join(".copilot/config.json"),
        format!("// This file is managed automatically.\n{copilot}"),
    )
    .unwrap();
}

/// A stand-in CLI: records its launch; `draws` makes it print a prompt; Pi reports `session_start`
/// through the component's log after `<bin>/pi.delay` seconds (0.3 by default). Everything typed
/// into it is appended to `<bin>/<command>.input`.
pub fn install_cli(bin: &Path, command: &str, draws: bool) {
    let dir = bin.display();
    let draw = if draws { "printf 'ready> '\n" } else { "" };
    let report = if command == "pi" {
        format!(
            "( sleep \"$(cat '{dir}/pi.delay' 2>/dev/null || echo 0.3)\"; \
             printf '{{\"type\":\"session_start\"}}\\n' >> \"$MICOLD_PI_ACTIVITY_LOG\" ) &\n"
        )
    } else {
        String::new()
    };
    let path = bin.join(command);
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{dir}/{command}.launches'\n{draw}{report}\
             exec cat >> '{dir}/{command}.input'\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}
