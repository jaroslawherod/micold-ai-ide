//! Quoting one file name so a shell reads it back as exactly one literal argument (FR-002,
//! research R2). No crate: `shlex` and friends know POSIX only, and PowerShell, fish and cmd each
//! quote differently.

use std::ffi::OsStr;

use super::ShellKind;

/// The shell has no way to write this name as one literal argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unrepresentable;

/// `name` written for `shell`, or [`Unrepresentable`] when `shell` cannot.
///
/// Read by `shell`, the result is one argument equal to `name`. The empty name is `''` (or `""`).
pub fn quote(shell: ShellKind, name: &OsStr) -> Result<String, Unrepresentable> {
    let Some(text) = name.to_str() else {
        return match shell {
            ShellKind::Bash => non_utf8_bash(name),
            _ => Err(Unrepresentable),
        };
    };
    match shell {
        ShellKind::Posix | ShellKind::Bash => Ok(format!("'{}'", text.replace('\'', r"'\''"))),
        ShellKind::Fish => Ok(format!(
            "'{}'",
            text.replace('\\', r"\\").replace('\'', r"\'")
        )),
        // PowerShell also treats the typographic single quotes as quote characters.
        ShellKind::PowerShell => {
            let mut out = String::with_capacity(text.len() + 2);
            out.push('\'');
            for c in text.chars() {
                out.push(c);
                if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
                    out.push(c);
                }
            }
            out.push('\'');
            Ok(out)
        }
        ShellKind::Cmd => {
            if text.contains(['"', '%', '^', '!', '\n', '\r']) {
                return Err(Unrepresentable);
            }
            Ok(format!("\"{text}\""))
        }
    }
}

/// bash and zsh read `$'\xNN'` as a raw byte, which is the one way to write a name that is not
/// UTF-8. Only meaningful where names are bytes.
#[cfg(unix)]
fn non_utf8_bash(name: &OsStr) -> Result<String, Unrepresentable> {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("$'");
    for &b in name.as_bytes() {
        match b {
            b'\'' | b'\\' => {
                out.push('\\');
                out.push(char::from(b));
            }
            0x20..=0x7e => out.push(char::from(b)),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push('\'');
    Ok(out)
}

#[cfg(not(unix))]
fn non_utf8_bash(_name: &OsStr) -> Result<String, Unrepresentable> {
    Err(Unrepresentable)
}

/// File names that stress every quoting rule, shared with the real-shell round trip.
pub const AWKWARD_NAMES: &[&str] = &[
    "plain.png",
    "my file's (1).png",
    "two  spaces.txt",
    "it's",
    "'''",
    "say \"hi\".txt",
    "$HOME",
    "${x}",
    "$(whoami)",
    "`id`",
    "back\\slash",
    "trailing\\",
    "-rf",
    "--help",
    "tab\there",
    "new\nline",
    "naïve — café 日本語.png",
    "100% done!.txt",
    "a;b&c|d<e>f",
    "*?[a-z]~#",
    "C:\\Users\\Jo Doe\\pic.png",
    "\\\\server\\share\\a b.png",
];

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ShellKind; 5] = [
        ShellKind::Posix,
        ShellKind::Bash,
        ShellKind::Fish,
        ShellKind::PowerShell,
        ShellKind::Cmd,
    ];

    #[test]
    fn the_name_set_is_at_least_twenty_awkward_names() {
        assert!(AWKWARD_NAMES.len() >= 20);
    }

    #[test]
    fn posix_and_bash_use_single_quotes_and_close_around_an_apostrophe() {
        for shell in [ShellKind::Posix, ShellKind::Bash] {
            assert_eq!(quote(shell, OsStr::new("a b")).unwrap(), "'a b'");
            assert_eq!(
                quote(shell, OsStr::new("my file's (1).png")).unwrap(),
                r"'my file'\''s (1).png'"
            );
            assert_eq!(
                quote(shell, OsStr::new("$(x)`y`\\")).unwrap(),
                r"'$(x)`y`\'"
            );
            assert_eq!(quote(shell, OsStr::new("")).unwrap(), "''");
            assert_eq!(quote(shell, OsStr::new("-rf")).unwrap(), "'-rf'");
        }
    }

    #[test]
    fn fish_escapes_backslash_and_apostrophe_inside_single_quotes() {
        assert_eq!(
            quote(ShellKind::Fish, OsStr::new(r"a\b'c d")).unwrap(),
            r"'a\\b\'c d'"
        );
        assert_eq!(quote(ShellKind::Fish, OsStr::new("$x")).unwrap(), "'$x'");
    }

    #[test]
    fn powershell_doubles_straight_and_curly_single_quotes() {
        assert_eq!(
            quote(ShellKind::PowerShell, OsStr::new("it's")).unwrap(),
            "'it''s'"
        );
        assert_eq!(
            quote(ShellKind::PowerShell, OsStr::new("a\u{2019}b")).unwrap(),
            "'a\u{2019}\u{2019}b'"
        );
        assert_eq!(
            quote(ShellKind::PowerShell, OsStr::new("$x `y` C:\\a b")).unwrap(),
            "'$x `y` C:\\a b'"
        );
    }

    #[test]
    fn cmd_uses_double_quotes_and_refuses_what_they_cannot_hold() {
        assert_eq!(
            quote(ShellKind::Cmd, OsStr::new("C:\\Users\\a b\\f.png")).unwrap(),
            "\"C:\\Users\\a b\\f.png\""
        );
        for bad in ["a\"b", "50%", "a^b", "hi!", "a\nb", "a\rb"] {
            assert_eq!(
                quote(ShellKind::Cmd, OsStr::new(bad)),
                Err(Unrepresentable),
                "{bad:?}"
            );
        }
    }

    /// Name, then the expected text for posix/bash, fish, PowerShell and cmd (`None` = refused).
    type Row = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Option<&'static str>,
    );
    const EXPECTED: &[Row] = &[
        (
            "plain.png",
            "'plain.png'",
            "'plain.png'",
            "'plain.png'",
            Some("\"plain.png\""),
        ),
        (
            "it's",
            r"'it'\''s'",
            r"'it\'s'",
            "'it''s'",
            Some("\"it's\""),
        ),
        (
            "'''",
            r"''\'''\'''\'''",
            r"'\'\'\''",
            "''''''''",
            Some("\"'''\""),
        ),
        (
            "say \"hi\".txt",
            "'say \"hi\".txt'",
            "'say \"hi\".txt'",
            "'say \"hi\".txt'",
            None,
        ),
        (
            "$(whoami)",
            "'$(whoami)'",
            "'$(whoami)'",
            "'$(whoami)'",
            Some("\"$(whoami)\""),
        ),
        (
            "back\\slash",
            r"'back\slash'",
            r"'back\\slash'",
            r"'back\slash'",
            Some(r#""back\slash""#),
        ),
        (
            "trailing\\",
            r"'trailing\'",
            r"'trailing\\'",
            r"'trailing\'",
            Some(r#""trailing\""#),
        ),
        ("-rf", "'-rf'", "'-rf'", "'-rf'", Some("\"-rf\"")),
        (
            "tab\there",
            "'tab\there'",
            "'tab\there'",
            "'tab\there'",
            Some("\"tab\there\""),
        ),
        (
            "new\nline",
            "'new\nline'",
            "'new\nline'",
            "'new\nline'",
            None,
        ),
        (
            "100% done!.txt",
            "'100% done!.txt'",
            "'100% done!.txt'",
            "'100% done!.txt'",
            None,
        ),
        (
            "a;b&c|d<e>f",
            "'a;b&c|d<e>f'",
            "'a;b&c|d<e>f'",
            "'a;b&c|d<e>f'",
            Some("\"a;b&c|d<e>f\""),
        ),
    ];

    #[test]
    fn awkward_names_quote_to_the_exact_text_each_shell_expects() {
        for &(name, posix, fish, pwsh, cmd) in EXPECTED {
            assert!(
                AWKWARD_NAMES.contains(&name),
                "{name:?} is not an awkward name"
            );
            let n = OsStr::new(name);
            assert_eq!(
                quote(ShellKind::Posix, n).as_deref(),
                Ok(posix),
                "posix {name:?}"
            );
            assert_eq!(
                quote(ShellKind::Bash, n).as_deref(),
                Ok(posix),
                "bash {name:?}"
            );
            assert_eq!(
                quote(ShellKind::Fish, n).as_deref(),
                Ok(fish),
                "fish {name:?}"
            );
            assert_eq!(
                quote(ShellKind::PowerShell, n).as_deref(),
                Ok(pwsh),
                "pwsh {name:?}"
            );
            match cmd {
                Some(c) => assert_eq!(quote(ShellKind::Cmd, n).as_deref(), Ok(c), "cmd {name:?}"),
                None => assert_eq!(
                    quote(ShellKind::Cmd, n),
                    Err(Unrepresentable),
                    "cmd {name:?}"
                ),
            }
        }
    }

    #[test]
    fn only_cmd_refuses_and_only_the_names_it_cannot_hold() {
        for name in AWKWARD_NAMES {
            for shell in ALL {
                let held = quote(shell, OsStr::new(name)).is_ok();
                let cmd_cannot = name.contains(['"', '%', '^', '!', '\n', '\r']);
                assert_eq!(
                    held,
                    !(shell == ShellKind::Cmd && cmd_cannot),
                    "{shell:?} {name:?}"
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_utf8_is_written_as_bytes_for_bash_and_refused_elsewhere() {
        use std::os::unix::ffi::OsStrExt;
        let name = OsStr::from_bytes(b"a\xffb'c");
        assert_eq!(quote(ShellKind::Bash, name).unwrap(), r"$'a\xffb\'c'");
        for shell in [
            ShellKind::Posix,
            ShellKind::Fish,
            ShellKind::PowerShell,
            ShellKind::Cmd,
        ] {
            assert_eq!(quote(shell, name), Err(Unrepresentable));
        }
    }
}
