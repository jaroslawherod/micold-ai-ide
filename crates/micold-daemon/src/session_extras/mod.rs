//! What the configuration adds to every session beyond its git credentials (#1536, FR-036..038):
//! the SSH command and signing key for git, the CA bundle, and the files copied into the home. The
//! same CA bundle and the proxy named in `[session]` also configure the daemon's own HTTPS.

use std::path::{Path, PathBuf};

use micold_core::daemon_config::{
    session_file_refusal, DaemonConfig, Kind, SessionFile, DEFAULT_FILE_MODE,
};

use crate::supervisor::{shell_word, EnvEdit};

/// The variables that point a session's tools at the extra CA bundle (FR-037).
pub(crate) const CA_VARIABLES: [&str; 4] = [
    "SSL_CERT_FILE",
    "NODE_EXTRA_CA_CERTS",
    "GIT_SSL_CAINFO",
    "REQUESTS_CA_BUNDLE",
];

/// What the configuration adds to a session: plain variables and `git config` entries (which join
/// the session's `GIT_CONFIG_COUNT` block).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Extras {
    pub env: Vec<(String, String)>,
    pub git: Vec<(String, String)>,
}

/// The extras of `config`. The git ones apply to a sandbox (container, Kubernetes) only: on a host
/// the user's own git configuration and keys stand.
pub(crate) fn extras(config: &DaemonConfig) -> Extras {
    let mut out = Extras::default();
    if matches!(config.daemon.kind, Kind::Container | Kind::Kubernetes) {
        let git = &config.git;
        if let (Some(key), Some(known)) = (&git.ssh_key_file, &git.known_hosts_file) {
            out.env.push((
                "GIT_SSH_COMMAND".into(),
                format!(
                    "ssh -i {} -o UserKnownHostsFile={} -o IdentitiesOnly=yes",
                    shell_word(&key.path().to_string_lossy()),
                    shell_word(&known.to_string_lossy())
                ),
            ));
        }
        match &git.signing_key_file {
            Some(key) => out.git.push((
                "user.signingkey".into(),
                key.path().to_string_lossy().into(),
            )),
            // The shared copy of the user's configuration may ask for signing, and no key reached
            // the sandbox: signing is off rather than every commit failing.
            None => out.git.extend([
                ("commit.gpgsign".to_string(), "false".to_string()),
                ("tag.gpgsign".to_string(), "false".to_string()),
            ]),
        }
    }
    if let Some(ca) = config.network.as_ref().and_then(|n| n.ca_file.as_ref()) {
        let ca = ca.to_string_lossy().into_owned();
        out.env
            .extend(CA_VARIABLES.map(|name| (name.to_string(), ca.clone())));
    }
    out
}

/// `edits` with `extras` added: the variables as `Set`s, the git entries appended to the
/// `GIT_CONFIG_COUNT` block `edits` already carry (or starting one).
pub(crate) fn add_extras(mut edits: Vec<EnvEdit>, extras: Extras) -> Vec<EnvEdit> {
    let count_at = |edits: &[EnvEdit]| {
        edits
            .iter()
            .position(|e| matches!(e, EnvEdit::Set(key, _) if key == "GIT_CONFIG_COUNT"))
    };
    if !extras.git.is_empty() {
        let at = count_at(&edits);
        let mut count = at
            .and_then(|i| match &edits[i] {
                EnvEdit::Set(_, v) => v.to_str()?.parse::<usize>().ok(),
                EnvEdit::Remove(_) => None,
            })
            .unwrap_or(0);
        for (key, value) in extras.git {
            edits.push(EnvEdit::Set(
                format!("GIT_CONFIG_KEY_{count}").into(),
                key.into(),
            ));
            edits.push(EnvEdit::Set(
                format!("GIT_CONFIG_VALUE_{count}").into(),
                value.into(),
            ));
            count += 1;
        }
        let total = EnvEdit::Set("GIT_CONFIG_COUNT".into(), count.to_string().into());
        match at {
            Some(i) => edits[i] = total,
            None => edits.push(total),
        }
    }
    edits.extend(
        extras
            .env
            .into_iter()
            .map(|(k, v)| EnvEdit::Set(k.into(), v.into())),
    );
    edits
}

/// Copy each `[[session.files]]` source to its place under `home` with its mode. A target the
/// rules of V18 refuse is skipped, as is a file that cannot be copied; each failure is returned
/// for the log (never the file's content).
pub(crate) fn deliver_files(files: &[SessionFile], home: &Path) -> Vec<String> {
    let mut failures = Vec::new();
    for file in files {
        if let Err(why) = deliver_file(file, home) {
            failures.push(format!("{}: {why}", file.as_path.display()));
        }
    }
    failures
}

fn deliver_file(file: &SessionFile, home: &Path) -> Result<(), String> {
    let target = file.as_path.to_string_lossy();
    if let Some(why) = session_file_refusal(&target) {
        return Err(why);
    }
    let mode = u32::from_str_radix(file.mode.as_deref().unwrap_or(DEFAULT_FILE_MODE), 8)
        .map_err(|_| "the mode is not octal".to_string())?;
    let to: PathBuf = home.join(&file.as_path);
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create its directory: {e}"))?;
    }
    // A copy that is already there may be read-only (0400): remove it so the copy can replace it.
    let _ = std::fs::remove_file(&to);
    std::fs::copy(&file.file, &to)
        .map_err(|e| format!("cannot copy {}: {e}", file.file.display()))?;
    set_mode(&to, mode)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| format!("cannot set its mode: {e}"))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

/// The proxy and CA bundle the daemon's own HTTPS honours (FR-037).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Egress {
    pub ca_file: Option<PathBuf>,
    pub https_proxy: Option<String>,
    pub no_proxy: Option<String>,
}

impl Egress {
    /// `HTTPS_PROXY`, `NO_PROXY` and the CA file from `config`'s `[session]` (a `secret_env` file's
    /// first line) and `[network]`.
    pub(crate) fn of(config: &DaemonConfig) -> Self {
        let value = |name: &str| {
            let session = &config.session;
            session.env.get(name).cloned().or_else(|| {
                let text = std::fs::read_to_string(session.secret_env.get(name)?.path()).ok()?;
                Some(text.lines().next()?.trim().to_string())
            })
        };
        let pick = |upper: &str, lower: &str| value(upper).or_else(|| value(lower));
        Self {
            ca_file: config.network.as_ref().and_then(|n| n.ca_file.clone()),
            https_proxy: pick("HTTPS_PROXY", "https_proxy").filter(|v| !v.is_empty()),
            no_proxy: pick("NO_PROXY", "no_proxy"),
        }
    }

    /// The daemon's egress for the configuration file at `path`: nothing when it cannot be read.
    pub(crate) fn load(path: Option<&Path>) -> Self {
        path.and_then(|p| crate::config_load::load_file(p).ok())
            .map(|loaded| Self::of(&loaded.config))
            .unwrap_or_default()
    }

    /// Whether a request to `host` skips the proxy: `NO_PROXY` lists it, a domain it falls under
    /// (`.corp` or `corp`), or `*`.
    pub(crate) fn bypasses_proxy(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        self.no_proxy
            .as_deref()
            .unwrap_or("")
            .split(',')
            .any(|entry| {
                let entry = entry.trim().trim_start_matches('.').to_ascii_lowercase();
                entry == "*"
                    || (!entry.is_empty()
                        && (host == entry || host.ends_with(&format!(".{entry}"))))
            })
    }

    /// The `ureq` agent for these settings; `base` is the builder the caller has configured.
    pub(crate) fn agent(
        &self,
        base: ureq::config::ConfigBuilder<ureq::typestate::AgentScope>,
        host: &str,
    ) -> ureq::Agent {
        let mut builder = base;
        if let Some(pem) = self.ca_file.as_ref().and_then(|p| std::fs::read(p).ok()) {
            let certs: Vec<_> = ureq::tls::parse_pem(&pem)
                .filter_map(Result::ok)
                .filter_map(|item| match item {
                    ureq::tls::PemItem::Certificate(cert) => Some(cert),
                    _ => None,
                })
                .collect();
            if !certs.is_empty() {
                builder = builder.tls_config(
                    ureq::tls::TlsConfig::builder()
                        .root_certs(ureq::tls::RootCerts::Specific(std::sync::Arc::new(certs)))
                        .build(),
                );
            }
        }
        let proxy = self
            .https_proxy
            .as_deref()
            .filter(|_| !self.bypasses_proxy(host))
            .and_then(|url| ureq::Proxy::new(url).ok());
        builder.proxy(proxy).build().into()
    }
}

#[cfg(test)]
mod tests;
