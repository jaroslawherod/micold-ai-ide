//! The AI CLI install step's data and scripts (feature 806, FR-011..FR-013, contracts/provisioning.md
//! *Install script*): the pinned package table, the fixed scripts that install into the target's
//! state directory, and the `offered.json` file the daemon reads.
//!
//! Every byte of a script comes from the pinned table or the state directory the runtime chose,
//! never from user text, and no script names a credential path (FR-013): sign-ins reach a target
//! only through the credential shares the user allowed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::HostPort;
use crate::session::AiCli;
use crate::ssh::argv::RELAY_REMOTE_PORT;

/// Where an AI CLI package comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The npm registry, at this exact version.
    Registry {
        /// The pinned version.
        version: String,
    },
    /// A local tarball (the package field is its path): tests install a stand-in with no network.
    Tarball,
}

/// One AI CLI's pinned package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCliRow {
    /// The CLI.
    pub cli: AiCli,
    /// The npm package, or a tarball path for [`Source::Tarball`].
    pub package: String,
    /// Where it comes from.
    pub source: Source,
}

impl AiCliRow {
    /// The directory name under `<state>/ai-clis`: `<cli>@<version>` (`@local` for a tarball).
    pub fn dir_name(&self) -> String {
        let version = match &self.source {
            Source::Registry { version } => version.as_str(),
            Source::Tarball => "local",
        };
        format!("{:?}@{version}", self.cli)
    }

    /// What `npm install` is given: `<package>@<version>`, or the tarball path.
    fn spec(&self) -> String {
        match &self.source {
            Source::Registry { version } => format!("{}@{version}", self.package),
            Source::Tarball => self.package.clone(),
        }
    }
}

/// The pinned table of AI CLI packages: one row per supported CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCliTable {
    rows: Vec<AiCliRow>,
}

/// The registry host the pinned packages come from, as the relay allows it.
const REGISTRY: (&str, u16) = ("registry.npmjs.org", 443);

impl AiCliTable {
    /// The table this application release pins. The only table production code builds: the client
    /// passes it, and no setting or user text can change a row. Versions are those the sandbox
    /// image pins (`packaging/sandbox/Containerfile`).
    pub fn pinned() -> Self {
        let row = |cli, package: &str, version: &str| AiCliRow {
            cli,
            package: package.to_string(),
            source: Source::Registry {
                version: version.to_string(),
            },
        };
        Self {
            rows: vec![
                row(AiCli::ClaudeCode, "@anthropic-ai/claude-code", "2.1.247"),
                row(AiCli::Copilot, "@github/copilot", "1.0.80"),
                row(AiCli::Pi, "@earendil-works/pi-coding-agent", "0.85.1"),
                row(AiCli::Codex, "@openai/codex", "0.162.0"),
                row(AiCli::OpenCode, "opencode-ai", "1.18.35"),
            ],
        }
    }

    /// A table whose rows point at local tarballs (tests only: real-runtime suites install a
    /// stand-in with no network, FR-025).
    pub fn from_tarballs(rows: impl IntoIterator<Item = (AiCli, PathBuf)>) -> Self {
        Self {
            rows: rows
                .into_iter()
                .map(|(cli, tarball)| AiCliRow {
                    cli,
                    package: tarball.display().to_string(),
                    source: Source::Tarball,
                })
                .collect(),
        }
    }

    /// Every row.
    pub fn rows(&self) -> &[AiCliRow] {
        &self.rows
    }

    /// The row for `cli`.
    pub fn row(&self, cli: AiCli) -> Option<&AiCliRow> {
        self.rows.iter().find(|r| r.cli == cli)
    }

    /// The destinations an install relay may reach: the registry when any row comes from it.
    pub fn allow(&self) -> Vec<HostPort> {
        if self
            .rows
            .iter()
            .any(|r| matches!(r.source, Source::Registry { .. }))
        {
            vec![HostPort {
                host: REGISTRY.0.to_string(),
                port: REGISTRY.1,
            }]
        } else {
            Vec::new()
        }
    }
}

/// `s` as one single-quoted shell word.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn ai_clis_dir(state: &Path) -> String {
    format!("{}/ai-clis", state.display())
}

/// The marker a check prints per installed CLI.
const HAVE: &str = "MICOLD_HAVE_CLI";

/// The check script: prints `MICOLD_HAVE_CLI <cli>` for each of `rows` already installed.
pub fn check_script(rows: &[&AiCliRow], state: &Path) -> String {
    let dir = ai_clis_dir(state);
    let mut script = String::new();
    for row in rows {
        script.push_str(&format!(
            "test -d {} && echo \"{HAVE} {:?}\"\n",
            quote(&format!("{dir}/{}", row.dir_name())),
            row.cli
        ));
    }
    script.push_str("true\n");
    script
}

/// The CLIs a check reported as installed.
pub fn parse_installed(stdout: &str) -> BTreeSet<AiCli> {
    stdout
        .lines()
        .filter_map(|l| l.trim().strip_prefix(HAVE))
        .filter_map(|name| cli_named(name.trim()))
        .collect()
}

fn cli_named(name: &str) -> Option<AiCli> {
    AiCli::ALL.into_iter().find(|c| format!("{c:?}") == name)
}

/// The text printed when the target has no npm.
pub const NO_NPM: &str = "the image has no npm (Node.js), which this AI CLI's installer needs";

/// The install script for `missing`, over the `selected` rows. `proxy` points npm at the relay's
/// remote end (`Egress::Relay`). Runs on stdin of `sh -s`, under `flock` where there is one, and
/// ends by writing `offered.json` (selected ∩ installed) atomically.
pub fn install_script(
    selected: &[&AiCliRow],
    missing: &[&AiCliRow],
    state: &Path,
    proxy: bool,
) -> String {
    let dir = ai_clis_dir(state);
    let mut s = String::new();
    s.push_str(&format!("mkdir -p {} || exit 1\n", quote(&dir)));
    s.push_str(&format!(
        "if command -v flock >/dev/null 2>&1; then exec 9>{}; flock 9; fi\n",
        quote(&format!("{dir}/.lock"))
    ));
    if proxy {
        let url = format!("http://127.0.0.1:{RELAY_REMOTE_PORT}");
        s.push_str(&format!(
            "export HTTPS_PROXY={url}\nexport npm_config_https_proxy={url}\n"
        ));
    }
    s.push_str("if command -v npm >/dev/null 2>&1; then have_npm=1; else have_npm=0; fi\n");
    for row in missing {
        let dest = quote(&format!("{dir}/{}", row.dir_name()));
        let part = quote(&format!("{dir}/{}.part", row.dir_name()));
        let cli = format!("{:?}", row.cli);
        s.push_str(&format!(
            "if [ -d {dest} ]; then echo \"MICOLD_INSTALL {cli} ok\"\n\
             elif [ \"$have_npm\" = 0 ]; then echo {no_npm}; echo \"MICOLD_INSTALL {cli} failed\"\n\
             else\n\
             \x20 rm -rf {part}\n\
             \x20 if npm install -g --prefix {part} {spec} 2>&1 && mv {part} {dest}; then echo \"MICOLD_INSTALL {cli} ok\"\n\
             \x20 else rm -rf {part}; echo \"MICOLD_INSTALL {cli} failed\"; fi\n\
             fi\n",
            no_npm = quote(NO_NPM),
            spec = quote(&row.spec()),
        ));
    }
    let file = quote(&format!("{dir}/offered.json"));
    let tmp = quote(&format!("{dir}/offered.json.part"));
    s.push_str(&format!("{{ printf '{{\"version\":1,\"clis\":['; sep=\n"));
    for row in selected {
        let bin = format!("{dir}/{}/bin", row.dir_name());
        s.push_str(&format!(
            "  if [ -d {q} ]; then printf '%s{{\"cli\":\"{cli:?}\",\"bin\":\"%s\"}}' \"$sep\" {q}; sep=,; fi\n",
            q = quote(&bin),
            cli = row.cli
        ));
    }
    s.push_str(&format!(
        "  printf ']}}\\n'; }} > {tmp} && mv {tmp} {file}\n"
    ));
    s
}

/// One CLI's install outcome as the script reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallResult {
    /// The CLI.
    pub cli: AiCli,
    /// Whether it is installed.
    pub ok: bool,
    /// The installer's output (the last 40 lines before the marker).
    pub output: String,
}

/// Read the `MICOLD_INSTALL <cli> ok|failed` markers: the lines before each marker are that CLI's
/// installer output.
pub fn parse_install_output(stdout: &str) -> Vec<InstallResult> {
    let mut results = Vec::new();
    let mut lines: Vec<&str> = Vec::new();
    for line in stdout.lines() {
        let marker = line
            .trim()
            .strip_prefix("MICOLD_INSTALL ")
            .and_then(|rest| rest.split_once(' '))
            .and_then(|(name, status)| Some((cli_named(name)?, status.trim())))
            .filter(|(_, status)| matches!(*status, "ok" | "failed"));
        match marker {
            Some((cli, status)) => {
                let from = lines.len().saturating_sub(40);
                results.push(InstallResult {
                    cli,
                    ok: status == "ok",
                    output: lines[from..].join("\n"),
                });
                lines.clear();
            }
            None => lines.push(line),
        }
    }
    results
}

/// `<state>/ai-clis/offered.json`: which CLIs the daemon offers and where their binaries are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offered {
    /// The file format's version.
    pub version: u32,
    /// The installed CLIs.
    pub clis: Vec<OfferedCli>,
}

/// One entry of [`Offered`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfferedCli {
    /// The CLI.
    pub cli: AiCli,
    /// The directory holding its executable.
    pub bin: PathBuf,
}

/// Read `offered.json` from the `ai-clis` directory `dir`: `Ok(None)` when there is none, `Err`
/// (what to log) when it cannot be read or is malformed. The daemon ignores an `Err` and behaves
/// as it does without the file.
pub fn read_offered(dir: &Path) -> Result<Option<Offered>, String> {
    let text = match std::fs::read_to_string(dir.join("offered.json")) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("unreadable: {e}")),
    };
    match serde_json::from_str::<Offered>(&text) {
        Ok(offered) if offered.version == 1 => Ok(Some(offered)),
        Ok(offered) => Err(format!("unknown version {}", offered.version)),
        Err(e) => Err(format!("malformed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(table: &AiCliTable) -> Vec<&AiCliRow> {
        table.rows().iter().collect()
    }

    fn state() -> &'static Path {
        Path::new("/var/lib/micold-ai-ide")
    }

    #[test]
    fn the_pinned_table_has_one_row_per_cli_with_the_sandbox_images_versions() {
        let t = AiCliTable::pinned();
        let clis: Vec<_> = t.rows().iter().map(|r| r.cli).collect();
        assert_eq!(clis, AiCli::ALL);
        let want = [
            ("@anthropic-ai/claude-code", "2.1.247"),
            ("@github/copilot", "1.0.80"),
            ("@earendil-works/pi-coding-agent", "0.85.1"),
            ("@openai/codex", "0.162.0"),
            ("opencode-ai", "1.18.35"),
        ];
        for (row, (package, version)) in t.rows().iter().zip(want) {
            assert_eq!(row.package, package);
            assert_eq!(
                row.source,
                Source::Registry {
                    version: version.into()
                }
            );
        }
        assert_eq!(
            t.allow(),
            [HostPort {
                host: "registry.npmjs.org".into(),
                port: 443
            }]
        );
    }

    #[test]
    fn the_install_script_installs_each_missing_cli_into_a_part_directory_then_renames_it() {
        let t = AiCliTable::pinned();
        let rows = all(&t);
        let missing = [rows[0], rows[3]];
        let s = install_script(&rows, &missing, state(), false);
        let dir = "/var/lib/micold-ai-ide/ai-clis";
        for (name, package) in [
            ("ClaudeCode@2.1.247", "@anthropic-ai/claude-code@2.1.247"),
            ("Codex@0.162.0", "@openai/codex@0.162.0"),
        ] {
            assert!(
                s.contains(&format!(
                    "npm install -g --prefix '{dir}/{name}.part' '{package}'"
                )),
                "{s}"
            );
            assert!(s.contains(&format!("mv '{dir}/{name}.part' '{dir}/{name}'")));
            assert!(s.contains(&format!("rm -rf '{dir}/{name}.part'")));
        }
        assert!(s.contains("MICOLD_INSTALL ClaudeCode failed"));
        assert!(
            !s.contains("--prefix '/var/lib/micold-ai-ide/ai-clis/Copilot@"),
            "only the missing ones are installed"
        );
        assert!(s.contains(&format!("exec 9>'{dir}/.lock'; flock 9")));
        assert!(!s.contains("HTTPS_PROXY"), "Direct egress sets no proxy");
    }

    #[test]
    fn the_script_writes_offered_json_atomically_for_the_selected_cli_that_are_installed() {
        let t = AiCliTable::pinned();
        let rows = all(&t);
        let s = install_script(&rows[..2], &[rows[0]], state(), false);
        let dir = "/var/lib/micold-ai-ide/ai-clis";
        assert!(s.contains("{\"version\":1,\"clis\":["), "{s}");
        assert!(s.contains(&format!("-d '{dir}/ClaudeCode@2.1.247/bin'")));
        assert!(s.contains(&format!("-d '{dir}/Copilot@1.0.80/bin'")));
        assert!(!s.contains("Pi@"), "deselected CLIs are not offered");
        assert!(s.contains(&format!(
            "> '{dir}/offered.json.part' && mv '{dir}/offered.json.part' '{dir}/offered.json'"
        )));
    }

    #[test]
    fn a_missing_npm_fails_each_cli_with_the_images_sentence() {
        let t = AiCliTable::pinned();
        let rows = all(&t);
        let s = install_script(&rows, &rows, state(), false);
        assert!(s.contains("command -v npm"));
        assert_eq!(s.matches(&quote(NO_NPM)).count(), 5, "one per CLI");
        assert_eq!(
            NO_NPM,
            "the image has no npm (Node.js), which this AI CLI's installer needs"
        );
    }

    #[test]
    fn only_a_relay_install_exports_the_proxy_and_it_is_the_remote_end_of_the_forward() {
        let t = AiCliTable::pinned();
        let rows = all(&t);
        let s = install_script(&rows, &rows, state(), true);
        assert!(s.contains("export HTTPS_PROXY=http://127.0.0.1:17728\n"));
        assert!(s.contains("export npm_config_https_proxy=http://127.0.0.1:17728\n"));
    }

    #[test]
    fn no_script_names_a_credential_path_or_carries_user_text() {
        let t = AiCliTable::pinned();
        let rows = all(&t);
        let s = install_script(&rows, &rows, state(), true) + &check_script(&rows, state());
        for credential in [
            ".ssh", ".claude", ".codex", ".config", "token", "id_", "$HOME",
        ] {
            assert!(!s.contains(credential), "{credential} in the script");
        }
    }

    /// FR-025: a tarball row installs the package as given, no version suffix, with nothing to
    /// allow through a relay.
    #[test]
    fn a_tarball_row_installs_the_tarball_with_no_version_and_allows_nothing() {
        let t = AiCliTable::from_tarballs([(AiCli::Pi, PathBuf::from("/tmp/standin.tgz"))]);
        assert!(t.allow().is_empty());
        let rows = all(&t);
        let s = install_script(&rows, &rows, state(), false);
        assert!(
            s.contains(
                "npm install -g --prefix '/var/lib/micold-ai-ide/ai-clis/Pi@local.part' '/tmp/standin.tgz'"
            ),
            "{s}"
        );
    }

    #[test]
    fn a_path_with_a_quote_cannot_end_its_shell_word() {
        let t = AiCliTable::from_tarballs([(AiCli::Pi, PathBuf::from("/tmp/it's.tgz"))]);
        let rows = all(&t);
        let s = install_script(&rows, &rows, state(), false);
        assert!(s.contains(r"'/tmp/it'\''s.tgz'"));
    }

    #[test]
    fn the_check_names_the_installed_ones_and_the_output_parser_splits_per_cli() {
        let t = AiCliTable::pinned();
        let rows = all(&t);
        let c = check_script(&rows[..2], state());
        assert!(c.contains("test -d '/var/lib/micold-ai-ide/ai-clis/ClaudeCode@2.1.247'"));
        assert_eq!(
            parse_installed("MICOLD_HAVE_CLI Pi\nnoise\nMICOLD_HAVE_CLI Nope\n"),
            BTreeSet::from([AiCli::Pi])
        );
        let out = "npm ERR! 404\nMICOLD_INSTALL ClaudeCode failed\nadded 1 package\nMICOLD_INSTALL Pi ok\n";
        assert_eq!(
            parse_install_output(out),
            [
                InstallResult {
                    cli: AiCli::ClaudeCode,
                    ok: false,
                    output: "npm ERR! 404".into()
                },
                InstallResult {
                    cli: AiCli::Pi,
                    ok: true,
                    output: "added 1 package".into()
                },
            ]
        );
    }

    #[test]
    fn offered_json_is_read_when_valid_and_ignored_when_missing_or_malformed() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_offered(dir.path()), Ok(None));
        std::fs::write(dir.path().join("offered.json"), "{nope").unwrap();
        assert!(read_offered(dir.path()).is_err());
        std::fs::write(
            dir.path().join("offered.json"),
            r#"{"version":1,"clis":[{"cli":"ClaudeCode","bin":"/x/bin"}]}"#,
        )
        .unwrap();
        assert_eq!(
            read_offered(dir.path()).unwrap().unwrap().clis,
            [OfferedCli {
                cli: AiCli::ClaudeCode,
                bin: "/x/bin".into()
            }]
        );
    }

    /// The script run by a real `sh` with a stand-in `npm`: the good package lands in its final
    /// directory, the bad one leaves no `.part`, `offered.json` lists only what installed, and a
    /// second run installs nothing.
    #[cfg(unix)]
    #[test]
    fn the_script_installs_what_npm_installs_and_offers_only_that() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("fake-bin");
        std::fs::create_dir(&fake).unwrap();
        let npm = fake.join("npm");
        std::fs::write(
            &npm,
            "#!/bin/sh\n# npm install -g --prefix <dir> <spec>\ncase \"$5\" in *bad*) echo 'npm ERR! boom'; exit 1;; esac\nmkdir -p \"$4/bin\" && : > \"$4/bin/cli\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&npm, std::fs::Permissions::from_mode(0o755)).unwrap();
        let state = tmp.path().join("state");
        let t = AiCliTable::from_tarballs([
            (AiCli::ClaudeCode, PathBuf::from("/x/good.tgz")),
            (AiCli::Pi, PathBuf::from("/x/bad.tgz")),
        ]);
        let rows = all(&t);
        let run = |missing: &[&AiCliRow]| {
            let script = install_script(&rows, missing, &state, false);
            let mut child = Command::new("sh")
                .arg("-s")
                .env("PATH", format!("{}:/usr/bin:/bin", fake.display()))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
            let out = child.wait_with_output().unwrap();
            assert!(out.status.success());
            String::from_utf8(out.stdout).unwrap()
        };
        let out = run(&rows);
        let results = parse_install_output(&out);
        assert_eq!(
            results.iter().map(|r| (r.cli, r.ok)).collect::<Vec<_>>(),
            [(AiCli::ClaudeCode, true), (AiCli::Pi, false)]
        );
        assert_eq!(results[1].output, "npm ERR! boom");
        let ai = state.join("ai-clis");
        assert!(ai.join("ClaudeCode@local/bin/cli").exists());
        assert!(!ai.join("Pi@local").exists() && !ai.join("Pi@local.part").exists());
        assert!(!ai.join("ClaudeCode@local.part").exists());
        let offered = read_offered(&ai).unwrap().unwrap();
        assert_eq!(
            offered.clis,
            [OfferedCli {
                cli: AiCli::ClaudeCode,
                bin: ai.join("ClaudeCode@local/bin")
            }]
        );
        // The check now finds the good one and not the bad one.
        let check = check_script(&rows, &state);
        let found = Command::new("sh")
            .args(["-c", &check])
            .output()
            .unwrap()
            .stdout;
        assert_eq!(
            parse_installed(&String::from_utf8(found).unwrap()),
            BTreeSet::from([AiCli::ClaudeCode])
        );
    }
}
