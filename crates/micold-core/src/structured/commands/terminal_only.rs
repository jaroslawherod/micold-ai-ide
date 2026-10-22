//! The commands that need Claude Code's terminal interface (#943 FR-010, FR-011, research R2).
//!
//! The user guide's terminal-only table is checked against [`TERMINAL_ONLY`] (FR-012, M9).

/// A command the app opens in the terminal instead of sending it, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalOnly {
    /// The command's name, without the `/`.
    pub name: &'static str,
    /// Other names Claude Code accepts for it (research P-1: `settings` for `config`), so a draft
    /// that uses one is caught even when the CLI's own list is not known.
    pub aliases: &'static [&'static str],
    /// Why it needs the terminal, in words a user reads (FR-010).
    pub why: &'static str,
}

/// The documented terminal-only commands (FR-011). In a structured session each of these answers
/// with text that does not do what the user wanted, or is not offered at all (research P-2).
pub const TERMINAL_ONLY: &[TerminalOnly] = &[
    TerminalOnly {
        name: "config",
        aliases: &["settings"],
        why: "Its settings screen is interactive and needs the terminal.",
    },
    TerminalOnly {
        name: "mcp",
        aliases: &[],
        why:
            "Managing MCP servers and signing in to them needs the terminal's interactive screens.",
    },
    TerminalOnly {
        name: "agents",
        aliases: &[],
        why: "Its subagent manager is an interactive screen of the terminal.",
    },
    TerminalOnly {
        name: "login",
        aliases: &[],
        why: "Signing in to Claude Code needs the terminal's sign-in flow.",
    },
    TerminalOnly {
        name: "logout",
        aliases: &[],
        why: "Signing out of Claude Code is done in the terminal.",
    },
];

/// The reason given for a command the CLI itself marks terminal-only (`terminal_slash_commands`).
pub const CLI_MARKED_WHY: &str = "Claude Code runs this command only in its terminal interface.";

/// The Claude Code version [`TERMINAL_ONLY`] was last checked against.
pub const CHECKED_AGAINST: &str = "2.1.307";

/// The built-in commands that work in a structured session, offered when the session's own list
/// is not known (FR-009, US2 scenario 8).
pub const BUILT_INS: &[&str] = &["clear", "compact", "context", "init", "usage"];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_documented_list_holds_fr_011s_five_each_with_a_reason_and_no_duplicate() {
        let names: Vec<&str> = TERMINAL_ONLY.iter().map(|t| t.name).collect();
        for name in ["config", "mcp", "agents", "login", "logout"] {
            assert!(names.contains(&name), "{name} in {names:?}");
        }
        for entry in TERMINAL_ONLY {
            assert!(!entry.why.trim().is_empty(), "{} has a reason", entry.name);
            assert!(
                !entry.name.starts_with('/'),
                "{}: a name, not a command",
                entry.name
            );
        }
        let unique: HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "no duplicate: {names:?}");
    }

    #[test]
    fn the_list_names_the_claude_code_version_it_was_checked_against() {
        assert_eq!(CHECKED_AGAINST, "2.1.307");
    }

    #[test]
    fn the_known_built_ins_hold_the_structured_ones_and_no_terminal_only_one() {
        for name in ["compact", "clear", "context", "usage"] {
            assert!(BUILT_INS.contains(&name), "{name} in {BUILT_INS:?}");
        }
        for entry in TERMINAL_ONLY {
            assert!(
                !BUILT_INS.contains(&entry.name),
                "{} is terminal-only",
                entry.name
            );
        }
        assert!(!BUILT_INS.contains(&"model"), "/model is the app's own");
    }
}
