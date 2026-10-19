//! The environment that makes git inside a sandbox ignore every inherited credential helper
//! (contracts/git-in-container.md §1, FR-004).

use std::collections::BTreeMap;

/// The environment block for git in a sandbox. `GIT_CONFIG_COUNT` entries are read after every
/// file, so the empty `credential.helper` resets the list the host's configuration built up: git
/// helpers accumulate, and an empty value is the only thing that clears them.
///
/// `_base` is the forge the scoped helper is added for; the helper entries join this block with the
/// in-container helper.
pub fn git_env(_base: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("GIT_CONFIG_COUNT".to_string(), "1".to_string()),
        (
            "GIT_CONFIG_KEY_0".to_string(),
            "credential.helper".to_string(),
        ),
        ("GIT_CONFIG_VALUE_0".to_string(), String::new()),
        // A missing credential fails at once instead of waiting on a prompt no one sees.
        ("GIT_TERMINAL_PROMPT".to_string(), "0".to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoped_credential::GITHUB_BASE;

    #[test]
    fn the_credential_helper_list_is_reset_through_the_config_env() {
        let env = git_env(GITHUB_BASE);
        assert_eq!(env["GIT_CONFIG_COUNT"], "1");
        assert_eq!(env["GIT_CONFIG_KEY_0"], "credential.helper");
        assert_eq!(env["GIT_CONFIG_VALUE_0"], "");
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
