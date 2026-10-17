//! The hosts of the user's SSH configuration and the effective settings `ssh -G` reports for one
//! (feature 489, FR-002, R12). Parsed and never written: the picker only reads.

use std::path::Path;

/// What `ssh -G <host>` resolved for one host: the settings the connection will really use.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectiveSsh {
    /// The `hostname` the alias resolves to (the alias itself when the file sets none).
    pub hostname: String,
    /// The effective remote user.
    pub user: Option<String>,
    /// The effective port.
    pub port: Option<u16>,
    /// The effective `ConnectTimeout` in seconds; `None` when unset or `none`.
    pub connect_timeout_secs: Option<u32>,
}

/// Who a daemon connects as: (hostname, user, port). Two entries with one identity are one machine
/// and one account (FR-016).
pub type SshIdentity = (String, Option<String>, u16);

impl EffectiveSsh {
    /// The seconds an attempt may take: the user's `ConnectTimeout`, else the default (FR-009).
    pub fn timeout_secs(&self) -> u32 {
        self.connect_timeout_secs
            .filter(|s| *s > 0)
            .unwrap_or(super::argv::DEFAULT_CONNECT_TIMEOUT_SECS)
    }

    /// The identity this resolution stands for.
    pub fn identity(&self) -> SshIdentity {
        (
            self.hostname.to_lowercase(),
            self.user.clone(),
            self.port.unwrap_or(22),
        )
    }
}

/// The identity of a target as entered, when nothing resolved it: the host as typed.
pub fn literal_identity(target: &super::SshTarget) -> SshIdentity {
    (
        target.host.to_lowercase(),
        target.user.clone(),
        target.port.unwrap_or(22),
    )
}

/// Parse the output of `ssh -G`: lines of `keyword value`, keywords lower-case.
pub fn parse_g(output: &str) -> EffectiveSsh {
    let mut out = EffectiveSsh::default();
    for line in output.lines() {
        let Some((key, value)) = line.trim().split_once(char::is_whitespace) else {
            continue;
        };
        let value = value.trim();
        match key {
            "hostname" => out.hostname = value.to_string(),
            "user" if !value.is_empty() => out.user = Some(value.to_string()),
            "port" => out.port = value.parse().ok(),
            "connecttimeout" => out.connect_timeout_secs = value.parse().ok(),
            _ => {}
        }
    }
    out
}

/// How long `ssh -G` may take: it only reads files, so a slow answer means something is wrong.
const RESOLVE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

/// Ask `ssh -G` what the user's configuration gives `target`. `None` when it could not be asked
/// (no `ssh`, a failure, no hostname in the answer): callers fall back to the host as entered and
/// to the default timeout, so a broken configuration never blocks an attempt that would then
/// report its own, named failure.
pub async fn resolve<R: super::SshRunner + ?Sized>(
    runner: &R,
    target: &super::SshTarget,
    platform: super::ClientPlatform,
) -> Option<EffectiveSsh> {
    let cmd = super::argv::resolve(target, platform);
    let out = runner
        .run(cmd, Vec::new(), Some(RESOLVE_DEADLINE), None)
        .await
        .ok()?;
    if out.status != Some(0) {
        return None;
    }
    let effective = parse_g(&String::from_utf8_lossy(&out.stdout));
    (!effective.hostname.is_empty()).then_some(effective)
}

/// The most `Include` levels followed; a loop in the user's files must not hang the picker.
const MAX_INCLUDE_DEPTH: usize = 4;

/// The non-wildcard `Host` aliases of `config`, in order, without repeats. `include` returns the
/// text of every file an `Include` pattern names.
pub fn host_aliases(config: &str, include: &dyn Fn(&str) -> Vec<String>) -> Vec<String> {
    let mut found = Vec::new();
    collect(config, include, MAX_INCLUDE_DEPTH, &mut found);
    found
}

fn collect(
    config: &str,
    include: &dyn Fn(&str) -> Vec<String>,
    depth: usize,
    found: &mut Vec<String>,
) {
    for line in config.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, rest)) = line.split_once(|c: char| c.is_whitespace() || c == '=') else {
            continue;
        };
        let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '=');
        if key.eq_ignore_ascii_case("host") {
            for alias in rest.split_whitespace() {
                let pattern = alias.contains(['*', '?', '!']);
                if !pattern && !found.iter().any(|f| f == alias) {
                    found.push(alias.to_string());
                }
            }
        } else if key.eq_ignore_ascii_case("include") && depth > 0 {
            for pattern in rest.split_whitespace() {
                for text in include(pattern) {
                    collect(&text, include, depth - 1, found);
                }
            }
        }
    }
}

/// The aliases of `~/.ssh/config` (and its `Include`s) under `home`; empty when there is none.
pub fn user_host_aliases(home: &Path) -> Vec<String> {
    let ssh_dir = home.join(".ssh");
    let Ok(config) = std::fs::read_to_string(ssh_dir.join("config")) else {
        return Vec::new();
    };
    host_aliases(&config, &|pattern| read_included(&ssh_dir, home, pattern))
}

/// The files an `Include` pattern names: relative to `~/.ssh`, `~` expanded, and a `*` in the last
/// component matched against the directory's file names.
fn read_included(ssh_dir: &Path, home: &Path, pattern: &str) -> Vec<String> {
    let path = match pattern.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => ssh_dir.join(pattern),
    };
    let (Some(dir), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str()))
    else {
        return Vec::new();
    };
    let mut names: Vec<_> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| wildcard_matches(name, n))
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
        .iter()
        .filter_map(|n| std::fs::read_to_string(dir.join(n)).ok())
        .collect()
}

fn wildcard_matches(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((head, tail)) => {
            name.len() >= head.len() + tail.len() && name.starts_with(head) && name.ends_with(tail)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Vec<String> {
        Vec::new()
    }

    #[test]
    fn host_aliases_lists_plain_aliases_and_skips_wildcards_and_negations() {
        let config = "Host build *.corp !skip\n  User me\nHost web db # comment\nHost *\n";
        assert_eq!(host_aliases(config, &none), ["build", "web", "db"]);
    }

    #[test]
    fn host_aliases_follows_includes_and_ignores_a_loop() {
        let config = "Include extra\nHost a\n";
        let include = |p: &str| {
            assert_eq!(p, "extra");
            vec!["Host b\nInclude extra\n".to_string()]
        };
        assert_eq!(host_aliases(config, &include), ["b", "a"]);
    }

    #[test]
    fn host_aliases_repeats_collapse_and_keyword_case_is_ignored() {
        assert_eq!(host_aliases("host a\nHOST=a b\n", &none), ["a", "b"]);
    }

    #[test]
    fn parse_g_reads_the_effective_hostname_user_port_and_timeout() {
        let out = "user dev\nhostname 10.0.0.5\nport 2222\nconnecttimeout 7\nidentityfile ~/.ssh/id\n";
        let e = parse_g(out);
        assert_eq!(e.hostname, "10.0.0.5");
        assert_eq!(e.user.as_deref(), Some("dev"));
        assert_eq!(e.port, Some(2222));
        assert_eq!(e.timeout_secs(), 7, "the user's ConnectTimeout is the deadline");
    }

    #[test]
    fn a_connect_timeout_of_none_or_absent_falls_back_to_fifteen_seconds() {
        assert_eq!(parse_g("connecttimeout none\n").timeout_secs(), 15);
        assert_eq!(parse_g("hostname h\n").timeout_secs(), 15);
    }

    #[test]
    fn two_aliases_of_one_machine_share_an_identity() {
        let a = parse_g("hostname Box.Example\nuser me\nport 22\n");
        let b = parse_g("hostname box.example\nuser me\n");
        assert_eq!(a.identity(), b.identity());
    }

    #[test]
    fn user_host_aliases_reads_the_config_and_wildcard_includes_under_home() {
        let home = std::env::temp_dir().join(format!("micold-489-hosts-{}", std::process::id()));
        let ssh = home.join(".ssh");
        std::fs::create_dir_all(ssh.join("conf.d")).unwrap();
        std::fs::write(ssh.join("config"), "Include conf.d/*.conf\nHost top\n").unwrap();
        std::fs::write(ssh.join("conf.d/a.conf"), "Host inc\n").unwrap();
        assert_eq!(user_host_aliases(&home), ["inc", "top"]);
        assert!(user_host_aliases(&home.join("nowhere")).is_empty());
        std::fs::remove_dir_all(&home).unwrap();
    }
}
