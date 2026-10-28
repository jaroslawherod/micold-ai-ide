//! The rules that need the filesystem: V14 (secret file modes), V15 (a required file is missing)
//! and V17 (a placed MCP config defining a server `micold`).
//!
//! Separate from [`super::parse`] because the application validates a file before the files it
//! names exist on the daemon's machine; the daemon and `config check --files` run both.

use std::path::Path;

use super::validate::Doc;
use super::{ConfigError, DaemonConfig, Severity};

/// Checks the files `config` names. `text` and `origin` are the file's content and name, for
/// positions.
pub fn check_files(text: &str, origin: &str, config: &DaemonConfig) -> Vec<ConfigError> {
    let Some(doc) = Doc::parse(text, origin) else {
        return Vec::new();
    };
    let sandbox = config.daemon.kind.is_sandbox();
    let mut out = Vec::new();
    let mut push = |rule: &'static str, key: &str, severity: Severity, message: String| {
        out.push(doc.finding(rule, key, severity, message));
    };

    // V15: required non-secret files. A missing secret is reported unavailable, not refused.
    let mut required: Vec<(String, &Path)> = Vec::new();
    if let Some(p) = config
        .auth
        .as_ref()
        .and_then(|a| a.client_token_file.as_ref())
    {
        required.push(("auth.client_token_file".into(), p.path()));
    }
    if let Some(startup) = &config.startup {
        required.push(("startup.script_file".into(), &startup.script_file));
    }
    for (i, cli) in config.ai.cli.iter().enumerate() {
        if let Some(p) = &cli.settings_file {
            required.push((format!("ai.cli[{i}].settings_file"), p));
        }
        for (j, place) in cli.place.iter().enumerate() {
            required.push((format!("ai.cli[{i}].place[{j}].file"), &place.file));
        }
    }
    for (key, path) in required {
        if !path.is_file() {
            let shown = path.display();
            push(
                "V15",
                &key,
                Severity::Error,
                format!("`{key}`: file {shown} does not exist"),
            );
        }
    }

    // V14: a secret must be owner-only.
    let mut secrets: Vec<(String, &Path)> = Vec::new();
    if let Some(p) = config
        .auth
        .as_ref()
        .and_then(|a| a.client_token_file.as_ref())
    {
        secrets.push(("auth.client_token_file".into(), p.path()));
    }
    if let Some(p) = &config.git.ssh_key_file {
        secrets.push(("git.ssh_key_file".into(), p.path()));
    }
    if let Some(p) = &config.git.signing_key_file {
        secrets.push(("git.signing_key_file".into(), p.path()));
    }
    for (i, token) in config.github.tokens.iter().enumerate() {
        secrets.push((
            format!("github.tokens[{i}].token_file"),
            token.token_file.path(),
        ));
    }
    for (i, cli) in config.ai.cli.iter().enumerate() {
        for (key, secret) in [
            ("sign_in_file", &cli.sign_in_file),
            ("token_file", &cli.token_file),
            ("api_key_file", &cli.api_key_file),
        ] {
            if let Some(p) = secret {
                secrets.push((format!("ai.cli[{i}].{key}"), p.path()));
            }
        }
    }
    for (name, secret) in &config.session.secret_env {
        secrets.push((format!("session.secret_env.{name}"), secret.path()));
    }
    if let Some(p) = config.startup.as_ref().and_then(|s| s.env_file.as_ref()) {
        secrets.push(("startup.env_file".into(), p.path()));
    }
    for (key, path) in secrets {
        match super::resolved::mode_of(path) {
            Some(Some(mode)) if mode & 0o077 != 0 => {
                let severity = if sandbox {
                    Severity::Error
                } else {
                    Severity::Warning
                };
                push(
                    "V14",
                    &key,
                    severity,
                    format!(
                        "`{key}`: secret file {} has mode {mode:04o}; it must be owner-only (0600)",
                        path.display()
                    ),
                );
            }
            _ => {}
        }
    }

    // V17: a placed MCP config must not define the server `micold`.
    for (i, cli) in config.ai.cli.iter().enumerate() {
        for (j, place) in cli.place.iter().enumerate() {
            if crate::mcp::binding::file_names_micold(&place.file) {
                push(
                    "V17",
                    &format!("ai.cli[{i}].place[{j}].file"),
                    Severity::Error,
                    format!(
                        "placed file {} defines a server named \"{}\", which the tool server binds itself",
                        place.file.display(),
                        crate::mcp::jsonrpc::SERVER_NAME
                    ),
                );
            }
        }
    }
    out
}
