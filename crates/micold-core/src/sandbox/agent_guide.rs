//! What the AI agent in an app-owned sandbox is told about it (#1439 FR-019, BUG-1500): a short
//! guide the daemon writes to `~/.micold/AGENTS.md`, and a marked block that points at it in the
//! user-level instruction files of Claude Code and Codex. Pure: the daemon does the I/O.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::session::AiCli;

/// The line that opens the block this module owns in an instruction file.
pub const BEGIN: &str = "<!-- micold:sandbox-guide begin -->";
/// The line that closes it.
pub const END: &str = "<!-- micold:sandbox-guide end -->";

/// The guide, written as `~/.micold/AGENTS.md`. Its wording follows the user guide's "Tools
/// installed in the sandbox's home" (`docs/user-guide/sandboxed-daemon.md`); keep them consistent.
pub const GUIDE: &str = "\
# Working in a Micold container sandbox

Micold writes this file each time the daemon starts; changes made here are lost.

You run in a container as a non-root user. There is no `sudo`, and `apt-get` cannot install system
packages.

## What persists

- Your home (`~`) and everything under it: `~/.local/bin`, the tools mise installs, `~/.cargo`,
  `~/.rustup` and npm's global prefix. It stays for the life of the daemon, across restarts and
  image updates.
- The project folders you work in, and their worktrees.

Nothing else does. `/tmp`, `/var/tmp` and every other path can be lost at any restart, and are lost
when the container is replaced (an image update) or the pod is recreated. Keep no work or tools
there.

## Installing tools

Install as yourself, into the home; what you install is on `PATH` for every later session, with
no change to startup files:

- `mise use -g <tool>@<version>`, or declare the tool in the project's `mise.toml` and run
  `mise install`.
- `npm install -g <package>`: `NPM_CONFIG_PREFIX` defaults to `~/.local`.
- `cargo install <crate>`: `CARGO_INSTALL_ROOT` defaults to `~/.local`.
- `go install <module>@<version>`: `GOBIN` defaults to `~/.local/bin`.
- `uv tool install <package>`.
- `pip install --user <package>` under a Python that mise or uv installed; the image's `python3`
  has no `pip`.

A value the environment already sets for `NPM_CONFIG_PREFIX`, `CARGO_INSTALL_ROOT` or `GOBIN` is
kept.

- mise refuses Python builds older than mid-2025 (\"No GitHub artifact attestations found\"): pick a
  newer patch release, or set `MISE_PYTHON_GITHUB_ATTESTATIONS=false` for that exact version.
- A tool built against a library of the image (a `cargo install` that links OpenSSL, an npm package
  with a native module) may need reinstalling after an image update.
- System libraries and packages come only from the image. When the project needs one, ask the user
  to add it through an image of their own built `FROM` the basic image.
";

/// The block merged into an instruction file: a summary of [`GUIDE`] naming `guide`, its path.
/// Ends with a newline.
pub fn pointer_block(guide: &Path) -> String {
    format!(
        "{BEGIN}\n\
         ## Micold container sandbox\n\
         \n\
         Micold keeps this block up to date; text outside it is yours.\n\
         \n\
         You run in a container sandbox as a non-root user: no `sudo`, no `apt-get`. Only your home\n\
         and the project folders persist; `/tmp` and everything else can be lost at a restart or an\n\
         image update. Install tools into the home with `mise use -g`, `npm install -g`, `cargo\n\
         install`, `uv tool install` or `go install`. Read `{}` before you install anything.\n\
         {END}\n",
        guide.display()
    )
}

/// `existing` with `block` in it: the marked block replaced in place, else `block` appended after
/// one blank line. Text outside the block is kept byte for byte, and merging the same block again
/// changes nothing.
///
/// The marked block is the last complete one: a line that is [`BEGIN`] (trailing whitespace and a
/// `\r` ignored) through the next line that is [`END`]. A marker quoted inside a line is not one;
/// a [`BEGIN`] line with no [`END`] line after it starts no block, and a later [`BEGIN`] line
/// restarts the search, so neither a dangling marker nor the user's text after it is ever taken
/// into the block.
pub fn merge_block(existing: &str, block: &str) -> String {
    let mut begin = None;
    let mut span = None;
    let mut at = 0;
    for line in existing.split_inclusive('\n') {
        match line.trim_end() {
            bare if bare == BEGIN => begin = Some(at),
            bare if bare == END => {
                if let Some(b) = begin.take() {
                    span = Some((b, at + line.len()));
                }
            }
            _ => {}
        }
        at += line.len();
    }
    if let Some((begin, end)) = span {
        return format!("{}{block}{}", &existing[..begin], &existing[end..]);
    }
    match existing {
        "" => block.to_string(),
        text if text.ends_with("\n\n") => format!("{text}{block}"),
        text if text.ends_with('\n') => format!("{text}\n{block}"),
        text => format!("{text}\n\n{block}"),
    }
}

/// Where the guide and the instruction files are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuideFiles {
    /// `home/.micold/AGENTS.md`.
    pub guide: PathBuf,
    /// Claude Code's `CLAUDE.md`, then Codex's `AGENTS.md`, each with its CLI.
    pub instructions: Vec<(AiCli, PathBuf)>,
}

/// The files for the sandbox home `home`: Claude Code reads `$CLAUDE_CONFIG_DIR/CLAUDE.md`, else
/// `home/.claude/CLAUDE.md`; Codex reads `$CODEX_HOME/AGENTS.md`, else `home/.codex/AGENTS.md`.
/// `var` gives a variable of the daemon's environment; an empty value counts as unset.
pub fn guide_files(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> GuideFiles {
    let dir = |name: &str, default: &str| {
        var(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(default))
    };
    GuideFiles {
        guide: home.join(".micold").join("AGENTS.md"),
        instructions: vec![
            (
                AiCli::ClaudeCode,
                dir("CLAUDE_CONFIG_DIR", ".claude").join("CLAUDE.md"),
            ),
            (AiCli::Codex, dir("CODEX_HOME", ".codex").join("AGENTS.md")),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> String {
        pointer_block(Path::new("/home/u/.micold/AGENTS.md"))
    }

    #[test]
    fn agent_guide_merge_into_an_empty_file_is_the_block_alone() {
        assert_eq!(merge_block("", &block()), block());
    }

    #[test]
    fn agent_guide_merge_appends_after_user_text_and_keeps_it() {
        for (user, sep) in [("mine\n", "\n"), ("mine", "\n\n"), ("mine\n\n", "")] {
            let merged = merge_block(user, &block());
            assert_eq!(merged, format!("{user}{sep}{}", block()), "{user:?}");
        }
    }

    #[test]
    fn agent_guide_merge_replaces_only_an_older_block() {
        let old = format!("{BEGIN}\nold text\n{END}\n");
        let file = format!("before\n\n{old}after\n");
        assert_eq!(
            merge_block(&file, &block()),
            format!("before\n\n{}after\n", block())
        );
        // No newline after the old end marker.
        let file = format!("before\n{BEGIN}\nold\n{END}");
        assert_eq!(merge_block(&file, &block()), format!("before\n{}", block()));
    }

    #[test]
    fn agent_guide_merge_twice_equals_once() {
        for user in [
            "",
            "mine",
            "a\n\nb\n",
            &format!("x\n{BEGIN}\nold\n{END}\ny"),
        ] {
            let once = merge_block(user, &block());
            assert_eq!(merge_block(&once, &block()), once, "{user:?}");
        }
    }

    #[test]
    fn agent_guide_begin_without_end_is_no_block() {
        let file = format!("mine\n{BEGIN}\ndangling\n");
        let once = merge_block(&file, &block());
        assert_eq!(once, format!("{file}\n{}", block()));
        // The second merge must not pair the dangling line with the appended block's end.
        assert_eq!(merge_block(&once, &block()), once);
    }

    #[test]
    fn agent_guide_a_stray_begin_after_a_block_and_crlf_markers_keep_one_block() {
        let file = format!("{BEGIN}\nold\n{END}\nnotes\n{BEGIN}\n");
        assert_eq!(
            merge_block(&file, &block()),
            format!("{}notes\n{BEGIN}\n", block())
        );
        let crlf = format!("mine\r\n{BEGIN}\r\nold\r\n{END}\r\ntail\r\n");
        assert_eq!(
            merge_block(&crlf, &block()),
            format!("mine\r\n{}tail\r\n", block())
        );
    }

    #[test]
    fn agent_guide_a_marker_quoted_inside_a_line_is_no_marker() {
        let file = format!("see `{BEGIN}` and `{END}` in the guide\n");
        let once = merge_block(&file, &block());
        assert_eq!(once, format!("{file}\n{}", block()));
        assert_eq!(merge_block(&once, &block()), once);
    }

    #[test]
    fn agent_guide_files_honour_the_config_variables_and_fall_back_to_the_home() {
        let home = Path::new("/home/u");
        let none = guide_files(home, |_| None);
        assert_eq!(none.guide, PathBuf::from("/home/u/.micold/AGENTS.md"));
        assert_eq!(
            none.instructions,
            vec![
                (
                    AiCli::ClaudeCode,
                    PathBuf::from("/home/u/.claude/CLAUDE.md")
                ),
                (AiCli::Codex, PathBuf::from("/home/u/.codex/AGENTS.md")),
            ]
        );
        let empty = guide_files(home, |_| Some(OsString::new()));
        assert_eq!(empty, none);
        let set = guide_files(home, |name| match name {
            "CLAUDE_CONFIG_DIR" => Some("/cfg/claude".into()),
            "CODEX_HOME" => Some("/cfg/codex".into()),
            _ => None,
        });
        assert_eq!(
            set.instructions,
            vec![
                (AiCli::ClaudeCode, PathBuf::from("/cfg/claude/CLAUDE.md")),
                (AiCli::Codex, PathBuf::from("/cfg/codex/AGENTS.md")),
            ]
        );
    }

    #[test]
    fn agent_guide_names_what_an_agent_needs() {
        for needle in [
            "sudo",
            "/tmp",
            "mise use -g",
            "mise install",
            "npm install -g",
            "cargo install",
            "uv tool install",
            "go install",
            "pip install --user",
            "NPM_CONFIG_PREFIX",
            "CARGO_INSTALL_ROOT",
            "GOBIN",
            "FROM",
        ] {
            assert!(GUIDE.contains(needle), "the guide lacks {needle:?}");
        }
        let block = block();
        assert!(block.starts_with(BEGIN) && block.ends_with(&format!("{END}\n")));
        assert!(block.contains("/home/u/.micold/AGENTS.md"));
    }
}
