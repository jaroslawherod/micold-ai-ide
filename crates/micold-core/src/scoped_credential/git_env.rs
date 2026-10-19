//! The environment that makes git inside a sandbox ignore every inherited credential helper
//! (contracts/git-in-container.md §1, FR-004).

use std::collections::BTreeMap;

/// The environment block for git in a sandbox. `GIT_CONFIG_COUNT` entries are read after every
/// file, so the empty `credential.helper` resets the list the host's configuration built up: git
/// helpers accumulate, and an empty value is the only thing that clears them. Then the in-container
/// helper is added for `base` alone, with the repository path sent to it, and the SSH forms of
/// `base`'s remote are rewritten to HTTPS, so the host's own configuration is never touched (FR-007).
pub fn git_env(base: &str) -> BTreeMap<String, String> {
    let base = base.trim_end_matches('/');
    let entries = [
        ("credential.helper".to_string(), String::new()),
        (
            format!("credential.{base}.helper"),
            "!micold-daemon git-credential".to_string(),
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
}
