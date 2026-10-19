//! What a Kubernetes daemon may take from this machine (feature 686, FR-008). Each allowed entry
//! becomes exactly one secret and one mount at start (US3); nothing is copied unless listed.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::kube::classify::KubeFailure;
use crate::session::AiCli;

/// Where the allowed secrets that are not single files are mounted.
pub const CREDS_DIR: &str = "/run/micold/creds";

/// The daemon's `HOME` in the pod, on the volume.
pub const HOME_DIR: &str = "/data/home";

/// Where the daemon keeps its clones (research K8).
pub const REPOS_ROOT: &str = "/data/repos";

/// The `type` of a registry pull secret.
pub const DOCKER_CONFIG_TYPE: &str = "kubernetes.io/dockerconfigjson";

/// Reads a source file of a credential on this machine; the client passes its real reader (M5).
pub type ReadFile<'a> = &'a (dyn Fn(&Path) -> io::Result<String> + Send + Sync);

/// Where an allowed secret appears in the container, read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// The path in the container.
    pub path: String,
    /// The secret key mounted as that one file; `None` mounts the whole secret as a directory.
    pub sub_path: Option<String>,
}

/// One credential the user allowed a Kubernetes daemon to receive.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KubeCredential {
    /// `~/.gitconfig`.
    GitConfig,
    /// One AI CLI's sign-in.
    SignIn {
        /// Which CLI.
        cli: AiCli,
    },
    /// A registry `auths` file for pulling a private image.
    RegistryPull {
        /// The docker-config JSON on this machine.
        auths: PathBuf,
    },
}

impl KubeCredential {
    /// The credential as the form and the failures name it.
    pub fn label(&self) -> String {
        match self {
            Self::GitConfig => "the git config".into(),
            Self::SignIn { cli } => format!("the {} sign-in", cli.provider().display_name()),
            Self::RegistryPull { .. } => "the registry credentials".into(),
        }
    }

    /// The secret's `type`.
    pub fn secret_type(&self) -> &'static str {
        match self {
            Self::RegistryPull { .. } => DOCKER_CONFIG_TYPE,
            _ => "Opaque",
        }
    }

    /// The file of a sign-in inside the pod, and its name as the secret's key.
    fn sign_in_file(cli: AiCli) -> Option<(String, String)> {
        let path = cli.provider().sandbox_auth_file(Path::new(HOME_DIR))?;
        let name = path.file_name()?.to_string_lossy().into_owned();
        // The pod is Linux: the client's own separator (a backslash on Windows) must not reach it.
        Some((path.to_string_lossy().replace('\\', "/"), name))
    }

    /// Where the secret is mounted; `None` for a pull secret, which is no mount.
    pub fn mount(&self) -> Option<Mount> {
        let file = |path: String, key: &str| Mount {
            path,
            sub_path: Some(key.to_string()),
        };
        match self {
            Self::GitConfig => Some(file(format!("{HOME_DIR}/.gitconfig"), "gitconfig")),
            Self::SignIn { cli } => Self::sign_in_file(*cli).map(|(path, key)| file(path, &key)),
            Self::RegistryPull { .. } => None,
        }
    }

    /// The secret's text data, read from this machine through `read`. `home` is this machine's home
    /// directory.
    pub fn secret_data(
        &self,
        home: &Path,
        read: ReadFile<'_>,
    ) -> Result<BTreeMap<String, String>, KubeFailure> {
        let source = |path: &Path| {
            read(path).map_err(|_| KubeFailure::CredentialSource {
                credential: self.label(),
                path: path.to_path_buf(),
            })
        };
        let one = |key: &str, text: String| BTreeMap::from([(key.to_string(), text)]);
        match self {
            Self::GitConfig => {
                let path = home.join(".gitconfig");
                // The shared copy carries no credential helper, embedded token, extra header or
                // include (FR-004). A host without `git` cannot be sanitized, so shares nothing.
                let text = source(&path)?;
                let sanitized =
                    crate::scoped_credential::sanitize_gitconfig_text(&text).map_err(|_| {
                        KubeFailure::CredentialSource {
                            credential: self.label(),
                            path: path.clone(),
                        }
                    })?;
                Ok(one("gitconfig", sanitized))
            }
            Self::SignIn { cli } => {
                let (_, key) =
                    Self::sign_in_file(*cli).ok_or_else(|| KubeFailure::CredentialSource {
                        credential: self.label(),
                        path: home.to_path_buf(),
                    })?;
                let path = cli.provider().sandbox_auth_file(home).unwrap_or_default();
                Ok(one(&key, source(&path)?))
            }
            Self::RegistryPull { auths } => Ok(one(".dockerconfigjson", source(auths)?)),
        }
    }
}

/// One of each kind of credential, to find the secrets a withdrawn one left behind (the names do
/// not depend on the payload).
pub fn every_kind() -> Vec<KubeCredential> {
    let mut all = vec![
        KubeCredential::GitConfig,
        KubeCredential::RegistryPull {
            auths: PathBuf::new(),
        },
    ];
    all.extend(AiCli::ALL.map(|cli| KubeCredential::SignIn { cli }));
    all
}

/// The environment the daemon runs with (the pod's own env does not reach it: sshd starts sessions
/// clean). No allowed credential adds to it.
pub fn daemon_env() -> BTreeMap<String, String> {
    // Nothing allowed reaches the env since #1002 removed the git credential store and the SSH key
    // (FR-001): what remains is mounted as files where the tools look for them. Git is told to
    // ignore every credential helper the shared configuration might still name (FR-004).
    let mut env = BTreeMap::from([
        ("MICOLD_IDLE_STOP".to_string(), "off".to_string()),
        ("MICOLD_REPOS_ROOT".to_string(), REPOS_ROOT.to_string()),
    ]);
    // The pod's daemon is not on `PATH`: name it by the build directory the client installs it in
    // (quoted so the shell git runs the helper through expands `$HOME`).
    let daemon = format!(
        "\"{}/micold-daemon\"",
        crate::ssh::argv::remote_dir(&crate::ssh::argv::current_build_id())
    );
    env.extend(crate::scoped_credential::git_env_with_helper(
        crate::scoped_credential::GITHUB_BASE,
        &daemon,
    ));
    env
}

/// A credential a stored `allowed` list may name: one of today's, or one #1002 removed (the git
/// credential store and an SSH key, FR-002). Read, never written.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum RawKubeCredential {
    Kept(KubeCredential),
    // Read only to recognise the removed names; what it held is dropped.
    Removed(#[allow(dead_code)] RemovedKubeCredential),
}

/// The items #1002 removed: they carried the user's whole git identity into the cluster.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum RemovedKubeCredential {
    GitCredentials,
    GitSshKey {},
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::names::ResourceNames;

    fn names() -> ResourceNames {
        ResourceNames::new("kind-dev", "dev", "Build box")
    }

    fn reader(files: &[(&str, &str)]) -> impl Fn(&Path) -> io::Result<String> + Send + Sync {
        let files: BTreeMap<PathBuf, String> = files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect();
        move |path| {
            files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }

    fn mount(path: &str, sub: Option<&str>) -> Option<Mount> {
        Some(Mount {
            path: path.into(),
            sub_path: sub.map(str::to_string),
        })
    }

    #[test]
    fn each_credential_has_one_secret_and_one_mount() {
        let claude = KubeCredential::SignIn {
            cli: AiCli::ClaudeCode,
        };
        assert_eq!(
            KubeCredential::GitConfig.mount(),
            mount("/data/home/.gitconfig", Some("gitconfig"))
        );
        assert_eq!(
            claude.mount(),
            mount(
                "/data/home/.claude/.credentials.json",
                Some(".credentials.json")
            )
        );
        let pull = KubeCredential::RegistryPull { auths: "/a".into() };
        assert_eq!(pull.mount(), None);
        assert_eq!(pull.secret_type(), "kubernetes.io/dockerconfigjson");
        assert_eq!(names().secret(&pull), format!("{}-pull", names().pod()));
        assert_eq!(claude.secret_type(), "Opaque");
    }

    #[test]
    fn secret_data_is_read_from_the_sources() {
        let read = reader(&[
            (
                "/h/.gitconfig",
                "[user]\n\tname = Ada\n[credential]\n\thelper = store\n",
            ),
            ("/h/.claude/.credentials.json", "{}"),
            ("/a", "{\"auths\":{}}"),
        ]);
        let data = |c: KubeCredential| c.secret_data(Path::new("/h"), &read).unwrap();
        let one = |k: &str, v: &str| BTreeMap::from([(k.to_string(), v.to_string())]);
        assert_eq!(
            data(KubeCredential::GitConfig),
            one("gitconfig", "[user]\n\tname = \"Ada\"\n")
        );
        assert_eq!(
            data(KubeCredential::SignIn {
                cli: AiCli::ClaudeCode
            }),
            one(".credentials.json", "{}")
        );
        assert_eq!(
            data(KubeCredential::RegistryPull { auths: "/a".into() }),
            one(".dockerconfigjson", "{\"auths\":{}}")
        );
    }

    #[test]
    fn a_missing_source_names_the_credential_and_the_path() {
        let read = reader(&[]);
        let failure = KubeCredential::GitConfig
            .secret_data(Path::new("/h"), &read)
            .unwrap_err();
        assert_eq!(
            failure,
            KubeFailure::CredentialSource {
                credential: "the git config".into(),
                path: "/h/.gitconfig".into()
            }
        );
        let no_file = KubeCredential::SignIn {
            cli: AiCli::Copilot,
        };
        assert!(no_file.mount().is_none());
        assert!(matches!(
            no_file.secret_data(Path::new("/h"), &read),
            Err(KubeFailure::CredentialSource { .. })
        ));
    }

    /// FR-001: no kind carries the user's git identity — no agent, no credential store, no key.
    #[test]
    fn no_credential_carries_the_users_git_identity() {
        let json = serde_json::to_string(&every_kind()).unwrap();
        for retired in ["agent", "git_credentials", "ssh_key"] {
            assert!(!json.contains(retired), "{retired} in {json}");
        }
        assert_eq!(every_kind().len(), 2 + AiCli::ALL.len());
    }

    #[test]
    fn the_daemon_env_carries_no_credential_and_resets_the_helpers() {
        let env = daemon_env();
        assert_eq!(env["MICOLD_IDLE_STOP"], "off");
        assert_eq!(env["MICOLD_REPOS_ROOT"], "/data/repos");
        for (key, value) in crate::scoped_credential::git_env("https://github.com") {
            if key == "GIT_CONFIG_VALUE_1" {
                // The pod's daemon is named by its build directory, not found on PATH.
                assert!(
                    env[&key].starts_with("!\"$HOME/.micold/remote/")
                        && env[&key].ends_with("/micold-daemon\" git-credential"),
                    "{key}: {}",
                    env[&key]
                );
                continue;
            }
            assert_eq!(env[&key], value, "{key}");
        }
        assert!(!env.keys().any(|k| k.contains("SSH") || k.contains("TOKEN")));
    }
}
