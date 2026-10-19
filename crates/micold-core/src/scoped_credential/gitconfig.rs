//! A host `~/.gitconfig` with everything that could carry a credential taken out
//! (contracts/git-in-container.md §3, FR-004).

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Parses the output of `git config --list --null`: entries end in NUL, a key and its value are
/// split by a newline, and a key with no value (`[core] bare`) is the boolean `true`.
pub fn parse_config_list(output: &str) -> Vec<(String, String)> {
    output
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.split_once('\n') {
            Some((key, value)) => (key.to_string(), value.to_string()),
            None => (entry.to_string(), "true".to_string()),
        })
        .collect()
}

/// A key of `git config --list`: `section[.subsection].name`, the section and name in lower case,
/// the subsection (which may itself contain dots) as written.
struct Key<'a> {
    section: String,
    subsection: Option<&'a str>,
    name: String,
}

impl<'a> Key<'a> {
    fn parse(key: &'a str) -> Option<Self> {
        let (section, rest) = key.split_once('.')?;
        let (subsection, name) = match rest.rsplit_once('.') {
            Some((sub, name)) => (Some(sub), name),
            None => (None, rest),
        };
        Some(Self {
            section: section.to_ascii_lowercase(),
            subsection,
            name: name.to_ascii_lowercase(),
        })
    }

    /// Whether this setting can carry or reach a credential.
    fn is_credential_bearing(&self, value: &str) -> bool {
        // A URL with a user name or token in it, whichever setting holds it (`remote.*.url`,
        // `http.proxy`, ...): FR-004 forbids an embedded host credential.
        if value.contains("://") && has_userinfo(value) {
            return true;
        }
        match (self.section.as_str(), self.name.as_str()) {
            ("credential" | "include" | "includeif", _) => true,
            ("url", "insteadof" | "pushinsteadof") => self.subsection.is_some_and(has_userinfo),
            ("http", "extraheader") => true,
            ("core", "sshcommand" | "askpass") => self.subsection.is_none(),
            ("sendemail", "smtppass") | ("imap", "pass") | ("http", "proxyauthmethod") => true,
            _ => false,
        }
    }
}

/// Whether a URL prefix names a user before its host: an `@` before the first `/` that follows the
/// scheme (`https://tok@github.com/`, and `git@github.com:` too).
fn has_userinfo(base: &str) -> bool {
    let rest = base.split_once("://").map_or(base, |(_, rest)| rest);
    rest.split('/')
        .next()
        .is_some_and(|host| host.contains('@'))
}

/// Writes a value as git reads it back: quoted, with backslash, quote, newline and tab escaped.
fn quoted(value: &str, out: &mut String) {
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The configuration `entries` (the parsed `git config --file … --list --null`) with every
/// credential-bearing setting removed, written back in order in a form `git config` re-reads.
///
/// Removed: `credential.*` and `credential.<url>.*`; `url.<base>.insteadOf` and `pushInsteadOf`
/// whose `<base>` has userinfo; `http.extraHeader` and `http.<url>.extraHeader`; `include.*` and
/// `includeIf.*`; `core.sshCommand` and `core.askPass`; `sendemail.smtpPass`, `imap.pass`; and any
/// setting whose value is a URL with userinfo (`remote.*.url`, `http.proxy`, ...).
pub fn sanitize_gitconfig(entries: &[(String, String)]) -> String {
    let mut out = String::new();
    let mut current: Option<(String, Option<String>)> = None;
    for (key, value) in entries {
        let Some(parsed) = Key::parse(key) else {
            continue;
        };
        if parsed.is_credential_bearing(value) {
            continue;
        }
        let header = (
            parsed.section.clone(),
            parsed.subsection.map(str::to_string),
        );
        if current.as_ref() != Some(&header) {
            out.push('[');
            out.push_str(&header.0);
            if let Some(sub) = &header.1 {
                out.push_str(" \"");
                out.push_str(&sub.replace('\\', "\\\\").replace('"', "\\\""));
                out.push('"');
            }
            out.push_str("]\n");
            current = Some(header);
        }
        out.push('\t');
        out.push_str(&parsed.name);
        out.push_str(" = ");
        quoted(value, &mut out);
        out.push('\n');
    }
    out
}

/// [`sanitize_gitconfig`] over the configuration text `text`, read by the host's own `git`.
///
/// The text goes to a private file for the one `git config --file` call, so what is parsed is what
/// git itself would parse. Without `git` on the host, or when the text is not a valid
/// configuration, this is an error and the caller shares nothing.
pub fn sanitize_gitconfig_text(text: &str) -> io::Result<String> {
    let dir = std::env::temp_dir().join(format!("micold-gitconfig-{}", uuid::Uuid::new_v4()));
    crate::owner_only::ensure_dir(&dir)?;
    let result = crate::owner_only::write(&dir, "config", text.as_bytes())
        .and_then(|file| list_entries(&file));
    let _ = std::fs::remove_dir_all(&dir);
    result.map(|entries| sanitize_gitconfig(&entries))
}

fn list_entries(file: &Path) -> io::Result<Vec<(String, String)>> {
    let mut git = Command::new("git");
    crate::process::no_window(&mut git);
    let output = git
        .args(["config", "--file"])
        .arg(file)
        .args(["--list", "--null"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "git could not read the configuration",
        ));
    }
    Ok(parse_config_list(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(key: &str, value: &str) -> (String, String) {
        (key.to_string(), value.to_string())
    }

    #[test]
    fn every_credential_helper_setting_is_dropped() {
        let out = sanitize_gitconfig(&[
            e("credential.helper", "store"),
            e("credential.https://github.com.helper", "!x"),
            e("Credential.UseHttpPath", "true"),
            e("user.name", "Ada"),
        ]);
        assert!(
            !out.contains("helper") && !out.contains("usehttppath"),
            "{out}"
        );
        assert!(out.contains("name = \"Ada\""));
    }

    #[test]
    fn an_insteadof_with_userinfo_is_dropped_and_one_without_is_kept() {
        let entries = [
            e(
                "url.https://tok@github.com/.insteadof",
                "https://github.com/",
            ),
            e("url.https://u:p@host/.pushinsteadof", "https://host/"),
            e("url.git@github.com:.insteadof", "gh:"),
            e("url.https://github.com/.insteadof", "gh:"),
        ];
        let out = sanitize_gitconfig(&entries);
        assert!(!out.contains('@'), "{out}");
        assert!(out.contains("[url \"https://github.com/\"]"), "{out}");
        assert!(out.contains("insteadof = \"gh:\""), "{out}");
    }

    #[test]
    fn extra_headers_includes_and_command_hooks_are_dropped() {
        let out = sanitize_gitconfig(&[
            e("http.extraheader", "Authorization: bearer x"),
            e(
                "http.https://github.com/.extraHeader",
                "Authorization: bearer y",
            ),
            e("include.path", "~/secrets"),
            e("includeif.gitdir:~/w/.path", "~/w.cfg"),
            e("core.sshCommand", "ssh -i key"),
            e("core.askpass", "/bin/ask"),
            e("http.sslverify", "true"),
            e("core.editor", "vi"),
        ]);
        for gone in ["bearer", "secrets", "w.cfg", "ssh -i", "/bin/ask"] {
            assert!(!out.contains(gone), "{gone} in {out}");
        }
        assert!(out.contains("sslverify") && out.contains("editor"), "{out}");
    }

    #[test]
    fn a_url_value_with_userinfo_is_dropped_wherever_it_is_set() {
        let out = sanitize_gitconfig(&[
            e("remote.origin.url", "https://tok@github.com/o/r.git"),
            e("http.proxy", "http://u:p@proxy:3128"),
            e("sendemail.smtppass", "pw"),
            e("remote.up.url", "https://github.com/o/r.git"),
        ]);
        assert!(
            !out.contains("tok@") && !out.contains("u:p") && !out.contains("smtp"),
            "{out}"
        );
        assert!(out.contains("https://github.com/o/r.git"), "{out}");
    }

    #[test]
    fn everything_else_is_kept_in_order() {
        let out = sanitize_gitconfig(&[
            e("user.name", "Ada"),
            e("user.email", "ada@example.com"),
            e("core.editor", "vi"),
            e("alias.co", "checkout"),
        ]);
        assert_eq!(
            out,
            "[user]\n\tname = \"Ada\"\n\temail = \"ada@example.com\"\n[core]\n\teditor = \"vi\"\n[alias]\n\tco = \"checkout\"\n"
        );
    }

    #[test]
    fn a_value_with_quotes_and_newlines_survives_a_round_trip_through_git() {
        let text = "[user]\n\tname = \"A \\\"B\\\" \\\\ C\"\n[credential]\n\thelper = store\n[core]\n\tbare\n";
        let Ok(out) = sanitize_gitconfig_text(text) else {
            eprintln!("skipped: no git on this host");
            return;
        };
        assert!(!out.contains("helper"), "{out}");
        let again = sanitize_gitconfig_text(&out).expect("the output re-parses");
        assert_eq!(again, out);
        assert!(out.contains(r#"name = "A \"B\" \\ C""#), "{out}");
        assert!(out.contains(r#"bare = "true""#), "{out}");
    }

    #[test]
    fn the_list_output_is_split_on_nul_and_newline() {
        assert_eq!(
            parse_config_list("user.name\nAda\0core.bare\0a.b\nx\ny\0"),
            vec![
                e("user.name", "Ada"),
                e("core.bare", "true"),
                e("a.b", "x\ny")
            ]
        );
    }
}
