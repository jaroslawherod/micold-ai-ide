//! Parsing and the pure validation rules (V1–V13, V16, V18, V19). The rules that need the
//! filesystem (V14, V15, V17) are in [`super::files`].
//!
//! Every finding carries `file:line:column`. Positions come from the TOML document's spans: a
//! finding is raised against a key path such as `github.tokens[1].token_file`, which [`Doc`]
//! resolves to the value's (or the key's) span.

use std::ops::Range;

use toml::de::{DeTable, DeValue};
use toml::Spanned;

use super::{
    AiCli, CliId, DaemonConfig, Kind, ListenTable, Session, SocketSpec, DEFAULT_FILE_MODE,
    OLDEST_SCHEMA, SCHEMA_VERSION,
};

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// One finding, located in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub origin: String,
    pub line: usize,
    pub column: usize,
    /// The rule: `V1`..`V19` of the data model, or `syntax` / `type` for a file that is not
    /// well-formed.
    pub rule: &'static str,
    /// The key path the finding is about, e.g. `github.tokens[1].token_file`.
    pub key: String,
    pub message: String,
    pub severity: Severity,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let warning = match self.severity {
            Severity::Error => "",
            Severity::Warning => "warning: ",
        };
        write!(
            f,
            "{}:{}:{}: {warning}{}",
            self.origin, self.line, self.column, self.message
        )
    }
}

/// Parses and validates `text`, the content of `origin`. Returns the configuration, or every
/// error found. Warnings are available through [`diagnose`].
pub fn parse(text: &str, origin: &str) -> Result<DaemonConfig, Vec<ConfigError>> {
    let (config, findings) = diagnose(text, origin);
    match config {
        Some(config) => Ok(config),
        None => Err(findings),
    }
}

/// Like [`parse`], but also returns the warnings. The configuration is `Some` exactly when there
/// is no finding of severity [`Severity::Error`].
pub fn diagnose(text: &str, origin: &str) -> (Option<DaemonConfig>, Vec<ConfigError>) {
    let doc = match DeTable::parse(text) {
        Ok(doc) => Doc { text, origin, doc },
        Err(error) => {
            let mut sink = Vec::new();
            let doc = Doc::empty(text, origin);
            doc.push(&mut sink, "syntax", "", error.message(), error.span());
            return (None, sink);
        }
    };
    let mut findings = Vec::new();
    if !doc.check_schema(&mut findings) {
        return (None, findings);
    }
    let config: DaemonConfig = match toml::from_str(text) {
        Ok(config) => config,
        Err(error) => {
            doc.structural(&mut findings, &error);
            return (None, findings);
        }
    };
    Checker {
        doc: &doc,
        config: &config,
        out: &mut findings,
    }
    .run();
    let failed = findings.iter().any(|f| f.severity == Severity::Error);
    (if failed { None } else { Some(config) }, findings)
}

/// The parsed document with its spans, for locating findings.
pub(super) struct Doc<'a> {
    text: &'a str,
    origin: &'a str,
    doc: Spanned<DeTable<'a>>,
}

enum Seg<'p> {
    Name(&'p str),
    Index(usize),
}

fn segments(path: &str) -> Vec<Seg<'_>> {
    let mut out = Vec::new();
    for part in path.split('.') {
        let (name, mut rest) = match part.find('[') {
            Some(at) => (&part[..at], &part[at..]),
            None => (part, ""),
        };
        if !name.is_empty() {
            out.push(Seg::Name(name));
        }
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            if let Ok(index) = stripped[..end].parse() {
                out.push(Seg::Index(index));
            }
            rest = &stripped[end + 1..];
        }
    }
    out
}

impl<'a> Doc<'a> {
    fn empty(text: &'a str, origin: &'a str) -> Self {
        Self {
            text,
            origin,
            doc: Spanned::new(0..0, DeTable::new()),
        }
    }

    /// Parses `text` for locating findings; `None` when it is not well-formed TOML.
    pub(super) fn parse(text: &'a str, origin: &'a str) -> Option<Self> {
        DeTable::parse(text)
            .ok()
            .map(|doc| Self { text, origin, doc })
    }

    /// A finding at the value of `key`.
    pub(super) fn finding(
        &self,
        rule: &'static str,
        key: &str,
        severity: Severity,
        message: String,
    ) -> ConfigError {
        let mut out = Vec::new();
        self.push_with(
            &mut out,
            rule,
            key,
            &message,
            self.locate(key, false),
            severity,
        );
        out.remove(0)
    }

    /// `(line, column)`, both 1-based, of a byte offset.
    fn position(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let before = &self.text[..offset];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        (line, column)
    }

    fn push(
        &self,
        out: &mut Vec<ConfigError>,
        rule: &'static str,
        key: &str,
        message: &str,
        span: Option<Range<usize>>,
    ) {
        self.push_with(out, rule, key, message, span, Severity::Error);
    }

    fn push_with(
        &self,
        out: &mut Vec<ConfigError>,
        rule: &'static str,
        key: &str,
        message: &str,
        span: Option<Range<usize>>,
        severity: Severity,
    ) {
        let (line, column) = self.position(span.map_or(0, |s| s.start));
        out.push(ConfigError {
            origin: self.origin.to_string(),
            line,
            column,
            rule,
            key: key.to_string(),
            message: message.to_string(),
            severity,
        });
    }

    /// The span of the key at `path` (`want_key`) or of its value; for a path that does not
    /// exist, of the longest prefix that does.
    pub(super) fn locate(&self, path: &str, want_key: bool) -> Option<Range<usize>> {
        let mut table = Some(self.doc.get_ref());
        let mut value: Option<&DeValue<'_>> = None;
        let mut key_span = None;
        let mut value_span = None;
        for seg in segments(path) {
            match seg {
                Seg::Name(name) => {
                    let Some(table) = table.or_else(|| value.and_then(DeValue::as_table)) else {
                        break;
                    };
                    let Some((k, v)) = table.get_key_value(name) else {
                        break;
                    };
                    key_span = Some(k.span());
                    value_span = Some(v.span());
                    value = Some(v.get_ref());
                }
                Seg::Index(index) => {
                    let Some(item) = value.and_then(DeValue::as_array).and_then(|a| a.get(index))
                    else {
                        break;
                    };
                    value_span = Some(item.span());
                    value = Some(item.get_ref());
                }
            }
            table = None;
        }
        if want_key {
            key_span.or(value_span)
        } else {
            value_span.or(key_span)
        }
    }

    /// The line of the value at `path`, for messages that name another entry.
    pub(super) fn line_of(&self, path: &str) -> usize {
        self.position(self.locate(path, false).map_or(0, |s| s.start))
            .0
    }

    /// V2: the schema is present and supported. False when the rest must not be read.
    fn check_schema(&self, out: &mut Vec<ConfigError>) -> bool {
        let Some(value) = self.doc.get_ref().get("schema") else {
            self.push(
                out,
                "V2",
                "schema",
                &format!(
                    "`schema` is missing (this daemon reads schema {OLDEST_SCHEMA} to {SCHEMA_VERSION})"
                ),
                Some(0..0),
            );
            return false;
        };
        let found = value
            .get_ref()
            .as_integer()
            .and_then(|i| i.as_str().parse::<i64>().ok());
        match found {
            Some(n) if n > i64::from(SCHEMA_VERSION) => {
                self.push(
                    out,
                    "V2",
                    "schema",
                    &format!(
                        "this configuration is schema {n}; this daemon reads up to schema {SCHEMA_VERSION}; update the daemon"
                    ),
                    Some(value.span()),
                );
                false
            }
            Some(n) if n >= i64::from(OLDEST_SCHEMA) => true,
            _ => {
                self.push(
                    out,
                    "V2",
                    "schema",
                    &format!(
                        "`schema` must be an integer from {OLDEST_SCHEMA} to {SCHEMA_VERSION}"
                    ),
                    Some(value.span()),
                );
                false
            }
        }
    }

    /// A deserialization failure: V1 for an unknown key, otherwise a type or missing-key error.
    fn structural(&self, out: &mut Vec<ConfigError>, error: &toml::de::Error) {
        let message = error.message();
        let span = error.span();
        if let Some(rest) = message.strip_prefix("unknown field `") {
            let key = rest.split('`').next().unwrap_or_default();
            let table = span
                .as_ref()
                .and_then(|s| table_of_key(self.doc.get_ref(), "", false, s.start))
                .unwrap_or_else(|| "the top level".to_string());
            self.push(
                out,
                "V1",
                key,
                &format!("unknown key \"{key}\" in {table}"),
                span,
            );
        } else {
            self.push(out, "type", "", message, span);
        }
    }
}

/// The table (`[a.b]`, `[[a.b]]` or "the top level") that holds a key starting at `start`.
fn table_of_key(table: &DeTable<'_>, prefix: &str, array: bool, start: usize) -> Option<String> {
    for (key, value) in table {
        if key.span().start == start {
            return Some(match (prefix.is_empty(), array) {
                (true, _) => "the top level".to_string(),
                (false, true) => format!("[[{prefix}]]"),
                (false, false) => format!("[{prefix}]"),
            });
        }
        let name = if prefix.is_empty() {
            key.get_ref().to_string()
        } else {
            format!("{prefix}.{}", key.get_ref())
        };
        match value.get_ref() {
            DeValue::Table(inner) => {
                if let Some(found) = table_of_key(inner, &name, false, start) {
                    return Some(found);
                }
            }
            DeValue::Array(items) => {
                for item in items {
                    if let DeValue::Table(inner) = item.get_ref() {
                        if let Some(found) = table_of_key(inner, &name, true, start) {
                            return Some(found);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// A path is absolute on any platform: `/x`, `\x` or `C:\x` / `C:/x`. `~` is not expanded.
pub(super) fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || path.starts_with('\\')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && (bytes[2] == b'/' || bytes[2] == b'\\'))
}

fn components(path: &str) -> Vec<&str> {
    path.split(['/', '\\']).filter(|c| !c.is_empty()).collect()
}

/// A relative path that stays inside its base: no `..`, not absolute, not empty.
fn stays_inside(path: &str) -> bool {
    !path.is_empty() && !is_absolute(path) && !components(path).contains(&"..")
}

fn is_under(path: &str, root: &str) -> bool {
    let (path, root) = (components(path), components(root));
    path.len() >= root.len() && path[..root.len()] == root[..]
}

const RESERVED_NAMES: &[&str] = &[
    "PATH",
    "HOME",
    "TERM",
    "COLORTERM",
    "GIT_SSH_COMMAND",
    "GH_TOKEN",
    "GH_HOST",
    "GITHUB_TOKEN",
    "CLAUDE_CONFIG_DIR",
    "CODEX_HOME",
    "COPILOT_HOME",
    "PI_CODING_AGENT_DIR",
    "XDG_DATA_HOME",
];

fn is_reserved(name: &str) -> bool {
    RESERVED_NAMES.contains(&name) || name.starts_with("GIT_CONFIG_") || name.starts_with("MICOLD_")
}

/// The variables a CLI's key or token file is delivered through.
fn key_variables(cli: &AiCli) -> Vec<&'static str> {
    let mut out = Vec::new();
    if cli.api_key_file.is_some() {
        match cli.id {
            CliId::ClaudeCode => out.push("ANTHROPIC_API_KEY"),
            CliId::Codex | CliId::Opencode => out.push("OPENAI_API_KEY"),
            CliId::Copilot | CliId::Pi => {}
        }
    }
    if cli.token_file.is_some() {
        match cli.id {
            CliId::ClaudeCode => out.push("CLAUDE_CODE_OAUTH_TOKEN"),
            CliId::Copilot => out.push("COPILOT_GITHUB_TOKEN"),
            _ => {}
        }
    }
    out
}

/// Arguments the daemon owns on every launch (research D9).
const DAEMON_ARGS: &[&str] = &["--session-id", "--resume", "--settings", "--mcp-config"];

/// Where each CLI keeps its configuration, relative to the home (V18).
const CLI_CONFIG_DIRS: &[&str] = &[
    ".claude",
    ".claude.json",
    ".codex",
    ".copilot",
    ".pi",
    ".config/opencode",
    ".local/share/opencode",
];

fn version_ok(version: &str) -> bool {
    let mut parts = version.splitn(3, '.');
    let (Some(major), Some(minor), Some(tail)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let patch = tail.bytes().take_while(u8::is_ascii_digit).count();
    if !(digits(major) && digits(minor) && patch > 0) {
        return false;
    }
    let rest = &tail[patch..];
    rest.is_empty()
        || (rest.starts_with(['-', '.'])
            && rest.len() > 1
            && rest[1..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.'))
}

struct Checker<'a, 'd> {
    doc: &'a Doc<'d>,
    config: &'a DaemonConfig,
    out: &'a mut Vec<ConfigError>,
}

impl Checker<'_, '_> {
    fn run(&mut self) {
        self.listen();
        self.paths();
        self.duplicates();
        self.projects();
        let config = self.config;
        for (index, cli) in config.ai.cli.iter().enumerate() {
            self.cli(index, cli);
        }
        self.session(&config.session);
        self.startup();
        self.git();
        self.tokens();
    }

    fn at(&mut self, rule: &'static str, path: &str, message: String) {
        let span = self.doc.locate(path, false);
        self.doc.push(self.out, rule, path, &message, span);
    }

    fn at_key(&mut self, rule: &'static str, path: &str, message: String) {
        let span = self.doc.locate(path, true);
        self.doc.push(self.out, rule, path, &message, span);
    }

    fn warn(&mut self, rule: &'static str, path: &str, message: String) {
        let span = self.doc.locate(path, true);
        self.doc
            .push_with(self.out, rule, path, &message, span, Severity::Warning);
    }

    fn kind(&self) -> Kind {
        self.config.daemon.kind
    }

    /// V3.
    fn listen(&mut self) {
        let sandbox = self.kind().is_sandbox();
        let Some(table) = &self.config.listen else {
            if sandbox {
                self.at_key(
                    "V3",
                    "daemon",
                    "[listen] is required for a container or kubernetes daemon: set `tcp`"
                        .to_string(),
                );
            }
            return;
        };
        let ListenTable { socket, tcp } = table;
        match (socket.is_some(), tcp.is_some()) {
            (false, false) | (true, true) => {
                self.at_key(
                    "V3",
                    "listen",
                    "[listen] must have exactly one key: `socket` or `tcp`".to_string(),
                );
            }
            (true, false) if sandbox => self.at_key(
                "V3",
                "listen.socket",
                "`listen.socket` is for host and ssh daemons; a container or kubernetes daemon uses `tcp`"
                    .to_string(),
            ),
            (false, true) if !sandbox => self.at_key(
                "V3",
                "listen.tcp",
                "`listen.tcp` is for container and kubernetes daemons; a host or ssh daemon uses `socket`"
                    .to_string(),
            ),
            (false, true) => {
                let has_token = self
                    .config
                    .auth
                    .as_ref()
                    .is_some_and(|auth| auth.client_token_file.is_some());
                if !has_token {
                    self.at_key(
                        "V3",
                        "listen.tcp",
                        "`listen.tcp` requires `auth.client_token_file`".to_string(),
                    );
                }
            }
            _ => {}
        }
    }

    /// V4: every path is absolute.
    fn paths(&mut self) {
        let c = self.config;
        let mut paths: Vec<(String, String)> = Vec::new();
        let mut add = |key: String, path: &std::path::Path| {
            paths.push((key, path.to_string_lossy().into_owned()));
        };
        if let Some(p) = &c.daemon.data_dir {
            add("daemon.data_dir".into(), p);
        }
        if let Some(p) = &c.daemon.home {
            add("daemon.home".into(), p);
        }
        if let Some(ListenTable {
            socket: Some(SocketSpec::Path(p)),
            ..
        }) = &c.listen
        {
            add("listen.socket".into(), p);
        }
        if let Some(p) = c.auth.as_ref().and_then(|a| a.client_token_file.as_ref()) {
            add("auth.client_token_file".into(), p.path());
        }
        if let Some(p) = c.ssh.as_ref().and_then(|s| s.dir.as_ref()) {
            add("ssh.dir".into(), p);
        }
        if let Some(p) = c.storage.as_ref().and_then(|s| s.clone_root.as_ref()) {
            add("storage.clone_root".into(), p);
        }
        for (i, project) in c.projects.iter().enumerate() {
            add(format!("projects[{i}].path"), &project.path);
        }
        if let Some(p) = &c.git.config_file {
            add("git.config_file".into(), p);
        }
        if let Some(p) = &c.git.ssh_key_file {
            add("git.ssh_key_file".into(), p.path());
        }
        if let Some(p) = &c.git.known_hosts_file {
            add("git.known_hosts_file".into(), p);
        }
        if let Some(p) = &c.git.signing_key_file {
            add("git.signing_key_file".into(), p.path());
        }
        for (i, token) in c.github.tokens.iter().enumerate() {
            add(
                format!("github.tokens[{i}].token_file"),
                token.token_file.path(),
            );
        }
        for (i, cli) in c.ai.cli.iter().enumerate() {
            let base = format!("ai.cli[{i}]");
            for (key, secret) in [
                ("sign_in_file", &cli.sign_in_file),
                ("token_file", &cli.token_file),
                ("api_key_file", &cli.api_key_file),
            ] {
                if let Some(p) = secret {
                    add(format!("{base}.{key}"), p.path());
                }
            }
            if let Some(p) = &cli.settings_file {
                add(format!("{base}.settings_file"), p);
            }
            for (j, place) in cli.place.iter().enumerate() {
                add(format!("{base}.place[{j}].file"), &place.file);
            }
        }
        for (name, secret) in &c.session.secret_env {
            add(format!("session.secret_env.{name}"), secret.path());
        }
        for (i, file) in c.session.files.iter().enumerate() {
            add(format!("session.files[{i}].file"), &file.file);
        }
        if let Some(p) = c.network.as_ref().and_then(|n| n.ca_file.as_ref()) {
            add("network.ca_file".into(), p);
        }
        if let Some(startup) = &c.startup {
            add("startup.script_file".into(), &startup.script_file);
            if let Some(p) = &startup.shell {
                add("startup.shell".into(), p);
            }
            if let Some(p) = &startup.env_file {
                add("startup.env_file".into(), p.path());
            }
        }
        for (key, path) in paths {
            if !is_absolute(&path) {
                self.at(
                    "V4",
                    &key,
                    format!(
                        "`{key}` must be an absolute path, got \"{path}\" (`~` is not expanded)"
                    ),
                );
            }
        }
    }

    /// V5.
    fn duplicates(&mut self) {
        let c = self.config;
        let names = c.github.tokens.iter().map(|t| t.name.clone());
        self.duplicates_of("github.tokens", "name", names);
        let ids = c.ai.cli.iter().map(|cli| cli.id.as_str().to_string());
        self.duplicates_of("ai.cli", "id", ids);
        let paths = c
            .projects
            .iter()
            .map(|p| p.path.to_string_lossy().into_owned());
        self.duplicates_of("projects", "path", paths);
    }

    fn duplicates_of(&mut self, table: &str, field: &str, values: impl Iterator<Item = String>) {
        let mut seen: Vec<String> = Vec::new();
        for (index, value) in values.enumerate() {
            if let Some(first) = seen.iter().position(|s| *s == value) {
                let line = self.doc.line_of(&format!("{table}[{first}]"));
                self.at(
                    "V5",
                    &format!("{table}[{index}].{field}"),
                    format!(
                        "duplicate {table}.{field} \"{value}\": already defined by entry {} at line {line}",
                        first + 1
                    ),
                );
            }
            seen.push(value);
        }
    }

    /// V12.
    fn projects(&mut self) {
        let config = self.config;
        let clone_root = config
            .storage
            .as_ref()
            .and_then(|s| s.clone_root.as_ref())
            .map(|p| p.to_string_lossy().into_owned());
        for (index, project) in config.projects.iter().enumerate() {
            if project.remote.is_none() {
                continue;
            }
            let path = project.path.to_string_lossy().into_owned();
            let key = format!("projects[{index}].path");
            match &clone_root {
                None => self.at(
                    "V12",
                    &key,
                    format!(
                        "project \"{path}\" has a `remote` but `storage.clone_root` is not set"
                    ),
                ),
                Some(root) if !is_under(&path, root) => self.at(
                    "V12",
                    &key,
                    format!("project \"{path}\" is not under `storage.clone_root` \"{root}\""),
                ),
                Some(_) => {}
            }
        }
    }

    /// V7–V10, V16 (CLI env), V19.
    fn cli(&mut self, index: usize, cli: &AiCli) {
        let id = cli.id.as_str();
        let base = format!("ai.cli[{index}]");
        let sandbox = self.kind().is_sandbox();
        if cli.model.is_some() && cli.id != CliId::ClaudeCode {
            self.at(
                "V7",
                &format!("{base}.model"),
                format!("`model` is not supported by {id}"),
            );
        }
        if cli.api_base_url.is_some() && !matches!(cli.id, CliId::ClaudeCode | CliId::Codex) {
            self.at(
                "V7",
                &format!("{base}.api_base_url"),
                format!("`api_base_url` is not supported by {id}: it has no endpoint variable"),
            );
        }
        if cli.api_key_file.is_some() && matches!(cli.id, CliId::Copilot | CliId::Pi) {
            self.at(
                "V7",
                &format!("{base}.api_key_file"),
                format!("`api_key_file` is not supported by {id}: it has no key variable (use `session.secret_env` for pi, `token_file` for copilot)"),
            );
        }
        if cli.sign_in_file.is_some() && matches!(cli.id, CliId::Copilot | CliId::Pi) {
            self.at(
                "V7",
                &format!("{base}.sign_in_file"),
                format!("`sign_in_file` is not supported by {id}: it has no shareable sign-in"),
            );
        }
        if let Some(version) = &cli.version {
            if !version_ok(version) {
                self.at(
                    "V7",
                    &format!("{base}.version"),
                    format!("`version` \"{version}\" is not a version number"),
                );
            } else if self.kind() == Kind::Host {
                self.warn(
                    "V7",
                    &format!("{base}.version"),
                    format!("`version` of {id} is ignored on a host daemon"),
                );
            }
        }
        self.args(&base, cli);
        let credentials = [&cli.sign_in_file, &cli.token_file, &cli.api_key_file]
            .iter()
            .filter(|c| c.is_some())
            .count();
        if credentials > 1 {
            self.at(
                "V9",
                &base,
                format!("{id}: at most one of `sign_in_file`, `token_file`, `api_key_file`"),
            );
        }
        if cli.token_file.is_some() && !matches!(cli.id, CliId::ClaudeCode | CliId::Copilot) {
            self.at(
                "V9",
                &format!("{base}.token_file"),
                format!("`token_file` is only for claude_code and copilot, not {id}"),
            );
        }
        if !sandbox {
            if cli.settings_file.is_some() {
                self.at(
                    "V10",
                    &format!("{base}.settings_file"),
                    format!("`settings_file` of {id} is refused on a host or ssh daemon: the user's own files are the configuration there"),
                );
            }
            if !cli.place.is_empty() {
                self.at(
                    "V10",
                    &format!("{base}.place"),
                    format!("`place` of {id} is refused on a host or ssh daemon"),
                );
            }
        }
        for (j, place) in cli.place.iter().enumerate() {
            let target = place.as_path.to_string_lossy();
            if !stays_inside(&target) {
                self.at(
                    "V10",
                    &format!("{base}.place[{j}].as"),
                    format!("`place.as` \"{target}\" must stay inside the {id} config dir"),
                );
            }
        }
        if cli.use_machine_sign_in == Some(true) {
            if sandbox {
                self.at(
                    "V19",
                    &format!("{base}.use_machine_sign_in"),
                    format!("{id}: `use_machine_sign_in` is for host and ssh daemons only"),
                );
            }
            if credentials > 0 {
                self.at(
                    "V19",
                    &format!("{base}.use_machine_sign_in"),
                    format!("{id}: `use_machine_sign_in` excludes `sign_in_file`, `token_file` and `api_key_file`"),
                );
            }
        }
        let own = key_variables(cli);
        for name in cli.env.keys() {
            if is_reserved(name) || own.contains(&name.as_str()) {
                self.at(
                    "V16",
                    &format!("{base}.env.{name}"),
                    format!("`{name}` is reserved and cannot be set in `ai.cli.env` of {id}"),
                );
            }
        }
    }

    /// V8.
    fn args(&mut self, base: &str, cli: &AiCli) {
        let structured: Vec<&str> = cli
            .id
            .ai_cli()
            .provider()
            .structured_args()
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|a| a.starts_with('-'))
            .collect();
        for (index, arg) in cli.args.iter().enumerate() {
            let flag = arg.split('=').next().unwrap_or(arg);
            if DAEMON_ARGS.contains(&flag) || structured.contains(&flag) {
                self.at(
                    "V8",
                    &format!("{base}.args[{index}]"),
                    format!(
                        "argument #{} `{arg}` of {} is owned by the daemon and cannot be set in `args`",
                        index + 1,
                        cli.id.as_str()
                    ),
                );
            }
        }
    }

    /// V16, V18 for `[session]`.
    fn session(&mut self, session: &Session) {
        let cli_vars: Vec<&str> = self.config.ai.cli.iter().flat_map(key_variables).collect();
        let names = session
            .env
            .keys()
            .map(|n| ("session.env", n))
            .chain(session.secret_env.keys().map(|n| ("session.secret_env", n)));
        for (table, name) in names {
            if is_reserved(name) || cli_vars.contains(&name.as_str()) {
                self.at(
                    "V16",
                    &format!("{table}.{name}"),
                    format!("`{name}` is reserved and cannot be set in `{table}`"),
                );
            }
        }
        let mut targets: Vec<String> = Vec::new();
        for (index, file) in session.files.iter().enumerate() {
            let base = format!("session.files[{index}]");
            let target = file.as_path.to_string_lossy().into_owned();
            let normalized = components(&target)
                .into_iter()
                .filter(|c| *c != ".")
                .collect::<Vec<_>>()
                .join("/")
                .to_lowercase();
            if !stays_inside(&target) {
                self.at(
                    "V18",
                    &format!("{base}.as"),
                    format!(
                        "`session.files.as` \"{target}\" must be relative and stay inside the home"
                    ),
                );
            } else if let Some(dir) = CLI_CONFIG_DIRS.iter().find(|d| is_under(&normalized, d)) {
                self.at(
                    "V18",
                    &format!("{base}.as"),
                    format!("`session.files.as` \"{target}\" is inside a CLI config dir (~/{dir})"),
                );
            } else if let Some(first) = targets.iter().position(|t| *t == normalized) {
                self.at(
                    "V18",
                    &format!("{base}.as"),
                    format!(
                        "`session.files.as` \"{target}\" collides with entry {}",
                        first + 1
                    ),
                );
            }
            targets.push(normalized);
            let mode = file.mode.as_deref().unwrap_or(DEFAULT_FILE_MODE);
            let octal =
                (3..=4).contains(&mode.len()) && mode.chars().all(|c| ('0'..='7').contains(&c));
            if !octal {
                self.at(
                    "V18",
                    &format!("{base}.mode"),
                    format!(
                        "`session.files.mode` \"{mode}\" must be an octal mode such as \"0600\""
                    ),
                );
            }
        }
    }

    /// V13.
    fn startup(&mut self) {
        let config = self.config;
        let Some(startup) = &config.startup else {
            return;
        };
        if let Some(secs) = startup.timeout_secs {
            if !(1..=3600).contains(&secs) {
                self.at(
                    "V13",
                    "startup.timeout_secs",
                    format!("`startup.timeout_secs` {secs} is out of range 1 to 3600"),
                );
            }
        }
    }

    /// V18 for `[git]`.
    fn git(&mut self) {
        let git = &self.config.git;
        let (ssh_key, known_hosts) = (git.ssh_key_file.is_some(), git.known_hosts_file.is_none());
        if ssh_key && known_hosts {
            self.at_key(
                "V18",
                "git.ssh_key_file",
                "`git.ssh_key_file` requires `git.known_hosts_file`".to_string(),
            );
        }
    }

    fn tokens(&mut self) {
        let config = self.config;
        for (index, token) in config.github.tokens.iter().enumerate() {
            let Some(host) = &token.host else { continue };
            let url = host.contains("://");
            let valid = !host.is_empty()
                && !host.contains(char::is_whitespace)
                && (!url || host.starts_with("https://") || host.starts_with("http://"));
            if !valid {
                self.at(
                    "type",
                    &format!("github.tokens[{index}].host"),
                    format!("`host` \"{host}\" must be a host name or an http(s) base URL"),
                );
            }
        }
    }
}
