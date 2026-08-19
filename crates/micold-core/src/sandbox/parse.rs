//! PURE: the runtime's `--format '{{json .}}'` output → typed facts.
//!
//! Truncated or unexpected input is classified as a [`RuntimeError`], never unwrapped
//! (obligation C-6, conformance check K-12). That matters more here than it looks: this module
//! reads text produced by a program we do not control, on a machine we have never seen, and a
//! panic in it would take down the client at exactly the moment the user was told the sandbox was
//! starting.

use serde::Deserialize;

use super::runtime::{RuntimeError, RuntimeKind, RuntimeVersion};

/// What the runtime reported about itself.
#[derive(Debug, Deserialize)]
struct VersionDoc {
    #[serde(alias = "Server", alias = "server")]
    server: Option<VersionHalf>,
    #[serde(alias = "Client", alias = "client")]
    client: Option<VersionHalf>,
}

#[derive(Debug, Deserialize)]
struct VersionHalf {
    #[serde(alias = "Version", alias = "version")]
    version: Option<String>,
}

/// Parse `docker version --format '{{json .}}'` (or podman's equivalent).
///
/// Prefers the **server** version: the client can be newer than the daemon it talks to, and it is
/// the daemon that decides whether a flag is accepted.
pub fn version(kind: RuntimeKind, stdout: &str) -> Result<RuntimeVersion, RuntimeError> {
    let doc: VersionDoc = serde_json::from_str(stdout).map_err(|e| unparseable("version", &e))?;
    let version = doc
        .server
        .and_then(|s| s.version)
        .or_else(|| doc.client.and_then(|c| c.version))
        .ok_or_else(|| RuntimeError::Unknown {
            stderr: "the runtime reported no version".to_string(),
        })?;
    Ok(RuntimeVersion { kind, version })
}

/// What `docker info` says about the storage driver, which is what decides whether a writable
/// storage limit can be enforced at all (research R5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInfo {
    /// e.g. `overlayfs`, `overlay2`, `btrfs`.
    pub storage_driver: String,
    /// Whether the runtime reports itself as rootless. Podman usually is; Docker usually is not.
    pub rootless: bool,
}

#[derive(Debug, Deserialize)]
struct InfoDoc {
    #[serde(alias = "Driver")]
    driver: Option<String>,
    #[serde(alias = "store")]
    store: Option<PodmanStore>,
    #[serde(alias = "host")]
    host: Option<PodmanHost>,
}

#[derive(Debug, Deserialize)]
struct PodmanStore {
    #[serde(alias = "graphDriverName")]
    graph_driver_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PodmanHost {
    security: Option<PodmanSecurity>,
}

#[derive(Debug, Deserialize)]
struct PodmanSecurity {
    rootless: Option<bool>,
}

/// Parse `docker info --format '{{json .}}'` or `podman info --format json`.
///
/// The two runtimes disagree about the shape, which is precisely the kind of difference a dialect
/// is for — but the *reading* of it is one function, because the fact wanted is the same fact.
pub fn info(stdout: &str) -> Result<RuntimeInfo, RuntimeError> {
    let doc: InfoDoc = serde_json::from_str(stdout).map_err(|e| unparseable("info", &e))?;
    let storage_driver = doc
        .driver
        .or_else(|| doc.store.and_then(|s| s.graph_driver_name))
        .unwrap_or_default();
    let rootless = doc
        .host
        .and_then(|h| h.security)
        .and_then(|s| s.rootless)
        .unwrap_or(false);
    Ok(RuntimeInfo {
        storage_driver,
        rootless,
    })
}

/// What a container is doing now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerFacts {
    pub id: String,
    pub running: bool,
    /// The image the container was created from — how a sandbox left over from a previous version
    /// is recognised rather than attached to (US6 scenario 5).
    pub image: String,
    /// The build fingerprint label, when the image carries one (research R8).
    pub fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContainerDoc {
    #[serde(alias = "Id")]
    id: Option<String>,
    #[serde(alias = "State")]
    state: Option<StateDoc>,
    #[serde(alias = "Config")]
    config: Option<ConfigDoc>,
}

#[derive(Debug, Deserialize)]
struct StateDoc {
    #[serde(alias = "Running")]
    running: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ConfigDoc {
    #[serde(alias = "Image")]
    image: Option<String>,
    #[serde(alias = "Labels")]
    labels: Option<std::collections::BTreeMap<String, String>>,
}

/// The label the image carries its build fingerprint in.
pub const FINGERPRINT_LABEL: &str = "io.micold.fingerprint";

/// Parse a container inspection.
pub fn container(stdout: &str) -> Result<ContainerFacts, RuntimeError> {
    // `docker inspect` returns an array even for one target; accept both shapes rather than making
    // every caller remember which.
    let doc: ContainerDoc = one_of(stdout, "container")?;
    Ok(ContainerFacts {
        id: doc.id.unwrap_or_default(),
        running: doc.state.and_then(|s| s.running).unwrap_or(false),
        image: doc
            .config
            .as_ref()
            .and_then(|c| c.image.clone())
            .unwrap_or_default(),
        fingerprint: doc
            .config
            .and_then(|c| c.labels)
            .and_then(|l| l.get(FINGERPRINT_LABEL).cloned()),
    })
}

/// What an image is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageFacts {
    pub id: String,
    pub tags: Vec<String>,
    /// The build fingerprint, when present. Absent on an image built before this label existed,
    /// which is itself a reason to refuse a local build (research R8).
    pub fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageDoc {
    #[serde(alias = "Id")]
    id: Option<String>,
    #[serde(alias = "RepoTags")]
    repo_tags: Option<Vec<String>>,
    #[serde(alias = "Config")]
    config: Option<ConfigDoc>,
}

/// Parse an image inspection.
pub fn image(stdout: &str) -> Result<ImageFacts, RuntimeError> {
    let doc: ImageDoc = one_of(stdout, "image")?;
    Ok(ImageFacts {
        id: doc.id.unwrap_or_default(),
        tags: doc.repo_tags.unwrap_or_default(),
        fingerprint: doc
            .config
            .and_then(|c| c.labels)
            .and_then(|l| l.get(FINGERPRINT_LABEL).cloned()),
    })
}

/// Accept either a bare object or a one-element array of one.
fn one_of<T: for<'de> Deserialize<'de>>(stdout: &str, what: &str) -> Result<T, RuntimeError> {
    let trimmed = stdout.trim();
    if trimmed.starts_with('[') {
        let mut v: Vec<T> = serde_json::from_str(trimmed).map_err(|e| unparseable(what, &e))?;
        if v.is_empty() {
            return Err(RuntimeError::Unknown {
                stderr: format!("the runtime returned no {what}"),
            });
        }
        Ok(v.remove(0))
    } else {
        serde_json::from_str(trimmed).map_err(|e| unparseable(what, &e))
    }
}

fn unparseable(what: &str, e: &serde_json::Error) -> RuntimeError {
    RuntimeError::Unknown {
        stderr: format!("could not read the runtime's {what} output: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/runtime")
                .join(name),
        )
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
    }

    #[test]
    fn docker_version_reports_the_server_not_the_client() {
        let v = version(RuntimeKind::Docker, &fixture("docker_version.json")).unwrap();
        assert_eq!(v.version, "29.5.1");
        assert_eq!(v.kind, RuntimeKind::Docker);
    }

    #[test]
    fn podman_version_parses_from_its_own_shape() {
        let v = version(RuntimeKind::Podman, &fixture("podman_version.json")).unwrap();
        assert_eq!(v.version, "5.4.0");
    }

    #[test]
    fn the_storage_driver_is_read_from_either_runtimes_shape() {
        // The fact wanted is the same fact; only the spelling differs. This is what a single
        // parser buys over a per-runtime one.
        assert_eq!(
            info(&fixture("docker_info.json")).unwrap().storage_driver,
            "overlayfs"
        );
        let podman = info(&fixture("podman_info.json")).unwrap();
        assert_eq!(podman.storage_driver, "overlay");
        assert!(podman.rootless, "podman reports itself rootless");
    }

    #[test]
    fn a_container_inspection_yields_its_state_and_fingerprint() {
        let c = container(&fixture("docker_inspect_container.json")).unwrap();
        assert!(c.running);
        assert_eq!(c.image, "micold-daemon:dev");
        assert_eq!(c.fingerprint.as_deref(), Some("b7f3a1c9"));
    }

    #[test]
    fn an_image_inspection_yields_its_tags_and_fingerprint() {
        let i = image(&fixture("docker_inspect_image.json")).unwrap();
        assert_eq!(i.tags, vec!["micold-daemon:dev"]);
        assert_eq!(i.fingerprint.as_deref(), Some("b7f3a1c9"));
    }

    #[test]
    fn an_array_wrapped_inspection_parses_the_same_as_a_bare_object() {
        // `docker inspect` returns an array even for one target. Accepting both here means no
        // caller has to remember which, and a runtime that changes its mind does not break us.
        let bare = fixture("docker_inspect_container.json");
        let wrapped = format!("[{}]", bare.trim());
        assert_eq!(container(&bare).unwrap(), container(&wrapped).unwrap());
    }

    #[test]
    fn truncated_json_is_classified_never_panicked() {
        // Conformance check K-12. A panic here would take down the client at exactly the moment
        // the user was told the sandbox was starting.
        let truncated = fixture("err_truncated.json");
        for result in [
            container(&truncated).err(),
            image(&truncated).err(),
            info(&truncated).err(),
            version(RuntimeKind::Docker, &truncated).err(),
        ] {
            match result {
                Some(RuntimeError::Unknown { stderr }) => {
                    assert!(
                        !stderr.is_empty(),
                        "an unclassified error must keep its detail"
                    )
                }
                other => panic!("expected a classified error, got {other:?}"),
            }
        }
    }

    #[test]
    fn an_empty_inspection_array_is_an_error_not_a_default() {
        // "No such container" comes back as `[]`. Reading that as a stopped container with an
        // empty id would make the app attach to nothing and report success.
        assert!(container("[]").is_err());
        assert!(image("[]").is_err());
    }

    #[test]
    fn a_valid_document_missing_the_fingerprint_label_reports_none() {
        // An image built before the label existed. `None` is the honest answer, and it is what
        // makes a local build refusable rather than silently accepted (research R8).
        let doc = r#"{"Id":"sha256:x","RepoTags":["micold-daemon:dev"],"Config":{"Labels":{}}}"#;
        assert_eq!(image(doc).unwrap().fingerprint, None);
    }
}
