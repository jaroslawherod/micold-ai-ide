//! The environment that makes git inside a sandbox ignore every inherited credential helper
//! (contracts/git-in-container.md §1, FR-004).

use std::collections::BTreeMap;

/// The environment block for git in a sandbox. `GIT_CONFIG_COUNT` entries are read after every
/// file, so the empty `credential.helper` resets the list the host's configuration built up: git
/// helpers accumulate, and an empty value is the only thing that clears them. Then the in-container
/// helper is added for `base` alone, with the repository path sent to it, and the SSH forms of
/// `base`'s remote are rewritten to HTTPS, so the host's own configuration is never touched (FR-007).
pub fn git_env(base: &str) -> BTreeMap<String, String> {
    git_env_with_helper(base, "micold-daemon")
}

/// [`git_env`] with the daemon named by `daemon` (a command word the shell git runs the helper
/// through expands): a pod's daemon is not on `PATH`, so the client names its build directory.
pub fn git_env_with_helper(base: &str, daemon: &str) -> BTreeMap<String, String> {
    let base = base.trim_end_matches('/');
    let entries = [
        ("credential.helper".to_string(), String::new()),
        (
            format!("credential.{base}.helper"),
            format!("!{daemon} git-credential"),
        ),
        (format!("credential.{base}.useHttpPath"), "true".to_string()),
        (
            format!("url.{base}/.insteadOf"),
            "git@github.com:".to_string(),
        ),
        (
            format!("url.{base}/.insteadOf"),
            "ssh://git@github.com/".to_string(),
        ),
    ];
    let mut env = BTreeMap::from([
        ("GIT_CONFIG_COUNT".to_string(), entries.len().to_string()),
        // A missing credential fails at once instead of waiting on a prompt no one sees.
        ("GIT_TERMINAL_PROMPT".to_string(), "0".to_string()),
    ]);
    for (i, (key, value)) in entries.into_iter().enumerate() {
        env.insert(format!("GIT_CONFIG_KEY_{i}"), key);
        env.insert(format!("GIT_CONFIG_VALUE_{i}"), value);
    }
    env
}

/// The session variable that names the configuration file the helper reads on every call (R8).
pub const CONFIG_ENV: &str = "MICOLD_DAEMON_CONFIG";
/// The session variable that carries the helpers the session's git named before our reset, one per
/// line, for the helper to run when no configured token matches (research D7).
pub const FALLBACK_ENV: &str = "MICOLD_GIT_FALLBACK_HELPERS";

/// What a session's git needs from the configuration (#1530, FR-010, research D7).
#[derive(Debug, Clone, Copy)]
pub struct SessionGit<'a> {
    /// The web base of each token's forge, once each (`https://github.com`).
    pub bases: &'a [String],
    /// The command word the shell git runs the helper through expands.
    pub daemon: &'a str,
    /// The configuration file the helper reads.
    pub config: &'a str,
    /// `[git] isolate_host_helpers`: clear every inherited helper.
    pub isolate: bool,
    /// `[git] ssh_to_https`.
    pub ssh_to_https: bool,
    /// The helpers the session's git named before the reset, for a daemon that does not isolate.
    pub fallback: &'a [String],
}

/// `host` of `base` without a port: the form `git@<host>:` uses.
fn ssh_host(base: &str) -> &str {
    let rest = base.split_once("://").map_or(base, |(_, rest)| rest);
    rest.split(':').next().unwrap_or(rest)
}

/// The environment of a session's git for the daemon's configuration. A sandbox (`isolate`)
/// always resets the inherited helpers (#1002 FR-004). Otherwise nothing changes until a token is
/// configured, and then the list is reset, our helper added for each token's forge and the
/// session's own helpers handed to ours as the fallback. `ssh_to_https` rewrites each forge's SSH
/// forms, only with at least one token.
pub fn session_git_env(git: &SessionGit<'_>) -> BTreeMap<String, String> {
    if !git.isolate && git.bases.is_empty() {
        return BTreeMap::new();
    }
    // A sandbox clears every helper. Otherwise only each token forge's list is reset, so the
    // session's helpers for any other host keep answering.
    let mut entries = if git.isolate {
        vec![("credential.helper".to_string(), String::new())]
    } else {
        Vec::new()
    };
    for base in git.bases {
        if !git.isolate {
            entries.push((format!("credential.{base}.helper"), String::new()));
        }
        entries.push((
            format!("credential.{base}.helper"),
            format!("!{} git-credential", git.daemon),
        ));
        entries.push((format!("credential.{base}.useHttpPath"), "true".to_string()));
        if git.ssh_to_https {
            let host = ssh_host(base);
            for form in [format!("git@{host}:"), format!("ssh://git@{host}/")] {
                entries.push((format!("url.{base}/.insteadOf"), form));
            }
        }
    }
    let mut env = BTreeMap::from([
        ("GIT_CONFIG_COUNT".to_string(), entries.len().to_string()),
        (CONFIG_ENV.to_string(), git.config.to_string()),
    ]);
    if git.isolate {
        // A missing credential fails at once instead of waiting on a prompt no one sees.
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());
    } else if !git.fallback.is_empty() {
        env.insert(FALLBACK_ENV.to_string(), git.fallback.join("\n"));
    }
    for (i, (key, value)) in entries.into_iter().enumerate() {
        env.insert(format!("GIT_CONFIG_KEY_{i}"), key);
        env.insert(format!("GIT_CONFIG_VALUE_{i}"), value);
    }
    env
}

/// The helpers in the output of `git config --get-all credential[.<base>].helper`: an empty value
/// resets what came before it, as git does.
pub fn parse_helper_list(output: &str) -> Vec<String> {
    let mut helpers = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            helpers.clear();
        } else {
            helpers.push(line.to_string());
        }
    }
    helpers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoped_credential::GITHUB_BASE;

    #[test]
    fn the_credential_helper_list_is_reset_through_the_config_env() {
        let env = git_env(GITHUB_BASE);
        assert_eq!(env["GIT_CONFIG_COUNT"], "5");
        assert_eq!(env["GIT_CONFIG_KEY_0"], "credential.helper");
        assert_eq!(env["GIT_CONFIG_VALUE_0"], "");
    }

    #[test]
    fn the_scoped_helper_is_added_for_the_base_with_the_repository_path() {
        let env = git_env(GITHUB_BASE);
        let pair = |i: usize| {
            (
                env[&format!("GIT_CONFIG_KEY_{i}")].as_str(),
                env[&format!("GIT_CONFIG_VALUE_{i}")].as_str(),
            )
        };
        assert_eq!(
            pair(1),
            (
                "credential.https://github.com.helper",
                "!micold-daemon git-credential"
            )
        );
        assert_eq!(
            pair(2),
            ("credential.https://github.com.useHttpPath", "true")
        );
        assert_eq!(
            pair(3),
            ("url.https://github.com/.insteadOf", "git@github.com:")
        );
        assert_eq!(
            pair(4),
            ("url.https://github.com/.insteadOf", "ssh://git@github.com/")
        );
    }

    #[test]
    fn git_never_prompts() {
        assert_eq!(git_env(GITHUB_BASE)["GIT_TERMINAL_PROMPT"], "0");
    }

    #[test]
    fn no_ssh_key_or_agent_is_named() {
        let env = git_env(GITHUB_BASE);
        assert!(!env.contains_key("GIT_SSH_COMMAND"));
        assert!(!env.contains_key("SSH_AUTH_SOCK"));
    }

    fn session<'a>(bases: &'a [String], isolate: bool, fallback: &'a [String]) -> SessionGit<'a> {
        SessionGit {
            bases,
            daemon: "micold-daemon",
            config: "/etc/daemon.toml",
            isolate,
            ssh_to_https: true,
            fallback,
        }
    }

    fn entries(env: &BTreeMap<String, String>) -> Vec<(&str, &str)> {
        let count: usize = env["GIT_CONFIG_COUNT"].parse().unwrap();
        (0..count)
            .map(|i| {
                (
                    env[&format!("GIT_CONFIG_KEY_{i}")].as_str(),
                    env[&format!("GIT_CONFIG_VALUE_{i}")].as_str(),
                )
            })
            .collect()
    }

    #[test]
    fn a_sandbox_clears_inherited_helpers_even_with_no_token() {
        let env = session_git_env(&session(&[], true, &[]));
        assert_eq!(entries(&env), [("credential.helper", "")]);
        assert_eq!(env["GIT_TERMINAL_PROMPT"], "0");
        assert_eq!(env[CONFIG_ENV], "/etc/daemon.toml");
    }

    #[test]
    fn a_host_session_with_no_token_is_left_untouched() {
        assert!(session_git_env(&session(&[], false, &["osxkeychain".into()])).is_empty());
    }

    #[test]
    fn a_host_session_with_a_token_resets_adds_ours_and_passes_the_fallback() {
        let bases = ["https://github.com".to_string()];
        let fallback = [
            "osxkeychain".to_string(),
            "!gh auth git-credential".to_string(),
        ];
        let env = session_git_env(&session(&bases, false, &fallback));
        assert_eq!(
            entries(&env)[0],
            ("credential.https://github.com.helper", "")
        );
        assert!(!entries(&env).contains(&("credential.helper", "")));
        assert_eq!(
            entries(&env)[1],
            (
                "credential.https://github.com.helper",
                "!micold-daemon git-credential"
            )
        );
        assert_eq!(env[FALLBACK_ENV], "osxkeychain\n!gh auth git-credential");
        assert!(!env.contains_key("GIT_TERMINAL_PROMPT"));
    }

    #[test]
    fn the_ssh_rewrite_is_per_host_and_can_be_turned_off() {
        let bases = [
            "https://github.com".to_string(),
            "https://ghe.example".to_string(),
            "http://127.0.0.1:4000".to_string(),
        ];
        let env = session_git_env(&session(&bases, true, &[]));
        let rewrites: Vec<_> = entries(&env)
            .into_iter()
            .filter(|(k, _)| k.starts_with("url."))
            .collect();
        assert_eq!(rewrites.len(), 6);
        assert!(rewrites.contains(&("url.https://ghe.example/.insteadOf", "git@ghe.example:")));
        assert!(rewrites.contains(&(
            "url.http://127.0.0.1:4000/.insteadOf",
            "ssh://git@127.0.0.1/"
        )));
        let mut off = session(&bases, true, &[]);
        off.ssh_to_https = false;
        assert!(!entries(&session_git_env(&off))
            .iter()
            .any(|(k, _)| k.starts_with("url.")));
    }

    #[test]
    fn captured_helpers_honour_an_empty_reset() {
        assert_eq!(parse_helper_list("a\n\nb\n!c d\n"), ["b", "!c d"]);
        assert!(parse_helper_list("").is_empty());
    }
}
