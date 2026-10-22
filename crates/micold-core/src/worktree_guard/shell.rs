//! Best-effort reading of a shell command for the worktree guard (FR-004): which directories does
//! it start in, change into, or name as a path argument?
//!
//! Only literal text is read. Variables, command substitution, globs, `~` and the contents of
//! nested scripts are not analysed: a word holding `$` or a backtick is skipped (the edit and write
//! tools remain the strong guard).

use std::path::{Path, PathBuf};

use super::paths::has_root;

/// Every absolute location `command` touches, in order, when run from `start`: `start` itself, each
/// `cd`/`pushd` target, and each word that is an absolute, `./` or `../` path. A relative word is
/// resolved against the directory the command is in at that point, tracked through `cd`/`pushd`.
pub fn targets(command: &str, start: &Path) -> Vec<PathBuf> {
    let mut found = vec![start.to_path_buf()];
    let mut running = start.to_path_buf();
    for words in segments(command) {
        let mut rest = words.iter().map(String::as_str);
        let first = rest.next();
        let is_cd = matches!(first, Some("cd" | "pushd"));
        if is_cd {
            // The directory is the first word that is not an option; `cd -` and `cd` alone go
            // nowhere we can name.
            let dir = rest
                .clone()
                .find(|word| !word.starts_with('-'))
                .filter(|word| literal(word));
            if let Some(dir) = dir {
                let dir = absolute(&running, dir);
                found.push(dir.clone());
                running = dir;
            }
            continue;
        }
        for word in words.iter().map(String::as_str) {
            let word = word.trim_start_matches(['<', '>']);
            if literal(word) && is_path(word) {
                found.push(absolute(&running, word));
            }
        }
    }
    found
}

/// A word with nothing the shell would expand that we cannot follow.
fn literal(word: &str) -> bool {
    !word.is_empty() && !word.contains(['$', '`', '*', '?', '~'])
}

/// An absolute path, or an explicitly relative one (`.`, `..`, `./x`, `../x`).
fn is_path(word: &str) -> bool {
    let word = word.replace('\\', "/");
    has_root(Path::new(&word)) || word == ".." || word.starts_with("./") || word.starts_with("../")
}

fn absolute(running: &Path, word: &str) -> PathBuf {
    let path = Path::new(word);
    if has_root(path) {
        path.to_path_buf()
    } else {
        running.join(path)
    }
}

/// Split into commands at `&&`, `||`, `;`, `|`, `&` and newlines outside quotes, each as its words
/// with quotes removed. Empty commands are dropped.
fn segments(command: &str) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = command.chars().peekable();
    let end_word = |words: &mut Vec<String>, word: &mut String, in_word: &mut bool| {
        if *in_word {
            words.push(std::mem::take(word));
            *in_word = false;
        }
    };
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                for q in chars.by_ref() {
                    if q == '\'' {
                        break;
                    }
                    word.push(q);
                }
            }
            '"' => {
                in_word = true;
                while let Some(q) = chars.next() {
                    match q {
                        '"' => break,
                        '\\' if matches!(chars.peek(), Some('"' | '\\' | '$' | '`')) => {
                            word.push(chars.next().unwrap_or('\\'));
                        }
                        _ => word.push(q),
                    }
                }
            }
            // A substitution is one opaque, non-literal word: its text is never read.
            '$' if chars.peek() == Some(&'(') => {
                in_word = true;
                let mut depth = 0;
                for q in chars.by_ref() {
                    match q {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                }
                word.push('$');
            }
            '`' => {
                in_word = true;
                for q in chars.by_ref() {
                    if q == '`' {
                        break;
                    }
                }
                word.push('`');
            }
            '\\' => {
                in_word = true;
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            ';' | '&' | '|' | '\n' | '(' | ')' => {
                end_word(&mut words, &mut word, &mut in_word);
                if !words.is_empty() {
                    result.push(std::mem::take(&mut words));
                }
            }
            c if c.is_whitespace() => end_word(&mut words, &mut word, &mut in_word),
            c => {
                in_word = true;
                word.push(c);
            }
        }
    }
    end_word(&mut words, &mut word, &mut in_word);
    if !words.is_empty() {
        result.push(words);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(command: &str) -> Vec<String> {
        targets(command, Path::new("/p/a"))
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_starting_directory_is_always_a_target() {
        assert_eq!(run("ls"), ["/p/a"]);
        assert_eq!(run(""), ["/p/a"]);
    }

    #[test]
    fn commands_split_on_and_or_semicolon_pipe_and_newline() {
        for command in [
            "true && cd /x",
            "true || cd /x",
            "true; cd /x",
            "true | cd /x",
            "true\ncd /x",
            "true & cd /x",
        ] {
            assert_eq!(run(command), ["/p/a", "/x"], "{command}");
        }
    }

    #[test]
    fn the_running_directory_follows_cd_and_pushd() {
        assert_eq!(
            run("cd ../b && ls ./src"),
            ["/p/a", "/p/a/../b", "/p/a/../b/./src"]
        );
        assert_eq!(run("pushd /x; cat ./f"), ["/p/a", "/x", "/x/./f"]);
        assert_eq!(run("cd -P /x"), ["/p/a", "/x"]);
    }

    #[test]
    fn absolute_dot_and_dotdot_words_are_targets_but_plain_words_are_not() {
        assert_eq!(run("cp /x/f ../g h i/j"), ["/p/a", "/x/f", "/p/a/../g"]);
        assert_eq!(run("ls .."), ["/p/a", "/p/a/.."]);
    }

    #[test]
    fn quotes_group_words_and_hide_separators() {
        assert_eq!(run("cd '/x y' && ls"), ["/p/a", "/x y"]);
        assert_eq!(run("cd \"/x y\""), ["/p/a", "/x y"]);
        assert_eq!(run("echo 'a && cd /x'"), ["/p/a"]);
        assert_eq!(run("echo \"a; cd /x\" ; true"), ["/p/a"]);
        assert_eq!(run("cd /x\\ y"), ["/p/a", "/x y"]);
    }

    #[test]
    fn a_redirection_target_is_a_path() {
        assert_eq!(run("echo hi > ../b/f"), ["/p/a", "/p/a/../b/f"]);
        assert_eq!(run("echo hi >/x/f"), ["/p/a", "/x/f"]);
    }

    #[test]
    fn expansion_substitution_variables_and_nested_scripts_are_not_analysed() {
        for command in [
            "cd $B && ls",
            "cd \"$B/x\"",
            "ls $(echo /x)",
            "ls `echo /x`",
            "cat /x/*",
            "cd ~/x",
            "bash -c 'cd /x'",
            "sh script.sh",
        ] {
            let found = run(command);
            // `bash -c 'cd /x'` is one word: the script text is never read.
            assert_eq!(found, ["/p/a"], "{command}");
        }
    }

    #[test]
    fn a_windows_spelling_is_followed() {
        assert_eq!(run("cd 'C:\\x\\y'"), ["/p/a", "C:\\x\\y"]);
    }
}
